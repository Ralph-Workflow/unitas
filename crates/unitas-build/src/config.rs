use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;

use crate::targets::{BuildTarget, TargetStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildMode {
    Development,
    Release,
}

impl BuildMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            BuildMode::Development => "development",
            BuildMode::Release => "release",
        }
    }

    pub const fn is_development(self) -> bool {
        matches!(self, BuildMode::Development)
    }
}

impl fmt::Display for BuildMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductIdentity {
    pub company_name: String,
    pub product_name: String,
    pub application_identifier: String,
    pub version: String,
}

impl ProductIdentity {
    pub fn new(
        company_name: impl Into<String>,
        product_name: impl Into<String>,
        application_identifier: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            company_name: company_name.into(),
            product_name: product_name.into(),
            application_identifier: application_identifier.into(),
            version: version.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildValidationError {
    pub setting: String,
    pub consequence: String,
    pub remedy: String,
}

impl BuildValidationError {
    pub fn new(
        setting: impl Into<String>,
        consequence: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self {
            setting: setting.into(),
            consequence: consequence.into(),
            remedy: remedy.into(),
        }
    }
}

impl fmt::Display for BuildValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid build setting '{}': {} (remedy: {})",
            self.setting, self.consequence, self.remedy
        )
    }
}

impl std::error::Error for BuildValidationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildConfiguration {
    pub scenes: Vec<PathBuf>,
    pub company_name: String,
    pub product_name: String,
    pub application_identifier: String,
    pub version: String,
    pub target: BuildTarget,
    pub development_build: bool,
    pub development_diagnostics: bool,
}

impl BuildConfiguration {
    pub fn new(
        scenes: Vec<PathBuf>,
        company_name: impl Into<String>,
        product_name: impl Into<String>,
        application_identifier: impl Into<String>,
        version: impl Into<String>,
        target: BuildTarget,
    ) -> Self {
        Self {
            scenes,
            company_name: company_name.into(),
            product_name: product_name.into(),
            application_identifier: application_identifier.into(),
            version: version.into(),
            target,
            development_build: false,
            development_diagnostics: false,
        }
    }

    pub fn for_host(scenes: Vec<PathBuf>, product_name: impl Into<String>) -> Self {
        let product = product_name.into();
        let identifier = format!("com.default.{}", product.to_lowercase().replace(' ', "-"));
        Self::new(
            scenes,
            "DefaultCompany",
            product,
            identifier,
            "0.1.0",
            BuildTarget::host(),
        )
    }

    pub fn with_mode(mut self, mode: BuildMode) -> Self {
        let is_dev = mode.is_development();
        self.development_build = is_dev;
        self.development_diagnostics = is_dev;
        self
    }

    pub fn with_development_build(mut self, enabled: bool) -> Self {
        self.development_build = enabled;
        self.development_diagnostics = enabled;
        self
    }

    pub fn with_development_diagnostics(mut self, enabled: bool) -> Self {
        self.development_diagnostics = enabled;
        self
    }

    pub const fn mode(&self) -> BuildMode {
        if self.development_build {
            BuildMode::Development
        } else {
            BuildMode::Release
        }
    }

    pub const fn is_development(&self) -> bool {
        self.development_build
    }

    pub const fn include_diagnostics(&self) -> bool {
        self.development_build && self.development_diagnostics
    }

    pub fn startup_scene(&self) -> Option<&PathBuf> {
        self.scenes.first()
    }

