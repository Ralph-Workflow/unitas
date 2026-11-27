use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TargetOs {
    Windows,
    MacOs,
    Linux,
}

impl TargetOs {
    pub const fn as_str(self) -> &'static str {
        match self {
            TargetOs::Windows => "windows",
            TargetOs::MacOs => "macos",
            TargetOs::Linux => "linux",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            TargetOs::Windows => "Windows",
            TargetOs::MacOs => "macOS",
            TargetOs::Linux => "Linux",
        }
    }

    pub fn from_env_os(os: &str) -> Option<Self> {
        match os {
            "windows" => Some(TargetOs::Windows),
            "macos" => Some(TargetOs::MacOs),
            "linux" => Some(TargetOs::Linux),
            _ => None,
        }
    }
}

impl fmt::Display for TargetOs {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TargetArch {
    X86_64,
    Arm64,
}

impl TargetArch {
    pub const fn as_str(self) -> &'static str {
        match self {
            TargetArch::X86_64 => "x86_64",
            TargetArch::Arm64 => "aarch64",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            TargetArch::X86_64 => "x86-64",
            TargetArch::Arm64 => "ARM64",
        }
    }

    pub fn from_env_arch(arch: &str) -> Option<Self> {
        match arch {
            "x86_64" => Some(TargetArch::X86_64),
            "aarch64" | "arm64" => Some(TargetArch::Arm64),
            _ => None,
        }
    }
}

impl fmt::Display for TargetArch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BuildTarget {
    pub os: TargetOs,
    pub arch: TargetArch,
}

impl BuildTarget {
    pub const WINDOWS_X86_64: Self = Self {
        os: TargetOs::Windows,
        arch: TargetArch::X86_64,
    };
    pub const WINDOWS_ARM64: Self = Self {
        os: TargetOs::Windows,
        arch: TargetArch::Arm64,
    };
    pub const MACOS_X86_64: Self = Self {
        os: TargetOs::MacOs,
        arch: TargetArch::X86_64,
    };
    pub const MACOS_ARM64: Self = Self {
        os: TargetOs::MacOs,
        arch: TargetArch::Arm64,
    };
    pub const LINUX_X86_64: Self = Self {
        os: TargetOs::Linux,
        arch: TargetArch::X86_64,
    };
    pub const LINUX_ARM64: Self = Self {
        os: TargetOs::Linux,
        arch: TargetArch::Arm64,
    };

    pub const ALL: [Self; 6] = [
        Self::WINDOWS_X86_64,
        Self::WINDOWS_ARM64,
        Self::MACOS_X86_64,
        Self::MACOS_ARM64,
        Self::LINUX_X86_64,
        Self::LINUX_ARM64,
    ];

    pub const fn new(os: TargetOs, arch: TargetArch) -> Self {
        Self { os, arch }
    }

