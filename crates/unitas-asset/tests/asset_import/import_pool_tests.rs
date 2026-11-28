use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread::ThreadId;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use unitas_asset::dependencies::{
    ImportCandidate, ImportDiagnostic, ImportOutcome, ImportSession, ImportSessionReport,
};
use unitas_asset::import_cache::{ImportCache, LIBRARY_DIRECTORY};
use unitas_asset::project::{AssetKind, Project};
use unitas_asset::texture::artifact::texture_artifact;
use unitas_asset::texture::import::TextureImportSettings;
use unitas_asset::trust::TrustStore;
use unitas_asset::unity_yaml::{self, Node};

type TestResult = Result<(), Box<dyn Error>>;

const WORKER_CAP: usize = 4;
const GENERATED_ASSETS: usize = 16;

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("unitas-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn sample_project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/marble-rush")
}

fn generated_candidate(index: usize) -> ImportCandidate {
    ImportCandidate {
        path: PathBuf::from(format!("Assets/Generated/Asset{index:02}.png")),
        guid: format!("{index:032x}"),
        source: format!("synthetic-source-{index}").into_bytes(),
        metadata: String::new(),
        settings: "TextureImporter".to_string(),
        importer_version: "1".to_string(),
        dependencies: Vec::new(),
    }
}

fn meta_path(asset_path: &Path) -> PathBuf {
    let name = asset_path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    asset_path.with_file_name(format!("{name}.meta"))
}

fn referenced_guids(source: &[u8], known: &BTreeSet<String>) -> Vec<String> {
    let text = String::from_utf8_lossy(source);
    let mut found = BTreeSet::new();
    for token in text.split(|character: char| !character.is_ascii_hexdigit()) {
        if token.len() == 32 {
            let lowered = token.to_ascii_lowercase();
            if known.contains(&lowered) {
                found.insert(lowered);
            }
        }
    }
    found.into_iter().collect()
}

fn sample_candidates() -> Result<Vec<ImportCandidate>, Box<dyn Error>> {
    let root = sample_project_root();
    let store = TrustStore::new(root.join("Library/UnitasTrust.json"));
    let (project, _report) = Project::open(&root, &store)?;
    let known: BTreeSet<String> = project
        .inventory()
        .entries
        .iter()
        .map(|entry| entry.guid.clone())
        .collect();

    let mut candidates = Vec::new();
    for entry in &project.inventory().entries {
        if entry.kind != AssetKind::Main {
            continue;
        }
        let asset_path = root.join(&entry.path);
        let source = fs::read(&asset_path)?;
        let metadata = fs::read_to_string(meta_path(&asset_path)).unwrap_or_default();
        let mut dependencies = referenced_guids(&source, &known);
        dependencies.retain(|guid| guid != &entry.guid);
        candidates.push(ImportCandidate {
            path: entry.path.clone(),
            guid: entry.guid.clone(),
            source,
            metadata,
            settings: entry.class.clone().unwrap_or_default(),
            importer_version: "1".to_string(),
            dependencies,
        });
    }
    candidates.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(candidates)
}

fn artifact_path(candidate: &ImportCandidate) -> PathBuf {
    let name = candidate
        .path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    candidate.path.with_file_name(format!("{name}.artifact"))
}

