use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::packages::{
    attempt_package_edit, detect_package_modifications, open_offline, parse_lock, parse_manifest,
    resolve, PackageAssessmentClassification, PackageEditOutcome, PackageModificationReason,
    PackageSource, ResolutionConflict, UnresolvedReason,
};

type TestResult = Result<(), Box<dyn Error>>;

const MANIFEST: &str = r#"{
  "dependencies": {
    "com.example.direct": "1.0.0",
    "com.example.a": {
      "version": "1.0.0",
      "dependencies": { "com.example.shared": "1.0.0" }
    },
    "com.example.b": {
      "version": "2.0.0",
      "dependencies": { "com.example.shared": "2.0.0" }
    },
    "com.example.git": {
      "git": "https://example.invalid/com.example.git.git",
      "hash": "0123456789abcdef"
    },
    "com.example.local": { "file": "../local-package" },
    "com.example.embedded": { "file": "com.example.embedded" },
    "com.example.tarball": { "file": "../package.tgz" },
    "com.example.scoped": {
      "version": "3.0.0",
      "scopedRegistry": "https://packages.example.invalid"
    },
    "com.unity.modules.ui": "1.0.0"
  },
  "scopedRegistries": [
    {
      "name": "example",
      "url": "https://packages.example.invalid",
      "scopes": ["com.example.scoped"]
    }
  ],
  "testables": ["com.example.direct"],
  "resolutionStrategy": "highest",
  "enableLockFile": true
}"#;

const LOCK: &str = r#"{
  "dependencies": {
    "com.example.a": {
      "version": "1.0.0",
      "depth": 0,
      "source": "registry",
      "dependencies": { "com.example.b": "1.0.0" }
    },
    "com.example.b": {
      "version": "1.0.0",
      "depth": 1,
      "source": "registry",
      "dependencies": { "com.example.shared": "1.0.0" }
    },
    "com.example.c": {
      "version": "1.0.0",
      "depth": 0,
      "source": "registry",
      "dependencies": { "com.example.shared": "2.0.0" }
    },
    "com.example.shared": {
      "version": "1.0.0",
      "depth": 2,
      "source": "registry"
    }
  }
}"#;

#[test]
fn manifest_records_every_dependency_source() -> TestResult {
    let manifest = parse_manifest(MANIFEST)?;
    let source_of = |name: &str| {
        manifest
            .dependencies
            .iter()
            .find(|dependency| dependency.name == name)
            .map(|dependency| dependency.source.clone())
    };
    assert_eq!(
        source_of("com.example.direct"),
        Some(PackageSource::Registry)
    );
    assert_eq!(
        source_of("com.example.git"),
        Some(PackageSource::Git(
            "https://example.invalid/com.example.git.git".to_string()
        ))
    );
    assert_eq!(
        source_of("com.example.local"),
        Some(PackageSource::Local("../local-package".to_string()))
    );
    assert_eq!(
        source_of("com.example.embedded"),
        Some(PackageSource::Embedded)
    );
    assert_eq!(
        source_of("com.example.tarball"),
        Some(PackageSource::Tarball("../package.tgz".to_string()))
    );
    assert_eq!(
        source_of("com.example.scoped"),
        Some(PackageSource::ScopedRegistry(
            "https://packages.example.invalid".to_string()
        ))
    );
    assert_eq!(
        source_of("com.unity.modules.ui"),
        Some(PackageSource::BuiltIn)
    );
    let git = manifest
        .dependencies
        .iter()
        .find(|dependency| dependency.name == "com.example.git")
        .ok_or("git dependency missing")?;
    assert_eq!(git.hash.as_deref(), Some("0123456789abcdef"));
    Ok(())
}

#[test]
fn conflicting_transitive_requests_report_full_chains() -> TestResult {
    let manifest = parse_manifest(MANIFEST)?;
    let report = resolve(&manifest, None);
    assert_eq!(report.conflicts.len(), 1);
    match &report.conflicts[0] {
        ResolutionConflict::VersionMismatch { package, requests } => {
            assert_eq!(package, "com.example.shared");
            assert_eq!(requests.len(), 2);
            let first = requests
                .iter()
                .find(|request| request.version == "1.0.0")
                .ok_or("missing 1.0.0 request")?;
            let second = requests
                .iter()
                .find(|request| request.version == "2.0.0")
                .ok_or("missing 2.0.0 request")?;
            assert_eq!(
                first.chain,
                vec![
                    "com.example.a".to_string(),
                    "com.example.shared".to_string()
                ]
            );
            assert_eq!(
                second.chain,
                vec![
                    "com.example.b".to_string(),
                    "com.example.shared".to_string()
                ]
            );
        }
    }
    Ok(())
}

