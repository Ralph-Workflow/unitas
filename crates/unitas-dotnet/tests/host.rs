use std::error::Error;
use std::path::{Path, PathBuf};

use unitas_asset::trust::ProjectCapabilities;
use unitas_dotnet::{DotnetHost, DotnetHostError, install_code_execution_capabilities};

const RUNTIME_CONFIG: &str = r#"{"runtimeOptions":{"tfm":"net8.0","framework":{"name":"Microsoft.NETCore.App","version":"8.0.0"},"rollForward":"LatestMinor"}}"#;
const MINIMAL_HOST_SOURCE: &str = "using System;\nusing System.Runtime.InteropServices;\nnamespace Unitas.PlayerHost {\n    public static class PlayerHostEntry {\n        [UnmanagedCallersOnly]\n        public static int UnitasPlayerInit(ulong runtimeHandle, IntPtr bundleRoot, IntPtr persistRoot, IntPtr startupScenePath) => runtimeHandle == 0 ? 1 : 0;\n    }\n}\n";
const MINIMAL_HOST_PROJECT: &str = "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n    <AssemblyName>Unitas.PlayerHost</AssemblyName>\n    <GenerateRuntimeConfigurationFiles>true</GenerateRuntimeConfigurationFiles>\n    <RollForward>LatestMinor</RollForward>\n  </PropertyGroup>\n</Project>\n";

fn trust_code_execution() {
    install_code_execution_capabilities(ProjectCapabilities {
        may_run_code: true,
        may_fetch: true,
        restricted: false,
    });
}

fn write_runtime_config(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let directory =
        std::env::temp_dir().join(format!("unitas-dotnet-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let path = directory.join("runtimeconfig.json");
    std::fs::write(&path, RUNTIME_CONFIG)?;
    Ok(path)
}

fn copy_host_file(source: &Path, destination: &Path) -> Result<bool, Box<dyn Error>> {
    match std::fs::read(source) {
        Ok(bytes) if !bytes.is_empty() => {
            std::fs::write(destination, bytes)?;
            Ok(true)
        }
        Ok(_) | Err(_) => Ok(false),
    }
}

fn stage_prebuilt_host(directory: &Path) -> Result<Option<(PathBuf, PathBuf)>, Box<dyn Error>> {
    let source_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../bridge/Unitas.PlayerHost/bin/Release/net8.0");
    for filename in [
        "Unitas.PlayerHost.dll",
        "Unitas.PlayerHost.deps.json",
        "Unitas.PlayerHost.runtimeconfig.json",
        "Unitas.Bridge.dll",
        "Unitas.Scripting.dll",
        "UnityEngine.dll",
    ] {
        let source = source_dir.join(filename);
        let destination = directory.join(filename);
        if !copy_host_file(&source, &destination)? {
            return Ok(None);
        }
    }
    Ok(Some((
        directory.join("Unitas.PlayerHost.runtimeconfig.json"),
        directory.join("Unitas.PlayerHost.dll"),
    )))
}

fn build_minimal_host(directory: &Path) -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let project_dir = directory.join("src");
    let out_dir = directory.join("out");
    std::fs::create_dir_all(&project_dir)?;
    std::fs::create_dir_all(&out_dir)?;
    let project_path = project_dir.join("Unitas.PlayerHost.csproj");
    std::fs::write(&project_path, MINIMAL_HOST_PROJECT)?;
    std::fs::write(project_dir.join("PlayerHostEntry.cs"), MINIMAL_HOST_SOURCE)?;
    let output = std::process::Command::new("dotnet")
        .arg("build")
        .arg(&project_path)
        .arg("--configuration")
        .arg("Release")
        .arg("--nologo")
        .arg("-v")
        .arg("q")
        .arg("-o")
        .arg(&out_dir)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "dotnet build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok((
        out_dir.join("Unitas.PlayerHost.runtimeconfig.json"),
        out_dir.join("Unitas.PlayerHost.dll"),
    ))
}

fn stage_boot_host() -> Result<(PathBuf, PathBuf, PathBuf), Box<dyn Error>> {
    let directory = std::env::temp_dir().join(format!("unitas-dotnet-{}-boot", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory)?;
    let (config, assembly) = match stage_prebuilt_host(&directory)? {
        Some(paths) => paths,
        None => build_minimal_host(&directory)?,
    };
    Ok((directory, config, assembly))
}

#[test]
fn boots_coreclr_and_resolves_a_managed_entry_point() -> Result<(), Box<dyn Error>> {
    let (directory, config, assembly) = stage_boot_host()?;
    trust_code_execution();
    let host = DotnetHost::boot(&config)?;
    let entry: extern "C" fn(u64, *const u8, *const u8, *const u8) -> i32 = host.load_function(
        &assembly,
        "Unitas.PlayerHost.PlayerHostEntry",
        "UnitasPlayerInit",
    )?;
    assert!(entry as usize != 0);
    assert_eq!(
        entry(0, std::ptr::null(), std::ptr::null(), std::ptr::null()),
        1
    );
    let _ = std::fs::remove_dir_all(&directory);
    Ok(())
}

#[test]
fn discovery_failure_is_reported_as_a_typed_error() -> Result<(), Box<dyn Error>> {
    let config = write_runtime_config("discovery")?;
    let roots = [PathBuf::from("/nonexistent/unitas-dotnet-root")];
    trust_code_execution();
    let result = DotnetHost::boot_with_search_paths(&config, &roots);
    let searched = match result {
        Err(DotnetHostError::HostFxrNotFound { searched }) => searched,
        Err(other) => return Err(format!("expected HostFxrNotFound, got {other}").into()),
        Ok(_) => return Err("the host booted without a hostfxr library".into()),
    };
    assert!(!searched.is_empty());
    let _ = std::fs::remove_dir_all(config.parent().unwrap_or(Path::new(".")));
    Ok(())
}
