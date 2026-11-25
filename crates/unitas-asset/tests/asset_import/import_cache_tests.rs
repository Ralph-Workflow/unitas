use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread::ThreadId;
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::dependencies::{ImportCandidate, ImportOutcome, ImportSession};
use unitas_asset::import_cache::{ImportCache, LIBRARY_DIRECTORY};
use unitas_asset::json::{self, JsonValue};

type TestResult = Result<(), Box<dyn Error>>;

const TEXTURE_GUID: &str = "11111111111111111111111111111111";
const MATERIAL_GUID: &str = "22222222222222222222222222222222";
const UNRELATED_GUID: &str = "33333333333333333333333333333333";

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("unitas-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn candidate(path: &str, guid: &str, source: &[u8]) -> ImportCandidate {
    ImportCandidate {
        path: PathBuf::from(path),
        guid: guid.to_string(),
        source: source.to_vec(),
        metadata: String::new(),
        settings: "TextureImporter".to_string(),
        importer_version: "1".to_string(),
        dependencies: Vec::new(),
    }
}

fn dependent(path: &str, guid: &str, source: &[u8], dependency: &str) -> ImportCandidate {
    ImportCandidate {
        dependencies: vec![dependency.to_string()],
        ..candidate(path, guid, source)
    }
}

fn texture_project() -> Vec<ImportCandidate> {
    vec![
        candidate("Assets/Textures/Albedo.png", TEXTURE_GUID, b"albedo-bytes"),
        dependent(
            "Assets/Materials/Stone.mat",
            MATERIAL_GUID,
            b"stone-bytes",
            TEXTURE_GUID,
        ),
        candidate(
            "Assets/Textures/Unrelated.png",
            UNRELATED_GUID,
            b"unrelated-bytes",
        ),
    ]
}

fn cache_file(library: &Path) -> PathBuf {
    ImportCache::open(library).file_path()
}

fn cache_assets(library: &Path) -> Result<JsonValue, Box<dyn Error>> {
    let text = fs::read_to_string(cache_file(library))?;
    let value = json::parse(&text)?;
    Ok(value)
}

fn entry_for<'a>(value: &'a JsonValue, path: &str) -> Option<&'a JsonValue> {
    value
        .get("assets")?
        .as_array()?
        .iter()
        .find(|asset| asset.get("path").and_then(JsonValue::as_str) == Some(path))
}

