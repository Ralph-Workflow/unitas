use std::error::Error;
use std::path::PathBuf;

use unitas_build::toolchain::{
    evaluate_availability, packaging_targets, probe_target_availability, required_toolchain,
    target_availability_report, web_runtime_wasm32_gaps, ToolPresence, ToolchainCause,
    ToolchainRequirement, WebRuntimeCompileGap,
};
use unitas_build::{
    attempt_desktop_target, build_project, BuildConfiguration, BuildReport, StartupStatus,
};

use unitas_build::targets::{BuildTarget, TargetArch, TargetOs};

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
    assert_startup_time_record(&staged, &report)?;

    let json = report.to_json();
    assert!(json.contains("\"startup_time\""));
    assert!(json.contains("\"target_availability\""));
    let parsed = BuildReport::from_json(&json)?;
    assert_eq!(parsed, report);
    std::fs::remove_dir_all(&workspace)?;
    Ok(())
}

fn assert_startup_time_record(
    staged: &unitas_build::StagedBundle,
    report: &BuildReport,
) -> TestResult {
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
        assert!(
            staged.launch_artifact_path.is_file(),
            "a packaged application bundle carries no launch artifact at '{}'",
            staged.launch_artifact_path.display()
        );
        let reason = report.startup_time.reason.as_deref().unwrap_or_default();
        assert!(
            !reason.contains("produced no launchable player"),
            "a packaged player on this host is launchable, so the build measured its startup: {reason}"
        );
    }
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
    assert!(windows_tools.contains(&"dotnet-runtime-pack-win-x64"));
    assert_eq!(windows.target.rust_triple(), "x86_64-pc-windows-gnu");
    assert_eq!(windows.target.dotnet_runtime_identifier(), "win-x64");
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
    assert!(linux_tools.contains(&"dotnet-runtime-pack-linux-x64"));
    assert_eq!(linux.target.rust_triple(), "x86_64-unknown-linux-gnu");
    assert_eq!(linux.target.dotnet_runtime_identifier(), "linux-x64");
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