#[test]
fn conflict_from_lock_names_the_full_requesting_chain() -> TestResult {
    let manifest = parse_manifest(
        r#"{
          "dependencies": {
            "com.example.a": "1.0.0",
            "com.example.b": "1.0.0",
            "com.example.c": "1.0.0"
          }
        }"#,
    )?;
    let lock = parse_lock(LOCK)?;
    let report = resolve(&manifest, Some(&lock));
    match report
        .conflicts
        .iter()
        .find(|conflict| match conflict {
            ResolutionConflict::VersionMismatch { package, .. } => package == "com.example.shared",
        })
        .ok_or("expected a conflict on com.example.shared")?
    {
        ResolutionConflict::VersionMismatch { requests, .. } => {
            let deep = requests
                .iter()
                .find(|request| request.version == "1.0.0")
                .ok_or("missing 1.0.0 request")?;
            assert_eq!(
                deep.chain,
                vec![
                    "com.example.a".to_string(),
                    "com.example.b".to_string(),
                    "com.example.shared".to_string()
                ]
            );
            let shallow = requests
                .iter()
                .find(|request| request.version == "2.0.0")
                .ok_or("missing 2.0.0 request")?;
            assert_eq!(
                shallow.chain,
                vec![
                    "com.example.c".to_string(),
                    "com.example.shared".to_string()
                ]
            );
        }
    }
    Ok(())
}

#[test]
fn direct_dependency_version_wins_over_transitive_request() -> TestResult {
    let manifest = parse_manifest(
        r#"{
          "dependencies": {
            "com.example.shared": "2.0.0",
            "com.example.a": {
              "version": "1.0.0",
              "dependencies": { "com.example.shared": "1.0.0" }
            }
          }
        }"#,
    )?;
    let report = resolve(&manifest, None);
    assert!(report.conflicts.is_empty());
    let shared = report
        .resolved_package("com.example.shared")
        .ok_or("com.example.shared is not resolved")?;
    assert_eq!(shared.version.as_deref(), Some("2.0.0"));
    Ok(())
}

#[test]
fn offline_open_reproduces_the_lock_and_names_missing_packages() -> TestResult {
    let manifest = parse_manifest(
        r#"{
          "dependencies": {
            "com.example.present": "1.0.0",
            "com.example.missing": "2.0.0"
          }
        }"#,
    )?;
    let lock = parse_lock(
        r#"{
          "dependencies": {
            "com.example.present": {
              "version": "1.0.0",
              "depth": 0,
              "source": "registry"
            }
          }
        }"#,
    )?;
    let report = open_offline(&manifest, &lock);
    assert_eq!(report.resolved.len(), 1);
    let present = report
        .resolved_package("com.example.present")
        .ok_or("present package is not resolved")?;
    assert_eq!(present.version.as_deref(), Some("1.0.0"));
    let missing = report
        .unresolved_package("com.example.missing")
        .ok_or("missing package is not reported")?;
    assert_eq!(missing.reason, UnresolvedReason::MissingFromLock);
    Ok(())
}