    pub fn identity(&self) -> ProductIdentity {
        ProductIdentity {
            company_name: self.company_name.clone(),
            product_name: self.product_name.clone(),
            application_identifier: self.application_identifier.clone(),
            version: self.version.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), Vec<BuildValidationError>> {
        self.validate_with_host(BuildTarget::current_host())
    }

    pub fn validate_with_host(
        &self,
        host: Option<BuildTarget>,
    ) -> Result<(), Vec<BuildValidationError>> {
        let mut errors = Vec::new();

        if self.scenes.is_empty() {
            errors.push(BuildValidationError::new(
                "scenes",
                "no startup scene is configured, so the built player has no initial scene to launch",
                "add at least one '.unity' scene path to BuildConfiguration::scenes",
            ));
        } else {
            let mut seen = BTreeSet::new();
            for (index, scene_path) in self.scenes.iter().enumerate() {
                let raw = scene_path.as_os_str();
                let is_unity_ext = scene_path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("unity"));
                if raw.is_empty() || !is_unity_ext {
                    errors.push(BuildValidationError::new(
                        "scenes",
                        format!(
                            "scene entry at index {index} ('{}') is not a valid '.unity' scene path",
                            scene_path.display()
                        ),
                        "provide a non-empty file path ending with the '.unity' extension",
                    ));
                }
                if !seen.insert(scene_path.clone()) {
                    errors.push(BuildValidationError::new(
                        "scenes",
                        format!(
                            "duplicate scene path '{}' creates ambiguous build scene ordering",
                            scene_path.display()
                        ),
                        "remove the duplicate scene entry so each included scene path appears once",
                    ));
                }
            }
        }

        if self.company_name.trim().is_empty() {
            errors.push(BuildValidationError::new(
                "company_name",
                "company name is empty, leaving product metadata incomplete",
                "set company_name to a non-empty company or studio identifier",
            ));
        }

        if self.product_name.trim().is_empty() {
            errors.push(BuildValidationError::new(
                "product_name",
                "product name is empty, so output bundle and executable names cannot be determined",
                "set product_name to a non-empty application name",
            ));
        }

        if self.application_identifier.trim().is_empty() {
            errors.push(BuildValidationError::new(
                "application_identifier",
                "application identifier is empty, leaving package identity undefined",
                "set application_identifier to a non-empty reverse-DNS identifier such as 'com.company.product'",
            ));
        }

        if self.version.trim().is_empty() {
            errors.push(BuildValidationError::new(
                "version",
                "version string is empty, so build output and report lack release version identity",
                "set version to a non-empty semantic or platform version string such as '0.1.0'",
            ));
        }

        match self.target.status_on(host) {
            TargetStatus::Supported => {}
            TargetStatus::UnavailableOnCurrentHost {
                current_host,
                reason,
            } => {
                let remedy = match current_host {
                    Some(active_host) => format!(
                        "select the current host target ({active_host}) or run the build on a {} host",
                        self.target
                    ),
                    None => format!(
                        "run the build on a supported {} host environment",
                        self.target
                    ),
                };
                errors.push(BuildValidationError::new("target", reason, remedy));
            }
            TargetStatus::Unsupported { reason } => {
                errors.push(BuildValidationError::new(
                    "target",
                    reason,
                    "select a supported desktop target matching the current host",
                ));
            }
        }

        if !self.development_build && self.development_diagnostics {
            errors.push(BuildValidationError::new(
                "development_diagnostics",
                "development diagnostics are enabled on a release build, which would leak development-only access into release output",
                "disable development_diagnostics for release builds or enable development_build",
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config_for(host: BuildTarget) -> BuildConfiguration {
        BuildConfiguration::new(
            vec![PathBuf::from("Assets/Scenes/Main.unity")],
            "Unitas",
            "SampleGame",
            "com.unitas.sample",
            "1.0.0",
            host,
        )
    }

    #[test]
    fn valid_release_and_development_configs_pass_validation() {
        let host = BuildTarget::MACOS_ARM64;
        let release = valid_config_for(host);
        assert_eq!(release.validate_with_host(Some(host)), Ok(()));
        assert_eq!(release.mode(), BuildMode::Release);
        assert!(!release.include_diagnostics());

        let dev = valid_config_for(host).with_development_build(true);
        assert_eq!(dev.validate_with_host(Some(host)), Ok(()));
        assert_eq!(dev.mode(), BuildMode::Development);
        assert!(dev.include_diagnostics());
    }

    #[test]
    fn empty_scene_list_reports_scenes_setting_consequence_and_remedy() {
        let host = BuildTarget::MACOS_ARM64;
        let mut config = valid_config_for(host);
        config.scenes.clear();

        let errors = config.validate_with_host(Some(host)).unwrap_err();
        assert!(errors.iter().any(|err| {
            err.setting == "scenes" && !err.consequence.is_empty() && !err.remedy.is_empty()
        }));
    }

    #[test]
    fn non_unity_scene_path_reports_scenes_setting_consequence_and_remedy() {
        let host = BuildTarget::MACOS_ARM64;
        let mut config = valid_config_for(host);
        config.scenes = vec![PathBuf::from("Assets/Scenes/Main.txt")];

        let errors = config.validate_with_host(Some(host)).unwrap_err();
        assert!(errors.iter().any(|err| {
            err.setting == "scenes" && !err.consequence.is_empty() && !err.remedy.is_empty()
        }));
    }

    #[test]
    fn duplicate_scene_path_reports_scenes_setting_consequence_and_remedy() {
        let host = BuildTarget::MACOS_ARM64;
        let mut config = valid_config_for(host);
        config.scenes = vec![
            PathBuf::from("Assets/Scenes/Main.unity"),
            PathBuf::from("Assets/Scenes/Main.unity"),
        ];

        let errors = config.validate_with_host(Some(host)).unwrap_err();
        assert!(errors.iter().any(|err| {
            err.setting == "scenes"
                && err.consequence.contains("duplicate")
                && !err.remedy.is_empty()
        }));
    }

    #[test]
    fn empty_identity_and_version_fields_report_individual_settings() {
        let host = BuildTarget::MACOS_ARM64;
        let mut config = valid_config_for(host);
        config.company_name = "   ".to_string();
        config.product_name = String::new();
        config.application_identifier = String::new();
        config.version = String::new();

        let errors = config.validate_with_host(Some(host)).unwrap_err();
        for expected_setting in [
            "company_name",
            "product_name",
            "application_identifier",
            "version",
        ] {
            assert!(errors.iter().any(|err| {
                err.setting == expected_setting
                    && !err.consequence.is_empty()
                    && !err.remedy.is_empty()
            }));
        }
    }

    #[test]
    fn non_host_target_reports_target_setting_consequence_and_remedy() {
        let host = BuildTarget::MACOS_ARM64;
        let mut config = valid_config_for(host);
        config.target = BuildTarget::WINDOWS_X86_64;

        let errors = config.validate_with_host(Some(host)).unwrap_err();
        assert!(errors.iter().any(|err| {
            err.setting == "target" && !err.consequence.is_empty() && !err.remedy.is_empty()
        }));
    }

    #[test]
    fn diagnostics_enabled_on_release_build_reports_invalid_combination() {
        let host = BuildTarget::MACOS_ARM64;
        let mut config = valid_config_for(host);
        config.development_build = false;
        config.development_diagnostics = true;

        let errors = config.validate_with_host(Some(host)).unwrap_err();
        assert!(errors.iter().any(|err| {
            err.setting == "development_diagnostics"
                && !err.consequence.is_empty()
                && !err.remedy.is_empty()
        }));
    }
}
