//! Target toolchain discovery for player packaging (BLD-001, PLT-001).
//!
//! Every packaging target needs a toolchain on the host that builds it.
//! This module probes the host for the tools a target requires and records
//! the state of each target: available, or Externally unavailable with the
//! tool, the version found, the required range, and the action that resolves
//! it.

use std::fs;
use std::process::Command;

use crate::targets::{BuildTarget, TargetArch, TargetOs};

pub const TOOLCHAIN_AVAILABLE: &str = "available";
pub const TOOLCHAIN_EXTERNALLY_UNAVAILABLE: &str = "externally_unavailable";

/// Why a target cannot be packaged on this host.
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

/// A command that reports the version of an installed tool.
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

/// How the presence of a tool is detected on the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolPresence {
    Command { program: String, args: Vec<String> },
    RustStandardLibrary { triple: String },
}

impl ToolPresence {
    fn is_present(&self) -> bool {
        match self {
            ToolPresence::Command { program, args } => Command::new(program)
                .args(args)
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false),
            ToolPresence::RustStandardLibrary { triple } => rust_standard_library_installed(triple),
        }
    }
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

/// A tool a target's packaging requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainRequirement {
    pub tool: String,
    pub required_range: String,
    pub purpose: String,
    pub presence: ToolPresence,
    pub version: Option<ToolCommand>,
}

/// The result of probing one required tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainProbe {
    pub tool: String,
    pub required_range: String,
    pub purpose: String,
    pub present: bool,
    pub version_found: Option<String>,
}

/// The record of a target that cannot be packaged on this host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnavailableToolchain {
    pub cause: ToolchainCause,
    pub tool: String,
    pub version_found: Option<String>,
    pub required_range: String,
    pub resolving_action: String,
}

/// The packaging state of one target on this host.
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

/// The tools a target's packaging requires and the action that resolves a
/// missing one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetToolchain {
    pub requirements: Vec<ToolchainRequirement>,
    pub resolving_action: String,
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

fn rust_requirement(triple: &str, purpose: &str) -> ToolchainRequirement {
    ToolchainRequirement {
        tool: String::from("rustc"),
        required_range: format!("rustc 1.93.0 or newer with the {triple} standard library"),
        purpose: purpose.to_string(),
        presence: ToolPresence::RustStandardLibrary {
            triple: triple.to_string(),
        },
        version: Some(ToolCommand::new("rustc", &["--version"])),
    }
}