#[test]
fn import_cache_persists_across_sessions_and_reimports_nothing_when_the_sources_are_unchanged(
) -> TestResult {
    let root = temp_dir("import-cache-persistence")?;
    let library = root.join("project").join(LIBRARY_DIRECTORY);
    let candidates = texture_project();
    let calls = AtomicUsize::new(0);
    let mut importer = |_: &ImportCandidate| {
        calls.fetch_add(1, Ordering::SeqCst);
        ImportOutcome::ready()
    };

    let mut first = ImportSession::with_cache(ImportCache::open(&library));
    let report = first.run(&candidates, &mut importer);
    assert_eq!(report.imported, 3, "every asset imports the first time");
    assert_eq!(report.reused, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert!(
        cache_file(&library).is_file(),
        "the cache is written under the project Library directory"
    );
    assert!(first.cache_error().is_none());

    let value = cache_assets(&library)?;
    assert_eq!(
        value.get("format").and_then(JsonValue::as_str),
        Some("unitas-import-cache")
    );
    assert_eq!(
        value.get("formatVersion").and_then(JsonValue::as_i64),
        Some(1)
    );
    let entry = entry_for(&value, "Assets/Textures/Albedo.png")
        .ok_or("expected the cache to carry the imported texture")?;
    assert!(
        entry.get("importerVersion").is_some(),
        "the cache format carries the importer version: {entry:?}"
    );
    assert!(entry.get("source").is_some());

    let mut second = ImportSession::with_cache(ImportCache::open(&library));
    let report = second.run(&candidates, &mut importer);
    assert_eq!(
        report.imported, 0,
        "a fresh session over unchanged sources reimports nothing"
    );
    assert_eq!(report.reused, 3, "every asset keeps its recorded result");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        3,
        "the importer is not called again for unchanged sources"
    );
    assert!(second.cache_error().is_none());
    assert_eq!(
        second.cache().map(|cache| cache.len()),
        Some(3),
        "the session keeps the cache it owns"
    );
    let reopened = ImportCache::open(&library);
    assert_eq!(reopened.len(), 3);
    assert!(reopened
        .record(Path::new("Assets/Textures/Albedo.png"))
        .is_some());

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn import_cache_reimports_only_the_changed_asset_and_its_recorded_dependents() -> TestResult {
    let root = temp_dir("import-cache-change")?;
    let library = root.join("project").join(LIBRARY_DIRECTORY);
    let mut importer = |_: &ImportCandidate| ImportOutcome::ready();

    let mut first = ImportSession::with_cache(ImportCache::open(&library));
    let report = first.run(&texture_project(), &mut importer);
    assert_eq!(report.imported, 3);

    let mut changed = texture_project();
    changed[0].source = b"changed-albedo-bytes".to_vec();
    let mut second = ImportSession::with_cache(ImportCache::open(&library));
    let report = second.run(&changed, &mut importer);
    assert_eq!(
        report.imported, 2,
        "only the changed asset and its dependent are imported again"
    );
    assert_eq!(report.reused, 1, "the unrelated asset keeps its result");
    let changed_record = report
        .record(Path::new("Assets/Textures/Albedo.png"))
        .ok_or("expected a record for the changed texture")?;
    assert!(!changed_record.reused);
    assert_eq!(changed_record.reason, "source content changed");
    let dependent_record = report
        .record(Path::new("Assets/Materials/Stone.mat"))
        .ok_or("expected a record for the dependent material")?;
    assert!(!dependent_record.reused);
    assert_eq!(
        dependent_record.reason,
        format!("dependency changed: {TEXTURE_GUID}")
    );
    let unrelated_record = report
        .record(Path::new("Assets/Textures/Unrelated.png"))
        .ok_or("expected a record for the unrelated texture")?;
    assert!(unrelated_record.reused);

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn import_cache_reimports_when_the_metadata_the_settings_or_the_importer_version_change(
) -> TestResult {
    let root = temp_dir("import-cache-invalidation")?;
    let library = root.join("project").join(LIBRARY_DIRECTORY);
    let mut importer = |_: &ImportCandidate| ImportOutcome::ready();

    let mut first = ImportSession::with_cache(ImportCache::open(&library));
    first.run(&texture_project(), &mut importer);

    let mut metadata_changed = texture_project();
    metadata_changed[0].metadata = "sRGBTexture: 0".to_string();
    let mut second = ImportSession::with_cache(ImportCache::open(&library));
    let report = second.run(&metadata_changed, &mut importer);
    assert_eq!(
        report.imported, 2,
        "the metadata change reimports the asset"
    );
    let record = report
        .record(Path::new("Assets/Textures/Albedo.png"))
        .ok_or("expected a record for the texture")?;
    assert_eq!(record.reason, "metadata changed");

    let mut settings_changed = texture_project();
    settings_changed[0].metadata = "sRGBTexture: 0".to_string();
    settings_changed[0].settings = "TextureImporter: maxSize 2048".to_string();
    let mut third = ImportSession::with_cache(ImportCache::open(&library));
    let report = third.run(&settings_changed, &mut importer);
    assert_eq!(
        report.imported, 2,
        "the settings change reimports the asset"
    );
    let record = report
        .record(Path::new("Assets/Textures/Albedo.png"))
        .ok_or("expected a record for the texture")?;
    assert_eq!(record.reason, "import settings changed");

    let mut version_changed = settings_changed.clone();
    version_changed[0].importer_version = "2".to_string();
    let mut fourth = ImportSession::with_cache(ImportCache::open(&library));
    let report = fourth.run(&version_changed, &mut importer);
    assert_eq!(
        report.imported, 2,
        "the importer version bump reimports the asset"
    );
    let record = report
        .record(Path::new("Assets/Textures/Albedo.png"))
        .ok_or("expected a record for the texture")?;
    assert_eq!(record.reason, "importer version changed");

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn import_cache_rebuilds_after_a_corrupt_cache_file() -> TestResult {
    let root = temp_dir("import-cache-corrupt")?;
    let library = root.join("project").join(LIBRARY_DIRECTORY);
    let mut importer = |_: &ImportCandidate| ImportOutcome::ready();

    fs::create_dir_all(&library)?;
    fs::write(cache_file(&library), b"{ this is not a cache file")?;
    let mut session = ImportSession::with_cache(ImportCache::open(&library));
    let report = session.run(&texture_project(), &mut importer);
    assert_eq!(
        report.imported, 3,
        "a corrupt cache is rebuilt by importing every asset"
    );
    assert!(session.cache_error().is_none());
    let value = cache_assets(&library)?;
    assert_eq!(
        value.get("formatVersion").and_then(JsonValue::as_i64),
        Some(1),
        "the rebuilt cache carries the current format version"
    );

    fs::write(
        cache_file(&library),
        b"{\"format\":\"unitas-import-cache\"}",
    )?;
    let mut stale = ImportSession::with_cache(ImportCache::open(&library));
    let report = stale.run(&texture_project(), &mut importer);
    assert_eq!(report.imported, 3, "a stale cache format is rebuilt");

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn import_cache_parallel_results_match_the_serial_results() -> TestResult {
    let root = temp_dir("import-cache-parallel")?;
    let candidates: Vec<ImportCandidate> = (0..8)
        .map(|index| {
            candidate(
                &format!("Assets/Generated/Asset{index}.png"),
                &format!("{index:032x}"),
                format!("source-{index}").as_bytes(),
            )
        })
        .collect();
    let threads = Mutex::new(HashSet::<ThreadId>::new());

    let mut serial_importer = |_: &ImportCandidate| ImportOutcome::ready();
    let mut serial = ImportSession::new();
    let serial_report = serial.run(&candidates, &mut serial_importer);

    let parallel_importer = |_: &ImportCandidate| {
        threads
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(std::thread::current().id());
        ImportOutcome::ready()
    };
    let mut parallel = ImportSession::new();
    let parallel_report = parallel.run_parallel_with_workers(&candidates, &parallel_importer, 4);

    assert_eq!(parallel_report.imported, serial_report.imported);
    assert_eq!(parallel_report.reused, serial_report.reused);
    assert_eq!(parallel_report.failed, serial_report.failed);
    assert_eq!(parallel_report.records.len(), serial_report.records.len());
    for record in &serial_report.records {
        let other = parallel_report
            .record(&record.path)
            .ok_or("expected a record for every candidate")?;
        assert_eq!(other.state, record.state);
        assert_eq!(other.fingerprint, record.fingerprint);
        assert_eq!(other.reason, record.reason);
        assert_eq!(other.reused, record.reused);
        assert_eq!(other.diagnostics, record.diagnostics);
    }

    let paths: Vec<PathBuf> = parallel_report
        .records
        .iter()
        .map(|record| record.path.clone())
        .collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted, "the parallel results merge in path order");

    let distinct = threads
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .len();
    assert!(
        distinct <= 4,
        "the worker pool is bounded by the requested worker count"
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}
