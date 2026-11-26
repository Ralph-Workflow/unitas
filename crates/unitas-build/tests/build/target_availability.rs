use std::error::Error;
use std::path::PathBuf;

use unitas_build::toolchain::{
    evaluate_availability, packaging_targets, required_toolchain, target_availability_report,
    ToolPresence, ToolchainCause, ToolchainRequirement,
};
use unitas_build::{build_project, BuildConfiguration, BuildReport, StartupStatus};

use unitas_build::targets::BuildTarget;

type TestResult = Result<(), Box<dyn Error>>;

fn fixture_scene() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("scene_alpha.unity")
}

fn host_config() -> BuildConfiguration {
    BuildConfiguration::new(
        vec![fixture_scene()],
        "Unitas",
        "AvailabilityGame",
        "com.unitas.availability",
        "1.0.0",
        BuildTarget::host(),
    )
}

#[test]
fn build_report_carries_build_identifier_licensing_record_and_startup_time() -> TestResult {
    let workspace = std::env::temp_dir().join(format!(
        "unitas_build_availability_report_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&workspace)?;
    let output_dir = workspace.join("out");
    let config = host_config();
    let (staged, report) = match build_project(&config, &output_dir, 11) {
        Ok(pair) => pair,
        Err(boxed) => return Err(Box::new(boxed.0)),
    };

    assert!(
        !report.build_identifier.is_empty(),
        "the build report carries no build identifier"
    );
    let licensing = report
        .licensing
        .as_ref()
        .ok_or("the build report carries no licensing record")?;
    assert!(!licensing.governing_license.is_empty());
    assert!(!licensing.license_texts.is_empty());
    assert!(
        !report.startup_time.basis.is_empty(),
        "the build report carries no startup measurement basis"
    );
    match report.startup_time.status {
        StartupStatus::Measured => {
            let duration = report
                .startup_time
                .duration_ms
                .ok_or("a measured startup time carries no duration")?;
            assert!(duration > 0, "a measured startup time is not positive");
            assert!(report.startup_time.reason.is_none());
        }
        StartupStatus::Unavailable => {
            assert!(
                report.startup_time.duration_ms.is_none(),
                "an unavailable startup time carries a duration"
            );
            let reason = report
                .startup_time
                .reason
                .as_deref()
                .ok_or("an unavailable startup time carries no reason")?;
            assert!(!reason.is_empty());
        }
    }
    if staged.application_bundle_path.is_some() {
        assert_eq!(
            report.startup_time.status,
            StartupStatus::Measured,
            "a packaged player on this host is launchable, so its startup time is measured"
        );
    }

    let json = report.to_json();
    assert!(json.contains("\"startup_time\""));
    assert!(json.contains("\"target_availability\""));
    let parsed = BuildReport::from_json(&json)?;
    assert_eq!(parsed, report);
    std::fs::remove_dir_all(&workspace)?;
    Ok(())
}

#[test]
fn every_reported_target_carries_a_complete_packaging_record() -> TestResult {
    let availability = target_availability_report();
    assert!(
        !availability.is_empty(),
        "the packaging slice reports on no target"
    );
    for entry in &availability {
        assert!(
            !entry.tools.is_empty(),
            "the target '{}' reports no required tool",
            entry.target
        );
        for tool in &entry.tools {
            assert!(!tool.tool.is_empty());
            assert!(!tool.required_range.is_empty());
            assert!(!tool.purpose.is_empty());
        }
        match &entry.unavailable {
            None => assert_eq!(entry.state(), "available"),
            Some(unavailable) => {
                assert_eq!(entry.state(), "externally_unavailable");
                assert!(!unavailable.tool.is_empty());
                assert!(!unavailable.required_range.is_empty());
                assert!(!unavailable.resolving_action.is_empty());
                assert!(
                    matches!(
                        unavailable.cause,
                        ToolchainCause::CurrentHost | ToolchainCause::MissingToolchain
                    ),
                    "the unavailable record carries an unknown cause"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn a_missing_cross_toolchain_is_recorded_with_the_tool_the_range_and_the_fix() -> TestResult {
    let requirement = ToolchainRequirement {
        tool: String::from("unitas-absent-cross-compiler"),
        required_range: String::from("test cross toolchain 1.0 or newer"),
        purpose: String::from("cross compiler that builds the player binary"),
        presence: ToolPresence::Command {
            program: String::from("unitas-absent-cross-compiler"),
            args: vec![String::from("--version")],
        },
        version: None,
    };
    let availability = evaluate_availability(
        BuildTarget::WINDOWS_X86_64,
        &[requirement],
        "install the absent cross toolchain",
    );

    assert!(!availability.is_available());
    let unavailable = availability
        .unavailable
        .as_ref()
        .ok_or("an absent tool leaves the target externally unavailable")?;
    assert_eq!(unavailable.cause, ToolchainCause::MissingToolchain);
    assert_eq!(unavailable.tool, "unitas-absent-cross-compiler");
    assert_eq!(
        unavailable.required_range,
        "test cross toolchain 1.0 or newer"
    );
    assert_eq!(
        unavailable.resolving_action,
        "install the absent cross toolchain"
    );
    assert!(unavailable.version_found.is_none());
    Ok(())
}

#[test]
fn windows_and_linux_packaging_name_the_cross_toolchain_each_target_requires() -> TestResult {
    let windows = required_toolchain(BuildTarget::WINDOWS_X86_64);
    let windows_tools: Vec<&str> = windows
        .requirements
        .iter()
        .map(|requirement| requirement.tool.as_str())
        .collect();
    assert!(windows_tools.contains(&"x86_64-w64-mingw32-gcc"));
    assert!(windows_tools.contains(&"x86_64-w64-mingw32-windres"));
    assert!(windows
        .resolving_action
        .contains("rustup target add x86_64-pc-windows-gnu"));

    let linux = required_toolchain(BuildTarget::LINUX_X86_64);
    let linux_tools: Vec<&str> = linux
        .requirements
        .iter()
        .map(|requirement| requirement.tool.as_str())
        .collect();
    assert!(linux_tools.contains(&"x86_64-linux-gnu-gcc"));
    assert!(linux
        .resolving_action
        .contains("rustup target add x86_64-unknown-linux-gnu"));

    let targets = packaging_targets();
    assert!(targets.contains(&BuildTarget::WINDOWS_X86_64));
    assert!(targets.contains(&BuildTarget::LINUX_X86_64));
    if let Some(host) = BuildTarget::current_host() {
        assert!(targets.contains(&host));
    }
    Ok(())
}
