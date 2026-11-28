use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::audio_import::{
    decode_ogg_vorbis, import_audio, import_audio_for_target, AudioCompressionFormat,
    AudioImportError, AudioImportSettings, AudioImportState, AudioLoadType,
};
use unitas_asset::project::{AssetEntry, GuidLookup, OpenReport, Project};
use unitas_asset::project_settings::load_project_settings;
use unitas_asset::texture::import::{import_texture, TextureImportSettings, TextureImporterType};
use unitas_asset::trust::TrustStore;
use unitas_asset::unity_yaml::{self, Node};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const OGG_ASSET: &str = "Assets/Audio/BounceChime.ogg";
const OGG_GUID: &str = "1b2c3d4e5f60718293a4b5c6d7e8f901";
const NORMAL_MAP_ASSET: &str = "Assets/Textures/PickupNormal.png";
const NORMAL_MAP_GUID: &str = "2c3d4e5f60718293a4b5c6d7e8f90123";
const PICKUP_MATERIAL: &str = "Assets/Materials/Pickup.mat";
const TEXTURE_FILE_ID: i64 = 2800000;
const NORMAL_MAP_SIZE: u32 = 64;

fn sample_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/marble-rush")
}

fn sample_asset(relative: &str) -> TestResult<PathBuf> {
    let path = sample_root().join(relative);
    if !path.is_file() {
        return Err(format!("the sample must ship {relative}").into());
    }
    Ok(path)
}

fn sample_meta(relative: &str) -> TestResult<String> {
    let path = sample_root().join(format!("{relative}.meta"));
    if !path.is_file() {
        return Err(
            format!("the sample must ship {relative}.meta with its import settings").into(),
        );
    }
    Ok(fs::read_to_string(&path)?)
}

fn sample_asset_guid(relative: &str) -> TestResult<String> {
    let meta = sample_meta(relative)?;
    for line in meta.lines() {
        if let Some(guid) = line.trim().strip_prefix("guid:") {
            return Ok(guid.trim().to_string());
        }
    }
    Err(format!("the sample must declare a guid for {relative}").into())
}

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!(
        "unitas_marble_rush_breadth_{name}_{}_{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    let mut entries: Vec<PathBuf> = fs::read_dir(source)?
        .flatten()
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for entry in entries {
        // Library is generated, gitignored state that concurrent gate lanes
        // (player/editor test runs) rewrite in place; copying it races their
        // deletions and the reopened project regenerates it anyway.
        if entry.file_name().is_some_and(|name| name == "Library") {
            continue;
        }
        let target = destination.join(entry.file_name().unwrap_or_default());
        if entry.is_dir() {
            copy_tree(&entry, &target)?;
        } else {
            fs::copy(&entry, &target)?;
        }
    }
    Ok(())
}

fn copied_sample() -> TestResult<PathBuf> {
    let root = temp_dir("project")?.join("MarbleRush");
    copy_tree(&sample_root(), &root)?;
    Ok(root)
}

fn open_sample(root: &Path, trust_dir: &Path) -> TestResult<(Project, OpenReport)> {
    let store = TrustStore::new(trust_dir.join("trust.json"));
    Ok(Project::open(root, &store)?)
}

fn asset_entry(report: &OpenReport, guid: &str) -> TestResult<AssetEntry> {
    match report.inventory.lookup(guid) {
        GuidLookup::Asset(entry) => Ok(entry.clone()),
        other => Err(format!("the inventory must hold {guid}: {other:?}").into()),
    }
}

fn material_texture_reference(path: &Path, property: &str) -> TestResult<(String, i64)> {
    let stream = unity_yaml::parse_file(path)?;
    let body = stream
        .documents
        .iter()
        .find(|document| document.class_id == 21 && !document.stripped)
        .and_then(|document| document.root.get("Material"))
        .ok_or("the material must declare a Material document")?;
    let entries = body
        .get("m_SavedProperties")
        .and_then(|saved| saved.get("m_TexEnvs"))
        .and_then(Node::as_sequence)
        .ok_or("the material must declare m_TexEnvs")?;
    for entry in entries {
        let Some(texture) = entry.get(property) else {
            continue;
        };
        let reference = texture
            .get("m_Texture")
            .ok_or("the material property must reference a texture")?;
        let guid = reference
            .get("guid")
            .and_then(Node::as_str)
            .ok_or("the texture reference must carry a guid")?
            .to_string();
        let file_id = reference
            .get("fileID")
            .and_then(Node::as_i64)
            .ok_or("the texture reference must carry a fileID")?;
        return Ok((guid, file_id));
    }
    Err(format!("the material must wire {property} to a texture").into())
}

