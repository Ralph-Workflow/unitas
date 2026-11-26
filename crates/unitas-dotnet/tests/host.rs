use std::error::Error;
use std::path::{Path, PathBuf};

use unitas_asset::trust::ProjectCapabilities;
use unitas_dotnet::{DotnetHost, DotnetHostError, install_code_execution_capabilities};

const RUNTIME_CONFIG: &str = r#"{"runtimeOptions":{"tfm":"net8.0","framework":{"name":"Microsoft.NETCore.App","version":"8.0.0"},"rollForward":"LatestMinor"}}"#;

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

fn bridge_assembly() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../bridge/Unitas.PlayerHost/bin/Release/net8.0/Unitas.PlayerHost.dll")
}

#[ignore = "the CoreCLR boot runs in the make verify-samples lane"]
#[test]
fn boots_coreclr_and_resolves_a_managed_entry_point() -> Result<(), Box<dyn Error>> {
    let config = write_runtime_config("boot")?;
    let assembly = bridge_assembly();
    trust_code_execution();
    let host = DotnetHost::boot(&config)?;
    let entry: extern "C" fn(u64, *const u8, *const u8) -> i32 = host.load_function(
        &assembly,
        "Unitas.PlayerHost.PlayerHostEntry",
        "UnitasPlayerInit",
    )?;
    assert!(entry as usize != 0);
    let _ = std::fs::remove_dir_all(config.parent().unwrap_or(Path::new(".")));
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
