use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use unitas_asset::project::{
    BASELINE_UNITY_VERSION, MigrationStatus, Project, build_compatibility_report,
};
use unitas_asset::project_settings::load_project_settings;
use unitas_asset::scene;
use unitas_asset::scene_save::emit_stream;
use unitas_asset::texture::import::{
    FilterMode, SpriteMode, TextureImportSettings, TextureImporterType, WrapMode, import_texture,
};
use unitas_asset::trust::TrustStore;
use unitas_asset::unity_yaml;

fn platformer_2d_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/2d-platformer")
}

fn temp_trust_store(name: &str) -> std::io::Result<TrustStore> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "unitas_platformer_2d_{name}_{}_{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir)?;
    Ok(TrustStore::new(dir.join("trust.json")))
}

#[test]
fn platformer_2d_project_opens_current_with_supported_compatibility_and_2d_settings()
-> Result<(), Box<dyn Error>> {
    let root = platformer_2d_root();
    let trust = temp_trust_store("open")?;
    let (_, report) = Project::open(&root, &trust)?;

    assert_eq!(report.declared_version.as_deref(), Some("2022.3.62f1"));
    assert_eq!(report.effective_behavior_version, BASELINE_UNITY_VERSION);
    assert_eq!(report.migration, MigrationStatus::Current);
    assert!(
        report.inventory.duplicate_guids.is_empty(),
        "sample project must not have duplicate GUIDs: {:?}",
        report.inventory.duplicate_guids
    );
    assert!(
        report.trust_gate.blocked_vectors.is_empty(),
        "sample project must not trigger blocked trust vectors: {:?}",
        report.trust_gate.blocked_vectors
    );
    assert!(
        report.package_modifications.is_empty(),
        "sample packages must open without modification findings: {:?}",
        report.package_modifications
    );

    let inventory_paths: BTreeSet<String> = report
        .inventory
        .entries
        .iter()
        .map(|entry| entry.path.to_string_lossy().replace('\\', "/"))
        .collect();
    for expected in [
        "Assets/Scenes/Rootway.unity",
        "Assets/Scripts/PlayerController2D.cs",
        "Assets/Sprites/Wick.png",
        "Assets/Sprites/GroundTiles.png",
        "Assets/Sprites/Ember.png",
        "Assets/Sprites/OneWayPlatform.png",
    ] {
        assert!(
            inventory_paths.contains(expected),
            "inventory missing {expected}; got {inventory_paths:?}"
        );
    }

    let compatibility = build_compatibility_report(&root, &report);
    assert!(
        compatibility.accounts_for_all(&report),
        "compatibility report must account for every asset, package, and setting"
    );
    let unsupported: Vec<String> = compatibility
        .asset_findings
        .iter()
        .chain(compatibility.package_findings.iter())
        .chain(compatibility.setting_findings.iter())
        .filter(|finding| finding.state != "Supported")
        .map(|finding| format!("{}: {}", finding.identity, finding.state))
        .collect();
    assert!(
        unsupported.is_empty(),
        "every feature the 2d-platformer sample uses must be Supported: {unsupported:?}"
    );

    let settings = load_project_settings(&root)?;
    assert_eq!(
        settings.identity.product_name.as_deref(),
        Some("WicksLantern")
    );
    assert_eq!(settings.identity.company_name.as_deref(), Some("Unitas"));
    assert!((settings.fixed_timestep - 0.02).abs() < 1e-6);
    assert_eq!(settings.gravity_2d, [0.0, -9.81]);
    assert!(settings.physics_2d.queries_hit_triggers);

    let build = settings
        .editor_build
        .as_ref()
        .ok_or("EditorBuildSettings.asset must be loaded")?;
    assert_eq!(build.scenes.len(), 1);
    assert_eq!(build.scenes[0].path, "Assets/Scenes/Rootway.unity");
    assert!(build.scenes[0].enabled);

    let tags_and_layers = settings
        .tag_manager
        .as_ref()
        .ok_or("TagManager.asset must be loaded")?;
    for tag in ["MainCamera", "Player", "Ember", "Checkpoint", "Hazard", "GoalBell"] {
        assert!(
            tags_and_layers.tags.iter().any(|entry| entry == tag),
            "TagManager must include tag {tag}"
        );
    }
    assert_eq!(tags_and_layers.layers[6], "Ground");
    assert_eq!(tags_and_layers.layers[7], "Player");
    assert_eq!(tags_and_layers.layers[8], "OneWay");
    assert_eq!(tags_and_layers.layers[9], "Hazard");
    assert_eq!(tags_and_layers.layers[10], "Pickup");
    assert_eq!(tags_and_layers.layers[11], "Checkpoint");

    let sorting_names: Vec<&str> = tags_and_layers
        .sorting_layers
        .iter()
        .map(|layer| layer.name.as_str())
        .collect();
    assert_eq!(
        sorting_names,
        vec![
            "Background",
            "Parallax",
            "Tiles",
            "Props",
            "Characters",
            "Foreground",
            "Effects"
        ]
    );
    Ok(())
}