#[test]
fn sample_ogg_decodes_to_pcm_with_the_load_settings_from_its_meta() -> TestResult {
    let bytes = fs::read(sample_asset(OGG_ASSET)?)?;
    let meta = sample_meta(OGG_ASSET)?;
    let settings = AudioImportSettings::from_meta_text(&meta)?;
    assert_eq!(
        settings.load_type,
        AudioLoadType::CompressedInMemory,
        "the meta load type must be read from the AudioImporter section"
    );
    assert_eq!(
        settings.compression_format,
        AudioCompressionFormat::Vorbis,
        "the meta compression format must be read from the AudioImporter section"
    );
    assert!(settings.force_to_mono);
    assert!(settings.preload_audio_data);
    assert_eq!(settings.quality, 1.0);
    assert_eq!(settings.platform_settings.len(), 1);
    let platform = &settings.platform_settings[0];
    assert_eq!(platform.build_target, "Standalone");
    assert!(platform.overridden);
    assert_eq!(platform.compression_format, AudioCompressionFormat::PCM);
    assert_eq!(platform.quality, 0.7);
    assert_eq!(platform.sample_rate, Some(22050));

    let decoded = decode_ogg_vorbis(&bytes)?;
    assert_eq!(decoded.channels, 2, "the shipped OGG must be a stereo clip");
    assert_eq!(decoded.sample_rate, 44100);
    assert!(!decoded.samples.is_empty());
    assert!(
        decoded.samples.iter().any(|sample| *sample != 0),
        "the shipped OGG must not decode to silence"
    );

    let imported = import_audio(&bytes, OGG_ASSET, &settings)?;
    assert_eq!(imported.state, AudioImportState::Imported);
    assert_eq!(
        imported.settings.load_type, settings.load_type,
        "the parsed meta load type must reach the import"
    );
    assert_eq!(
        imported.applied_compression,
        AudioCompressionFormat::Vorbis,
        "the meta compression format must reach the import"
    );
    assert_eq!(
        imported.clip.channels, 1,
        "the meta force-to-mono setting must be honoured"
    );
    assert_eq!(imported.clip.sample_rate, 44100);
    assert!(
        imported.clip.samples.iter().any(|sample| *sample != 0),
        "the imported clip must not be silent"
    );
    assert!(
        imported
            .notes
            .iter()
            .any(|note| note.contains("force to mono")),
        "the honoured force-to-mono setting must be reported: {:?}",
        imported.notes
    );
    assert!(
        imported.notes.iter().any(|note| note.contains("Vorbis")),
        "the honoured compression format must be reported: {:?}",
        imported.notes
    );

    let targeted = import_audio_for_target(&bytes, OGG_ASSET, &settings, Some("Standalone"))?;
    assert_eq!(targeted.applied_target.as_deref(), Some("Standalone"));
    assert_eq!(
        targeted.applied_compression,
        AudioCompressionFormat::PCM,
        "the meta platform override must reach the per-target import"
    );
    assert_eq!(
        targeted.clip.sample_rate, 22050,
        "the meta platform sample rate must reach the per-target import"
    );
    Ok(())
}

#[test]
fn the_audio_importer_meta_parser_reads_every_documented_load_setting() -> TestResult {
    let meta = "fileFormatVersion: 2\nguid: 00000000000000000000000000000000\nAudioImporter:\n  externalObjects: {}\n  serializedVersion: 2\n  defaultSettings:\n    serializedVersion: 2\n    loadType: 2\n    sampleRateSetting: 2\n    sampleRateOverride: 22050\n    compressionFormat: 3\n    quality: 0.5\n    forceToMono: 1\n    normalize: 1\n    preloadAudioData: 0\n";
    let settings = AudioImportSettings::from_meta_text(meta)?;
    assert_eq!(settings.load_type, AudioLoadType::Streaming);
    assert_eq!(settings.compression_format, AudioCompressionFormat::MP3);
    assert_eq!(settings.sample_rate_override, Some(22050));
    assert_eq!(settings.quality, 0.5);
    assert!(settings.force_to_mono);
    assert!(settings.normalize);
    assert!(!settings.preload_audio_data);
    assert!(settings.platform_settings.is_empty());

    let preserved = AudioImportSettings::from_meta_text(
        "AudioImporter:\n  defaultSettings:\n    loadType: 0\n    sampleRateSetting: 0\n    sampleRateOverride: 22050\n",
    )?;
    assert_eq!(preserved.load_type, AudioLoadType::DecompressOnLoad);
    assert_eq!(
        preserved.sample_rate_override, None,
        "a preserved sample rate must not override the decoded rate"
    );

    let broken = AudioImportSettings::from_meta_text(
        "AudioImporter:\n  defaultSettings:\n    loadType: 9\n",
    );
    assert!(
        matches!(broken, Err(AudioImportError::InvalidMetaText(_))),
        "an unknown load type must be reported"
    );

    let without_section = AudioImportSettings::from_meta_text(
        "fileFormatVersion: 2\nguid: 00000000000000000000000000000000\n",
    );
    assert!(
        matches!(without_section, Err(AudioImportError::InvalidMetaText(_))),
        "a meta without an AudioImporter section must be reported"
    );
    Ok(())
}

