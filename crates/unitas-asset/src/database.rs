use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::dependencies::{AssetImportState, ImportDiagnostic, ImportOutcome};
use crate::font::{self, FontImportSettings};
use crate::project::{build_inventory, AssetEntry, AssetKind, Project};
use crate::unity_yaml::{self, Node};

#[path = "tilemap/mod.rs"]
pub mod tilemap;

use self::tilemap::TilemapKind;

const TRASH_DIRECTORY: &str = ".unitas/trash";
const MAX_RECENT_ASSETS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetPreview {
    pub guid: String,
    pub path: PathBuf,
    pub family: Option<RecognizedAssetFamily>,
    pub class_name: String,
    pub byte_size: u64,
    pub state: AssetImportState,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedAssetRecord {
    pub original_path: PathBuf,
    pub asset_bytes: Vec<u8>,
    pub meta_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseOperation {
    Move {
        from: PathBuf,
        to: PathBuf,
    },
    Delete {
        record: DeletedAssetRecord,
    },
    Restore {
        path: PathBuf,
        record: DeletedAssetRecord,
    },
    Duplicate {
        created_path: PathBuf,
        source_path: PathBuf,
        guid: String,
        meta_bytes: Vec<u8>,
    },
}

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
    Tile,
    Tilemap,
    Grid,
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
            RecognizedAssetFamily::Tile => "Tile",
            RecognizedAssetFamily::Tilemap => "Tilemap",
            RecognizedAssetFamily::Grid => "Grid",
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
            RecognizedAssetFamily::Tile => "Tile",
            RecognizedAssetFamily::Tilemap => "Tilemap",
            RecognizedAssetFamily::Grid => "Grid",
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

    if let Some(identified) = identify_extension_family(&ext, &asset_label, bytes, meta_text) {
        return Some(identified);
    }

    if let Some(identified) = identify_meta_family(meta_text) {
        return Some(identified);
    }

    identify_yaml_family(bytes, &asset_label)
}

fn identify_extension_family(
    ext: &str,
    asset_label: &str,
    bytes: &[u8],
    meta_text: Option<&str>,
) -> Option<IdentifiedAssetFamily> {
    if let Some(family) = ready_extension_family(ext) {
        return Some(IdentifiedAssetFamily {
            family,
            outcome: ImportOutcome::ready(),
        });
    }

    match ext {
        "mixer" => Some(IdentifiedAssetFamily {
            family: RecognizedAssetFamily::AudioMixer,
            outcome: ImportOutcome::limited(
                ImportDiagnostic::new(
                    asset_label,
                    "AudioMixer routing and snapshot graph are recognized; advanced custom DSP effect plugins are not evaluated",
                    "the mixer asset is identified with its family; unsupported custom DSP effects remain inactive",
                )
                .partially_usable(),
            ),
        }),
        "anim" => {
            let outcome = match crate::anim_import::import_animation_clip(bytes, asset_label) {
                Ok(imported) => crate::anim_import::anim_import_outcome(asset_label, Ok(&imported)),
                Err(error) => crate::anim_import::anim_import_outcome(asset_label, Err(&error)),
            };
            Some(IdentifiedAssetFamily {
                family: RecognizedAssetFamily::AnimationClip,
                outcome,
            })
        }
        "ttf" | "otf" => Some(identify_font_family(bytes, asset_label, meta_text)),
        "gif" => Some(IdentifiedAssetFamily {
            family: RecognizedAssetFamily::Texture2D,
            outcome: ImportOutcome::limited(
                ImportDiagnostic::new(
                    asset_label,
                    "GIF imports first frame only; later animation frames are excluded",
                    "the asset imports with the first frame; later frames stay unavailable",
                )
                .partially_usable(),
            ),
        }),
        "pict" | "pct" | "pic" | "iff" => Some(IdentifiedAssetFamily {
            family: RecognizedAssetFamily::Texture2D,
            outcome: ImportOutcome::failed(ImportDiagnostic::new(
                asset_label,
                format!("{ext} texture format is Unsupported"),
                "the asset cannot be imported; its content stays unavailable",
            )),
        }),
        "xm" | "mod" | "it" | "s3m" => Some(IdentifiedAssetFamily {
            family: RecognizedAssetFamily::AudioClip,
            outcome: ImportOutcome::failed(ImportDiagnostic::new(
                asset_label,
                format!("{ext} tracker module format is Unsupported"),
                "the audio clip cannot be imported; the asset stays failed",
            )),
        }),
        "blend" | "max" | "ma" | "mb" | "c4d" | "lxo" | "3ds" => Some(IdentifiedAssetFamily {
            family: RecognizedAssetFamily::Model,
            outcome: ImportOutcome::failed(ImportDiagnostic::new(
                asset_label,
                format!("{ext} format is Unsupported without launching external applications"),
                "the model cannot be imported; content-creation application files are not converted",
            )),
        }),
        _ => None,
    }
}

fn ready_extension_family(ext: &str) -> Option<RecognizedAssetFamily> {
    let family = match ext {
        "mask" => RecognizedAssetFamily::AvatarMask,
        "spriteatlasv2" => RecognizedAssetFamily::SpriteAtlasV2,
        "spriteatlas" => RecognizedAssetFamily::SpriteAtlasV1,
        "unity" => RecognizedAssetFamily::Scene,
        "prefab" => RecognizedAssetFamily::Prefab,
        "mat" => RecognizedAssetFamily::Material,
        "shader" => RecognizedAssetFamily::Shader,
        "compute" => RecognizedAssetFamily::ComputeShader,
        "hlsl" | "cginc" => RecognizedAssetFamily::ShaderInclude,
        "shadervariants" => RecognizedAssetFamily::ShaderVariantCollection,
        "controller" => RecognizedAssetFamily::AnimatorController,
        "overridecontroller" => RecognizedAssetFamily::AnimatorOverrideController,
        "fontsettings" => RecognizedAssetFamily::Font,
        "guiskin" => RecognizedAssetFamily::GuiSkin,
        "mp4" | "webm" | "mov" => RecognizedAssetFamily::Video,
        "cs" => RecognizedAssetFamily::Script,
        "physicmaterial" => RecognizedAssetFamily::PhysicMaterial,
        "physicsmaterial2d" => RecognizedAssetFamily::PhysicsMaterial2D,
        "inputactions" => RecognizedAssetFamily::InputActions,
        "terrainlayer" => RecognizedAssetFamily::TerrainLayer,
        "lighting" => RecognizedAssetFamily::LightingSettings,
        "cubemap" => RecognizedAssetFamily::Cubemap,
        "flare" => RecognizedAssetFamily::Flare,
        "rendertexture" => RecognizedAssetFamily::RenderTexture,
        "preset" => RecognizedAssetFamily::Preset,
        "playable" => RecognizedAssetFamily::Timeline,
        "signal" => RecognizedAssetFamily::Signal,
        "png" | "jpg" | "jpeg" | "tga" | "targa" | "exr" | "psd" | "psb" | "hdr" => {
            RecognizedAssetFamily::Texture2D
        }
        "wav" | "wave" | "aif" | "aiff" | "aifc" | "ogg" | "mp3" => {
            RecognizedAssetFamily::AudioClip
        }
        "fbx" | "gltf" | "glb" | "obj" => RecognizedAssetFamily::Model,
        _ => return None,
    };
    Some(family)
}

fn identify_font_family(
    bytes: &[u8],
    asset_label: &str,
    meta_text: Option<&str>,
) -> IdentifiedAssetFamily {
    let settings = match meta_text {
        Some(text) => match FontImportSettings::from_meta_text(text) {
            Ok(settings) => settings,
            Err(error) => {
                return IdentifiedAssetFamily {
                    family: RecognizedAssetFamily::Font,
                    outcome: ImportOutcome::failed(ImportDiagnostic::new(
                        asset_label,
                        error.to_string(),
                        "the font cannot be imported; the asset stays failed and no glyph data is available",
                    )),
                };
            }
        },
        None => FontImportSettings::default(),
    };
    let outcome = match font::import_font(bytes, asset_label, &settings) {
        Ok(imported) => font::font_import_outcome(asset_label, Ok(&imported)),
        Err(error) => font::font_import_outcome(asset_label, Err(&error)),
    };
    IdentifiedAssetFamily {
        family: RecognizedAssetFamily::Font,
        outcome,
    }
}

fn identify_meta_family(meta_text: Option<&str>) -> Option<IdentifiedAssetFamily> {
    let meta = meta_text?;
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
    None
}

fn identify_yaml_family(bytes: &[u8], asset_label: &str) -> Option<IdentifiedAssetFamily> {
    let (family, outcome) = identify_yaml_document_family(bytes, asset_label)?;
    Some(IdentifiedAssetFamily { family, outcome })
}

fn identify_yaml_document_family(
    bytes: &[u8],
    asset_label: &str,
) -> Option<(RecognizedAssetFamily, ImportOutcome)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let stream = unity_yaml::parse(text).ok()?;
    let doc = stream.documents.iter().find(|doc| !doc.stripped)?;
    let Node::Mapping(entries) = &doc.root else {
        return None;
    };
    let (key, body) = entries.first()?;
    match key.as_str() {
        "AvatarMask" => Some((RecognizedAssetFamily::AvatarMask, ImportOutcome::ready())),
        "AudioMixerController" | "AudioMixer" => Some((
            RecognizedAssetFamily::AudioMixer,
            ImportOutcome::limited(
                ImportDiagnostic::new(
                    asset_label,
                    "AudioMixer routing and snapshot graph are recognized; advanced custom DSP effect plugins are not evaluated",
                    "the mixer asset is identified with its family; unsupported custom DSP effects remain inactive",
                )
                .partially_usable(),
            ),
        )),
        "SpriteAtlasAsset" => Some((RecognizedAssetFamily::SpriteAtlasV2, ImportOutcome::ready())),
        "SpriteAtlas" => Some((RecognizedAssetFamily::SpriteAtlasV1, ImportOutcome::ready())),
        "TerrainData" => Some((RecognizedAssetFamily::TerrainData, ImportOutcome::ready())),
        "Tilemap" => Some((
            RecognizedAssetFamily::Tilemap,
            tilemap_outcome(bytes, asset_label),
        )),
        "Grid" => Some((RecognizedAssetFamily::Grid, tilemap_outcome(bytes, asset_label))),
        "Tile" | "TileBase" | "RuleTile" | "ScriptableRuleTile" | "IsometricRuleTile" => Some((
            RecognizedAssetFamily::Tile,
            tilemap_outcome(bytes, asset_label),
        )),
        "MonoBehaviour" => identify_mono_behaviour_family(body, bytes, asset_label),
        _ => None,
    }
}

