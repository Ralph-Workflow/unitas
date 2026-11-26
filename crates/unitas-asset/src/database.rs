use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::dependencies::{AssetImportState, ImportDiagnostic, ImportOutcome};
use crate::font::{self, FontImportSettings};
use crate::project::{build_inventory, AssetEntry, AssetKind, Project};
use crate::unity_yaml::{self, Node};

const TRASH_DIRECTORY: &str = ".unitas/trash";

const TEXT_ASSET_EXTENSIONS: [&str; 9] = [
    "txt", "json", "bytes", "csv", "xml", "html", "htm", "yaml", "fnt",
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssetFilter {
    pub name_contains: Option<String>,
    pub class: Option<String>,
    pub label: Option<String>,
    pub folder: Option<PathBuf>,
    pub state: Option<AssetImportState>,
    pub include_sub_assets: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextAssetContent {
    pub name: String,
    pub text: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecognizedAssetFamily {
    TextAsset,
    AvatarMask,
    AudioMixer,
    SpriteAtlasV1,
    SpriteAtlasV2,
    Scene,
    Prefab,
    Material,
    Shader,
    ComputeShader,
    ShaderInclude,
    ShaderVariantCollection,
    AnimationClip,
    AnimatorController,
    AnimatorOverrideController,
    Font,
    GuiSkin,
    Video,
    Script,
    ScriptableObject,
    PhysicMaterial,
    PhysicsMaterial2D,
    InputActions,
    TerrainData,
    TerrainLayer,
    LightingSettings,
    Cubemap,
    Flare,
    RenderTexture,
    Preset,
    Timeline,
    Signal,
    Texture2D,
    AudioClip,
    Model,
}

impl RecognizedAssetFamily {
    pub fn class_name(self) -> &'static str {
        match self {
            RecognizedAssetFamily::TextAsset => "TextAsset",
            RecognizedAssetFamily::AvatarMask => "AvatarMask",
            RecognizedAssetFamily::AudioMixer => "AudioMixerController",
            RecognizedAssetFamily::SpriteAtlasV1 => "SpriteAtlas",
            RecognizedAssetFamily::SpriteAtlasV2 => "SpriteAtlasAsset",
            RecognizedAssetFamily::Scene => "SceneAsset",
            RecognizedAssetFamily::Prefab => "Prefab",
            RecognizedAssetFamily::Material => "Material",
            RecognizedAssetFamily::Shader => "Shader",
            RecognizedAssetFamily::ComputeShader => "ComputeShader",
            RecognizedAssetFamily::ShaderInclude => "ShaderInclude",
            RecognizedAssetFamily::ShaderVariantCollection => "ShaderVariantCollection",
            RecognizedAssetFamily::AnimationClip => "AnimationClip",
            RecognizedAssetFamily::AnimatorController => "AnimatorController",
            RecognizedAssetFamily::AnimatorOverrideController => "AnimatorOverrideController",
            RecognizedAssetFamily::Font => "Font",
            RecognizedAssetFamily::GuiSkin => "GUISkin",
            RecognizedAssetFamily::Video => "VideoClip",
            RecognizedAssetFamily::Script => "MonoScript",
            RecognizedAssetFamily::ScriptableObject => "ScriptableObject",
            RecognizedAssetFamily::PhysicMaterial => "PhysicMaterial",
            RecognizedAssetFamily::PhysicsMaterial2D => "PhysicsMaterial2D",
            RecognizedAssetFamily::InputActions => "InputActionAsset",
            RecognizedAssetFamily::TerrainData => "TerrainData",
            RecognizedAssetFamily::TerrainLayer => "TerrainLayer",
            RecognizedAssetFamily::LightingSettings => "LightingSettings",
            RecognizedAssetFamily::Cubemap => "Cubemap",
            RecognizedAssetFamily::Flare => "Flare",
            RecognizedAssetFamily::RenderTexture => "RenderTexture",
            RecognizedAssetFamily::Preset => "Preset",
            RecognizedAssetFamily::Timeline => "TimelineAsset",
            RecognizedAssetFamily::Signal => "SignalAsset",
            RecognizedAssetFamily::Texture2D => "Texture2D",
            RecognizedAssetFamily::AudioClip => "AudioClip",
            RecognizedAssetFamily::Model => "Mesh",
        }
    }

    pub fn family_label(self) -> &'static str {
        match self {
            RecognizedAssetFamily::TextAsset => "TextAsset",
            RecognizedAssetFamily::AvatarMask => "AvatarMask",
            RecognizedAssetFamily::AudioMixer => "AudioMixer",
            RecognizedAssetFamily::SpriteAtlasV1 => "SpriteAtlas v1",
            RecognizedAssetFamily::SpriteAtlasV2 => "SpriteAtlas v2",
            RecognizedAssetFamily::Scene => "Scene",
            RecognizedAssetFamily::Prefab => "Prefab",
            RecognizedAssetFamily::Material => "Material",
            RecognizedAssetFamily::Shader => "Shader",
            RecognizedAssetFamily::ComputeShader => "ComputeShader",
            RecognizedAssetFamily::ShaderInclude => "ShaderInclude",
            RecognizedAssetFamily::ShaderVariantCollection => "ShaderVariantCollection",
            RecognizedAssetFamily::AnimationClip => "AnimationClip",
            RecognizedAssetFamily::AnimatorController => "AnimatorController",
            RecognizedAssetFamily::AnimatorOverrideController => "AnimatorOverrideController",
            RecognizedAssetFamily::Font => "Font",
            RecognizedAssetFamily::GuiSkin => "GUISkin",
            RecognizedAssetFamily::Video => "Video",
            RecognizedAssetFamily::Script => "MonoScript",
            RecognizedAssetFamily::ScriptableObject => "ScriptableObject",
            RecognizedAssetFamily::PhysicMaterial => "PhysicMaterial",
            RecognizedAssetFamily::PhysicsMaterial2D => "PhysicsMaterial2D",
            RecognizedAssetFamily::InputActions => "InputActions",
            RecognizedAssetFamily::TerrainData => "TerrainData",
            RecognizedAssetFamily::TerrainLayer => "TerrainLayer",
            RecognizedAssetFamily::LightingSettings => "LightingSettings",
            RecognizedAssetFamily::Cubemap => "Cubemap",
            RecognizedAssetFamily::Flare => "Flare",
            RecognizedAssetFamily::RenderTexture => "RenderTexture",
            RecognizedAssetFamily::Preset => "Preset",
            RecognizedAssetFamily::Timeline => "Timeline",
            RecognizedAssetFamily::Signal => "Signal",
            RecognizedAssetFamily::Texture2D => "Texture2D",
            RecognizedAssetFamily::AudioClip => "AudioClip",
            RecognizedAssetFamily::Model => "Model",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifiedAssetFamily {
    pub family: RecognizedAssetFamily,
    pub outcome: ImportOutcome,
}

pub fn is_text_asset_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            TEXT_ASSET_EXTENSIONS
                .iter()
                .any(|known| ext.eq_ignore_ascii_case(known))
        })
}

pub fn import_text_asset(bytes: &[u8], file_name: &str) -> Result<TextAssetContent, DatabaseError> {
    let path = Path::new(file_name);
    if !is_text_asset_path(path) {
        return Err(DatabaseError::Io {
            path: path.to_path_buf(),
            message: format!("unsupported TextAsset extension for {file_name}"),
        });
    }
    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(file_name)
        .to_string();
    let text = String::from_utf8_lossy(bytes).into_owned();
    Ok(TextAssetContent {
        name,
        text,
        bytes: bytes.to_vec(),
    })
}

pub fn identify_asset_family(
    path: &Path,
    bytes: &[u8],
    meta_text: Option<&str>,
) -> Option<IdentifiedAssetFamily> {
    let asset_label = path.display().to_string();
    let ext = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .unwrap_or_default();

    if is_text_asset_path(path) {
        return Some(IdentifiedAssetFamily {
            family: RecognizedAssetFamily::TextAsset,
            outcome: ImportOutcome::ready(),
        });
    }

    match ext.as_str() {
        "mask" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::AvatarMask,
                outcome: ImportOutcome::ready(),
            });
        }
        "mixer" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::AudioMixer,
                outcome: ImportOutcome::limited(
                    ImportDiagnostic::new(
                        &asset_label,
                        "AudioMixer routing and snapshot graph are recognized; advanced custom DSP effect plugins are not evaluated",
                        "the mixer asset is identified with its family; unsupported custom DSP effects remain inactive",
                    )
                    .partially_usable(),
                ),
            });
        }
        "spriteatlasv2" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::SpriteAtlasV2,
                outcome: ImportOutcome::ready(),
            });
        }
        "spriteatlas" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::SpriteAtlasV1,
                outcome: ImportOutcome::ready(),
            });
        }
        "unity" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Scene,
                outcome: ImportOutcome::ready(),
            });
        }
        "prefab" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Prefab,
                outcome: ImportOutcome::ready(),
            });
        }
        "mat" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Material,
                outcome: ImportOutcome::ready(),
            });
        }
        "shader" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Shader,
                outcome: ImportOutcome::ready(),
            });
        }
        "compute" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::ComputeShader,
                outcome: ImportOutcome::ready(),
            });
        }
        "hlsl" | "cginc" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::ShaderInclude,
                outcome: ImportOutcome::ready(),
            });
        }
        "shadervariants" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::ShaderVariantCollection,
                outcome: ImportOutcome::ready(),
            });
        }
        "anim" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::AnimationClip,
                outcome: ImportOutcome::ready(),
            });
        }
        "controller" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::AnimatorController,
                outcome: ImportOutcome::ready(),
            });
        }
        "overridecontroller" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::AnimatorOverrideController,
                outcome: ImportOutcome::ready(),
            });
        }
        "ttf" | "otf" => {
            let settings = match meta_text {
                Some(text) => match FontImportSettings::from_meta_text(text) {
                    Ok(settings) => settings,
                    Err(error) => {
                        return Some(IdentifiedAssetFamily {
                            family: RecognizedAssetFamily::Font,
                            outcome: ImportOutcome::failed(ImportDiagnostic::new(
                                &asset_label,
                                error.to_string(),
                                "the font cannot be imported; the asset stays failed and no glyph data is available",
                            )),
                        });
                    }
                },
                None => FontImportSettings::default(),
            };
            let outcome = match font::import_font(bytes, &asset_label, &settings) {
                Ok(imported) => font::font_import_outcome(&asset_label, Ok(&imported)),
                Err(error) => font::font_import_outcome(&asset_label, Err(&error)),
            };
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Font,
                outcome,
            });
        }
        "fontsettings" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Font,
                outcome: ImportOutcome::ready(),
            });
        }
        "guiskin" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::GuiSkin,
                outcome: ImportOutcome::ready(),
            });
        }
        "mp4" | "webm" | "mov" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Video,
                outcome: ImportOutcome::ready(),
            });
        }
        "cs" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Script,
                outcome: ImportOutcome::ready(),
            });
        }
        "physicmaterial" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::PhysicMaterial,
                outcome: ImportOutcome::ready(),
            });
        }
        "physicsmaterial2d" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::PhysicsMaterial2D,
                outcome: ImportOutcome::ready(),
            });
        }
        "inputactions" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::InputActions,
                outcome: ImportOutcome::ready(),
            });
        }
        "terrainlayer" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::TerrainLayer,
                outcome: ImportOutcome::ready(),
            });
        }
        "lighting" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::LightingSettings,
                outcome: ImportOutcome::ready(),
            });
        }
        "cubemap" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Cubemap,
                outcome: ImportOutcome::ready(),
            });
        }
        "flare" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Flare,
                outcome: ImportOutcome::ready(),
            });
        }
        "rendertexture" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::RenderTexture,
                outcome: ImportOutcome::ready(),
            });
        }
        "preset" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Preset,
                outcome: ImportOutcome::ready(),
            });
        }
        "playable" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Timeline,
                outcome: ImportOutcome::ready(),
            });
        }
        "signal" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Signal,
                outcome: ImportOutcome::ready(),
            });
        }
        "png" | "jpg" | "jpeg" | "tga" | "targa" | "exr" | "psd" | "hdr" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Texture2D,
                outcome: ImportOutcome::ready(),
            });
        }
        "gif" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Texture2D,
                outcome: ImportOutcome::limited(
                    ImportDiagnostic::new(
                        &asset_label,
                        "GIF imports first frame only; later animation frames are excluded",
                        "the asset imports with the first frame; later frames stay unavailable",
                    )
                    .partially_usable(),
                ),
            });
        }
        "pict" | "pct" | "pic" | "iff" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Texture2D,
                outcome: ImportOutcome::failed(ImportDiagnostic::new(
                    &asset_label,
                    format!("{ext} texture format is Unsupported"),
                    "the asset cannot be imported; its content stays unavailable",
                )),
            });
        }
        "wav" | "wave" | "aif" | "aiff" | "aifc" | "ogg" | "mp3" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::AudioClip,
                outcome: ImportOutcome::ready(),
            });
        }
        "xm" | "mod" | "it" | "s3m" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::AudioClip,
                outcome: ImportOutcome::failed(ImportDiagnostic::new(
                    &asset_label,
                    format!("{ext} tracker module format is Unsupported"),
                    "the audio clip cannot be imported; the asset stays failed",
                )),
            });
        }
        "fbx" | "gltf" | "glb" | "obj" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Model,
                outcome: ImportOutcome::ready(),
            });
        }
        "blend" | "max" | "ma" | "mb" | "c4d" | "lxo" | "3ds" => {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::Model,
                outcome: ImportOutcome::failed(ImportDiagnostic::new(
                    &asset_label,
                    format!("{ext} format is Unsupported without launching external applications"),
                    "the model cannot be imported; content-creation application files are not converted",
                )),
            });
        }
        _ => {}
    }

    if let Some(meta) = meta_text {
        if meta.contains("SpriteAtlasImporter:") {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::SpriteAtlasV2,
                outcome: ImportOutcome::ready(),
            });
        }
        if meta.contains("TextScriptImporter:") {
            return Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::TextAsset,
                outcome: ImportOutcome::ready(),
            });
        }
    }

    if let Ok(text) = std::str::from_utf8(bytes) {
        if let Ok(stream) = unity_yaml::parse(text) {
            if let Some(doc) = stream.documents.iter().find(|doc| !doc.stripped) {
                if let Node::Mapping(entries) = &doc.root {
                    if let Some((key, _)) = entries.first() {
                        let family = match key.as_str() {
                            "AvatarMask" => Some((
                                RecognizedAssetFamily::AvatarMask,
                                ImportOutcome::ready(),
                            )),
                            "AudioMixerController" | "AudioMixer" => Some((
                                RecognizedAssetFamily::AudioMixer,
                                ImportOutcome::limited(
                                    ImportDiagnostic::new(
                                        &asset_label,
                                        "AudioMixer routing and snapshot graph are recognized; advanced custom DSP effect plugins are not evaluated",
                                        "the mixer asset is identified with its family; unsupported custom DSP effects remain inactive",
                                    )
                                    .partially_usable(),
                                ),
                            )),
                            "SpriteAtlasAsset" => Some((
                                RecognizedAssetFamily::SpriteAtlasV2,
                                ImportOutcome::ready(),
                            )),
                            "SpriteAtlas" => Some((
                                RecognizedAssetFamily::SpriteAtlasV1,
                                ImportOutcome::ready(),
                            )),
                            "TerrainData" => Some((
                                RecognizedAssetFamily::TerrainData,
                                ImportOutcome::ready(),
                            )),
                            "MonoBehaviour" => Some((
                                RecognizedAssetFamily::ScriptableObject,
                                ImportOutcome::ready(),
                            )),
                            _ => None,
                        };
                        if let Some((family, outcome)) = family {
                            return Some(IdentifiedAssetFamily { family, outcome });
                        }
                    }
                }
            }
        }
    }

    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseError {
    NotFound { path: PathBuf },
    DestinationExists { path: PathBuf },
    Io { path: PathBuf, message: String },
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseError::NotFound { path } => {
                write!(f, "asset not found: {}", path.display())
            }
            DatabaseError::DestinationExists { path } => {
                write!(f, "destination already exists: {}", path.display())
            }
            DatabaseError::Io { path, message } => {
                write!(f, "cannot access {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for DatabaseError {}

impl Project {
    pub fn search_assets(&self, filter: &AssetFilter) -> Vec<&AssetEntry> {
        self.inventory
            .entries
            .iter()
            .filter(|entry| matches_filter(self, entry, filter))
            .collect()
    }

    pub fn rename_asset(&mut self, from: &Path, new_file_name: &str) -> Result<(), DatabaseError> {
        let from_rel = self.relative_path(from);
        let to_rel = match from_rel.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.join(new_file_name),
            _ => PathBuf::from(new_file_name),
        };
        self.move_asset(&from_rel, &to_rel)
    }

    pub fn asset_import_state(&self, entry: &AssetEntry) -> AssetImportState {
        let full_path = self.root.join(&entry.path);
        if !full_path.is_file() {
            return AssetImportState::Missing;
        }
        let ext = entry
            .path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .unwrap_or_default();
        if ext == "cs" && self.capabilities().restricted {
            return AssetImportState::BlockedByTrust;
        }
        let bytes = fs::read(&full_path).unwrap_or_default();
        let meta_text = fs::read_to_string(meta_path_for(&full_path)).ok();
        if let Some(identified) = identify_asset_family(&entry.path, &bytes, meta_text.as_deref()) {
            return identified.outcome.state;
        }
        AssetImportState::Ready
    }

    pub fn move_asset(&mut self, from: &Path, to: &Path) -> Result<(), DatabaseError> {
        let from_rel = self.relative_path(from);
        let to_rel = self.relative_path(to);
        let from_asset = self.root.join(&from_rel);
        let from_meta = meta_path_for(&from_asset);
        let to_asset = self.root.join(&to_rel);
        let to_meta = meta_path_for(&to_asset);

        if !from_asset.is_file() || !from_meta.is_file() {
            return Err(DatabaseError::NotFound { path: from_rel });
        }
        if to_asset.exists() || to_meta.exists() {
            return Err(DatabaseError::DestinationExists { path: to_rel });
        }

        relocate_pair(&from_asset, &from_meta, &to_asset, &to_meta)?;
        self.refresh_inventory()
    }

    pub fn delete_asset(&mut self, path: &Path) -> Result<(), DatabaseError> {
        let rel = self.relative_path(path);
        let asset_path = self.root.join(&rel);
        let meta_path = meta_path_for(&asset_path);

        if !asset_path.is_file() || !meta_path.is_file() {
            return Err(DatabaseError::NotFound { path: rel });
        }

        let trash_asset = self.trash_root().join(&rel);
        let trash_meta = meta_path_for(&trash_asset);
        if trash_asset.exists() {
            let _ = fs::remove_file(&trash_asset);
        }
        if trash_meta.exists() {
            let _ = fs::remove_file(&trash_meta);
        }

        relocate_pair(&asset_path, &meta_path, &trash_asset, &trash_meta)?;
        self.refresh_inventory()
    }

    pub fn restore_asset(&mut self, path: &Path) -> Result<(), DatabaseError> {
        let rel = self.relative_path(path);
        let trash_asset = self.trash_root().join(&rel);
        let trash_meta = meta_path_for(&trash_asset);
        let dest_asset = self.root.join(&rel);
        let dest_meta = meta_path_for(&dest_asset);

        if !trash_asset.is_file() || !trash_meta.is_file() {
            return Err(DatabaseError::NotFound { path: rel });
        }
        if dest_asset.exists() || dest_meta.exists() {
            return Err(DatabaseError::DestinationExists { path: rel });
        }

        relocate_pair(&trash_asset, &trash_meta, &dest_asset, &dest_meta)?;
        self.refresh_inventory()
    }

    fn relative_path(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.root)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| path.to_path_buf())
    }

    fn trash_root(&self) -> PathBuf {
        self.root.join(TRASH_DIRECTORY)
    }

    fn refresh_inventory(&mut self) -> Result<(), DatabaseError> {
        self.inventory = build_inventory(&self.root).map_err(|error| DatabaseError::Io {
            path: self.root.clone(),
            message: error.to_string(),
        })?;
        Ok(())
    }
}