#[test]
fn sample_normal_map_imports_in_normal_map_mode_and_not_as_an_srgb_colour() -> TestResult {
    let bytes = fs::read(sample_asset(NORMAL_MAP_ASSET)?)?;
    let meta = sample_meta(NORMAL_MAP_ASSET)?;
    let settings = TextureImportSettings::from_meta_text(&meta)?;
    assert_eq!(
        settings.texture_type,
        TextureImporterType::NormalMap,
        "the meta must import the texture as a normal map"
    );
    assert!(
        !settings.srgb,
        "a normal map must not be imported as an sRGB colour texture"
    );
    assert!(settings.is_normal_map());
    assert!(settings.is_linear());

    let imported = import_texture(&bytes, NORMAL_MAP_ASSET, &settings)?;
    assert_eq!(imported.texture.width, NORMAL_MAP_SIZE);
    assert_eq!(imported.texture.height, NORMAL_MAP_SIZE);
    assert_eq!(imported.texture.channels(), 3);
    assert!(
        imported.mips.len() > 1,
        "the meta mipmap setting must build a mip chain"
    );

    for pixel in imported.texture.pixels.chunks_exact(3) {
        let normal = [
            f32::from(pixel[0]) / 127.5 - 1.0,
            f32::from(pixel[1]) / 127.5 - 1.0,
            f32::from(pixel[2]) / 127.5 - 1.0,
        ];
        let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        assert!(
            (length - 1.0).abs() < 0.05,
            "every pixel must be a unit tangent-space normal: {normal:?}"
        );
    }

    let centre = imported
        .texture
        .pixel(NORMAL_MAP_SIZE / 2, NORMAL_MAP_SIZE / 2)
        .ok_or("the normal map must expose its centre pixel")?;
    assert_eq!(
        centre[0], 128,
        "a flat normal must keep red at the mid value"
    );
    assert_eq!(
        centre[1], 128,
        "a flat normal must keep green at the mid value"
    );
    assert_eq!(centre[2], 255, "a flat normal must point along +Z");
    assert!(
        imported
            .texture
            .pixels
            .chunks_exact(3)
            .any(|pixel| pixel[0] < 124),
        "the normal map must carry a real surface slope"
    );
    assert!(
        imported
            .texture
            .pixels
            .chunks_exact(3)
            .any(|pixel| pixel[0] > 132),
        "the normal map must carry a real surface slope"
    );
    Ok(())
}

#[test]
fn reopening_the_sample_project_keeps_the_new_assets_guid_and_file_id_identity() -> TestResult {
    let root = copied_sample()?;
    let trust_dir = temp_dir("reopen_trust")?;
    let (_, report) = open_sample(&root, &trust_dir)?;
    assert!(
        report.inventory.duplicate_guids.is_empty(),
        "the sample must declare no duplicate guids: {:?}",
        report.inventory.duplicate_guids
    );

    let audio = asset_entry(&report, OGG_GUID)?;
    assert_eq!(audio.path, PathBuf::from(OGG_ASSET));
    assert_eq!(audio.class.as_deref(), Some("AudioClip"));
    let normal_map = asset_entry(&report, NORMAL_MAP_GUID)?;
    assert_eq!(normal_map.path, PathBuf::from(NORMAL_MAP_ASSET));
    assert_eq!(normal_map.class.as_deref(), Some("Texture2D"));

    let material_path = root.join(PICKUP_MATERIAL);
    let (guid, file_id) = material_texture_reference(&material_path, "_BumpMap")?;
    assert_eq!(
        guid, NORMAL_MAP_GUID,
        "the material must wire the new normal map"
    );
    assert_eq!(
        file_id, TEXTURE_FILE_ID,
        "the material must reference the normal map with the Texture2D fileID"
    );

    let audio_meta = fs::read_to_string(root.join(format!("{OGG_ASSET}.meta")))?;
    let normal_map_meta = fs::read_to_string(root.join(format!("{NORMAL_MAP_ASSET}.meta")))?;

    let (_, reopened) = open_sample(&root, &trust_dir)?;
    let reopened_audio = asset_entry(&reopened, OGG_GUID)?;
    assert_eq!(
        reopened_audio.path,
        PathBuf::from(OGG_ASSET),
        "the OGG must keep its identity across a reopen"
    );
    assert_eq!(reopened_audio.class.as_deref(), Some("AudioClip"));
    let reopened_map = asset_entry(&reopened, NORMAL_MAP_GUID)?;
    assert_eq!(
        reopened_map.path,
        PathBuf::from(NORMAL_MAP_ASSET),
        "the normal map must keep its identity across a reopen"
    );
    assert_eq!(reopened_map.class.as_deref(), Some("Texture2D"));
    let (reopened_guid, reopened_file_id) = material_texture_reference(&material_path, "_BumpMap")?;
    assert_eq!(reopened_guid, NORMAL_MAP_GUID);
    assert_eq!(reopened_file_id, TEXTURE_FILE_ID);
    assert_eq!(
        fs::read_to_string(root.join(format!("{OGG_ASSET}.meta")))?,
        audio_meta,
        "opening the project must not rewrite the OGG meta"
    );
    assert_eq!(
        fs::read_to_string(root.join(format!("{NORMAL_MAP_ASSET}.meta")))?,
        normal_map_meta,
        "opening the project must not rewrite the normal map meta"
    );
    Ok(())
}