fn identify_mono_behaviour_family(
    body: &Node,
    bytes: &[u8],
    asset_label: &str,
) -> Option<(RecognizedAssetFamily, ImportOutcome)> {
    match self::tilemap::classify_body(body) {
        Some(TilemapKind::Tile | TilemapKind::RuleTile) => Some((
            RecognizedAssetFamily::Tile,
            tilemap_outcome(bytes, asset_label),
        )),
        Some(TilemapKind::Tilemap) => Some((
            RecognizedAssetFamily::Tilemap,
            tilemap_outcome(bytes, asset_label),
        )),
        Some(TilemapKind::Grid) => Some((
            RecognizedAssetFamily::Grid,
            tilemap_outcome(bytes, asset_label),
        )),
        None => Some((
            RecognizedAssetFamily::ScriptableObject,
            ImportOutcome::ready(),
        )),
    }
}

fn tilemap_outcome(bytes: &[u8], asset_label: &str) -> ImportOutcome {
    match self::tilemap::import_tilemap(bytes, asset_label) {
        Ok(imported) => self::tilemap::tilemap_import_outcome(asset_label, Ok(&imported)),
        Err(error) => self::tilemap::tilemap_import_outcome(asset_label, Err(&error)),
    }
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
        self.undo_stack.push(DatabaseOperation::Move {
            from: from_rel,
            to: to_rel,
        });
        self.redo_stack.clear();
        self.refresh_inventory()
    }

    pub fn delete_asset(&mut self, path: &Path) -> Result<(), DatabaseError> {
        let rel = self.relative_path(path);
        let asset_path = self.root.join(&rel);
        let meta_path = meta_path_for(&asset_path);

        if !asset_path.is_file() || !meta_path.is_file() {
            return Err(DatabaseError::NotFound { path: rel });
        }

        let asset_bytes = fs::read(&asset_path).map_err(|error| DatabaseError::Io {
            path: asset_path.clone(),
            message: error.to_string(),
        })?;
        let meta_bytes = fs::read(&meta_path).map_err(|error| DatabaseError::Io {
            path: meta_path.clone(),
            message: error.to_string(),
        })?;

        let trash_asset = self.trash_root().join(&rel);
        let trash_meta = meta_path_for(&trash_asset);
        if trash_asset.exists() {
            let _ = fs::remove_file(&trash_asset);
        }
        if trash_meta.exists() {
            let _ = fs::remove_file(&trash_meta);
        }

        relocate_pair(&asset_path, &meta_path, &trash_asset, &trash_meta)?;
        self.undo_stack.push(DatabaseOperation::Delete {
            record: DeletedAssetRecord {
                original_path: rel,
                asset_bytes,
                meta_bytes,
            },
        });
        self.redo_stack.clear();
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

        let asset_bytes = fs::read(&trash_asset).map_err(|error| DatabaseError::Io {
            path: trash_asset.clone(),
            message: error.to_string(),
        })?;
        let meta_bytes = fs::read(&trash_meta).map_err(|error| DatabaseError::Io {
            path: trash_meta.clone(),
            message: error.to_string(),
        })?;

        relocate_pair(&trash_asset, &trash_meta, &dest_asset, &dest_meta)?;
        self.undo_stack.push(DatabaseOperation::Restore {
            path: rel.clone(),
            record: DeletedAssetRecord {
                original_path: rel,
                asset_bytes,
                meta_bytes,
            },
        });
        self.redo_stack.clear();
        self.refresh_inventory()
    }

    pub fn preview_asset(&mut self, relative_path: &Path) -> Result<AssetPreview, DatabaseError> {
        let rel = self.relative_path(relative_path);
        let entry = self
            .inventory
            .entries
            .iter()
            .find(|candidate| candidate.kind == AssetKind::Main && candidate.path == rel)
            .cloned()
            .ok_or_else(|| DatabaseError::NotFound { path: rel.clone() })?;
        let full_path = self.root.join(&rel);
        let metadata = fs::metadata(&full_path).map_err(|error| DatabaseError::Io {
            path: full_path.clone(),
            message: error.to_string(),
        })?;
        let byte_size = metadata.len();
        let bytes = fs::read(&full_path).map_err(|error| DatabaseError::Io {
            path: full_path.clone(),
            message: error.to_string(),
        })?;
        let meta_text = fs::read_to_string(meta_path_for(&full_path)).ok();
        let identified = identify_asset_family(&rel, &bytes, meta_text.as_deref());
        let family = identified.as_ref().map(|item| item.family);
        let class_name = entry
            .class
            .clone()
            .or_else(|| family.map(|fam| fam.class_name().to_string()))
            .unwrap_or_else(|| "DefaultAsset".to_string());
        let state = self.asset_import_state(&entry);
        let summary = build_preview_summary(&rel, &bytes, family, &class_name, byte_size);
        let guid = entry.guid.clone();
        self.record_recent(&guid);
        Ok(AssetPreview {
            guid,
            path: rel,
            family,
            class_name,
            byte_size,
            state,
            summary,
        })
    }

    pub fn add_favorite(&mut self, guid: &str) -> bool {
        if self.favorites.iter().any(|item| item == guid) {
            return false;
        }
        self.favorites.push(guid.to_string());
        true
    }

    pub fn remove_favorite(&mut self, guid: &str) -> bool {
        let initial_len = self.favorites.len();
        self.favorites.retain(|item| item != guid);
        self.favorites.len() != initial_len
    }

    pub fn favorites(&self) -> &[String] {
        &self.favorites
    }

    pub fn record_recent(&mut self, guid: &str) {
        self.recent_assets.retain(|item| item != guid);
        self.recent_assets.insert(0, guid.to_string());
        if self.recent_assets.len() > MAX_RECENT_ASSETS {
            self.recent_assets.truncate(MAX_RECENT_ASSETS);
        }
    }

    pub fn recent_assets(&self) -> &[String] {
        &self.recent_assets
    }

    pub fn duplicate_asset(&mut self, from: &Path, to: &Path) -> Result<String, DatabaseError> {
        let from_rel = self.relative_path(from);
        let to_rel = self.relative_path(to);
        let from_exists = self
            .inventory
            .entries
            .iter()
            .any(|entry| entry.kind == AssetKind::Main && entry.path == from_rel);
        let from_asset = self.root.join(&from_rel);
        let from_meta = meta_path_for(&from_asset);
        if !from_exists || !from_asset.is_file() || !from_meta.is_file() {
            return Err(DatabaseError::NotFound { path: from_rel });
        }
        let to_asset = self.root.join(&to_rel);
        let to_meta = meta_path_for(&to_asset);
        if to_asset.exists() || to_meta.exists() {
            return Err(DatabaseError::DestinationExists { path: to_rel });
        }

        if let Some(parent) = to_asset.parent() {
            fs::create_dir_all(parent).map_err(|error| DatabaseError::Io {
                path: parent.to_path_buf(),
                message: error.to_string(),
            })?;
        }
        fs::copy(&from_asset, &to_asset).map_err(|error| DatabaseError::Io {
            path: to_asset.clone(),
            message: error.to_string(),
        })?;

        let source_meta = fs::read_to_string(&from_meta).map_err(|error| DatabaseError::Io {
            path: from_meta.clone(),
            message: error.to_string(),
        })?;
        let new_guid = self.next_unique_guid(&to_rel);
        let rewritten_meta = rewrite_meta_guid(&source_meta, &new_guid);
        let meta_bytes = rewritten_meta.into_bytes();
        fs::write(&to_meta, &meta_bytes).map_err(|error| DatabaseError::Io {
            path: to_meta.clone(),
            message: error.to_string(),
        })?;

        self.undo_stack.push(DatabaseOperation::Duplicate {
            created_path: to_rel,
            source_path: from_rel,
            guid: new_guid.clone(),
            meta_bytes,
        });
        self.redo_stack.clear();
        self.refresh_inventory()?;
        Ok(new_guid)
    }

    pub fn undo_database_operation(&mut self) -> Result<bool, DatabaseError> {
        let Some(operation) = self.undo_stack.pop() else {
            return Ok(false);
        };
        match &operation {
            DatabaseOperation::Move { from, to } => {
                let from_asset = self.root.join(from);
                let from_meta = meta_path_for(&from_asset);
                let to_asset = self.root.join(to);
                let to_meta = meta_path_for(&to_asset);
                relocate_pair(&to_asset, &to_meta, &from_asset, &from_meta)?;
            }
            DatabaseOperation::Delete { record } => {
                let dest_asset = self.root.join(&record.original_path);
                let dest_meta = meta_path_for(&dest_asset);
                let trash_asset = self.trash_root().join(&record.original_path);
                let trash_meta = meta_path_for(&trash_asset);
                if trash_asset.is_file() && trash_meta.is_file() {
                    relocate_pair(&trash_asset, &trash_meta, &dest_asset, &dest_meta)?;
                } else {
                    write_asset_pair(
                        &dest_asset,
                        &dest_meta,
                        &record.asset_bytes,
                        &record.meta_bytes,
                    )?;
                }
            }
            DatabaseOperation::Restore { path, record } => {
                let asset_path = self.root.join(path);
                let meta_path = meta_path_for(&asset_path);
                let trash_asset = self.trash_root().join(path);
                let trash_meta = meta_path_for(&trash_asset);
                if asset_path.is_file() && meta_path.is_file() {
                    if trash_asset.exists() {
                        let _ = fs::remove_file(&trash_asset);
                    }
                    if trash_meta.exists() {
                        let _ = fs::remove_file(&trash_meta);
                    }
                    relocate_pair(&asset_path, &meta_path, &trash_asset, &trash_meta)?;
                } else {
                    remove_pair_if_exists(&asset_path, &meta_path)?;
                    write_asset_pair(
                        &trash_asset,
                        &trash_meta,
                        &record.asset_bytes,
                        &record.meta_bytes,
                    )?;
                }
            }
            DatabaseOperation::Duplicate { created_path, .. } => {
                let created_asset = self.root.join(created_path);
                let created_meta = meta_path_for(&created_asset);
                remove_pair_if_exists(&created_asset, &created_meta)?;
            }
        }
        self.redo_stack.push(operation);
        self.refresh_inventory()?;
        Ok(true)
    }

    pub fn redo_database_operation(&mut self) -> Result<bool, DatabaseError> {
        let Some(operation) = self.redo_stack.pop() else {
            return Ok(false);
        };
        match &operation {
            DatabaseOperation::Move { from, to } => {
                let from_asset = self.root.join(from);
                let from_meta = meta_path_for(&from_asset);
                let to_asset = self.root.join(to);
                let to_meta = meta_path_for(&to_asset);
                relocate_pair(&from_asset, &from_meta, &to_asset, &to_meta)?;
            }
            DatabaseOperation::Delete { record } => {
                let asset_path = self.root.join(&record.original_path);
                let meta_path = meta_path_for(&asset_path);
                let trash_asset = self.trash_root().join(&record.original_path);
                let trash_meta = meta_path_for(&trash_asset);
                if trash_asset.exists() {
                    let _ = fs::remove_file(&trash_asset);
                }
                if trash_meta.exists() {
                    let _ = fs::remove_file(&trash_meta);
                }
                if asset_path.is_file() && meta_path.is_file() {
                    relocate_pair(&asset_path, &meta_path, &trash_asset, &trash_meta)?;
                } else {
                    remove_pair_if_exists(&asset_path, &meta_path)?;
                    write_asset_pair(
                        &trash_asset,
                        &trash_meta,
                        &record.asset_bytes,
                        &record.meta_bytes,
                    )?;
                }
            }
            DatabaseOperation::Restore { path, record } => {
                let dest_asset = self.root.join(path);
                let dest_meta = meta_path_for(&dest_asset);
                let trash_asset = self.trash_root().join(path);
                let trash_meta = meta_path_for(&trash_asset);
                if trash_asset.is_file() && trash_meta.is_file() {
                    relocate_pair(&trash_asset, &trash_meta, &dest_asset, &dest_meta)?;
                } else {
                    write_asset_pair(
                        &dest_asset,
                        &dest_meta,
                        &record.asset_bytes,
                        &record.meta_bytes,
                    )?;
                }
            }
            DatabaseOperation::Duplicate {
                created_path,
                source_path,
                meta_bytes,
                ..
            } => {
                let from_asset = self.root.join(source_path);
                let to_asset = self.root.join(created_path);
                let to_meta = meta_path_for(&to_asset);
                if let Some(parent) = to_asset.parent() {
                    fs::create_dir_all(parent).map_err(|error| DatabaseError::Io {
                        path: parent.to_path_buf(),
                        message: error.to_string(),
                    })?;
                }
                fs::copy(&from_asset, &to_asset).map_err(|error| DatabaseError::Io {
                    path: to_asset.clone(),
                    message: error.to_string(),
                })?;
                fs::write(&to_meta, meta_bytes).map_err(|error| DatabaseError::Io {
                    path: to_meta,
                    message: error.to_string(),
                })?;
            }
        }
        self.undo_stack.push(operation);
        self.refresh_inventory()?;
        Ok(true)
    }

    fn next_unique_guid(&mut self, to_rel: &Path) -> String {
        let path_text = to_rel.to_string_lossy();
        loop {
            self.duplicate_counter = self.duplicate_counter.wrapping_add(1);
            let counter = self.duplicate_counter;
            let h1 = deterministic_hash(path_text.as_bytes(), counter ^ 0x9e37_79b9_7f4a_7c15);
            let h2 = deterministic_hash(
                path_text.as_bytes(),
                counter.wrapping_mul(0x517c_c1b7_2722_0a95) ^ 0xbf58_476d_1ce4_e5b9,
            );
            let candidate = format!("{h1:016x}{h2:016x}");
            let collides = self
                .inventory
                .entries
                .iter()
                .any(|entry| entry.guid == candidate)
                || self
                    .inventory
                    .duplicate_guids
                    .iter()
                    .any(|dup| dup.guid == candidate);
            if !collides && candidate.chars().any(|ch| ch != '0') {
                return candidate;
            }
        }
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

fn build_preview_summary(
    rel_path: &Path,
    bytes: &[u8],
    family: Option<RecognizedAssetFamily>,
    class_name: &str,
    byte_size: u64,
) -> String {
    if family == Some(RecognizedAssetFamily::AnimationClip) {
        let label = rel_path.display().to_string();
        if let Ok(clip) = crate::anim_import::import_animation_clip(bytes, &label) {
            return format!(
                "{class_name} '{}' ({} Hz, {byte_size} bytes)",
                clip.name, clip.sample_rate
            );
        }
    }
    if family == Some(RecognizedAssetFamily::Video) {
        let label = rel_path.display().to_string();
        if let Ok(meta) = crate::video_import::probe_video_stream(bytes, &label) {
            return format!(
                "{class_name} ({}x{}, {:.2} fps, {byte_size} bytes)",
                meta.width,
                meta.height,
                meta.frame_rate()
            );
        }
    }
    format!("{class_name} ({byte_size} bytes)")
}

fn deterministic_hash(bytes: &[u8], seed: u64) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64 ^ seed;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn rewrite_meta_guid(meta_text: &str, new_guid: &str) -> String {
    let mut replaced = false;
    let mut lines: Vec<String> = Vec::new();
    for line in meta_text.lines() {
        if !replaced && line.trim_start().starts_with("guid:") {
            lines.push(format!("guid: {new_guid}"));
            replaced = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !replaced {
        lines.push(format!("guid: {new_guid}"));
    }
    let mut out = lines.join("\n");
    if meta_text.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn write_asset_pair(
    asset_path: &Path,
    meta_path: &Path,
    asset_bytes: &[u8],
    meta_bytes: &[u8],
) -> Result<(), DatabaseError> {
    if let Some(parent) = asset_path.parent() {
        fs::create_dir_all(parent).map_err(|error| DatabaseError::Io {
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    fs::write(asset_path, asset_bytes).map_err(|error| DatabaseError::Io {
        path: asset_path.to_path_buf(),
        message: error.to_string(),
    })?;
    fs::write(meta_path, meta_bytes).map_err(|error| DatabaseError::Io {
        path: meta_path.to_path_buf(),
        message: error.to_string(),
    })?;
    Ok(())
}

fn remove_pair_if_exists(asset_path: &Path, meta_path: &Path) -> Result<(), DatabaseError> {
    if asset_path.exists() {
        fs::remove_file(asset_path).map_err(|error| DatabaseError::Io {
            path: asset_path.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    if meta_path.exists() {
        fs::remove_file(meta_path).map_err(|error| DatabaseError::Io {
            path: meta_path.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    Ok(())
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
