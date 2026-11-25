use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::database::{
    identify_asset_family, import_text_asset, AssetFilter, DatabaseError, RecognizedAssetFamily,
};
use unitas_asset::dependencies::AssetImportState;
use unitas_asset::project::{AssetKind, GuidLookup, Project, BASELINE_UNITY_VERSION};
use unitas_asset::trust::TrustStore;

type TestResult = Result<(), Box<dyn Error>>;

const MATERIAL_GUID: &str = "11111111111111111111111111111111";
const STONE_MATERIAL_GUID: &str = "55555555555555555555555555555555";
const TEXTURE_GUID: &str = "22222222222222222222222222222222";
const SCENE_GUID: &str = "33333333333333333333333333333333";

const MATERIAL: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  serializedVersion: 6\n  m_Name: Brick\n";

const STONE_MATERIAL: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  serializedVersion: 6\n  m_Name: StoneBrick\n";

const SCENE: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!29 &1\nScene:\n  m_Name: Main\n--- !u!1 &1000\nGameObject:\n  m_Name: Cube\n  m_Material: {fileID: 2100000, guid: 11111111111111111111111111111111, type: 2}\n";

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path =
        std::env::temp_dir().join(format!("unitas-db-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn write_file(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

fn meta_with_labels(guid: &str, importer: &str, labels: &[&str]) -> String {
    let mut text = format!("fileFormatVersion: 2\nguid: {guid}\n");
    if !labels.is_empty() {
        text.push_str("labels:\n");
        for label in labels {
            text.push_str(&format!("- {label}\n"));
        }
    }
    text.push_str(&format!(
        "{importer}\n  externalObjects: {{}}\n  serializedVersion: 2\n  userData: \n"
    ));
    text
}

fn texture_meta(guid: &str) -> String {
    format!(
        "fileFormatVersion: 2\nguid: {guid}\nlabels:\n- environment\nTextureImporter:\n  externalObjects: {{}}\n  serializedVersion: 12\n  spriteMode: 2\n  spriteSheet:\n    serializedVersion: 2\n    sprites:\n    - serializedVersion: 2\n      name: Brick_Slice_A\n      rect:\n        serializedVersion: 2\n        x: 0\n        y: 0\n        width: 8\n        height: 8\n    - serializedVersion: 2\n      name: Tile_Slice_B\n      rect:\n        serializedVersion: 2\n        x: 8\n        y: 0\n        width: 8\n        height: 8\n"
    )
}

fn build_fixture(root: &Path) -> std::io::Result<()> {
    write_file(&root.join("Assets/Materials/Brick.mat"), MATERIAL)?;
    write_file(
        &root.join("Assets/Materials/Brick.mat.meta"),
        &meta_with_labels(
            MATERIAL_GUID,
            "NativeFormatImporter:",
            &["architecture", "masonry"],
        ),
    )?;
    write_file(
        &root.join("Assets/Materials/StoneBrick.mat"),
        STONE_MATERIAL,
    )?;
    write_file(
        &root.join("Assets/Materials/StoneBrick.mat.meta"),
        &meta_with_labels(STONE_MATERIAL_GUID, "NativeFormatImporter:", &["nature"]),
    )?;
    write_file(
        &root.join("Assets/Textures/BrickSheet.png"),
        "not a real png",
    )?;
    write_file(
        &root.join("Assets/Textures/BrickSheet.png.meta"),
        &texture_meta(TEXTURE_GUID),
    )?;
    write_file(&root.join("Assets/Scenes/Main.unity"), SCENE)?;
    write_file(
        &root.join("Assets/Scenes/Main.unity.meta"),
        &meta_with_labels(SCENE_GUID, "NativeFormatImporter:", &["level"]),
    )?;
    write_file(
        &root.join("ProjectSettings/ProjectVersion.txt"),
        &format!(
            "m_EditorVersion: {BASELINE_UNITY_VERSION}.16f1\nm_EditorVersionWithRevision: {BASELINE_UNITY_VERSION}.16f1 (0cf5a3b0e6a3)\n"
        ),
    )?;
    Ok(())
}

fn referenced_material_guid(scene_path: &Path) -> Result<String, Box<dyn Error>> {
    let text = fs::read_to_string(scene_path)?;
    for line in text.lines() {
        if let Some(index) = line.find("guid:") {
            let value = line[index + "guid:".len()..]
                .split([',', '}'])
                .next()
                .unwrap_or("")
                .trim();
            if value.len() == 32 {
                return Ok(value.to_string());
            }
        }
    }
    Err("scene holds no GUID reference".into())
}

#[test]
fn move_preserves_guid_and_reference() -> TestResult {
    let root = temp_dir("move")?;
    let project_root = root.join("project");
    build_fixture(&project_root)?;
    let store = TrustStore::new(root.join("trust.json"));

    let (mut project, _) = Project::open(&project_root, &store)?;
    let ref_guid = referenced_material_guid(&project_root.join("Assets/Scenes/Main.unity"))?;
    assert_eq!(ref_guid, MATERIAL_GUID);

    project.move_asset(
        Path::new("Assets/Materials/Brick.mat"),
        Path::new("Assets/Moved/Walls/BrickWall.mat"),
    )?;

    assert!(!project_root.join("Assets/Materials/Brick.mat").exists());
    assert!(!project_root
        .join("Assets/Materials/Brick.mat.meta")
        .exists());
    assert!(project_root
        .join("Assets/Moved/Walls/BrickWall.mat")
        .is_file());
    assert!(project_root
        .join("Assets/Moved/Walls/BrickWall.mat.meta")
        .is_file());

    match project.inventory().lookup(&ref_guid) {
        GuidLookup::Asset(entry) => {
            assert_eq!(entry.guid, MATERIAL_GUID);
            assert_eq!(
                entry.path,
                PathBuf::from("Assets/Moved/Walls/BrickWall.mat")
            );
        }
        other => return Err(format!("expected moved asset, got {other:?}").into()),
    }

    let duplicate_dest = project.move_asset(
        Path::new("Assets/Moved/Walls/BrickWall.mat"),
        Path::new("Assets/Materials/StoneBrick.mat"),
    );
    assert_eq!(
        duplicate_dest,
        Err(DatabaseError::DestinationExists {
            path: PathBuf::from("Assets/Materials/StoneBrick.mat"),
        })
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn delete_exposes_missing_reference() -> TestResult {
    let root = temp_dir("delete")?;
    let project_root = root.join("project");
    build_fixture(&project_root)?;
    let store = TrustStore::new(root.join("trust.json"));

    let (mut project, _) = Project::open(&project_root, &store)?;
    let ref_guid = referenced_material_guid(&project_root.join("Assets/Scenes/Main.unity"))?;

    project.delete_asset(Path::new("Assets/Materials/Brick.mat"))?;

    assert!(!project_root.join("Assets/Materials/Brick.mat").exists());
    assert!(!project_root
        .join("Assets/Materials/Brick.mat.meta")
        .exists());
    assert!(project_root
        .join(".unitas/trash/Assets/Materials/Brick.mat")
        .is_file());
    assert!(project_root
        .join(".unitas/trash/Assets/Materials/Brick.mat.meta")
        .is_file());

    assert_eq!(project.inventory().lookup(&ref_guid), GuidLookup::Missing);

    let (_, reopened_report) = Project::open(&project_root, &store)?;
    assert_eq!(
        reopened_report.inventory.lookup(&ref_guid),
        GuidLookup::Missing
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn restore_re_resolves_reference() -> TestResult {
    let root = temp_dir("restore")?;
    let project_root = root.join("project");
    build_fixture(&project_root)?;
    let store = TrustStore::new(root.join("trust.json"));

    let (mut project, _) = Project::open(&project_root, &store)?;
    let ref_guid = referenced_material_guid(&project_root.join("Assets/Scenes/Main.unity"))?;

    project.delete_asset(Path::new("Assets/Materials/Brick.mat"))?;
    assert_eq!(project.inventory().lookup(&ref_guid), GuidLookup::Missing);

    project.restore_asset(Path::new("Assets/Materials/Brick.mat"))?;

    assert!(project_root.join("Assets/Materials/Brick.mat").is_file());
    assert!(project_root
        .join("Assets/Materials/Brick.mat.meta")
        .is_file());
    assert!(!project_root
        .join(".unitas/trash/Assets/Materials/Brick.mat")
        .exists());

    match project.inventory().lookup(&ref_guid) {
        GuidLookup::Asset(entry) => {
            assert_eq!(entry.guid, MATERIAL_GUID);
            assert_eq!(entry.path, PathBuf::from("Assets/Materials/Brick.mat"));
        }
        other => return Err(format!("expected restored asset, got {other:?}").into()),
    }

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn search_combines_filters_and_distinguishes_sub_assets() -> TestResult {
    let root = temp_dir("search")?;
    let project_root = root.join("project");
    build_fixture(&project_root)?;
    let store = TrustStore::new(root.join("trust.json"));

    let (project, _) = Project::open(&project_root, &store)?;

    let combined_main = project.search_assets(&AssetFilter {
        name_contains: Some("Brick".to_string()),
        class: Some("Material".to_string()),
        label: Some("architecture".to_string()),
        folder: None,
        state: None,
        include_sub_assets: false,
    });
    assert_eq!(combined_main.len(), 1);
    assert_eq!(combined_main[0].guid, MATERIAL_GUID);
    assert_eq!(combined_main[0].kind, AssetKind::Main);

    let all_brick_main = project.search_assets(&AssetFilter {
        name_contains: Some("Brick".to_string()),
        class: None,
        label: None,
        folder: None,
        state: None,
        include_sub_assets: false,
    });
    assert_eq!(all_brick_main.len(), 3);
    assert!(all_brick_main
        .iter()
        .all(|entry| entry.kind == AssetKind::Main));

    let all_brick_with_subs = project.search_assets(&AssetFilter {
        name_contains: Some("Brick".to_string()),
        class: None,
        label: None,
        folder: None,
        state: None,
        include_sub_assets: true,
    });
    assert_eq!(all_brick_with_subs.len(), 5);
    let sub_matches: Vec<_> = all_brick_with_subs
        .iter()
        .filter(|entry| entry.kind == AssetKind::Sub)
        .collect();
    assert_eq!(sub_matches.len(), 2);

    let specific_sprite_sub = project.search_assets(&AssetFilter {
        name_contains: Some("Slice_A".to_string()),
        class: Some("Sprite".to_string()),
        label: Some("Brick_Slice_A".to_string()),
        folder: None,
        state: None,
        include_sub_assets: true,
    });
    assert_eq!(specific_sprite_sub.len(), 1);
    assert_eq!(specific_sprite_sub[0].kind, AssetKind::Sub);
    assert_eq!(specific_sprite_sub[0].guid, TEXTURE_GUID);

    let sub_excluded_when_disabled = project.search_assets(&AssetFilter {
        name_contains: Some("Slice_A".to_string()),
        class: Some("Sprite".to_string()),
        label: Some("Brick_Slice_A".to_string()),
        folder: None,
        state: None,
        include_sub_assets: false,
    });
    assert!(sub_excluded_when_disabled.is_empty());

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn prj002_outcome3_move_rename_delete_and_restore_preserve_expose_and_restore_references(
) -> TestResult {
    let root = temp_dir("prj002-outcome3")?;
    let project_root = root.join("project");
    build_fixture(&project_root)?;
    let store = TrustStore::new(root.join("trust.json"));

    let (mut project, _) = Project::open(&project_root, &store)?;
    let ref_guid = referenced_material_guid(&project_root.join("Assets/Scenes/Main.unity"))?;
    assert_eq!(ref_guid, MATERIAL_GUID, "PRJ-002 Outcome 3 initial GUID matches");

    project.rename_asset(Path::new("Assets/Materials/Brick.mat"), "BrickRenamed.mat")?;
    match project.inventory().lookup(&ref_guid) {
        GuidLookup::Asset(entry) => {
            assert_eq!(entry.guid, MATERIAL_GUID, "PRJ-002 Outcome 3 rename preserves GUID");
            assert_eq!(entry.path, PathBuf::from("Assets/Materials/BrickRenamed.mat"));
        }
        other => return Err(format!("PRJ-002 Outcome 3 expected renamed asset, got {other:?}").into()),
    }

    project.move_asset(
        Path::new("Assets/Materials/BrickRenamed.mat"),
        Path::new("Assets/Moved/BrickFinal.mat"),
    )?;
    match project.inventory().lookup(&ref_guid) {
        GuidLookup::Asset(entry) => {
            assert_eq!(entry.guid, MATERIAL_GUID, "PRJ-002 Outcome 3 move preserves GUID");
            assert_eq!(entry.path, PathBuf::from("Assets/Moved/BrickFinal.mat"));
        }
        other => return Err(format!("PRJ-002 Outcome 3 expected moved asset, got {other:?}").into()),
    }

    project.delete_asset(Path::new("Assets/Moved/BrickFinal.mat"))?;
    assert_eq!(
        project.inventory().lookup(&ref_guid),
        GuidLookup::Missing,
        "PRJ-002 Outcome 3 delete exposes missing reference without losing GUID"
    );
    assert_eq!(
        referenced_material_guid(&project_root.join("Assets/Scenes/Main.unity"))?,
        MATERIAL_GUID,
        "PRJ-002 Outcome 3 referring scene retains original GUID while missing"
    );

    project.restore_asset(Path::new("Assets/Moved/BrickFinal.mat"))?;
    match project.inventory().lookup(&ref_guid) {
        GuidLookup::Asset(entry) => {
            assert_eq!(entry.guid, MATERIAL_GUID, "PRJ-002 Outcome 3 restore re-resolves reference");
            assert_eq!(entry.path, PathBuf::from("Assets/Moved/BrickFinal.mat"));
        }
        other => return Err(format!("PRJ-002 Outcome 3 expected restored asset, got {other:?}").into()),
    }

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn prj002_outcome4_combined_filter_search_matches_type_label_folder_name_state_and_sub_assets(
) -> TestResult {
    let root = temp_dir("prj002-outcome4")?;
    let project_root = root.join("project");
    build_fixture(&project_root)?;
    write_file(
        &project_root.join("Assets/Textures/Legacy.pict"),
        "pict-header",
    )?;
    write_file(
        &project_root.join("Assets/Textures/Legacy.pict.meta"),
        &meta_with_labels(
            "77777777777777777777777777777777",
            "TextureImporter:",
            &["environment"],
        ),
    )?;
    let store = TrustStore::new(root.join("trust.json"));
    let (project, _) = Project::open(&project_root, &store)?;

    let ready_materials_in_folder = project.search_assets(&AssetFilter {
        name_contains: Some("Brick".to_string()),
        class: Some("Material".to_string()),
        label: Some("architecture".to_string()),
        folder: Some(PathBuf::from("Assets/Materials")),
        state: Some(AssetImportState::Ready),
        include_sub_assets: false,
    });
    assert_eq!(
        ready_materials_in_folder.len(),
        1,
        "PRJ-002 Outcome 4 combined filter matches all 5 criteria"
    );
    assert_eq!(ready_materials_in_folder[0].guid, MATERIAL_GUID);
    assert_eq!(ready_materials_in_folder[0].kind, AssetKind::Main);

    let wrong_folder = project.search_assets(&AssetFilter {
        name_contains: Some("Brick".to_string()),
        class: Some("Material".to_string()),
        label: Some("architecture".to_string()),
        folder: Some(PathBuf::from("Assets/Textures")),
        state: Some(AssetImportState::Ready),
        include_sub_assets: false,
    });
    assert!(
        wrong_folder.is_empty(),
        "PRJ-002 Outcome 4 folder filter excludes non-matching folder"
    );

    let failed_in_textures = project.search_assets(&AssetFilter {
        name_contains: Some("Legacy".to_string()),
        class: Some("Texture2D".to_string()),
        label: Some("environment".to_string()),
        folder: Some(PathBuf::from("Assets/Textures")),
        state: Some(AssetImportState::Failed),
        include_sub_assets: false,
    });
    assert_eq!(
        failed_in_textures.len(),
        1,
        "PRJ-002 Outcome 4 state filter matches failed PICT texture"
    );
    assert_eq!(
        failed_in_textures[0].path,
        PathBuf::from("Assets/Textures/Legacy.pict")
    );

    let sub_sprite_in_textures = project.search_assets(&AssetFilter {
        name_contains: Some("Slice_B".to_string()),
        class: Some("Sprite".to_string()),
        label: Some("Tile_Slice_B".to_string()),
        folder: Some(PathBuf::from("Assets/Textures")),
        state: Some(AssetImportState::Ready),
        include_sub_assets: true,
    });
    assert_eq!(
        sub_sprite_in_textures.len(),
        1,
        "PRJ-002 Outcome 4 combined filter finds sub-asset when include_sub_assets is true"
    );
    assert_eq!(sub_sprite_in_textures[0].kind, AssetKind::Sub);

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn prj002_prj004_outcome17_json_and_bytes_import_as_text_asset_and_avatar_mask_audio_mixer_and_sprite_atlas_v2_are_identified_by_family(
) -> TestResult {
    let json_payload = br#"{"level":42,"title":"Dungeon"}"#;
    let json_asset = import_text_asset(json_payload, "Assets/Data/config.json")?;
    assert_eq!(json_asset.name, "config");
    assert_eq!(
        json_asset.text,
        r#"{"level":42,"title":"Dungeon"}"#,
        "PRJ-002/PRJ-004 Outcome 17 .json TextAsset text matches file content"
    );
    assert_eq!(
        json_asset.bytes, json_payload,
        "PRJ-002/PRJ-004 Outcome 17 .json TextAsset bytes match file content"
    );

    let binary_payload: &[u8] = &[0x00, 0xFF, 0x42, 0x13, 0x37, 0x80];
    let bytes_asset = import_text_asset(binary_payload, "Assets/Data/ raw_table.bytes")?;
    assert_eq!(
        bytes_asset.bytes, binary_payload,
        "PRJ-002/PRJ-004 Outcome 17 .bytes TextAsset bytes match file content exactly"
    );
    assert_eq!(
        bytes_asset.text,
        String::from_utf8_lossy(binary_payload),
        "PRJ-002/PRJ-004 Outcome 17 .bytes TextAsset text matches lossy UTF-8 content"
    );

    let json_family = identify_asset_family(Path::new("Assets/Data/config.json"), json_payload, None)
        .ok_or("expected family for config.json")?;
    assert_eq!(json_family.family, RecognizedAssetFamily::TextAsset);
    assert_eq!(json_family.family.class_name(), "TextAsset");
    assert_eq!(json_family.outcome.state, AssetImportState::Ready);

    let bytes_family = identify_asset_family(
        Path::new("Assets/Data/raw_table.bytes"),
        binary_payload,
        None,
    )
    .ok_or("expected family for raw_table.bytes")?;
    assert_eq!(bytes_family.family, RecognizedAssetFamily::TextAsset);
    assert_eq!(bytes_family.outcome.state, AssetImportState::Ready);

    let avatar_mask_yaml = b"%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!319 &31900000\nAvatarMask:\n  m_Name: UpperBody\n";
    let mask_family = identify_asset_family(
        Path::new("Assets/Animations/UpperBody.mask"),
        avatar_mask_yaml,
        None,
    )
    .ok_or("expected family for UpperBody.mask")?;
    assert_eq!(
        mask_family.family,
        RecognizedAssetFamily::AvatarMask,
        "PRJ-002 Outcome 17 avatar mask identified with family"
    );
    assert_eq!(mask_family.family.family_label(), "AvatarMask");
    assert_eq!(mask_family.outcome.state, AssetImportState::Ready);

    let audio_mixer_yaml = b"%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!241 &24100000\nAudioMixerController:\n  m_Name: MasterMixer\n";
    let mixer_family = identify_asset_family(
        Path::new("Assets/Audio/MasterMixer.mixer"),
        audio_mixer_yaml,
        None,
    )
    .ok_or("expected family for MasterMixer.mixer")?;
    assert_eq!(
        mixer_family.family,
        RecognizedAssetFamily::AudioMixer,
        "PRJ-002 Outcome 17 audio mixer identified with family"
    );
    assert_eq!(mixer_family.family.family_label(), "AudioMixer");
    assert_eq!(
        mixer_family.outcome.state,
        AssetImportState::ReadyWithLimitations,
        "PRJ-004 Outcome 5/17 audio mixer reports honest compatibility state"
    );
    assert!(
        mixer_family
            .outcome
            .diagnostics
            .iter()
            .any(|diag| !diag.consequence.is_empty()),
        "PRJ-004 Outcome 5 audio mixer diagnostic states consequence"
    );

    let sprite_atlas_v2_yaml = b"%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!687078895 &4343727234628468602\nSpriteAtlasAsset:\n  m_Name: UIAtlas\n";
    let atlas_v2_family = identify_asset_family(
        Path::new("Assets/Sprites/UIAtlas.spriteatlasv2"),
        sprite_atlas_v2_yaml,
        Some("fileFormatVersion: 2\nguid: 88888888888888888888888888888888\nSpriteAtlasImporter:\n"),
    )
    .ok_or("expected family for UIAtlas.spriteatlasv2")?;
    assert_eq!(
        atlas_v2_family.family,
        RecognizedAssetFamily::SpriteAtlasV2,
        "PRJ-002 Outcome 17 Sprite Atlas v2 identified with family"
    );
    assert_eq!(atlas_v2_family.family.family_label(), "SpriteAtlas v2");
    assert_eq!(atlas_v2_family.outcome.state, AssetImportState::Ready);

    let root = temp_dir("prj002-outcome17-project")?;
    let project_root = root.join("project");
    build_fixture(&project_root)?;
    write_file(
        &project_root.join("Assets/Data/config.json"),
        r#"{"level":42}"#,
    )?;
    write_file(
        &project_root.join("Assets/Data/config.json.meta"),
        &meta_with_labels(
            "81818181818181818181818181818181",
            "TextScriptImporter:",
            &["data"],
        ),
    )?;
    write_file(
        &project_root.join("Assets/Animations/UpperBody.mask"),
        std::str::from_utf8(avatar_mask_yaml)?,
    )?;
    write_file(
        &project_root.join("Assets/Animations/UpperBody.mask.meta"),
        &meta_with_labels(
            "82828282828282828282828282828282",
            "NativeFormatImporter:",
            &["anim"],
        ),
    )?;
    write_file(
        &project_root.join("Assets/Audio/MasterMixer.mixer"),
        std::str::from_utf8(audio_mixer_yaml)?,
    )?;
    write_file(
        &project_root.join("Assets/Audio/MasterMixer.mixer.meta"),
        &meta_with_labels(
            "83838383838383838383838383838383",
            "NativeFormatImporter:",
            &["audio"],
        ),
    )?;
    write_file(
        &project_root.join("Assets/Sprites/UIAtlas.spriteatlasv2"),
        std::str::from_utf8(sprite_atlas_v2_yaml)?,
    )?;
    write_file(
        &project_root.join("Assets/Sprites/UIAtlas.spriteatlasv2.meta"),
        &meta_with_labels(
            "84848484848484848484848484848484",
            "SpriteAtlasImporter:",
            &["ui"],
        ),
    )?;

    let store = TrustStore::new(root.join("trust.json"));
    let (project, _) = Project::open(&project_root, &store)?;

    assert_eq!(
        project
            .search_assets(&AssetFilter {
                class: Some("TextAsset".to_string()),
                ..AssetFilter::default()
            })
            .len(),
        1
    );
    assert_eq!(
        project
            .search_assets(&AssetFilter {
                class: Some("AvatarMask".to_string()),
                ..AssetFilter::default()
            })
            .len(),
        1
    );
    assert_eq!(
        project
            .search_assets(&AssetFilter {
                class: Some("AudioMixerController".to_string()),
                state: Some(AssetImportState::ReadyWithLimitations),
                ..AssetFilter::default()
            })
            .len(),
        1
    );
    assert_eq!(
        project
            .search_assets(&AssetFilter {
                class: Some("SpriteAtlasAsset".to_string()),
                ..AssetFilter::default()
            })
            .len(),
        1
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}