#[test]
fn the_sample_project_settings_expose_tags_layers_and_the_build_scene_list() -> TestResult {
    let settings_dir = sample_root().join("ProjectSettings");
    if !settings_dir.is_dir() {
        return Ok(());
    }

    let settings = load_project_settings(&settings_dir)?;

    assert!(
        settings.graphics.is_none(),
        "the sample ships no GraphicsSettings asset"
    );
    assert!(
        settings.audio.is_none(),
        "the sample ships no AudioManager asset"
    );
    assert!(
        settings.nav_mesh_areas.is_none(),
        "the sample ships no NavMeshAreas asset"
    );
    assert!(
        settings.editor.is_none(),
        "the sample ships no EditorSettings asset"
    );
    assert!(
        settings.mono_manager.is_none(),
        "the sample ships no MonoManager asset"
    );
    assert!(
        settings.preset_manager.is_none(),
        "the sample ships no PresetManager asset"
    );
    assert!(
        settings.package_manager.is_none(),
        "the sample ships no PackageManagerSettings asset"
    );
    assert!(
        settings.version_control.is_none(),
        "the sample ships no VersionControlSettings asset"
    );
    assert_eq!(settings.fixed_timestep(), 0.02);
    assert_eq!(settings.time_scale, 1.0);
    assert_eq!(settings.gravity, [0.0, -9.81, 0.0]);
    assert_eq!(
        settings.identity.product_name.as_deref(),
        Some("Marble Rush")
    );
    assert_eq!(settings.quality.levels.len(), 3);

    let tag_manager = settings
        .tag_manager
        .ok_or("the sample must ship a TagManager asset")?;
    assert_eq!(
        tag_manager.tags(),
        vec!["MainCamera".to_string(), "Player".to_string()],
        "the sample tags must be read from the TagManager asset"
    );
    let layers = tag_manager.layers();
    assert_eq!(layers.len(), 32, "Unity declares 32 layers");
    assert_eq!(layers[0], "Default");
    assert_eq!(layers[1], "TransparentFX");
    assert_eq!(layers[2], "Ignore Raycast");
    assert_eq!(layers[4], "Water");
    assert_eq!(layers[5], "UI");
    assert_eq!(layers[8], "User Layer 8");
    assert_eq!(layers[31], "User Layer 31");
    let sorting_layers = tag_manager.sorting_layers();
    assert_eq!(
        sorting_layers.len(),
        1,
        "the sample declares one sorting layer"
    );
    assert_eq!(sorting_layers[0].name, "Default");
    assert_eq!(sorting_layers[0].unique_id, 0);
    assert!(!sorting_layers[0].locked);

    let build = settings
        .editor_build
        .ok_or("the sample must ship an EditorBuildSettings asset")?;
    let scenes = build.scenes();
    assert_eq!(scenes.len(), 2, "the sample builds its two scenes");
    assert_eq!(scenes[0].path, "Assets/Scenes/MarbleRush.unity");
    assert_eq!(
        scenes[0].guid,
        sample_asset_guid("Assets/Scenes/MarbleRush.unity")?
    );
    assert!(scenes[0].enabled, "the main scene must be enabled");
    assert_eq!(scenes[1].path, "Assets/Scenes/PhysicsLab.unity");
    assert_eq!(
        scenes[1].guid,
        sample_asset_guid("Assets/Scenes/PhysicsLab.unity")?
    );
    assert!(scenes[1].enabled, "the physics lab scene must be enabled");
    Ok(())
}
