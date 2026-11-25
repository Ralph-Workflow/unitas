//! LIC-001: the license a conveyed player is governed by, the obligations that
//! license carries, and the third-party notices the player includes.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::report::{parse_json_document, ReportParseError};

pub const GPLV3_LICENSE_ID: &str = "GPL-3.0-only";
pub const GPLV3_LICENSE_NAME: &str = "GNU General Public License version 3 (GPLv3)";
pub const UNRESOLVED_LICENSE: &str = "UNRESOLVED";
pub const CARGO_ECOSYSTEM: &str = "cargo";
pub const NUGET_ECOSYSTEM: &str = "nuget";

pub const CONVEYANCE_STATEMENT: &str =
    "This player is conveyed under the GNU General Public License version 3 (GPLv3) as a work that includes the unitas player components.";
pub const CORRESPONDING_SOURCE_OBLIGATION: &str = "The obligation to provide the Corresponding Source extends to the whole conveyed work, including project code and content compiled or linked into it, except separate works merely aggregated with it.";
pub const CORRESPONDING_SOURCE_AVAILABILITY: &str =
    "The Corresponding Source for the whole conveyed work is offered by the party that conveys this player, as the unitas source tree and the project source tree this player was built from; the player itself carries the GPLv3 license text and the third-party notices it refers to.";
pub const LINKING_EXCEPTION_STATEMENT: &str =
    "The GPLv3 distribution of unitas carries no linking exception.";
pub const COMMERCIAL_LICENSE_STATEMENT: &str =
    "The commercial license is the route for distributing a player without these GPLv3 obligations.";
pub const NOT_LEGAL_ADVICE_STATEMENT: &str =
    "This license report is product information and not legal advice.";
pub const LICENSE_TEXTS: [&str; 2] = [
    "GNU General Public License version 3 (Resources/LICENSE)",
    "Third-party notices (Resources/THIRD-PARTY-NOTICES.txt)",
];

/// One third-party component the conveyed player includes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThirdPartyNotice {
    pub ecosystem: String,
    pub component: String,
    pub version: String,
    pub license: String,
}

/// The licensing record a build result carries (LIC-001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicensingRecord {
    pub application_identifier: String,
    pub governing_license: String,
    pub governing_license_name: String,
    pub conveyance_statement: String,
    pub corresponding_source_obligation: String,
    pub corresponding_source_availability: String,
    pub linking_exception_statement: String,
    pub commercial_license_statement: String,
    pub not_legal_advice_statement: String,
    pub license_texts: Vec<String>,
    pub third_party_notices: Vec<ThirdPartyNotice>,
}

impl LicensingRecord {
    pub fn new(application_identifier: &str, third_party_notices: Vec<ThirdPartyNotice>) -> Self {
        Self {
            application_identifier: application_identifier.to_string(),
            governing_license: GPLV3_LICENSE_ID.to_string(),
            governing_license_name: GPLV3_LICENSE_NAME.to_string(),
            conveyance_statement: CONVEYANCE_STATEMENT.to_string(),
            corresponding_source_obligation: CORRESPONDING_SOURCE_OBLIGATION.to_string(),
            corresponding_source_availability: CORRESPONDING_SOURCE_AVAILABILITY.to_string(),
            linking_exception_statement: LINKING_EXCEPTION_STATEMENT.to_string(),
            commercial_license_statement: COMMERCIAL_LICENSE_STATEMENT.to_string(),
            not_legal_advice_statement: NOT_LEGAL_ADVICE_STATEMENT.to_string(),
            license_texts: LICENSE_TEXTS
                .iter()
                .map(|text| (*text).to_string())
                .collect(),
            third_party_notices,
        }
    }
}

/// A component entry of `Cargo.lock`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoLockPackage {
    pub name: String,
    pub version: String,
    pub source: Option<String>,
}

/// A component entry of a NuGet `packages.lock.json` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NuGetLockPackage {
    pub id: String,
    pub version: String,
}

