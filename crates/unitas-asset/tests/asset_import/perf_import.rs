use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use unitas_asset::dependencies::{ImportCandidate, ImportOutcome, ImportSession};
use unitas_asset::import_cache::{ImportCache, LIBRARY_DIRECTORY};

type TestResult = Result<(), Box<dyn Error>>;

const ASSET_COUNT: usize = 64;
const CHANGED_INDEX: usize = 32;
const REIMPORT_BOUND: Duration = Duration::from_secs(5);

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("unitas-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn chain_asset(index: usize) -> ImportCandidate {
    let mut entry = ImportCandidate {
        path: PathBuf::from(format!("Assets/Generated/Asset{index:04}.png")),
        guid: format!("{:032x}", index + 1),
        source: format!("synthetic-source-{index}").into_bytes(),
        metadata: String::new(),
        settings: "TextureImporter".to_string(),
        importer_version: "1".to_string(),
        dependencies: Vec::new(),
    };
    if index > 0 {
        entry.dependencies.push(format!("{index:032x}"));
    }
    entry
}

fn synthetic_import(candidate: &ImportCandidate) -> ImportOutcome {
    let checksum: u64 = candidate.source.iter().map(|byte| u64::from(*byte)).sum();
    if checksum == 0 {
        return ImportOutcome::failed(unitas_asset::dependencies::ImportDiagnostic::new(
            candidate.path.display().to_string(),
            "the synthetic source is empty",
            "the asset stays failed",
        ));
    }
    ImportOutcome::ready()
}

#[test]
fn perf_import_reimport_after_a_one_file_change_touches_only_the_changed_asset_and_its_dependents(
) -> TestResult {
    let root = temp_dir("perf-import")?;
    let library = root.join("project").join(LIBRARY_DIRECTORY);
    let candidates: Vec<ImportCandidate> = (0..ASSET_COUNT).map(chain_asset).collect();
    let calls = AtomicUsize::new(0);
    let importer = |candidate: &ImportCandidate| {
        calls.fetch_add(1, Ordering::SeqCst);
        synthetic_import(candidate)
    };

    let started = Instant::now();
    let mut session = ImportSession::with_cache(ImportCache::open(&library));
    let report = session.run_parallel(&candidates, &importer);
    let first_import = started.elapsed();
    assert_eq!(report.imported, ASSET_COUNT, "every asset imports once");
    assert_eq!(report.failed, 0);
    assert_eq!(calls.load(Ordering::SeqCst), ASSET_COUNT);

    let mut changed = candidates.clone();
    changed[CHANGED_INDEX].source = b"changed-synthetic-source".to_vec();
    calls.store(0, Ordering::SeqCst);

    let started = Instant::now();
    let mut second = ImportSession::with_cache(ImportCache::open(&library));
    let report = second.run_parallel(&changed, &importer);
    let reimport = started.elapsed();

    assert_eq!(
        report.imported,
        ASSET_COUNT - CHANGED_INDEX,
        "the reimport touches the changed asset and its dependents"
    );
    assert_eq!(report.reused, CHANGED_INDEX, "the rest keeps its result");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        ASSET_COUNT - CHANGED_INDEX,
        "the importer runs only for the changed asset and its dependents"
    );
    assert!(
        reimport <= REIMPORT_BOUND,
        "the reimport finished in {reimport:?}, the bound is {REIMPORT_BOUND:?}"
    );
    eprintln!(
        "perf_import: {ASSET_COUNT} assets, initial import {first_import:?}, reimport {reimport:?}"
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}
