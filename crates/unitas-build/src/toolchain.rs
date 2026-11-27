use std::fs;
use std::process::Command;

use crate::targets::{BuildTarget, TargetArch, TargetOs};

pub const TOOLCHAIN_AVAILABLE: &str = "available";
pub const TOOLCHAIN_EXTERNALLY_UNAVAILABLE: &str = "externally_unavailable";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolchainCause {
    CurrentHost,
    MissingToolchain,
}

impl ToolchainCause {
    pub const fn as_str(self) -> &'static str {
        match self {
            ToolchainCause::CurrentHost => "current_host",
            ToolchainCause::MissingToolchain => "missing_toolchain",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "current_host" => Some(ToolchainCause::CurrentHost),
            "missing_toolchain" => Some(ToolchainCause::MissingToolchain),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl ToolCommand {
    pub fn new(program: &str, args: &[&str]) -> Self {
        Self {
            program: program.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
        }
    }

    fn first_line(&self) -> Option<String> {
        let output = Command::new(&self.program).args(&self.args).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let line = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        if line.is_empty() {
            None
        } else {
            Some(line)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolPresence {
    Command {
        program: String,
        args: Vec<String>,
    },
    RustStandardLibrary {
        triple: String,
        alternate_triples: Vec<String>,
    },
    DotnetRuntimePack {
        rid: String,
    },
}

impl ToolPresence {
    fn is_present(&self) -> bool {
        match self {
            ToolPresence::Command { program, args } => Command::new(program)
                .args(args)
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false),
            ToolPresence::RustStandardLibrary {
                triple,
                alternate_triples,
            } => {
                rust_standard_library_installed(triple)
                    || alternate_triples
                        .iter()
                        .any(|alt| rust_standard_library_installed(alt))
            }
            ToolPresence::DotnetRuntimePack { rid } => dotnet_runtime_pack_installed(rid),
        }
    }
}

fn dotnet_root_directories() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if let Some(root) = std::env::var_os("DOTNET_ROOT") {
        dirs.push(std::path::PathBuf::from(root));
    }
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(std::path::PathBuf::from(home).join(".dotnet"));
    }
    dirs.push(std::path::PathBuf::from("/usr/local/share/dotnet"));
    dirs.push(std::path::PathBuf::from("/usr/share/dotnet"));
    dirs
}

fn dotnet_runtime_pack_installed(rid: &str) -> bool {
    let host_pack = format!("Microsoft.NETCore.App.Host.{rid}");
    let runtime_pack = format!("Microsoft.NETCore.App.Runtime.{rid}");
    let nuget_host = format!("microsoft.netcore.app.host.{rid}");
    let nuget_runtime = format!("microsoft.netcore.app.runtime.{rid}");

    for root in dotnet_root_directories() {
        let packs_dir = root.join("packs");
        if packs_dir.join(&host_pack).is_dir() || packs_dir.join(&runtime_pack).is_dir() {
            return true;
        }
    }

    let nuget_dir = match std::env::var_os("NUGET_PACKAGES") {
        Some(dir) => std::path::PathBuf::from(dir),
        None => std::env::var_os("HOME")
            .map_or_else(|| std::path::PathBuf::from("."), std::path::PathBuf::from)
            .join(".nuget")
            .join("packages"),
    };
    nuget_dir.join(nuget_host).is_dir() || nuget_dir.join(nuget_runtime).is_dir()
}

fn rust_standard_library_installed(triple: &str) -> bool {
    let Ok(output) = Command::new("rustc")
        .args(["--print", "target-libdir", "--target", triple])
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let directory = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if directory.is_empty() {
        return false;
    }
    fs::read_dir(directory)
        .map(|entries| {
            entries.flatten().any(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                name.starts_with("libstd-") && name.ends_with(".rlib")
            })
        })
        .unwrap_or(false)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainRequirement {
    pub tool: String,
    pub required_range: String,
    pub purpose: String,
    pub presence: ToolPresence,
    pub version: Option<ToolCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainProbe {
    pub tool: String,
    pub required_range: String,
    pub purpose: String,
    pub present: bool,
    pub version_found: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnavailableToolchain {
    pub cause: ToolchainCause,
    pub tool: String,
    pub version_found: Option<String>,
    pub required_range: String,
    pub resolving_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetAvailability {
    pub target: BuildTarget,
    pub tools: Vec<ToolchainProbe>,
    pub unavailable: Option<UnavailableToolchain>,
}

impl TargetAvailability {
    pub fn is_available(&self) -> bool {
        self.unavailable.is_none()
    }

    pub fn state(&self) -> &'static str {
        if self.is_available() {
            TOOLCHAIN_AVAILABLE
        } else {
            TOOLCHAIN_EXTERNALLY_UNAVAILABLE
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetToolchain {
    pub target: BuildTarget,
    pub requirements: Vec<ToolchainRequirement>,
    pub resolving_action: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebRuntimeCompileGap {
    pub crate_name: &'static str,
    pub version: &'static str,
    pub dependency_chain: &'static str,
    pub failure_summary: &'static str,
}

pub const WEB_RUNTIME_WASM32_GAPS: [WebRuntimeCompileGap; 0] = [];

pub const fn web_runtime_wasm32_gaps() -> &'static [WebRuntimeCompileGap] {
    &WEB_RUNTIME_WASM32_GAPS
}

fn command_requirement(
    tool: &str,
    required_range: &str,
    purpose: &str,
    version_args: &[&str],
) -> ToolchainRequirement {
    ToolchainRequirement {
        tool: tool.to_string(),
        required_range: required_range.to_string(),
        purpose: purpose.to_string(),
        presence: ToolPresence::Command {
            program: tool.to_string(),
            args: version_args.iter().map(|arg| (*arg).to_string()).collect(),
        },
        version: Some(ToolCommand::new(tool, version_args)),
    }
}

fn rust_requirement(
    triple: &str,
    alternate_triples: &[&str],
    purpose: &str,
) -> ToolchainRequirement {
    let required_range = if alternate_triples.is_empty() {
        format!("rustc 1.93.0 or newer with the {triple} standard library")
    } else {
        let joined = alternate_triples.join(" or ");
        format!("rustc 1.93.0 or newer with the {triple} (or {joined}) standard library")
    };
    ToolchainRequirement {
        tool: String::from("rustc"),
        required_range,
        purpose: purpose.to_string(),
        presence: ToolPresence::RustStandardLibrary {
            triple: triple.to_string(),
            alternate_triples: alternate_triples
                .iter()
                .map(|alt| (*alt).to_string())
                .collect(),
        },
        version: Some(ToolCommand::new("rustc", &["--version"])),
    }
}

fn dotnet_runtime_pack_requirement(rid: &str, purpose: &str) -> ToolchainRequirement {
    ToolchainRequirement {
        tool: format!("dotnet-runtime-pack-{rid}"),
        required_range: format!(
            ".NET 8.0 runtime pack Microsoft.NETCore.App.Host.{rid} (8.0.0 or newer)"
        ),
        purpose: purpose.to_string(),
        presence: ToolPresence::DotnetRuntimePack {
            rid: rid.to_string(),
        },
        version: Some(ToolCommand::new("dotnet", &["--version"])),
    }
}

pub fn required_toolchain(target: BuildTarget) -> TargetToolchain {
    match (target.os(), target.arch()) {
        (TargetOs::Windows, TargetArch::X86_64) => TargetToolchain {
            target,
            requirements: vec![
                command_requirement(
                    "x86_64-w64-mingw32-gcc",
                    "mingw-w64 14.0.0 or newer",
                    "cross C compiler and linker that builds the Windows x86-64 player binary",
                    &["--version"],
                ),
                command_requirement(
                    "x86_64-w64-mingw32-windres",
                    "mingw-w64 14.0.0 or newer",
                    "Windows resource compiler that embeds the player resources",
                    &["--version"],
                ),
                rust_requirement(
                    target.rust_triple(),
                    target.alternate_rust_triples(),
                    "Rust compiler and standard library that builds the Windows x86-64 player",
                ),
                dotnet_runtime_pack_requirement(
                    target.dotnet_runtime_identifier(),
                    ".NET 8.0 runtime pack that hosts managed game scripts on Windows x86-64",
                ),
            ],
            resolving_action: String::from(
                "install the mingw-w64 cross toolchain with 'brew install mingw-w64', add the Rust target with 'rustup target add x86_64-pc-windows-gnu', and restore the .NET runtime pack with 'dotnet restore -r win-x64', then package the player on a Windows x86-64 host where unitas builds that target",
            ),
        },
        (TargetOs::Windows, TargetArch::Arm64) => TargetToolchain {
            target,
            requirements: vec![
                command_requirement(
                    "aarch64-w64-mingw32-gcc",
                    "mingw-w64 14.0.0 or newer",
                    "cross C compiler and linker that builds the Windows ARM64 player binary",
                    &["--version"],
                ),
                command_requirement(
                    "aarch64-w64-mingw32-windres",
                    "mingw-w64 14.0.0 or newer",
                    "Windows resource compiler that embeds the player resources",
                    &["--version"],
                ),
                rust_requirement(
                    target.rust_triple(),
                    target.alternate_rust_triples(),
                    "Rust compiler and standard library that builds the Windows ARM64 player",
                ),
                dotnet_runtime_pack_requirement(
                    target.dotnet_runtime_identifier(),
                    ".NET 8.0 runtime pack that hosts managed game scripts on Windows ARM64",
                ),
            ],
            resolving_action: String::from(
                "install the mingw-w64 cross toolchain with 'brew install mingw-w64', add the Rust target with 'rustup target add aarch64-pc-windows-gnu', and restore the .NET runtime pack with 'dotnet restore -r win-arm64', then package the player on a Windows ARM64 host where unitas builds that target",
            ),
        },
        (TargetOs::Linux, TargetArch::X86_64) => TargetToolchain {
            target,
            requirements: vec![
                command_requirement(
                    "x86_64-linux-gnu-gcc",
                    "GCC 12 or newer",
                    "cross C compiler and linker that builds the Linux x86-64 player binary",
                    &["--version"],
                ),
                rust_requirement(
                    target.rust_triple(),
                    target.alternate_rust_triples(),
                    "Rust compiler and standard library that builds the Linux x86-64 player",
                ),
                dotnet_runtime_pack_requirement(
                    target.dotnet_runtime_identifier(),
                    ".NET 8.0 runtime pack that hosts managed game scripts on Linux x86-64",
                ),
            ],
            resolving_action: String::from(
                "build the player in a Linux x86-64 environment that provides x86_64-linux-gnu-gcc (the 'rust:1.93' container image provides one), add the Rust target with 'rustup target add x86_64-unknown-linux-gnu', and restore the .NET runtime pack with 'dotnet restore -r linux-x64', then package the player on that Linux x86-64 host where unitas builds that target",
            ),
        },
        (TargetOs::Linux, TargetArch::Arm64) => TargetToolchain {
            target,
            requirements: vec![
                command_requirement(
                    "aarch64-linux-gnu-gcc",
                    "GCC 12 or newer",
                    "cross C compiler and linker that builds the Linux ARM64 player binary",
                    &["--version"],
                ),
                rust_requirement(
                    target.rust_triple(),
                    target.alternate_rust_triples(),
                    "Rust compiler and standard library that builds the Linux ARM64 player",
                ),
                dotnet_runtime_pack_requirement(
                    target.dotnet_runtime_identifier(),
                    ".NET 8.0 runtime pack that hosts managed game scripts on Linux ARM64",
                ),
            ],
            resolving_action: String::from(
                "build the player in a Linux ARM64 environment that provides aarch64-linux-gnu-gcc (the 'rust:1.93' container image provides one), add the Rust target with 'rustup target add aarch64-unknown-linux-gnu', and restore the .NET runtime pack with 'dotnet restore -r linux-arm64', then package the player on that Linux ARM64 host where unitas builds that target",
            ),
        },
        (TargetOs::MacOs, TargetArch::Arm64) => TargetToolchain {
            target,
            requirements: vec![
                command_requirement(
                    "clang",
                    "Apple clang 21 or newer (Xcode 26.0 or newer)",
                    "Apple C compiler and linker that builds the macOS ARM64 player binary",
                    &["--version"],
                ),
                rust_requirement(
                    target.rust_triple(),
                    target.alternate_rust_triples(),
                    "Rust compiler and standard library that builds the macOS ARM64 player",
                ),
                dotnet_runtime_pack_requirement(
                    target.dotnet_runtime_identifier(),
                    ".NET 8.0 runtime pack that hosts managed game scripts on macOS ARM64",
                ),
            ],
            resolving_action: String::from(
                "install the Xcode command line tools with 'xcode-select --install', add the Rust target with 'rustup target add aarch64-apple-darwin', and restore the .NET runtime pack with 'dotnet restore -r osx-arm64', then package the player on a macOS ARM64 host where unitas builds that target",
            ),
        },
        (TargetOs::MacOs, TargetArch::X86_64) => TargetToolchain {
            target,
            requirements: vec![
                command_requirement(
                    "clang",
                    "Apple clang 21 or newer (Xcode 26.0 or newer)",
                    "Apple C compiler and linker that builds the macOS x86-64 player binary",
                    &["--version"],
                ),
                rust_requirement(
                    target.rust_triple(),
                    target.alternate_rust_triples(),
                    "Rust compiler and standard library that builds the macOS x86-64 player",
                ),
                dotnet_runtime_pack_requirement(
                    target.dotnet_runtime_identifier(),
                    ".NET 8.0 runtime pack that hosts managed game scripts on macOS x86-64",
                ),
            ],
            resolving_action: String::from(
                "install the Xcode command line tools with 'xcode-select --install', add the Rust target with 'rustup target add x86_64-apple-darwin', and restore the .NET runtime pack with 'dotnet restore -r osx-x64', then package the player on a macOS x86-64 host where unitas builds that target",
            ),
        },
        (TargetOs::Web, _) | (_, TargetArch::Wasm32) => TargetToolchain {
            target,
            requirements: vec![
                command_requirement(
                    "emcc",
                    "Emscripten 3.1.56 or newer",
                    "Emscripten C/C++ compiler and WebAssembly bundler that links the Web player bundle",
                    &["--version"],
                ),
                rust_requirement(
                    target.rust_triple(),
                    target.alternate_rust_triples(),
                    "Rust compiler and standard library that builds the WebAssembly player for wasm32-unknown-unknown",
                ),
            ],
            resolving_action: String::from(
                "install and activate the Emscripten SDK with 'emsdk install latest && emsdk activate latest' so 'emcc' is on PATH, and add the Rust WebAssembly target with 'rustup target add wasm32-unknown-unknown'",
            ),
        },
    }
}

pub fn probe_requirement(requirement: &ToolchainRequirement) -> ToolchainProbe {
    ToolchainProbe {
        tool: requirement.tool.clone(),
        required_range: requirement.required_range.clone(),
        purpose: requirement.purpose.clone(),
        present: requirement.presence.is_present(),
        version_found: requirement
            .version
            .as_ref()
            .and_then(ToolCommand::first_line),
    }
}

pub fn evaluate_availability(
    target: BuildTarget,
    requirements: &[ToolchainRequirement],
    resolving_action: &str,
) -> TargetAvailability {
    let tools: Vec<ToolchainProbe> = requirements.iter().map(probe_requirement).collect();
    let unavailable = tools
        .iter()
        .find(|probe| !probe.present)
        .map(|absent| UnavailableToolchain {
            cause: ToolchainCause::MissingToolchain,
            tool: absent.tool.clone(),
            version_found: absent.version_found.clone(),
            required_range: absent.required_range.clone(),
            resolving_action: resolving_action.to_string(),
        })
        .or_else(|| {
            if target.os() == TargetOs::Web || BuildTarget::current_host() == Some(target) {
                None
            } else {
                Some(UnavailableToolchain {
                    cause: ToolchainCause::CurrentHost,
                    tool: format!("a {target} build host"),
                    version_found: None,
                    required_range: format!(
                        "a {target} host that provides the target's native toolchain"
                    ),
                    resolving_action: format!(
                        "run the packaging on a {target} host, where unitas builds that target"
                    ),
                })
            }
        });
    TargetAvailability {
        target,
        tools,
        unavailable,
    }
}

pub fn probe_target_availability(target: BuildTarget) -> TargetAvailability {
    let toolchain = required_toolchain(target);
    evaluate_availability(target, &toolchain.requirements, &toolchain.resolving_action)
}

pub fn packaging_targets() -> Vec<BuildTarget> {
    let mut targets = vec![
        BuildTarget::WINDOWS_X86_64,
        BuildTarget::LINUX_X86_64,
        BuildTarget::WEB_WASM32,
    ];
    if let Some(host) = BuildTarget::current_host() {
        targets.push(host);
    }
    targets.sort_by_key(|target| target.identifier());
    targets.dedup();
    targets
}

pub fn target_availability_report() -> Vec<TargetAvailability> {
    packaging_targets()
        .iter()
        .map(|target| probe_target_availability(*target))
        .collect()
}