/// The repository root that holds `Cargo.lock` and the editor project.
pub fn workspace_root() -> Option<PathBuf> {
    if let Ok(executable) = std::env::current_exe() {
        let mut directory = executable.parent();
        for _ in 0..6 {
            let Some(current) = directory else { break };
            if current.join("Cargo.lock").is_file() && current.join("crates").is_dir() {
                return Some(current.to_path_buf());
            }
            directory = current.parent();
        }
    }
    let manifest_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest_directory.parent().and_then(Path::parent);
    match candidate {
        Some(root) if root.join("Cargo.lock").is_file() => Some(root.to_path_buf()),
        _ => None,
    }
}

/// The licensing record for a build of `application_identifier`.
pub fn licensing_record(application_identifier: &str) -> LicensingRecord {
    let notices = match workspace_root() {
        Some(root) => third_party_notices(&root),
        None => Vec::new(),
    };
    LicensingRecord::new(application_identifier, notices)
}

/// The third-party notices included in a player, sorted for reproducible output.
pub fn third_party_notices(root: &Path) -> Vec<ThirdPartyNotice> {
    let mut notices = cargo_notices(root);
    notices.extend(nuget_notices(root));
    notices.sort_by(|left, right| {
        left.ecosystem
            .cmp(&right.ecosystem)
            .then_with(|| left.component.cmp(&right.component))
            .then_with(|| left.version.cmp(&right.version))
    });
    notices.dedup_by(|left, right| {
        left.ecosystem == right.ecosystem
            && left.component == right.component
            && left.version == right.version
    });
    notices
}

/// The notices file a conveyed player carries.
pub fn third_party_notices_text(record: &LicensingRecord) -> String {
    let mut text = format!(
        "Third-party notices for {}\nGoverning license: {} ({})\n\nIncluded third-party components:\n",
        record.application_identifier, record.governing_license_name, record.governing_license
    );
    for notice in &record.third_party_notices {
        text.push_str(&format!(
            "{} {} ({}) - {}\n",
            notice.component, notice.version, notice.ecosystem, notice.license
        ));
    }
    text.push_str(&format!("\n{}\n", record.not_legal_advice_statement));
    text
}

/// The `Cargo.lock` packages of a lock file.
pub fn cargo_lock_packages(text: &str) -> Vec<CargoLockPackage> {
    let mut packages: Vec<CargoLockPackage> = Vec::new();
    let mut current: Option<CargoLockPackage> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            if let Some(package) = current.take() {
                packages.push(package);
            }
            current = Some(CargoLockPackage {
                name: String::new(),
                version: String::new(),
                source: None,
            });
            continue;
        }
        let Some(package) = current.as_mut() else {
            continue;
        };
        if let Some(value) = trimmed.strip_prefix("name = ") {
            package.name = unquote(value);
        } else if let Some(value) = trimmed.strip_prefix("version = ") {
            package.version = unquote(value);
        } else if let Some(value) = trimmed.strip_prefix("source = ") {
            package.source = Some(unquote(value));
        }
    }
    if let Some(package) = current.take() {
        packages.push(package);
    }
    packages
}

/// The NuGet packages of a `packages.lock.json` document.
pub fn nuget_lock_packages(text: &str) -> Result<Vec<NuGetLockPackage>, ReportParseError> {
    let document = parse_json_document(text)?;
    let root = document.as_object("packages.lock.json")?;
    let frameworks = root
        .get("dependencies")
        .ok_or_else(|| ReportParseError::new("packages.lock.json has no 'dependencies' object"))?
        .as_object("dependencies")?;
    let mut packages = Vec::new();
    for entries in frameworks.values() {
        let entries = entries.as_object("dependencies")?;
        for (id, entry) in entries {
            let entry = entry.as_object(id)?;
            if entry
                .get("type")
                .and_then(|value| value.as_str("type").ok())
                == Some("Project")
            {
                continue;
            }
            let version = entry
                .get("resolved")
                .and_then(|value| value.as_str("resolved").ok())
                .unwrap_or_default()
                .to_string();
            packages.push(NuGetLockPackage {
                id: id.clone(),
                version,
            });
        }
    }
    Ok(packages)
}