#[test]
fn desktop_packaging_outcome_builds_supported_target_and_records_externally_unavailable_cross_targets(
) -> TestResult {
    let workspace = std::env::temp_dir().join(format!(
        "unitas_build_multi_target_outcome_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&workspace);
    std::fs::create_dir_all(&workspace)?;
    let config = host_config();
    let results = unitas_build::package::attempt_desktop_targets(&config, &workspace)?;
    assert_eq!(results.len(), 3);

    for outcome in &results {
        assert_desktop_packaging_outcome(outcome)?;
    }

    let (_, host_report) = match unitas_build::build_project_for_host(
        &config,
        &workspace.join("host_build"),
        10,
        None,
    ) {
        Ok(pair) => pair,
        Err(boxed) => return Err(Box::new(boxed.0)),
    };
    assert_eq!(host_report.desktop_packaging.len(), 3);
    let json = host_report.to_json();
    assert!(json.contains("\"desktop_packaging\""));
    let round_trip = BuildReport::from_json(&json)?;
    assert_eq!(round_trip.desktop_packaging, host_report.desktop_packaging);

    std::fs::remove_dir_all(&workspace)?;
    Ok(())
}

fn assert_desktop_packaging_outcome(outcome: &unitas_build::DesktopPackageOutcome) -> TestResult {
    if outcome.is_packaged() {
        assert_eq!(outcome.state, "packaged");
        assert!(outcome.app_path.as_ref().is_some_and(|p| p.is_dir()));
        assert!(outcome
            .launch_artifact
            .as_ref()
            .is_some_and(|p| p.is_file()));
        assert!(outcome.unavailable.is_none());
    } else {
        assert_eq!(outcome.state, "externally_unavailable");
        let unavailable = outcome
            .unavailable
            .as_ref()
            .ok_or("externally unavailable target must record precise cause")?;
        assert!(!unavailable.tool.is_empty());
        assert!(!unavailable.required_range.is_empty());
        assert!(!unavailable.resolving_action.is_empty());
        assert!(outcome.app_path.is_none());
        assert!(outcome.launch_artifact.is_none());
    }
    assert!(outcome.build_report_path.is_file());
    let target_report = BuildReport::read_from_file(&outcome.build_report_path)?;
    assert_eq!(target_report.target, outcome.target);
    assert_eq!(target_report.desktop_packaging.len(), 3);
    Ok(())
}

#[test]
fn web_target_probes_emcc_and_wasm32_and_reports_externally_unavailable_when_emcc_is_missing(
) -> TestResult {
    assert_web_target_properties();
    assert_web_toolchain_requirements();

    let availability = probe_target_availability(BuildTarget::WEB_WASM32);
    let emcc_probe = availability
        .tools
        .iter()
        .find(|probe| probe.tool == "emcc")
        .ok_or("web target availability must probe emcc")?;
    let rustc_probe = availability
        .tools
        .iter()
        .find(|probe| probe.tool == "rustc")
        .ok_or("web target availability must probe rustc wasm32-unknown-unknown")?;
    assert!(!rustc_probe.required_range.is_empty());

    if !emcc_probe.present {
        assert!(!availability.is_available());
        assert_eq!(availability.state(), "externally_unavailable");
        let unavailable = availability
            .unavailable
            .as_ref()
            .ok_or("missing emcc must mark web target externally unavailable")?;
        assert_eq!(unavailable.cause, ToolchainCause::MissingToolchain);
        assert_eq!(unavailable.tool, "emcc");
        assert!(unavailable.resolving_action.contains("emsdk"));
    }

    assert_web_attempt_outcome(emcc_probe.present)?;

    let gaps: &[WebRuntimeCompileGap] = web_runtime_wasm32_gaps();
    assert!(
        gaps.is_empty(),
        "the wasm32 runtime compiles cleanly (cargo check -p unitas-runtime --target wasm32-unknown-unknown), so no wasm32 compile gap may remain recorded: {gaps:?}"
    );
    Ok(())
}

fn assert_web_target_properties() {
    assert_eq!(BuildTarget::WEB_WASM32.os(), TargetOs::Web);
    assert_eq!(BuildTarget::WEB_WASM32.arch(), TargetArch::Wasm32);
    assert_eq!(BuildTarget::WEB_WASM32.identifier(), "web-wasm32");
    assert_eq!(
        BuildTarget::WEB_WASM32.rust_triple(),
        "wasm32-unknown-unknown"
    );
    assert_eq!(BuildTarget::WEB_WASM32.primary_linker(), "emcc");
    assert_eq!(
        BuildTarget::from_env("web", "wasm32"),
        Some(BuildTarget::WEB_WASM32)
    );
    assert_eq!(
        BuildTarget::from_env("webgl", "wasm32"),
        Some(BuildTarget::WEB_WASM32)
    );
}

fn assert_web_toolchain_requirements() {
    let web_toolchain = required_toolchain(BuildTarget::WEB_WASM32);
    let web_tools: Vec<&str> = web_toolchain
        .requirements
        .iter()
        .map(|req| req.tool.as_str())
        .collect();
    assert!(web_tools.contains(&"emcc"));
    assert!(web_tools.contains(&"rustc"));
    assert!(web_toolchain
        .resolving_action
        .contains("rustup target add wasm32-unknown-unknown"));
    assert!(web_toolchain.resolving_action.contains("emsdk"));

    let targets = packaging_targets();
    assert!(targets.contains(&BuildTarget::WEB_WASM32));
}

fn assert_web_attempt_outcome(emcc_present: bool) -> TestResult {
    let workspace = std::env::temp_dir().join(format!(
        "unitas_build_web_target_outcome_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&workspace);
    std::fs::create_dir_all(&workspace)?;
    let mut web_config = host_config();
    web_config.target = BuildTarget::WEB_WASM32;
    let outcome = attempt_desktop_target(&web_config, BuildTarget::WEB_WASM32, &workspace)?;
    if !emcc_present {
        assert_eq!(outcome.state, "externally_unavailable");
        let unavailable = outcome
            .unavailable
            .as_ref()
            .ok_or("web package outcome must record missing toolchain")?;
        assert_eq!(unavailable.cause, ToolchainCause::MissingToolchain);
        assert_eq!(unavailable.tool, "emcc");
        assert!(outcome.build_report_path.is_file());
        let report_text = std::fs::read_to_string(&outcome.build_report_path)?;
        assert!(report_text.contains("\"target\": \"web-wasm32\""));
        assert!(report_text.contains("\"architecture\": \"wasm32\""));
        assert!(report_text.contains("toolchain.missing_toolchain"));
    }
    std::fs::remove_dir_all(&workspace)?;
    Ok(())
}