#[test]
fn platformer_2d_main_scene_assembles_and_round_trips_byte_identically()
-> Result<(), Box<dyn Error>> {
    let root = platformer_2d_root();
    let scene_path = root.join("Assets/Scenes/Rootway.unity");
    let raw = fs::read_to_string(&scene_path)?;
    let stream = unity_yaml::parse_file(&scene_path)?;
    let assembled = scene::assemble_scene_file_bundle(&scene_path)?;

    assert!(
        assembled
            .diagnostics
            .iter()
            .all(|(_, message)| message.starts_with("unsupported component class")),
        "the saved scene must import without structural diagnostics: {:?}",
        assembled.diagnostics
    );

    let names: BTreeSet<String> = assembled
        .graph
        .roots()
        .iter()
        .filter_map(|id| assembled.graph.entity(*id).map(|entity| entity.name.clone()))
        .collect();
    for expected in [
        "Main Camera",
        "Wick",
        "Ground",
        "OneWayBridge",
        "ConveyorLedge",
        "Ember_1",
        "Ember_2",
        "Checkpoint_1",
        "ThornHazard",
        "GoalBell",
    ] {
        assert!(
            names.contains(expected),
            "assembled scene missing entity {expected}; got {names:?}"
        );
    }

    assert_eq!(assembled.render.cameras.len(), 1);
    let camera = &assembled.render.cameras[0];
    assert!(camera.orthographic);
    assert!((camera.orthographic_size - 5.625).abs() < 1e-4);

    let wick_script = assembled
        .scripts
        .iter()
        .find(|binding| {
            assembled
                .graph
                .entity(binding.entity)
                .is_some_and(|entity| entity.name == "Wick")
        })
        .ok_or("Wick entity must bind PlayerController2D script")?;
    assert_eq!(wick_script.guid, "2d000000000000000000000000000002");

    assert!(
        assembled.render.sprites.len() >= 4,
        "expected at least 4 SpriteRenderer bindings in Rootway scene, got {}",
        assembled.render.sprites.len()
    );
    for sprite in &assembled.render.sprites {
        assert!(
            sprite.sprite_path.is_some(),
            "SpriteRenderer on entity {:?} must resolve sprite_path via GUID",
            sprite.entity
        );
    }

    let emitted = emit_stream(&stream);
    assert_eq!(
        emitted, raw,
        "Rootway.unity must round-trip byte-identically through scene_save::emit_stream"
    );
    Ok(())
}

#[test]
fn platformer_2d_sprites_import_with_point_filter_and_no_mipmaps() -> Result<(), Box<dyn Error>> {
    let sprites_dir = platformer_2d_root().join("Assets/Sprites");
    for sprite_name in ["Wick.png", "GroundTiles.png", "Ember.png", "OneWayPlatform.png"] {
        let png_path = sprites_dir.join(sprite_name);
        let meta_path = sprites_dir.join(format!("{sprite_name}.meta"));
        let png_bytes = fs::read(&png_path)?;
        let meta_text = fs::read_to_string(&meta_path)?;
        let settings = TextureImportSettings::from_meta_text(&meta_text)?;

        assert_eq!(settings.texture_type, TextureImporterType::Sprite);
        assert_eq!(settings.filter_mode, FilterMode::Point);
        assert_eq!(settings.wrap_mode, WrapMode::Clamp);
        assert!(!settings.mipmaps);
        assert!(settings.srgb);
        assert!(settings.alpha_is_transparency);
        assert!(matches!(
            settings.sprite_mode,
            SpriteMode::Single | SpriteMode::Multiple
        ));

        let imported = import_texture(&png_bytes, sprite_name, &settings)?;
        assert!(imported.texture.width >= 16);
        assert!(imported.texture.height >= 16);
        assert_eq!(imported.mips.len(), 1);
        assert!(!imported.mips[0].pixels.is_empty());
    }
    Ok(())
}