fn cargo_notices(root: &Path) -> Vec<ThirdPartyNotice> {
    let Ok(text) = fs::read_to_string(root.join("Cargo.lock")) else {
        return Vec::new();
    };
    let workspace_license = repository_license(root);
    let registry = cargo_registry_index();
    cargo_lock_packages(&text)
        .into_iter()
        .map(|package| {
            let license = match package.source {
                None => workspace_license.clone(),
                Some(_) => registry_license(&registry, &package.name, &package.version),
            };
            ThirdPartyNotice {
                ecosystem: CARGO_ECOSYSTEM.to_string(),
                component: package.name,
                version: package.version,
                license,
            }
        })
        .collect()
}

fn nuget_notices(root: &Path) -> Vec<ThirdPartyNotice> {
    let lock_path = root
        .join("editor")
        .join("Unitas.Editor")
        .join("packages.lock.json");
    let Ok(text) = fs::read_to_string(&lock_path) else {
        return Vec::new();
    };
    let Ok(packages) = nuget_lock_packages(&text) else {
        return Vec::new();
    };
    packages
        .into_iter()
        .map(|package| ThirdPartyNotice {
            ecosystem: NUGET_ECOSYSTEM.to_string(),
            license: nuget_license(&package.id, &package.version),
            component: package.id,
            version: package.version,
        })
        .collect()
}

/// The license the repository itself is under, read from its license file.
fn repository_license(root: &Path) -> String {
    let Ok(text) = fs::read_to_string(root.join("LICENSE")) else {
        return UNRESOLVED_LICENSE.to_string();
    };
    let head = text.chars().take(4096).collect::<String>();
    if head.contains("GNU GENERAL PUBLIC LICENSE") && head.contains("Version 3") {
        GPLV3_LICENSE_ID.to_string()
    } else {
        UNRESOLVED_LICENSE.to_string()
    }
}

/// The unpacked registry sources of the local cargo home, keyed by `<name>-<version>`.
fn cargo_registry_index() -> BTreeMap<String, PathBuf> {
    let mut index = BTreeMap::new();
    let source_root = cargo_home().join("registry").join("src");
    let Ok(registries) = fs::read_dir(&source_root) else {
        return index;
    };
    for registry in registries.flatten() {
        let Ok(packages) = fs::read_dir(registry.path()) else {
            continue;
        };
        for package in packages.flatten() {
            let path = package.path();
            if !path.is_dir() {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                index.insert(name.to_string(), path);
            }
        }
    }
    index
}

fn cargo_home() -> PathBuf {
    match std::env::var_os("CARGO_HOME") {
        Some(home) => PathBuf::from(home),
        None => home_directory().join(".cargo"),
    }
}

fn nuget_packages_directory() -> PathBuf {
    match std::env::var_os("NUGET_PACKAGES") {
        Some(directory) => PathBuf::from(directory),
        None => home_directory().join(".nuget").join("packages"),
    }
}

fn home_directory() -> PathBuf {
    std::env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from)
}

fn registry_license(registry: &BTreeMap<String, PathBuf>, name: &str, version: &str) -> String {
    let key = format!("{name}-{version}");
    let Some(directory) = registry.get(&key) else {
        return UNRESOLVED_LICENSE.to_string();
    };
    let Ok(text) = fs::read_to_string(directory.join("Cargo.toml")) else {
        return UNRESOLVED_LICENSE.to_string();
    };
    manifest_license(&text).unwrap_or_else(|| UNRESOLVED_LICENSE.to_string())
}

fn manifest_license(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("license = ") {
            return Some(unquote(value));
        }
        if let Some(value) = trimmed.strip_prefix("license-file = ") {
            return Some(format!("license-file:{}", unquote(value)));
        }
    }
    None
}