#[test]
fn manifest_and_lock_round_trip_through_json_write() -> TestResult {
    let manifest = parse_manifest(MANIFEST)?;
    let written = manifest.write();
    assert_eq!(written, manifest.write());
    assert_eq!(parse_manifest(&written)?, manifest);

    let lock = parse_lock(LOCK)?;
    let written_lock = lock.write();
    assert_eq!(written_lock, lock.write());
    assert_eq!(parse_lock(&written_lock)?, lock);
    Ok(())
}

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("unitas-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn write_test_file(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

#[test]
fn editing_read_only_registry_package_reports_not_committed_while_embedded_and_local_are_committed(
) -> TestResult {
    let manifest = parse_manifest(MANIFEST)?;

    let registry_edit = attempt_package_edit(&manifest, "com.example.direct", "Runtime/Direct.cs");
    assert_eq!(
        registry_edit,
        PackageEditOutcome::NotCommitted {
            package: "com.example.direct".to_string(),
            source: PackageSource::Registry,
            path: "Runtime/Direct.cs".to_string(),
            remediation: "embed the package under Packages/ to edit its content".to_string(),
        }
    );

    let scoped_edit = attempt_package_edit(&manifest, "com.example.scoped", "Runtime/Scoped.cs");
    assert!(matches!(
        scoped_edit,
        PackageEditOutcome::NotCommitted {
            ref package,
            source: PackageSource::ScopedRegistry(_),
            ..
        } if package == "com.example.scoped"
    ));

    let git_edit = attempt_package_edit(&manifest, "com.example.git", "Runtime/GitFile.cs");
    assert!(matches!(
        git_edit,
        PackageEditOutcome::NotCommitted {
            ref package,
            source: PackageSource::Git(_),
            ..
        } if package == "com.example.git"
    ));

    let embedded_edit =
        attempt_package_edit(&manifest, "com.example.embedded", "Runtime/Embedded.cs");
    assert_eq!(
        embedded_edit,
        PackageEditOutcome::Committed {
            package: "com.example.embedded".to_string(),
            path: "Runtime/Embedded.cs".to_string(),
        }
    );

    let local_edit = attempt_package_edit(&manifest, "com.example.local", "Runtime/Local.cs");
    assert_eq!(
        local_edit,
        PackageEditOutcome::Committed {
            package: "com.example.local".to_string(),
            path: "Runtime/Local.cs".to_string(),
        }
    );

    Ok(())
}

#[test]
fn locally_edited_cached_registry_and_embedded_packages_are_reported_not_committed_and_third_party(
) -> TestResult {
    let root = temp_dir("pkg-modifications")?;
    let manifest = parse_manifest(
        r#"{
  "dependencies": {
    "com.unity.ide.rider": "3.0.28",
    "com.unity.render-pipelines.universal": { "file": "com.unity.render-pipelines.universal" }
  }
}"#,
    )?;
    let lock = parse_lock(
        r#"{
  "dependencies": {
    "com.unity.ide.rider": {
      "version": "3.0.28",
      "depth": 0,
      "source": "registry",
      "hash": "clean-hash-0001"
    },
    "com.unity.render-pipelines.universal": {
      "version": "14.0.10",
      "depth": 0,
      "source": "embedded",
      "hash": "clean-urp-hash"
    }
  }
}"#,
    )?;

    write_test_file(
        &root.join("Library/PackageCache/com.unity.ide.rider@3.0.28/package.json"),
        r#"{"name":"com.unity.ide.rider","version":"3.0.28"}"#,
    )?;
    write_test_file(
        &root.join("Library/PackageCache/com.unity.ide.rider@3.0.28/Editor/RiderScriptEditor.cs"),
        "// locally edited in cache\npublic class RiderScriptEditor {}\n",
    )?;
    write_test_file(
        &root.join("Packages/com.unity.render-pipelines.universal/package.json"),
        r#"{"name":"com.unity.render-pipelines.universal","version":"14.0.10-local-edit"}"#,
    )?;
    write_test_file(
        &root.join(
            "Packages/com.unity.render-pipelines.universal/Runtime/UniversalRenderPipeline.cs",
        ),
        "// modified embedded package\n",
    )?;

    let findings = detect_package_modifications(&root, &manifest, Some(&lock));
    assert_eq!(
        findings.len(),
        2,
        "expected 2 package modification findings: {findings:?}"
    );

    let rider = findings
        .iter()
        .find(|f| f.package == "com.unity.ide.rider")
        .ok_or("missing finding for com.unity.ide.rider")?;
    assert_eq!(rider.source, PackageSource::Registry);
    assert!(!rider.committed);
    assert_eq!(
        rider.reason,
        PackageModificationReason::ReadOnlyCacheModified
    );
    assert_eq!(
        rider.classification,
        PackageAssessmentClassification::ThirdPartyContent
    );
    assert!(rider.message.contains("not committed"));
    assert!(rider.message.contains("com.unity.ide.rider"));

    let urp = findings
        .iter()
        .find(|f| f.package == "com.unity.render-pipelines.universal")
        .ok_or("missing finding for com.unity.render-pipelines.universal")?;
    assert_eq!(urp.source, PackageSource::Embedded);
    assert_eq!(
        urp.reason,
        PackageModificationReason::EmbeddedDivergesFromRelease
    );
    assert_eq!(
        urp.classification,
        PackageAssessmentClassification::ThirdPartyContent
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}