fn meta_path_for(asset_path: &Path) -> PathBuf {
    let mut name = asset_path.as_os_str().to_os_string();
    name.push(".meta");
    PathBuf::from(name)
}

fn relocate_pair(
    from_asset: &Path,
    from_meta: &Path,
    to_asset: &Path,
    to_meta: &Path,
) -> Result<(), DatabaseError> {
    if let Some(parent) = to_asset.parent() {
        fs::create_dir_all(parent).map_err(|error| DatabaseError::Io {
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    fs::rename(from_asset, to_asset).map_err(|error| DatabaseError::Io {
        path: from_asset.to_path_buf(),
        message: error.to_string(),
    })?;
    if let Err(error) = fs::rename(from_meta, to_meta) {
        let _ = fs::rename(to_asset, from_asset);
        return Err(DatabaseError::Io {
            path: from_meta.to_path_buf(),
            message: error.to_string(),
        });
    }
    Ok(())
}

fn matches_filter(project: &Project, entry: &AssetEntry, filter: &AssetFilter) -> bool {
    if !filter.include_sub_assets && entry.kind != AssetKind::Main {
        return false;
    }
    if let Some(expected_folder) = &filter.folder {
        let rel_folder = project.relative_path(expected_folder);
        if !entry.path.starts_with(&rel_folder) {
            return false;
        }
    }
    if let Some(expected_class) = &filter.class {
        let actual_class = resolved_entry_class(project, entry);
        if actual_class.as_deref() != Some(expected_class.as_str()) {
            return false;
        }
    }
    if let Some(expected_state) = filter.state {
        if project.asset_import_state(entry) != expected_state {
            return false;
        }
    }
    if let Some(expected_label) = &filter.label {
        if !entry.labels.iter().any(|label| label == expected_label) {
            return false;
        }
    }
    if let Some(needle) = &filter.name_contains {
        if !matches_name(entry, needle) {
            return false;
        }
    }
    true
}

fn resolved_entry_class(project: &Project, entry: &AssetEntry) -> Option<String> {
    if let Some(class) = &entry.class {
        return Some(class.clone());
    }
    let full_path = project.root.join(&entry.path);
    let bytes = fs::read(&full_path).unwrap_or_default();
    let meta_text = fs::read_to_string(meta_path_for(&full_path)).ok();
    identify_asset_family(&entry.path, &bytes, meta_text.as_deref())
        .map(|identified| identified.family.class_name().to_string())
}

fn matches_name(entry: &AssetEntry, needle: &str) -> bool {
    let needle_lower = needle.to_ascii_lowercase();
    match entry.kind {
        AssetKind::Main => entry
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.to_ascii_lowercase().contains(&needle_lower)),
        AssetKind::Sub => {
            entry
                .labels
                .iter()
                .any(|label| label.to_ascii_lowercase().contains(&needle_lower))
                || entry
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.to_ascii_lowercase().contains(&needle_lower))
        }
    }
}