fn nuget_license(id: &str, version: &str) -> String {
    let directory = nuget_packages_directory()
        .join(id.to_lowercase())
        .join(version);
    let nuspec = directory.join(format!("{}.nuspec", id.to_lowercase()));
    let Ok(text) = fs::read_to_string(&nuspec) else {
        return UNRESOLVED_LICENSE.to_string();
    };
    nuspec_license(&text).unwrap_or_else(|| UNRESOLVED_LICENSE.to_string())
}

fn nuspec_license(text: &str) -> Option<String> {
    for tag in ["<license ", "<license>"] {
        let Some(start) = text.find(tag) else {
            continue;
        };
        let rest = &text[start..];
        let Some(open) = rest.find('>') else {
            continue;
        };
        let Some(close) = rest[open..].find("</license>") else {
            continue;
        };
        let value = rest[open + 1..open + close].trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    let start = text.find("<licenseUrl>")?;
    let close = text[start..].find("</licenseUrl>")?;
    let value = text[start + "<licenseUrl>".len()..start + close].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_lock_packages_are_read_from_package_tables() {
        let text = "\
# comment
version = 4

[[package]]
name = \"alpha\"
version = \"1.2.3\"
source = \"registry+https://github.com/rust-lang/crates.io-index\"
dependencies = [
 \"beta\",
]

[[package]]
name = \"local\"
version = \"0.1.0\"
";
        let packages = cargo_lock_packages(text);
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "alpha");
        assert_eq!(packages[0].version, "1.2.3");
        assert!(packages[0].source.is_some());
        assert_eq!(packages[1].name, "local");
        assert!(packages[1].source.is_none());
    }

    #[test]
    fn nuget_lock_packages_are_read_from_every_framework() -> Result<(), Box<dyn std::error::Error>>
    {
        let text = "\
{
  \"version\": 1,
  \"dependencies\": {
    \"net8.0\": {
      \"Alpha\": { \"type\": \"Direct\", \"resolved\": \"1.0.0\" },
      \"Beta\": { \"type\": \"Transitive\", \"resolved\": \"2.0.0\" }
    }
  }
}
";
        let packages = nuget_lock_packages(text)?;
        assert_eq!(packages.len(), 2);
        assert!(packages
            .iter()
            .any(|package| package.id == "Alpha" && package.version == "1.0.0"));
        assert!(packages
            .iter()
            .any(|package| package.id == "Beta" && package.version == "2.0.0"));
        Ok(())
    }

    #[test]
    fn nuget_lock_packages_skip_project_references() -> Result<(), Box<dyn std::error::Error>> {
        let text = "\
{
  \"version\": 1,
  \"dependencies\": {
    \"net8.0\": {
      \"Alpha\": { \"type\": \"Direct\", \"resolved\": \"1.0.0\" },
      \"unitas.bridge\": { \"type\": \"Project\", \"dependencies\": {} }
    }
  }
}
";
        let packages = nuget_lock_packages(text)?;
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].id, "Alpha");
        Ok(())
    }

    #[test]
    fn notices_are_sorted_and_carry_every_licensing_field() {
        let record = licensing_record("com.unitas.example");
        assert_eq!(record.application_identifier, "com.unitas.example");
        assert_eq!(record.governing_license, GPLV3_LICENSE_ID);
        assert!(record.conveyance_statement.contains("conveyed under"));
        assert!(record
            .corresponding_source_obligation
            .contains("whole conveyed work"));
        assert!(record
            .linking_exception_statement
            .contains("no linking exception"));
        assert!(record
            .not_legal_advice_statement
            .contains("not legal advice"));
        assert_eq!(record.license_texts.len(), LICENSE_TEXTS.len());
        assert!(!record.third_party_notices.is_empty());
        let mut sorted = record.third_party_notices.clone();
        sorted.sort_by(|left, right| {
            left.ecosystem
                .cmp(&right.ecosystem)
                .then_with(|| left.component.cmp(&right.component))
                .then_with(|| left.version.cmp(&right.version))
        });
        assert_eq!(sorted, record.third_party_notices);
        assert!(record
            .third_party_notices
            .iter()
            .all(|notice| !notice.component.is_empty() && !notice.version.is_empty()));
    }
}