fn digest(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn write_node(bytes: &mut Vec<u8>, node: &Node) {
    match node {
        Node::Scalar(text) => {
            bytes.push(0);
            bytes.extend_from_slice(&(text.len() as u32).to_le_bytes());
            bytes.extend_from_slice(text.as_bytes());
        }
        Node::Mapping(entries) => {
            bytes.push(1);
            bytes.extend_from_slice(&(entries.len() as u32).to_le_bytes());
            for (key, value) in entries {
                write_node(bytes, &Node::Scalar(key.clone()));
                write_node(bytes, value);
            }
        }
        Node::Sequence(items) => {
            bytes.push(2);
            bytes.extend_from_slice(&(items.len() as u32).to_le_bytes());
            for item in items {
                write_node(bytes, item);
            }
        }
    }
}

fn artifact_bytes(candidate: &ImportCandidate) -> Result<Vec<u8>, String> {
    let extension = candidate
        .path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();
    match extension.as_str() {
        "png" => {
            let name = candidate
                .path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_default();
            let artifact =
                texture_artifact(&candidate.source, &name, &TextureImportSettings::default())
                    .map_err(|error| error.to_string())?;
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&artifact.width.to_le_bytes());
            bytes.extend_from_slice(&artifact.height.to_le_bytes());
            bytes.extend_from_slice(format!("{:?}", artifact.format).as_bytes());
            bytes.push(u8::from(artifact.srgb));
            bytes.push(u8::from(artifact.normal_map));
            bytes.push(u8::from(artifact.generate_mips));
            bytes.extend_from_slice(&artifact.pixels);
            Ok(bytes)
        }
        "unity" | "prefab" | "mat" | "physicMaterial" => {
            let text = String::from_utf8_lossy(&candidate.source).to_string();
            let stream = unity_yaml::parse(&text).map_err(|error| error.to_string())?;
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&(stream.documents.len() as u32).to_le_bytes());
            for document in &stream.documents {
                bytes.extend_from_slice(&document.class_id.to_le_bytes());
                bytes.extend_from_slice(&document.file_id.to_le_bytes());
                bytes.push(u8::from(document.stripped));
                write_node(&mut bytes, &document.root);
            }
            Ok(bytes)
        }
        _ => {
            let mut bytes = (candidate.source.len() as u64).to_le_bytes().to_vec();
            bytes.extend_from_slice(&digest(&candidate.source).to_le_bytes());
            Ok(bytes)
        }
    }
}

fn write_artifact(candidate: &ImportCandidate, artifact_root: &Path) -> Result<(), String> {
    let target = artifact_root.join(artifact_path(candidate));
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let bytes = artifact_bytes(candidate)?;
    fs::write(&target, bytes).map_err(|error| error.to_string())
}

fn collect_tree(
    root: &Path,
    directory: &Path,
    files: &mut BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_tree(root, &path, files)?;
        } else {
            let relative = path.strip_prefix(root)?.to_path_buf();
            files.insert(relative, fs::read(&path)?);
        }
    }
    Ok(())
}

struct ImportRun {
    report: ImportSessionReport,
    cache: Vec<u8>,
    artifacts: BTreeMap<PathBuf, Vec<u8>>,
    threads: usize,
    completion: Vec<PathBuf>,
}

fn run_import(
    root: &Path,
    candidates: &[ImportCandidate],
    parallel: bool,
    workers: usize,
    threads: &Mutex<HashSet<ThreadId>>,
) -> Result<ImportRun, Box<dyn Error>> {
    let library = root.join(LIBRARY_DIRECTORY);
    let artifact_root = root.join("Artifacts");
    fs::create_dir_all(&artifact_root)?;
    let completion: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

    let mut session = ImportSession::with_cache(ImportCache::open(&library));
    let report = if parallel {
        let importer = recording_importer(&artifact_root, threads, &completion);
        session.run_parallel_with_workers(candidates, &importer, workers)
    } else {
        let mut importer = recording_importer(&artifact_root, threads, &completion);
        session.run(candidates, &mut importer)
    };

    let cache = fs::read(ImportCache::open(&library).file_path())?;
    let mut artifacts = BTreeMap::new();
    collect_tree(&artifact_root, &artifact_root, &mut artifacts)?;
    let used = threads
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .len();
    let completion = completion
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    Ok(ImportRun {
        report,
        cache,
        artifacts,
        threads: used,
        completion,
    })
}

fn recording_importer<'a>(
    artifact_root: &'a Path,
    threads: &'a Mutex<HashSet<ThreadId>>,
    completion: &'a Mutex<Vec<PathBuf>>,
) -> impl Fn(&ImportCandidate) -> ImportOutcome + 'a {
    move |candidate: &ImportCandidate| {
        threads
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(std::thread::current().id());
        let outcome = import_outcome(candidate, artifact_root);
        completion
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(candidate.path.clone());
        outcome
    }
}

