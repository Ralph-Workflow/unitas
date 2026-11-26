use std::error::Error;
use std::path::{Path, PathBuf};

use unitas_asset::trust::ProjectCapabilities;
use unitas_dotnet::{DotnetHost, DotnetHostError, install_code_execution_capabilities};

const RUNTIME_CONFIG: &str = r#"{"runtimeOptions":{"tfm":"net8.0","framework":{"name":"Microsoft.NETCore.App","version":"8.0.0"},"rollForward":"LatestMinor"}}"#;

fn write_runtime_config(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let directory =
        std::env::temp_dir().join(format!("unitas-dotnet-trust-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let path = directory.join("runtimeconfig.json");
    std::fs::write(&path, RUNTIME_CONFIG)?;
    Ok(path)
}

#[test]
fn untrusted_project_cannot_execute_code() -> Result<(), Box<dyn Error>> {
    let config = write_runtime_config("gate")?;

    install_code_execution_capabilities(ProjectCapabilities::default());
    match DotnetHost::boot(&config) {
        Err(DotnetHostError::CodeExecutionRefused { operation }) => {
            assert_eq!(operation, "player host boot");
        }
        Err(other) => return Err(format!("expected a trust refusal, got {other}").into()),
        Ok(_) => return Err("the host booted the managed runtime for an untrusted project".into()),
    }

    install_code_execution_capabilities(ProjectCapabilities {
        may_run_code: true,
        may_fetch: true,
        restricted: false,
    });
    let roots = [PathBuf::from("/nonexistent/unitas-dotnet-trust-root")];
    match DotnetHost::boot_with_search_paths(&config, &roots) {
        Err(DotnetHostError::HostFxrNotFound { .. }) => {}
        Err(other) => {
            return Err(format!("expected the trust gate to allow the boot, got {other}").into());
        }
        Ok(_) => return Err("the host booted without a hostfxr library".into()),
    }

    let _ = std::fs::remove_dir_all(config.parent().unwrap_or(Path::new(".")));
    Ok(())
}
