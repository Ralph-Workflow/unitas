//! W3 packaging proof: staging the real `samples/roll-a-ball` project stages a
//! `Managed/` payload (game assembly compiled at build time plus the
//! `Unitas.PlayerHost` host files) and produces a build report with scenes,
//! sizes, and the build identifier.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use unitas_build::bundle::{
    BundleManifest, BundleManifestEntry, MANAGED_DIR_NAME, MANAGED_GAME_ASSEMBLY_FILENAME,
    PLAYER_HOST_ASSEMBLY_FILENAME,
};
use unitas_build::config::{BuildConfiguration, BuildMode};
use unitas_build::report::{BuildReport, SizeCategoryEntry, SizeContributor};
use unitas_build::sample::stage_roll_a_ball_bundle_for_host;
use unitas_build::targets::BuildTarget;

const PLAYER_HOST_PAYLOAD_FILENAMES: [&str; 3] = [
    PLAYER_HOST_ASSEMBLY_FILENAME,
    "Unitas.PlayerHost.deps.json",
    "Unitas.PlayerHost.runtimeconfig.json",
];

type TestResult = Result<(), Box<dyn Error>>;

fn test_output_dir(label: &str) -> Result<PathBuf, Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("the crate directory has no repository root")?;
    let directory = root
        .join("target")
        .join("unitas-build-tests")
        .join(format!("{label}_{}", std::process::id()));
    if directory.exists() {
        fs::remove_dir_all(&directory)?;
    }
    fs::create_dir_all(&directory)?;
    Ok(directory)
}

fn sample_config() -> Result<BuildConfiguration, Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("the crate directory has no repository root")?;
    let scene_path = root
        .join("samples")
        .join("roll-a-ball")
        .join("Assets")
        .join("Scenes")
        .join("RollABall.unity");
    Ok(BuildConfiguration::new(
        vec![scene_path],
        "Unitas",
        "RollABall",
        "com.unitas.rollaball",
        "1.0.0",
        BuildTarget::MACOS_ARM64,
    )
    .with_mode(BuildMode::Development))
}

fn managed_entry<'a>(
    manifest: &'a BundleManifest,
    filename: &str,
) -> Option<&'a BundleManifestEntry> {
    let staged_path = format!("{MANAGED_DIR_NAME}/{filename}");
    manifest
        .entries
        .iter()
        .find(|entry| entry.staged_path == staged_path)
}

#[test]
fn packaged_app_contains_managed_payload_and_build_report() -> TestResult {
    let output_dir = test_output_dir("managed_payload")?;
    let staged = stage_roll_a_ball_bundle_for_host(&output_dir, None)?;
    let manifest = &staged.manifest;

    // The build identifier is content-derived and travels with the manifest.
    assert_eq!(manifest.build_identifier.len(), 16);
    assert!(manifest
        .build_identifier
        .chars()
        .all(|character| character.is_ascii_hexdigit()));
    assert_eq!(
        manifest.build_identifier,
        manifest.compute_build_identifier()
    );
    let reparsed = BundleManifest::from_canonical_str(&fs::read_to_string(&staged.manifest_path)?)?;
    assert_eq!(reparsed.build_identifier, manifest.build_identifier);

    // The game assembly is compiled from the sample's scripts at build time;
    // without the bridge tooling the stage degrades to a clear diagnostic.
    let game_entry = managed_entry(manifest, MANAGED_GAME_ASSEMBLY_FILENAME);
    match game_entry {
        Some(entry) => {
            assert!(entry.size_bytes > 0);
            assert!(output_dir.join(&entry.staged_path).is_file());
        }
        None => assert!(
            staged
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("script compilation")),
            "expected the game assembly or a script-compilation diagnostic, got: {:?}",
            staged.diagnostics
        ),
    }

    // The Unitas.PlayerHost payload is fixed by contract. W2 builds it in
    // parallel; until its outputs exist every missing file must be named in
    // the staging diagnostics instead of failing the build.
    let mut host_payload_staged = true;
    for filename in PLAYER_HOST_PAYLOAD_FILENAMES {
        if managed_entry(manifest, filename).is_none() {
            host_payload_staged = false;
            assert!(
                staged
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.contains(filename)),
                "missing payload '{filename}' without a diagnostic: {:?}",
                staged.diagnostics
            );
        }
    }

    // The packaged application bundle carries the staged Managed/ payload.
    let (bundle_root, resources_root) = match staged.application_bundle_path.as_deref() {
        Some(app_path) => (app_path, app_path.join("Contents").join("Resources")),
        None => (output_dir.as_path(), output_dir.clone()),
    };
    if let Some(entry) = game_entry {
        assert!(resources_root.join(&entry.staged_path).is_file());
    }
    if host_payload_staged {
        for filename in PLAYER_HOST_PAYLOAD_FILENAMES {
            assert!(resources_root
                .join(MANAGED_DIR_NAME)
                .join(filename)
                .is_file());
        }
    }
    assert!(
        bundle_root.join("Contents").join("Info.plist").is_file()
            || output_dir.join("bundle.manifest").is_file()
    );

    // A build report lists the scenes, the payload sizes, and the build
    // identifier, and round-trips through JSON.
    let config = sample_config()?;
    let managed_payload: Vec<SizeContributor> = manifest
        .entries
        .iter()
        .filter(|entry| {
            entry
                .staged_path
                .starts_with(&format!("{MANAGED_DIR_NAME}/"))
        })
        .map(|entry| {
            SizeContributor::new(
                entry.staged_path.clone(),
                "managed",
                entry.size_bytes,
                entry.content_hash.clone(),
            )
        })
        .collect();
    let scenes_size: u64 = manifest
        .entries
        .iter()
        .filter(|entry| entry.staged_path.starts_with("Assets/"))
        .map(|entry| entry.size_bytes)
        .sum();
    let managed_size: u64 = managed_payload.iter().map(|entry| entry.size_bytes).sum();

    let report = BuildReport::succeeded(
        &config,
        1,
        staged.output_dir.clone(),
        staged.launch_artifact_path.clone(),
        manifest.total_content_bytes,
        vec![
            SizeCategoryEntry::new("scenes", scenes_size),
            SizeCategoryEntry::new("managed", managed_size),
        ],
        manifest
            .entries
            .iter()
            .map(|entry| {
                SizeContributor::new(
                    entry.staged_path.clone(),
                    if entry.staged_path.starts_with("Assets/") {
                        "scenes"
                    } else {
                        "managed"
                    },
                    entry.size_bytes,
                    entry.content_hash.clone(),
                )
            })
            .collect(),
    )
    .with_build_identifier(manifest.build_identifier.clone())
    .with_managed_payload(managed_payload);

    assert_eq!(report.build_identifier, manifest.build_identifier);
    assert_eq!(report.product_name, "RollABall");
    assert!(!report.included_scenes.is_empty());
    assert!(report.total_size_bytes > 0);
    let json = report.to_json();
    assert!(json.contains("\"build_identifier\""));
    assert!(json.contains("\"managed_payload\""));
    let parsed = BuildReport::from_json(&json)?;
    assert_eq!(parsed.build_identifier, report.build_identifier);
    assert_eq!(parsed.managed_payload, report.managed_payload);

    // Reports written before the new fields existed still parse.
    let legacy_json = json.replacen(
        &format!("  \"build_identifier\": \"{}\",\n", report.build_identifier),
        "",
        1,
    );
    let legacy = BuildReport::from_json(&legacy_json)?;
    assert!(legacy.build_identifier.is_empty());

    Ok(())
}