fn import_outcome(candidate: &ImportCandidate, artifact_root: &Path) -> ImportOutcome {
    match write_artifact(candidate, artifact_root) {
        Ok(()) => ImportOutcome::ready(),
        Err(message) => ImportOutcome::failed(ImportDiagnostic::new(
            candidate.path.display().to_string(),
            message,
            "the asset stays failed",
        )),
    }
}

#[test]
fn import_pool_produces_the_same_artifacts_and_cache_as_the_serial_import() -> TestResult {
    let root = temp_dir("import-pool-determinism")?;
    let candidates = sample_candidates()?;
    assert!(
        candidates.len() >= 8,
        "the sample project carries a real asset set, it has {}",
        candidates.len()
    );

    let serial_threads = Mutex::new(HashSet::new());
    let parallel_threads = Mutex::new(HashSet::new());
    let serial = run_import(
        &root.join("serial"),
        &candidates,
        false,
        WORKER_CAP,
        &serial_threads,
    )?;
    let parallel = run_import(
        &root.join("parallel"),
        &candidates,
        true,
        WORKER_CAP,
        &parallel_threads,
    )?;

    assert_eq!(
        serial.report.records, parallel.report.records,
        "the parallel import records match the serial records"
    );
    assert_eq!(serial.report.imported, parallel.report.imported);
    assert_eq!(serial.report.reused, parallel.report.reused);
    assert_eq!(serial.report.failed, parallel.report.failed);
    assert_eq!(
        serial.cache, parallel.cache,
        "the written cache is byte-identical"
    );
    assert_eq!(
        serial.artifacts, parallel.artifacts,
        "the produced artifacts are byte-identical"
    );
    assert!(
        !parallel.artifacts.is_empty(),
        "the sample import produced artifacts"
    );

    let serial_fingerprints: BTreeMap<PathBuf, _> = serial
        .report
        .records
        .iter()
        .map(|record| (record.path.clone(), record.fingerprint))
        .collect();
    let parallel_fingerprints: BTreeMap<PathBuf, _> = parallel
        .report
        .records
        .iter()
        .map(|record| (record.path.clone(), record.fingerprint))
        .collect();
    assert_eq!(
        serial_fingerprints, parallel_fingerprints,
        "the fingerprint sets match"
    );

    let paths: Vec<&PathBuf> = parallel
        .report
        .records
        .iter()
        .map(|record| &record.path)
        .collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(
        paths, sorted,
        "the merged records are ordered by asset path"
    );

    assert!(
        parallel.threads <= WORKER_CAP,
        "the import ran on {} worker threads, the cap is {WORKER_CAP}",
        parallel.threads
    );

    let positions: BTreeMap<&Path, usize> = parallel
        .completion
        .iter()
        .enumerate()
        .map(|(index, path)| (path.as_path(), index))
        .collect();
    let mut edges = 0usize;
    for candidate in &candidates {
        edges += candidate.dependencies.len();
    }
    assert!(
        edges > 0,
        "the sample project carries dependency edges between its assets"
    );
    for candidate in &candidates {
        for dependency in &candidate.dependencies {
            let dependency_path = candidates
                .iter()
                .find(|entry| &entry.guid == dependency)
                .map(|entry| entry.path.as_path())
                .ok_or("expected the dependency to be a candidate of the project")?;
            let before = positions
                .get(dependency_path)
                .copied()
                .ok_or("expected the dependency to be imported")?;
            let after = positions
                .get(candidate.path.as_path())
                .copied()
                .ok_or("expected the asset to be imported")?;
            assert!(
                before < after,
                "the dependency {} is imported before {}",
                dependency_path.display(),
                candidate.path.display()
            );
        }
    }

    let again_threads = Mutex::new(HashSet::new());
    let again = run_import(
        &root.join("parallel"),
        &candidates,
        true,
        WORKER_CAP,
        &again_threads,
    )?;
    assert_eq!(
        again.report.imported, 0,
        "an unchanged project re-imports nothing"
    );
    assert_eq!(again.report.reused, candidates.len());
    assert_eq!(
        again.cache, parallel.cache,
        "the cache stays byte-identical when nothing changed"
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn import_pool_never_runs_more_workers_than_the_configured_cap() -> TestResult {
    let root = temp_dir("import-pool-bound")?;
    let candidates: Vec<ImportCandidate> = (0..GENERATED_ASSETS).map(generated_candidate).collect();
    let running = AtomicUsize::new(0);
    let peak = AtomicUsize::new(0);
    let threads = Mutex::new(HashSet::new());

    let mut session = ImportSession::new();
    let importer = |_: &ImportCandidate| {
        let current = running.fetch_add(1, Ordering::SeqCst) + 1;
        peak.fetch_max(current, Ordering::SeqCst);
        threads
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(std::thread::current().id());
        std::thread::sleep(Duration::from_millis(2));
        running.fetch_sub(1, Ordering::SeqCst);
        ImportOutcome::ready()
    };
    let report = session.run_parallel_with_workers(&candidates, &importer, 3);

    assert_eq!(report.imported, GENERATED_ASSETS);
    let observed = peak.load(Ordering::SeqCst);
    assert!(
        observed <= 3,
        "the pool ran {observed} workers at once, the cap is 3"
    );
    assert!(
        observed >= 2,
        "the pool ran the work items concurrently, the peak was {observed}"
    );
    let distinct = threads
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .len();
    assert!(
        distinct <= 3,
        "the import used {distinct} worker threads, the cap is 3"
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn import_pool_releases_its_workers_when_an_importer_panics() -> TestResult {
    let candidates: Vec<ImportCandidate> = (0..GENERATED_ASSETS).map(generated_candidate).collect();
    let calls = AtomicUsize::new(0);
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let importer = |_: &ImportCandidate| {
            if calls.fetch_add(1, Ordering::SeqCst) == 1 {
                let empty: Vec<u8> = Vec::new();
                let _ = &empty[0];
            }
            ImportOutcome::ready()
        };
        let mut session = ImportSession::new();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            session.run_parallel_with_workers(&candidates, &importer, 3)
        }));
        let _ = sender.send(outcome.is_err());
    });
    let panicked = receiver
        .recv_timeout(Duration::from_secs(10))
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "the import pool hung after the importer panicked",
            )
        })?;
    assert!(panicked, "the importer panic reaches the caller");
    Ok(())
}