    pub const fn all() -> &'static [Self] {
        &Self::ALL
    }

    pub fn current_host() -> Option<Self> {
        Self::from_env(std::env::consts::OS, std::env::consts::ARCH)
    }

    pub fn host() -> Self {
        Self::current_host().unwrap_or(Self::LINUX_X86_64)
    }

    pub fn from_env(os: &str, arch: &str) -> Option<Self> {
        let target_os = TargetOs::from_env_os(os)?;
        let target_arch = TargetArch::from_env_arch(arch)?;
        Some(Self::new(target_os, target_arch))
    }

    pub const fn os(self) -> TargetOs {
        self.os
    }

    pub const fn arch(self) -> TargetArch {
        self.arch
    }

    pub const fn os_name(self) -> &'static str {
        self.os.as_str()
    }

    pub const fn platform_name(self) -> &'static str {
        self.os.display_name()
    }

    pub const fn arch_name(self) -> &'static str {
        self.arch.display_name()
    }

    pub const fn arch_id(self) -> &'static str {
        self.arch.as_str()
    }

    pub const fn rust_triple(self) -> &'static str {
        match (self.os, self.arch) {
            (TargetOs::Windows, TargetArch::X86_64) => "x86_64-pc-windows-gnu",
            (TargetOs::Windows, TargetArch::Arm64) => "aarch64-pc-windows-gnu",
            (TargetOs::MacOs, TargetArch::X86_64) => "x86_64-apple-darwin",
            (TargetOs::MacOs, TargetArch::Arm64) => "aarch64-apple-darwin",
            (TargetOs::Linux, TargetArch::X86_64) => "x86_64-unknown-linux-gnu",
            (TargetOs::Linux, TargetArch::Arm64) => "aarch64-unknown-linux-gnu",
        }
    }

    pub const fn alternate_rust_triples(self) -> &'static [&'static str] {
        match (self.os, self.arch) {
            (TargetOs::Windows, TargetArch::X86_64) => &["x86_64-pc-windows-msvc"],
            (TargetOs::Windows, TargetArch::Arm64) => &["aarch64-pc-windows-msvc"],
            _ => &[],
        }
    }

    pub const fn dotnet_runtime_identifier(self) -> &'static str {
        match (self.os, self.arch) {
            (TargetOs::Windows, TargetArch::X86_64) => "win-x64",
            (TargetOs::Windows, TargetArch::Arm64) => "win-arm64",
            (TargetOs::MacOs, TargetArch::X86_64) => "osx-x64",
            (TargetOs::MacOs, TargetArch::Arm64) => "osx-arm64",
            (TargetOs::Linux, TargetArch::X86_64) => "linux-x64",
            (TargetOs::Linux, TargetArch::Arm64) => "linux-arm64",
        }
    }

    pub const fn primary_linker(self) -> &'static str {
        match (self.os, self.arch) {
            (TargetOs::Windows, TargetArch::X86_64) => "x86_64-w64-mingw32-gcc",
            (TargetOs::Windows, TargetArch::Arm64) => "aarch64-w64-mingw32-gcc",
            (TargetOs::MacOs, _) => "clang",
            (TargetOs::Linux, TargetArch::X86_64) => "x86_64-linux-gnu-gcc",
            (TargetOs::Linux, TargetArch::Arm64) => "aarch64-linux-gnu-gcc",
        }
    }

    pub fn identifier(self) -> String {
        format!("{}-{}", self.os.as_str(), self.arch.as_str())
    }

    pub fn executable_name(self, product_name: &str) -> String {
        let base = if product_name.trim().is_empty() {
            "unitas-player"
        } else {
            product_name.trim()
        };
        match self.os {
            TargetOs::Windows => {
                if base.ends_with(".exe") {
                    base.to_string()
                } else {
                    format!("{base}.exe")
                }
            }
            TargetOs::MacOs | TargetOs::Linux => base.to_string(),
        }
    }

    pub fn status(self) -> TargetStatus {
        self.status_on(Self::current_host())
    }

    pub fn status_on_host(self) -> TargetStatus {
        self.status_on(Self::current_host())
    }

    pub fn status_for_host(self, host: Self) -> TargetStatus {
        self.status_on(Some(host))
    }

    pub fn status_on(self, host: Option<Self>) -> TargetStatus {
        match host {
            Some(current_host) if current_host == self => TargetStatus::Supported,
            Some(current_host) => TargetStatus::UnavailableOnCurrentHost {
                current_host: Some(current_host),
                reason: format!(
                    "target {self} requires host {self} or cross-compilation support, but current host is {current_host}"
                ),
            },
            None => TargetStatus::UnavailableOnCurrentHost {
                current_host: None,
                reason: format!(
                    "target {self} cannot be verified because current host OS/architecture is unrecognized"
                ),
            },
        }
    }

    pub fn is_available_on_host(self) -> bool {
        self.status().is_supported()
    }

    pub fn is_available_on(self, host: Option<Self>) -> bool {
        self.status_on(host).is_supported()
    }
}

impl fmt::Display for BuildTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {}",
            self.os.display_name(),
            self.arch.display_name()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetStatus {
    Supported,
    UnavailableOnCurrentHost {
        current_host: Option<BuildTarget>,
        reason: String,
    },
    Unsupported {
        reason: String,
    },
}

impl TargetStatus {
    pub const fn is_supported(&self) -> bool {
        matches!(self, TargetStatus::Supported)
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            TargetStatus::Supported => None,
            TargetStatus::UnavailableOnCurrentHost { reason, .. }
            | TargetStatus::Unsupported { reason } => Some(reason.as_str()),
        }
    }
}

impl fmt::Display for TargetStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetStatus::Supported => write!(formatter, "supported"),
            TargetStatus::UnavailableOnCurrentHost { reason, .. } => {
                write!(formatter, "unavailable on current host: {reason}")
            }
            TargetStatus::Unsupported { reason } => {
                write!(formatter, "unsupported: {reason}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_current_host_from_env_consts() {
        let host = BuildTarget::current_host();
        assert!(host.is_some());
        if let Some(target) = host {
            assert_eq!(target.status(), TargetStatus::Supported);
            assert!(target.is_available_on_host());
        }
    }

    #[test]
    fn marks_non_host_targets_unavailable_on_current_host() {
        let host = BuildTarget::MACOS_ARM64;
        assert_eq!(host.status_for_host(host), TargetStatus::Supported);

        for target in BuildTarget::all() {
            let status = target.status_for_host(host);
            if *target == host {
                assert_eq!(status, TargetStatus::Supported);
            } else {
                assert!(matches!(
                    status,
                    TargetStatus::UnavailableOnCurrentHost {
                        current_host: Some(resolved_host),
                        ref reason,
                    } if resolved_host == host && !reason.is_empty()
                ));
            }
        }
    }

    #[test]
    fn executable_name_matches_target_os_conventions() {
        assert_eq!(
            BuildTarget::WINDOWS_X86_64.executable_name("DemoGame"),
            "DemoGame.exe"
        );
        assert_eq!(
            BuildTarget::MACOS_ARM64.executable_name("DemoGame"),
            "DemoGame"
        );
        assert_eq!(
            BuildTarget::LINUX_X86_64.executable_name("DemoGame"),
            "DemoGame"
        );
    }
}
