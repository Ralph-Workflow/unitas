use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::project::{build_compatibility_report, Project, ProjectCompatibilityReport};
use unitas_asset::trust::TrustStore;
use unitas_core::compat::{CompatibilityState, SUPPORTED_PACKAGE_MATRIX};

type TestResult = Result<(), Box<dyn Error>>;

/// The compatibility state names the project report uses for a package.
fn expected_state(state: CompatibilityState) -> &'static str {
    match state {
        CompatibilityState::Supported => "Supported",
        CompatibilityState::PartiallySupported => "Partially_Supported",
        CompatibilityState::Unsupported => "Unsupported",
        CompatibilityState::Unknown => "Unknown",
        CompatibilityState::PlatformLimited => "Platform_Limited",
        CompatibilityState::ExternallyUnavailable => "Externally_Unavailable",
    }
}

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!(
        "unitas_package_compatibility_{name}_{}_{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn write_project(root: &Path, packages: &[(&str, &str)]) -> std::io::Result<()> {
    fs::create_dir_all(root.join("Assets"))?;
    fs::create_dir_all(root.join("ProjectSettings"))?;
    fs::create_dir_all(root.join("Packages"))?;
    fs::write(
        root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.20f1\nm_EditorVersionWithRevision: 2022.3.20f1 (0cf5a3b0e6a3)\n",
    )?;
    let dependencies: Vec<String> = packages
        .iter()
        .map(|(name, version)| format!("    \"{name}\": \"{version}\""))
        .collect();
    fs::write(
        root.join("Packages/manifest.json"),
        format!(
            "{{\n  \"dependencies\": {{\n{}\n  }}\n}}\n",
            dependencies.join(",\n")
        ),
    )?;
    Ok(())
}

fn compatibility_report(root: &Path) -> Result<ProjectCompatibilityReport, Box<dyn Error>> {
    let store = TrustStore::new(root.join("trust.json"));
    let (_project, report) = Project::open(root, &store)?;
    Ok(build_compatibility_report(root, &report))
}

fn finding<'a>(
    report: &'a ProjectCompatibilityReport,
    package: &str,
) -> Result<&'a unitas_asset::project::CompatibilityFinding, Box<dyn Error>> {
    report
        .package_findings
        .iter()
        .find(|finding| finding.identity == package)
        .ok_or_else(|| format!("the report must account for the package '{package}'").into())
}

#[test]
fn cmp_002_every_matrix_package_reports_its_matrix_state() -> TestResult {
    let root = temp_dir("matrix")?;
    let project = root.join("project");
    let packages: Vec<(&str, &str)> = SUPPORTED_PACKAGE_MATRIX
        .iter()
        .map(|entry| (entry.package_id, "1.0.0"))
        .collect();
    write_project(&project, &packages)?;
    let report = compatibility_report(&project)?;

    for entry in SUPPORTED_PACKAGE_MATRIX {
        let finding = finding(&report, entry.package_id)?;
        assert_eq!(
            finding.state,
            expected_state(entry.state),
            "CMP-002: the matrix package '{}' must report its matrix state",
            entry.package_id
        );
    }
    Ok(())
}

#[test]
fn cmp_002_a_matrix_flagged_package_reports_its_flagged_state_not_its_name_prefix() -> TestResult {
    let root = temp_dir("flagged")?;
    let project = root.join("project");
    write_project(
        &project,
        &[
            ("com.unity.ugui", "1.0.0"),
            ("com.unity.shadergraph", "14.0.0"),
            ("com.unity.localization", "1.0.0"),
            ("com.unity.mathematics", "1.0.0"),
            ("com.unity.not-a-listed-package", "1.0.0"),
        ],
    )?;
    let report = compatibility_report(&project)?;

    assert_eq!(
        finding(&report, "com.unity.ugui")?.state,
        "Supported",
        "CMP-002: a supported matrix package must report Supported"
    );
    assert_eq!(
        finding(&report, "com.unity.shadergraph")?.state,
        "Partially_Supported",
        "CMP-002: a partially supported matrix package must not read Supported"
    );
    assert_eq!(
        finding(&report, "com.unity.localization")?.state,
        "Unsupported",
        "CMP-002: an unsupported matrix package must not read Supported"
    );
    assert_eq!(
        finding(&report, "com.unity.mathematics")?.state,
        "Unknown",
        "CMP-002: a matrix package the matrix marks Unknown must report Unknown"
    );
    assert_ne!(
        finding(&report, "com.unity.not-a-listed-package")?.state,
        "Supported",
        "CMP-002: a package the matrix does not list must not be claimed Supported by its name"
    );
    Ok(())
}

#[test]
fn cmp_002_an_unlisted_package_reports_a_precise_unknown_state() -> TestResult {
    let root = temp_dir("unlisted")?;
    let project = root.join("project");
    write_project(
        &project,
        &[
            ("com.vendor.custom-toolkit", "2.1.0"),
            ("com.unity.addressables", "1.21.0"),
            ("com.unity.visualscripting", "1.9.1"),
        ],
    )?;
    let report = compatibility_report(&project)?;

    let unlisted = finding(&report, "com.vendor.custom-toolkit")?;
    assert_eq!(
        unlisted.state, "Unknown",
        "CMP-002: a package the matrix does not list must report Unknown"
    );
    assert!(
        unlisted
            .notes
            .contains("not listed in the Unity 2022.3 LTS supported package matrix"),
        "CMP-002: an unlisted package must state why it is unknown: {:?}",
        unlisted.notes
    );
    let matrix_unsupported = finding(&report, "com.unity.addressables")?;
    assert_eq!(
        matrix_unsupported.state, "Unsupported",
        "CMP-002: a matrix package the matrix marks unsupported must report Unsupported"
    );
    assert_ne!(
        unlisted.state, matrix_unsupported.state,
        "CMP-002: an unlisted package must not read like a matrix-listed unsupported package"
    );
    let unsupported_family = finding(&report, "com.unity.visualscripting")?;
    assert_eq!(
        unsupported_family.state, "Unsupported",
        "CMP-002: a recognized unsupported package the matrix does not list must report Unsupported"
    );
    assert!(
        !unsupported_family.notes.is_empty(),
        "CMP-002: a recognized unsupported package must state its consequence"
    );
    Ok(())
}