#[test]
fn import_pool_merges_the_results_in_asset_path_order() -> TestResult {
    let root = temp_dir("import-pool-order")?;
    let candidates: Vec<ImportCandidate> = (0..8).map(generated_candidate).collect();

    let mut serial_session = ImportSession::new();
    let mut serial_importer = |_: &ImportCandidate| ImportOutcome::ready();
    let serial = serial_session.run(&candidates, &mut serial_importer);

    let completion: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    let completion = &completion;
    let mut session = ImportSession::new();
    let importer = |candidate: &ImportCandidate| {
        let index = candidate
            .path
            .file_stem()
            .and_then(|value| value.to_str())
            .and_then(|value| value.trim_start_matches("Asset").parse::<usize>().ok())
            .unwrap_or(0);
        std::thread::sleep(Duration::from_millis(4 * (8 - index) as u64));
        completion
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(candidate.path.clone());
        ImportOutcome::ready()
    };
    let parallel = session.run_parallel_with_workers(&candidates, &importer, 4);

    let completed = completion
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    assert_eq!(completed.len(), candidates.len());
    let mut sorted_completion = completed.clone();
    sorted_completion.sort();
    assert_ne!(
        completed, sorted_completion,
        "the work items complete out of asset path order"
    );

    let paths: Vec<&PathBuf> = parallel.records.iter().map(|record| &record.path).collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(
        paths, sorted,
        "the merged results are ordered by asset path"
    );
    assert_eq!(
        parallel.records, serial.records,
        "the parallel merge matches the serial import"
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}
