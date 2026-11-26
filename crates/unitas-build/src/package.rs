//! macOS application bundle and Windows/Linux directory packaging for the
//! player (BLD-001, BLD-002).

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::bundle::{
    BundleManifest, BUNDLE_MANIFEST_FILENAME, DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME,
    MANAGED_DIR_NAME,
};
use crate::licensing::{third_party_notices_text, LicensingRecord};
use crate::targets::BuildTarget;

pub const APP_BUNDLE_EXTENSION: &str = "app";
pub const CONTENTS_DIR_NAME: &str = "Contents";
pub const MACOS_DIR_NAME: &str = "MacOS";
pub const RESOURCES_DIR_NAME: &str = "Resources";
pub const INFO_PLIST_FILENAME: &str = "Info.plist";
pub const PLAYER_BINARY_FILENAME: &str = "unitas-player";
pub const LICENSE_TEXT_FILENAME: &str = "LICENSE";
pub const THIRD_PARTY_NOTICES_FILENAME: &str = "THIRD-PARTY-NOTICES.txt";
pub const STAGED_ASSETS_DIR_NAME: &str = "Assets";
/// Suffix of the per-game data directory in Windows/Linux layouts, matching
/// the convention players expect (`<Product>_Data`).
pub const DATA_DIR_SUFFIX: &str = "_Data";

/// A macOS application bundle produced from a staged player bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagedApp {
    pub app_path: PathBuf,
    pub info_plist_path: PathBuf,
    pub executable_path: PathBuf,
    pub player_binary_path: PathBuf,
    pub resources_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageError {
    Io { path: PathBuf, message: String },
    MissingPlayerBinary { path: PathBuf },
    CodeSign { path: PathBuf, message: String },
}