/// The toolchain a target's packaging requires on the host that builds it.
pub fn required_toolchain(target: BuildTarget) -> TargetToolchain {
    match (target.os(), target.arch()) {
        (TargetOs::Windows, TargetArch::X86_64) => TargetToolchain {
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
                    "x86_64-pc-windows-gnu",
                    "Rust compiler and standard library that builds the Windows x86-64 player",
                ),
            ],
            resolving_action: String::from(
                "install the mingw-w64 cross toolchain with 'brew install mingw-w64' and add the Rust target with 'rustup target add x86_64-pc-windows-gnu', then package the player on a Windows x86-64 host where unitas builds that target",
            ),
        },
        (TargetOs::Windows, TargetArch::Arm64) => TargetToolchain {
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
                    "aarch64-pc-windows-gnu",
                    "Rust compiler and standard library that builds the Windows ARM64 player",
                ),
            ],
            resolving_action: String::from(
                "install the mingw-w64 cross toolchain with 'brew install mingw-w64' and add the Rust target with 'rustup target add aarch64-pc-windows-gnu', then package the player on a Windows ARM64 host where unitas builds that target",
            ),
        },
        (TargetOs::Linux, TargetArch::X86_64) => TargetToolchain {
            requirements: vec![
                command_requirement(
                    "x86_64-linux-gnu-gcc",
                    "GCC 12 or newer",
                    "cross C compiler and linker that builds the Linux x86-64 player binary",
                    &["--version"],
                ),
                rust_requirement(
                    "x86_64-unknown-linux-gnu",
                    "Rust compiler and standard library that builds the Linux x86-64 player",
                ),
            ],
            resolving_action: String::from(
                "build the player in a Linux x86-64 environment that provides x86_64-linux-gnu-gcc (the 'rust:1.93' container image provides one) and add the Rust target with 'rustup target add x86_64-unknown-linux-gnu', then package the player on that Linux x86-64 host where unitas builds that target",
            ),
        },
        (TargetOs::Linux, TargetArch::Arm64) => TargetToolchain {
            requirements: vec![
                command_requirement(
                    "aarch64-linux-gnu-gcc",
                    "GCC 12 or newer",
                    "cross C compiler and linker that builds the Linux ARM64 player binary",
                    &["--version"],
                ),
                rust_requirement(
                    "aarch64-unknown-linux-gnu",
                    "Rust compiler and standard library that builds the Linux ARM64 player",
                ),
            ],
            resolving_action: String::from(
                "build the player in a Linux ARM64 environment that provides aarch64-linux-gnu-gcc (the 'rust:1.93' container image provides one) and add the Rust target with 'rustup target add aarch64-unknown-linux-gnu', then package the player on that Linux ARM64 host where unitas builds that target",
            ),
        },
        (TargetOs::MacOs, TargetArch::Arm64) => TargetToolchain {
            requirements: vec![
                command_requirement(
                    "clang",
                    "Apple clang 21 or newer (Xcode 26.0 or newer)",
                    "Apple C compiler and linker that builds the macOS ARM64 player binary",
                    &["--version"],
                ),
                rust_requirement(
                    "aarch64-apple-darwin",
                    "Rust compiler and standard library that builds the macOS ARM64 player",
                ),
            ],
            resolving_action: String::from(
                "install the Xcode command line tools with 'xcode-select --install' and add the Rust target with 'rustup target add aarch64-apple-darwin', then package the player on a macOS ARM64 host where unitas builds that target",
            ),
        },
        (TargetOs::MacOs, TargetArch::X86_64) => TargetToolchain {
            requirements: vec![
                command_requirement(
                    "clang",
                    "Apple clang 21 or newer (Xcode 26.0 or newer)",
                    "Apple C compiler and linker that builds the macOS x86-64 player binary",
                    &["--version"],
                ),
                rust_requirement(
                    "x86_64-apple-darwin",
                    "Rust compiler and standard library that builds the macOS x86-64 player",
                ),
            ],
            resolving_action: String::from(
                "install the Xcode command line tools with 'xcode-select --install' and add the Rust target with 'rustup target add x86_64-apple-darwin', then package the player on a macOS x86-64 host where unitas builds that target",
            ),
        },
    }
}

/// Probes one required tool on this host.
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

/// Evaluates the requirements of a target and records why it cannot be
/// packaged on this host.
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
            if BuildTarget::current_host() == Some(target) {
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

/// The packaging state of a target on this host.
pub fn probe_target_availability(target: BuildTarget) -> TargetAvailability {
    let toolchain = required_toolchain(target);
    evaluate_availability(target, &toolchain.requirements, &toolchain.resolving_action)
}

/// The targets the packaging slice reports on: the host that builds a player
/// and the desktop targets it cannot build here.
pub fn packaging_targets() -> Vec<BuildTarget> {
    let mut targets = vec![BuildTarget::WINDOWS_X86_64, BuildTarget::LINUX_X86_64];
    if let Some(host) = BuildTarget::current_host() {
        targets.push(host);
    }
    targets.sort_by_key(|target| target.identifier());
    targets.dedup();
    targets
}

/// The packaging state of every target the packaging slice reports on.
pub fn target_availability_report() -> Vec<TargetAvailability> {
    packaging_targets()
        .iter()
        .map(|target| probe_target_availability(*target))
        .collect()
}