impl fmt::Display for PackageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageError::Io { path, message } => {
                write!(formatter, "I/O error at '{}': {message}", path.display())
            }
            PackageError::MissingPlayerBinary { path } => {
                write!(
                    formatter,
                    "the unitas player binary does not exist at '{}'",
                    path.display()
                )
            }
            PackageError::CodeSign { path, message } => {
                write!(
                    formatter,
                    "failed to code sign '{}': {message}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for PackageError {}

/// A Windows or Linux player directory produced from a staged player bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagedPlayer {
    pub app_dir: PathBuf,
    pub executable_path: PathBuf,
    pub data_dir: PathBuf,
    pub manifest_path: PathBuf,
}

/// The name the application bundle and its executable carry.
pub fn bundle_name(product_name: &str) -> String {
    let trimmed = product_name.trim();
    let base = if trimmed.is_empty() {
        "unitas-player"
    } else {
        trimmed
    };
    base.chars()
        .map(|character| match character {
            '/' | ':' => '-',
            other => other,
        })
        .collect()
}

/// The unitas player binary of the local build, when one has been built.
pub fn locate_player_binary() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join(PLAYER_BINARY_FILENAME));
            if let Some(parent) = directory.parent() {
                candidates.push(parent.join(PLAYER_BINARY_FILENAME));
            }
        }
    }
    let manifest_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    if let Some(root) = manifest_directory.parent().and_then(Path::parent) {
        for profile in ["debug", "release"] {
            candidates.push(
                root.join("target")
                    .join(profile)
                    .join(PLAYER_BINARY_FILENAME),
            );
        }
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

/// Packages a staged player bundle as a runnable macOS application bundle.
///
/// The bundle carries the player binary, the staged bundle contents in
/// `Contents/Resources`, a launcher that starts the player with those
/// contents, the license texts the licensing record refers to, and an
/// `Info.plist` that names the executable macOS runs.
pub fn package_macos_app(
    manifest: &BundleManifest,
    staged_dir: &Path,
    app_path: &Path,
    player_binary: &Path,
    licensing: &LicensingRecord,
    license_text: Option<&Path>,
) -> Result<PackagedApp, PackageError> {
    if !player_binary.is_file() {
        return Err(PackageError::MissingPlayerBinary {
            path: player_binary.to_path_buf(),
        });
    }
    let contents_dir = app_path.join(CONTENTS_DIR_NAME);
    let macos_dir = contents_dir.join(MACOS_DIR_NAME);
    let resources_dir = contents_dir.join(RESOURCES_DIR_NAME);
    create_dir_all(&macos_dir)?;
    create_dir_all(&resources_dir)?;

    copy_staged_bundle(staged_dir, &resources_dir)?;

    let player_binary_path = macos_dir.join(PLAYER_BINARY_FILENAME);
    copy_file(player_binary, &player_binary_path)?;
    make_executable(&player_binary_path)?;

    let executable_name = bundle_name(&manifest.product_name);
    let executable_path = macos_dir.join(&executable_name);
    write_file(
        &executable_path,
        launcher_script(&executable_name).as_bytes(),
    )?;
    make_executable(&executable_path)?;

    let info_plist_path = contents_dir.join(INFO_PLIST_FILENAME);
    write_file(
        &info_plist_path,
        info_plist(manifest, &executable_name).as_bytes(),
    )?;

    if let Some(text) = license_text {
        if text.is_file() {
            copy_file(text, &resources_dir.join(LICENSE_TEXT_FILENAME))?;
        }
    }
    write_file(
        &resources_dir.join(THIRD_PARTY_NOTICES_FILENAME),
        third_party_notices_text(licensing).as_bytes(),
    )?;

    ad_hoc_sign_if_needed(app_path)?;

    Ok(PackagedApp {
        app_path: app_path.to_path_buf(),
        info_plist_path,
        executable_path,
        player_binary_path,
        resources_dir,
    })
}

/// Packages a staged player bundle as a Windows player directory.
///
/// The layout follows the convention Windows players expect: the product
/// directory carries `<Product>.exe` next to `<Product>_Data/`, which holds
/// the staged bundle contents (manifest, assets, managed payload) and the
/// license texts.
pub fn package_windows_app(
    manifest: &BundleManifest,
    staged_dir: &Path,
    output_dir: &Path,
    player_binary: &Path,
    licensing: &LicensingRecord,
    license_text: Option<&Path>,
) -> Result<PackagedPlayer, PackageError> {
    package_player_directory(
        manifest,
        staged_dir,
        output_dir,
        player_binary,
        licensing,
        license_text,
        BuildTarget::WINDOWS_X86_64,
    )
}

/// Packages a staged player bundle as a Linux player directory.
///
/// The layout mirrors the Windows one: the product directory carries the
/// `<Product>` executable next to `<Product>_Data/` with the staged bundle
/// contents and license texts.
pub fn package_linux_app(
    manifest: &BundleManifest,
    staged_dir: &Path,
    output_dir: &Path,
    player_binary: &Path,
    licensing: &LicensingRecord,
    license_text: Option<&Path>,
) -> Result<PackagedPlayer, PackageError> {
    package_player_directory(
        manifest,
        staged_dir,
        output_dir,
        player_binary,
        licensing,
        license_text,
        BuildTarget::LINUX_X86_64,
    )
}

fn package_player_directory(
    manifest: &BundleManifest,
    staged_dir: &Path,
    output_dir: &Path,
    player_binary: &Path,
    licensing: &LicensingRecord,
    license_text: Option<&Path>,
    target: BuildTarget,
) -> Result<PackagedPlayer, PackageError> {
    if !player_binary.is_file() {
        return Err(PackageError::MissingPlayerBinary {
            path: player_binary.to_path_buf(),
        });
    }
    let base_name = bundle_name(&manifest.product_name);
    let app_dir = output_dir.join(&base_name);
    let data_dir = app_dir.join(format!("{base_name}{DATA_DIR_SUFFIX}"));
    create_dir_all(&data_dir)?;

    copy_staged_bundle(staged_dir, &data_dir)?;

    let executable_path = app_dir.join(target.executable_name(&manifest.product_name));
    copy_file(player_binary, &executable_path)?;
    make_executable(&executable_path)?;

    if let Some(text) = license_text {
        if text.is_file() {
            copy_file(text, &data_dir.join(LICENSE_TEXT_FILENAME))?;
        }
    }
    write_file(
        &data_dir.join(THIRD_PARTY_NOTICES_FILENAME),
        third_party_notices_text(licensing).as_bytes(),
    )?;

    let manifest_path = data_dir.join(BUNDLE_MANIFEST_FILENAME);
    Ok(PackagedPlayer {
        app_dir,
        executable_path,
        data_dir,
        manifest_path,
    })
}

/// Ad-hoc code-signs the app bundle when the host can sign and no signature
/// is present yet, so a locally packaged app stays launchable.
#[cfg(target_os = "macos")]
fn ad_hoc_sign_if_needed(app_path: &Path) -> Result<(), PackageError> {
    let already_signed = Command::new("codesign")
        .arg("--verify")
        .arg(app_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if already_signed {
        return Ok(());
    }
    let output = Command::new("codesign")
        .args(["--force", "--sign", "-"])
        .arg(app_path)
        .output()
        .map_err(|error| PackageError::CodeSign {
            path: app_path.to_path_buf(),
            message: error.to_string(),
        })?;
    if !output.status.success() {
        return Err(PackageError::CodeSign {
            path: app_path.to_path_buf(),
            message: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn ad_hoc_sign_if_needed(path: &Path) -> Result<(), PackageError> {
    let _ = path;
    Ok(())
}

fn launcher_script(executable_name: &str) -> String {
    format!(
        "#!/bin/sh\n\
         # Launcher for the {executable_name} player bundle: runs the staged\n\
         # bundle that ships inside this application.\n\
         here=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\n\
         exec \"$here/{PLAYER_BINARY_FILENAME}\" \"$here/../{RESOURCES_DIR_NAME}\" --windowed \"$@\"\n"
    )
}

fn info_plist(manifest: &BundleManifest, executable_name: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \x20 <key>CFBundleDevelopmentRegion</key>\n\
         \x20 <string>en</string>\n\
         \x20 <key>CFBundleExecutable</key>\n\
         \x20 <string>{executable}</string>\n\
         \x20 <key>CFBundleIdentifier</key>\n\
         \x20 <string>{identifier}</string>\n\
         \x20 <key>CFBundleInfoDictionaryVersion</key>\n\
         \x20 <string>6.0</string>\n\
         \x20 <key>CFBundleName</key>\n\
         \x20 <string>{product}</string>\n\
         \x20 <key>CFBundlePackageType</key>\n\
         \x20 <string>APPL</string>\n\
         \x20 <key>CFBundleShortVersionString</key>\n\
         \x20 <string>{version}</string>\n\
         \x20 <key>CFBundleVersion</key>\n\
         \x20 <string>{version}</string>\n\
         \x20 <key>NSHighResolutionCapable</key>\n\
         \x20 <true/>\n\
         </dict>\n\
         </plist>\n",
        executable = xml_escape(executable_name),
        identifier = xml_escape(&manifest.application_identifier),
        product = xml_escape(&manifest.product_name),
        version = xml_escape(&manifest.version),
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn copy_staged_bundle(staged_dir: &Path, resources_dir: &Path) -> Result<(), PackageError> {
    let manifest = staged_dir.join(BUNDLE_MANIFEST_FILENAME);
    if manifest.is_file() {
        copy_file(&manifest, &resources_dir.join(BUNDLE_MANIFEST_FILENAME))?;
    }
    let assets = staged_dir.join(STAGED_ASSETS_DIR_NAME);
    if assets.is_dir() {
        copy_directory(&assets, &resources_dir.join(STAGED_ASSETS_DIR_NAME))?;
    }
    let managed = staged_dir.join(MANAGED_DIR_NAME);
    if managed.is_dir() {
        copy_directory(&managed, &resources_dir.join(MANAGED_DIR_NAME))?;
    }
    let marker = staged_dir.join(DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME);
    if marker.is_file() {
        copy_file(
            &marker,
            &resources_dir.join(DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME),
        )?;
    }
    Ok(())
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), PackageError> {
    create_dir_all(destination)?;
    let entries = fs::read_dir(source).map_err(|error| PackageError::Io {
        path: source.to_path_buf(),
        message: error.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| PackageError::Io {
            path: source.to_path_buf(),
            message: error.to_string(),
        })?;
        let target = destination.join(entry.file_name());
        if entry.path().is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else {
            copy_file(&entry.path(), &target)?;
        }
    }
    Ok(())
}

fn copy_file(source: &Path, destination: &Path) -> Result<(), PackageError> {
    fs::copy(source, destination)
        .map(|_| ())
        .map_err(|error| PackageError::Io {
            path: destination.to_path_buf(),
            message: error.to_string(),
        })
}

fn write_file(path: &Path, contents: &[u8]) -> Result<(), PackageError> {
    fs::write(path, contents).map_err(|error| PackageError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

fn create_dir_all(path: &Path) -> Result<(), PackageError> {
    fs::create_dir_all(path).map_err(|error| PackageError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

#[cfg(unix)]
pub(crate) fn make_executable(path: &Path) -> Result<(), PackageError> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|error| PackageError::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).map_err(|error| PackageError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

#[cfg(not(unix))]
pub(crate) fn make_executable(path: &Path) -> Result<(), PackageError> {
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    use super::*;
    use crate::bundle::{stage_bundle_for_host, BUNDLE_MANIFEST_FILENAME};
    use crate::config::BuildConfiguration;
    use crate::licensing::licensing_record;
    use crate::targets::BuildTarget;

    fn test_output_dir(label: &str) -> Result<PathBuf, Box<dyn Error>> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .ok_or("the crate directory has no repository root")?;
        let directory = root
            .join("target")
            .join("unitas-build-tests")
            .join(format!("{label}_{}", std::process::id()));
        if directory.exists() {
            fs::remove_dir_all(&directory)?;
        }
        fs::create_dir_all(&directory)?;
        Ok(directory)
    }

    fn fixture_scene() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("scene_alpha.unity")
    }

    fn is_executable(path: &Path) -> Result<bool, Box<dyn Error>> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(path)?.permissions().mode();
            Ok(mode & 0o111 != 0)
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Ok(true)
        }
    }

    #[test]
    fn packaged_macos_app_carries_the_player_bundle_and_launches_windowed(
    ) -> Result<(), Box<dyn Error>> {
        let output_dir = test_output_dir("macos_app")?;
        let config = BuildConfiguration::new(
            vec![fixture_scene()],
            "Unitas",
            "PackagedGame",
            "com.unitas.packaged",
            "1.0.0",
            BuildTarget::MACOS_ARM64,
        );
        let staged = stage_bundle_for_host(&config, &output_dir, Some(BuildTarget::MACOS_ARM64))?;
        let app_path = staged
            .application_bundle_path
            .clone()
            .ok_or("a macOS build produces an application bundle")?;
        assert!(app_path.is_dir());
        assert_eq!(
            app_path.file_name().and_then(|name| name.to_str()),
            Some("PackagedGame.app")
        );
        assert!(staged.launch_artifact_path.starts_with(&app_path));

        let contents = app_path.join(CONTENTS_DIR_NAME);
        let macos = contents.join(MACOS_DIR_NAME);
        let resources = contents.join(RESOURCES_DIR_NAME);
        let info_plist_path = contents.join(INFO_PLIST_FILENAME);
        assert!(info_plist_path.is_file());
        let executable = macos.join("PackagedGame");
        assert!(executable.is_file());
        assert!(is_executable(&executable)?);
        assert!(macos.join(PLAYER_BINARY_FILENAME).is_file());
        assert!(resources.join(BUNDLE_MANIFEST_FILENAME).is_file());
        assert!(resources
            .join(STAGED_ASSETS_DIR_NAME)
            .join("scene_000_scene_alpha.unity")
            .is_file());
        assert!(resources.join(THIRD_PARTY_NOTICES_FILENAME).is_file());
        let info_plist = fs::read_to_string(&info_plist_path)?;
        assert!(info_plist.contains("<key>CFBundleExecutable</key>"));
        assert!(info_plist.contains("<string>PackagedGame</string>"));
        assert!(info_plist.contains("<string>com.unitas.packaged</string>"));

        let launcher = fs::read_to_string(&executable)?;
        assert!(
            launcher.contains("--windowed"),
            "the packaged launcher does not start the player windowed: {launcher}"
        );
        assert!(
            launcher.contains("\"$@\""),
            "the packaged launcher does not forward caller arguments: {launcher}"
        );

        #[cfg(target_os = "macos")]
        {
            let output = Command::new(macos.join(PLAYER_BINARY_FILENAME))
                .arg(&resources)
                .arg("--frames")
                .arg("1")
                .output()?;
            assert!(
                output.status.success(),
                "the packaged player exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
            let stdout = String::from_utf8(output.stdout)?;
            assert!(
                stdout.contains("launched_product=PackagedGame"),
                "unexpected startup output: {stdout}"
            );
            assert!(
                stdout.contains("frames_stepped=1"),
                "unexpected startup output: {stdout}"
            );

            let launcher_run = Command::new(&executable)
                .arg("--control-stdio")
                .stdin(std::process::Stdio::null())
                .output()?;
            assert!(
                launcher_run.status.success(),
                "the packaged launcher exited with {}: {}",
                launcher_run.status,
                String::from_utf8_lossy(&launcher_run.stderr)
            );
        }
        Ok(())
    }

    fn dummy_player_binary(output_dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
        let binary = output_dir.join("unitas-player-dummy");
        fs::write(&binary, b"unitas dummy player binary")?;
        Ok(binary)
    }

    #[test]
    fn packaged_windows_app_lays_out_executable_and_data_directory() -> Result<(), Box<dyn Error>> {
        let output_dir = test_output_dir("windows_app")?;
        let config = BuildConfiguration::new(
            vec![fixture_scene()],
            "Unitas",
            "PackagedGame",
            "com.unitas.packaged",
            "1.0.0",
            BuildTarget::MACOS_ARM64,
        );
        let staged = stage_bundle_for_host(&config, &output_dir, Some(BuildTarget::MACOS_ARM64))?;
        let licensing = licensing_record("com.unitas.packaged");
        let binary = dummy_player_binary(&output_dir)?;
        let packaged = package_windows_app(
            &staged.manifest,
            &output_dir,
            &output_dir.join("windows-layout"),
            &binary,
            &licensing,
            None,
        )?;

        assert_eq!(
            packaged.app_dir,
            output_dir.join("windows-layout").join("PackagedGame")
        );
        assert_eq!(
            packaged.executable_path,
            packaged.app_dir.join("PackagedGame.exe")
        );
        assert!(packaged.executable_path.is_file());
        assert!(is_executable(&packaged.executable_path)?);
        assert_eq!(
            packaged.data_dir,
            packaged.app_dir.join("PackagedGame_Data")
        );
        assert_eq!(
            packaged.manifest_path,
            packaged.data_dir.join(BUNDLE_MANIFEST_FILENAME)
        );
        assert!(packaged.manifest_path.is_file());
        assert!(packaged
            .data_dir
            .join(STAGED_ASSETS_DIR_NAME)
            .join("scene_000_scene_alpha.unity")
            .is_file());
        assert!(packaged
            .data_dir
            .join(THIRD_PARTY_NOTICES_FILENAME)
            .is_file());
        Ok(())
    }

    #[test]
    fn packaged_linux_app_lays_out_executable_and_data_directory() -> Result<(), Box<dyn Error>> {
        let output_dir = test_output_dir("linux_app")?;
        let config = BuildConfiguration::new(
            vec![fixture_scene()],
            "Unitas",
            "PackagedGame",
            "com.unitas.packaged",
            "1.0.0",
            BuildTarget::MACOS_ARM64,
        );
        let staged = stage_bundle_for_host(&config, &output_dir, Some(BuildTarget::MACOS_ARM64))?;
        let licensing = licensing_record("com.unitas.packaged");
        let binary = dummy_player_binary(&output_dir)?;
        let packaged = package_linux_app(
            &staged.manifest,
            &output_dir,
            &output_dir.join("linux-layout"),
            &binary,
            &licensing,
            None,
        )?;

        assert_eq!(
            packaged.app_dir,
            output_dir.join("linux-layout").join("PackagedGame")
        );
        assert_eq!(
            packaged.executable_path,
            packaged.app_dir.join("PackagedGame")
        );
        assert!(packaged.executable_path.is_file());
        assert!(is_executable(&packaged.executable_path)?);
        assert_eq!(
            packaged.data_dir,
            packaged.app_dir.join("PackagedGame_Data")
        );
        assert!(packaged.manifest_path.is_file());
        assert!(packaged
            .data_dir
            .join(STAGED_ASSETS_DIR_NAME)
            .join("scene_000_scene_alpha.unity")
            .is_file());
        assert!(packaged
            .data_dir
            .join(THIRD_PARTY_NOTICES_FILENAME)
            .is_file());
        Ok(())
    }

    #[test]
    fn windows_and_linux_packaging_fail_without_a_player_binary() -> Result<(), Box<dyn Error>> {
        let output_dir = test_output_dir("desktop_app_without_binary")?;
        let config = BuildConfiguration::new(
            vec![fixture_scene()],
            "Unitas",
            "PackagedGame",
            "com.unitas.packaged",
            "1.0.0",
            BuildTarget::MACOS_ARM64,
        );
        let staged = stage_bundle_for_host(&config, &output_dir, Some(BuildTarget::MACOS_ARM64))?;
        let licensing = licensing_record("com.unitas.packaged");
        for packaged in [
            package_windows_app(
                &staged.manifest,
                &output_dir,
                &output_dir.join("windows-layout"),
                Path::new("target/does-not-exist/unitas-player"),
                &licensing,
                None,
            ),
            package_linux_app(
                &staged.manifest,
                &output_dir,
                &output_dir.join("linux-layout"),
                Path::new("target/does-not-exist/unitas-player"),
                &licensing,
                None,
            ),
        ] {
            assert!(matches!(
                packaged,
                Err(PackageError::MissingPlayerBinary { .. })
            ));
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn packaged_macos_app_carries_an_ad_hoc_code_signature() -> Result<(), Box<dyn Error>> {
        let output_dir = test_output_dir("macos_app_signed")?;
        let config = BuildConfiguration::new(
            vec![fixture_scene()],
            "Unitas",
            "PackagedGame",
            "com.unitas.packaged",
            "1.0.0",
            BuildTarget::MACOS_ARM64,
        );
        let staged = stage_bundle_for_host(&config, &output_dir, Some(BuildTarget::MACOS_ARM64))?;
        let app_path = staged
            .application_bundle_path
            .clone()
            .ok_or("a macOS build produces an application bundle")?;
        let verify = Command::new("codesign")
            .arg("--verify")
            .arg("--deep")
            .arg(&app_path)
            .output()?;
        assert!(
            verify.status.success(),
            "the packaged app has no valid code signature: {}",
            String::from_utf8_lossy(&verify.stderr)
        );
        Ok(())
    }

    #[test]
    fn packaging_fails_without_a_player_binary() -> Result<(), Box<dyn Error>> {
        let output_dir = test_output_dir("macos_app_without_binary")?;
        let config = BuildConfiguration::new(
            vec![fixture_scene()],
            "Unitas",
            "PackagedGame",
            "com.unitas.packaged",
            "1.0.0",
            BuildTarget::MACOS_ARM64,
        );
        let staged = stage_bundle_for_host(&config, &output_dir, Some(BuildTarget::MACOS_ARM64))?;
        let licensing = licensing_record("com.unitas.packaged");
        let result = package_macos_app(
            &staged.manifest,
            &output_dir,
            &output_dir.join("PackagedGame.app"),
            Path::new("target/does-not-exist/unitas-player"),
            &licensing,
            None,
        );
        assert!(matches!(
            result,
            Err(PackageError::MissingPlayerBinary { .. })
        ));
        Ok(())
    }
}
