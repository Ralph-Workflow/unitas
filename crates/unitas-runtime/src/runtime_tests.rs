use super::*;
use unitas_audio::{AudioClip, AudioMixerGroup, SilenceReason};

#[test]
fn roll_a_ball_scene_binds_collect_chime_audio_source_clip()
-> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = Runtime::new()?;
    let scene_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples/roll-a-ball/Assets/Scenes/RollABall.unity");
    runtime.load_scene(&scene_path)?;

    let scene_text = std::fs::read_to_string(&scene_path)?;
    let expected_guid = scene_text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("m_audioClip: "))
        .find_map(|value| {
            value
                .split_once("guid: ")
                .map(|(_, guid)| guid.split(',').next().unwrap_or_default().trim())
        })
        .ok_or("scene audio clip reference not found")?;

    let scene = runtime.scenes.first().ok_or("scene not loaded")?;
    let chime_entity = scene
        .graph
        .breadth_first_order()
        .into_iter()
        .find(|entity| runtime.entity_name(*entity) == Some("CollectChime"))
        .ok_or("CollectChime entity not found")?;

    let (_, config) = scene
        .audio_sources
        .iter()
        .find(|(entity, _)| *entity == chime_entity)
        .ok_or("CollectChime audio source not bound")?;
    assert_eq!(config.clip_guid.as_deref(), Some(expected_guid));

    let source = runtime
        .audio()
        .source(chime_entity)
        .ok_or("CollectChime audio source missing")?;
    let clip_id = source.clip.ok_or("CollectChime clip unresolved")?;
    assert!(clip_id > 0);
    let clip = runtime
        .audio()
        .clip(clip_id)
        .ok_or("CollectChime clip not loaded")?;
    assert!(clip.duration > 0.0);
    Ok(())
}

#[test]
fn runtime_time_scale_pause_routes_sfx_music_and_menu_groups()
-> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = Runtime::new()?;
    let clip_id = runtime.audio_mut().register_clip(AudioClip {
        channels: 1,
        sample_rate: 100,
        samples: vec![0.4; 10],
        duration: 0.10,
    });
    let sfx = EntityId::new(100);
    let music = EntityId::new(101);
    let menu = EntityId::new(102);
    for entity in [sfx, music, menu] {
        runtime.audio_mut().ensure_source(entity);
        runtime.audio_mut().set_source_clip(entity, Some(clip_id));
    }
    runtime
        .audio_mut()
        .set_source_group(music, AudioMixerGroup::Music);
    runtime
        .audio_mut()
        .set_source_group(menu, AudioMixerGroup::Menu);
    for entity in [sfx, music, menu] {
        runtime.audio_mut().play(entity, false);
    }

    runtime.tick(0.0)?;
    runtime.set_time_scale(0.0);
    assert!(runtime.audio().is_gameplay_paused());
    assert_eq!(
        runtime.audio().inaudible_reason(sfx),
        Some(SilenceReason::GameplayPaused)
    );
    assert_eq!(
        runtime.audio().inaudible_reason(music),
        Some(SilenceReason::GameplayPaused)
    );
    assert_eq!(runtime.audio().inaudible_reason(menu), None);

    runtime.tick(0.04)?;
    assert_eq!(runtime.audio().source(sfx).map(|s| s.time), Some(0.0));
    assert_eq!(runtime.audio().source(music).map(|s| s.time), Some(0.0));
    assert_eq!(runtime.audio().source(menu).map(|s| s.time), Some(0.04));

    runtime.set_time_scale(1.0);
    assert!(!runtime.audio().is_gameplay_paused());
    assert_eq!(runtime.audio().inaudible_reason(sfx), None);
    assert_eq!(runtime.audio().inaudible_reason(music), None);
    Ok(())
}

#[test]
fn physics2d_project_settings_and_abi_entry_points_configure_world_2d_and_survive_clear()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{
        unitas_runtime_destroy, unitas_runtime_new, unitas_runtime_physics2d_apply_settings,
        unitas_runtime_physics2d_clear, unitas_runtime_physics2d_get_layer_collision,
        unitas_runtime_physics2d_set_layer_collision,
    };

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_root = std::env::temp_dir().join(format!(
        "unitas_runtime_p2d_settings_{}_{nanos}",
        std::process::id()
    ));
    let settings_dir = temp_root.join("ProjectSettings");
    let scenes_dir = temp_root.join("Assets").join("Scenes");
    std::fs::create_dir_all(&settings_dir)?;
    std::fs::create_dir_all(&scenes_dir)?;

    let mut matrix_bits = [0xFFFF_FFFFu32; 32];
    matrix_bits[0] &= !(1u32 << 2);
    matrix_bits[2] &= !(1u32 << 0);
    let matrix_hex = unitas_asset::project_settings::format_layer_collision_matrix(&matrix_bits);
    let physics2d_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!19 &1\nPhysics2DSettings:\n  m_ObjectHideFlags: 0\n  serializedVersion: 5\n  m_Gravity: {{x: 2.5, y: -14.0}}\n  m_VelocityIterations: 11\n  m_PositionIterations: 6\n  m_QueriesHitTriggers: 0\n  m_QueriesStartInColliders: 0\n  m_CallbacksOnDisable: 0\n  m_AutoSyncTransforms: 0\n  m_LayerCollisionMatrix: {matrix_hex}\n"
    );
    std::fs::write(settings_dir.join("Physics2DSettings.asset"), physics2d_yaml)?;

    let scene_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Root\n  m_Layer: 0\n--- !u!4 &101\nTransform:\n  m_GameObject: {fileID: 100}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n";
    let scene_path = scenes_dir.join("Test2D.unity");
    std::fs::write(&scene_path, scene_yaml)?;

    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene_path)?;

    let world_2d = runtime.physics_mut().world_2d();
    assert_eq!(world_2d.gravity(), [2.5, -14.0]);
    assert_eq!(world_2d.velocity_iterations(), 11);
    assert_eq!(world_2d.position_iterations(), 6);
    assert!(!world_2d.queries_hit_triggers());
    assert!(!world_2d.queries_start_in_colliders());
    assert!(!world_2d.callbacks_on_disable());
    assert!(!world_2d.auto_sync_transforms());
    assert!(world_2d.get_ignore_layer_collision(0, 2));
    assert!(!world_2d.get_ignore_layer_collision(0, 1));

    runtime.physics_mut().clear_2d();
    let cleared_2d = runtime.physics_mut().world_2d();
    assert_eq!(cleared_2d.gravity(), [2.5, -14.0]);
    assert_eq!(cleared_2d.velocity_iterations(), 11);
    assert_eq!(cleared_2d.position_iterations(), 6);
    assert!(!cleared_2d.queries_hit_triggers());
    assert!(cleared_2d.get_ignore_layer_collision(0, 2));

    std::fs::remove_dir_all(&temp_root)?;

    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    assert_eq!(
        unitas_runtime_physics2d_apply_settings(handle, 0.0, -20.0, 9, 4, 0),
        0
    );
    assert_eq!(
        unitas_runtime_physics2d_get_layer_collision(handle, 1, 2),
        1
    );
    assert_eq!(
        unitas_runtime_physics2d_set_layer_collision(handle, 1, 2, 0),
        0
    );
    assert_eq!(
        unitas_runtime_physics2d_get_layer_collision(handle, 1, 2),
        0
    );
    assert_eq!(
        unitas_runtime_physics2d_get_layer_collision(handle, 2, 1),
        0
    );
    assert_eq!(unitas_runtime_physics2d_clear(handle), 0);
    assert_eq!(
        unitas_runtime_physics2d_get_layer_collision(handle, 1, 2),
        0
    );
    assert_eq!(
        unitas_runtime_physics2d_set_layer_collision(handle, 32, 0, 1),
        -2
    );
    assert_eq!(
        unitas_runtime_physics2d_get_layer_collision(handle, 0, 32),
        -2
    );
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn update_audio_syncs_listener_pose_from_active_camera_and_rolloff_ffi_round_trips()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{
        test_write_path_buffer, unitas_runtime_audio_create, unitas_runtime_audio_rolloff_mode,
        unitas_runtime_audio_set_custom_rolloff, unitas_runtime_audio_set_rolloff_mode,
        unitas_runtime_audio_source_set_custom_rolloff,
        unitas_runtime_audio_source_set_rolloff_mode, unitas_runtime_destroy, unitas_runtime_new,
        unitas_runtime_object_create, unitas_runtime_object_set_transform,
        unitas_runtime_path_buffer, unitas_runtime_render_camera_create, unitas_runtime_tick,
    };
    use unitas_audio::RolloffMode;

    let mut runtime = Runtime::new()?;
    let camera_entity = runtime.create_runtime_object("MainCamera")?;
    let sin_45 = std::f64::consts::FRAC_1_SQRT_2;
    runtime.set_runtime_object_transform(
        camera_entity,
        [10.0, 2.0, -5.0],
        [0.0, sin_45, 0.0, sin_45],
        [1.0, 1.0, 1.0],
    )?;
    let _cam_handle = runtime.create_render_camera(Some(camera_entity));
    let src = runtime.ensure_audio_source(0);
    runtime.audio_mut().set_spatial_blend(src, 1.0);
    runtime.audio_mut().set_min_distance(src, 2.0);
    runtime.audio_mut().set_max_distance(src, 20.0);
    runtime
        .audio_mut()
        .set_source_position(src, [14.0, 2.0, -5.0]);
    runtime.audio_mut().play(src, false);
    runtime.tick(0.0)?;
    runtime.tick(1.0 / 60.0)?;

    let listener = runtime
        .audio()
        .listener()
        .ok_or("listener not synced from active camera")?;
    assert!((listener.position[0] - 10.0).abs() < 1e-5);
    assert!((listener.position[1] - 2.0).abs() < 1e-5);
    assert!((listener.position[2] - (-5.0)).abs() < 1e-5);
    assert!((listener.forward[0] - 1.0).abs() < 1e-5);
    assert!(listener.forward[1].abs() < 1e-5 && listener.forward[2].abs() < 1e-5);

    let (gain_l, gain_r) = runtime.audio().source_gains(src).unwrap_or((0.0, 0.0));
    assert!((gain_l - 0.25).abs() < 1e-5 && (gain_r - 0.25).abs() < 1e-5);

    runtime.set_runtime_object_transform(
        camera_entity,
        [12.0, 2.0, -5.0],
        [0.0, sin_45, 0.0, sin_45],
        [1.0, 1.0, 1.0],
    )?;
    runtime.tick(1.0 / 60.0)?;
    let (near_l, near_r) = runtime.audio().source_gains(src).unwrap_or((0.0, 0.0));
    assert!((near_l - 0.5).abs() < 1e-5 && (near_r - 0.5).abs() < 1e-5);

    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    let cam_id = unitas_runtime_object_create(handle, 0, 0);
    assert_eq!(
        unitas_runtime_object_set_transform(
            handle, cam_id, 3.0, 4.0, 5.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0
        ),
        0
    );
    assert_ne!(unitas_runtime_render_camera_create(handle, cam_id), 0);
    let ffi_src = unitas_runtime_audio_create(handle, 0);
    assert_eq!(
        unitas_runtime_audio_rolloff_mode(handle, ffi_src),
        RolloffMode::Logarithmic.as_i32()
    );
    assert_eq!(
        unitas_runtime_audio_source_set_rolloff_mode(handle, ffi_src, 1),
        0
    );
    assert_eq!(
        unitas_runtime_audio_rolloff_mode(handle, ffi_src),
        RolloffMode::Linear.as_i32()
    );
    assert_eq!(unitas_runtime_audio_set_rolloff_mode(handle, ffi_src, 0), 0);
    assert_eq!(
        unitas_runtime_audio_rolloff_mode(handle, ffi_src),
        RolloffMode::Logarithmic.as_i32()
    );

    let buf_addr = unitas_runtime_path_buffer(handle, 64);
    let mut curve_bytes = Vec::new();
    for (dist, gain) in [(0.0f32, 1.0f32), (0.5, 0.25), (1.0, 0.0)] {
        curve_bytes.extend_from_slice(&dist.to_le_bytes());
        curve_bytes.extend_from_slice(&gain.to_le_bytes());
    }
    test_write_path_buffer(handle, &curve_bytes)?;
    assert_eq!(
        unitas_runtime_audio_source_set_custom_rolloff(handle, ffi_src, 3, buf_addr),
        0
    );
    assert_eq!(
        unitas_runtime_audio_rolloff_mode(handle, ffi_src),
        RolloffMode::Custom.as_i32()
    );
    assert_eq!(
        unitas_runtime_audio_set_custom_rolloff(handle, ffi_src, 3, buf_addr),
        0
    );
    assert_eq!(unitas_runtime_tick(handle, 0.0), 0);
    assert_eq!(unitas_runtime_tick(handle, 0.016), 0);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn scene_load_decodes_ogg_mp3_aiff_and_wav_audio_source_clips()
-> Result<(), Box<dyn std::error::Error>> {
    let ogg_bytes = include_bytes!("../../unitas-asset/tests/fixtures/sine_mono.ogg");
    let mp3_bytes = include_bytes!("../../unitas-asset/tests/fixtures/sine_mono.mp3");
    let mut aiff_form = Vec::new();
    aiff_form.extend_from_slice(b"AIFFCOMM");
    aiff_form.extend_from_slice(&18u32.to_be_bytes());
    aiff_form.extend_from_slice(&1u16.to_be_bytes());
    aiff_form.extend_from_slice(&2u32.to_be_bytes());
    aiff_form.extend_from_slice(&16u16.to_be_bytes());
    aiff_form.extend_from_slice(&[0x40, 0x0e, 0xac, 0x44, 0, 0, 0, 0, 0, 0]);
    aiff_form.extend_from_slice(b"SSND");
    aiff_form.extend_from_slice(&12u32.to_be_bytes());
    aiff_form.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0x03, 0xe8, 0xfc, 0x18]);
    let mut aiff_bytes = Vec::from(&b"FORM"[..]);
    aiff_bytes.extend_from_slice(&(aiff_form.len() as u32).to_be_bytes());
    aiff_bytes.extend_from_slice(&aiff_form);

    let temp_root = std::env::temp_dir().join(format!(
        "unitas_runtime_scene_audio_formats_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_root);
    let audio_dir = temp_root.join("Assets").join("Audio");
    let scenes_dir = temp_root.join("Assets").join("Scenes");
    std::fs::create_dir_all(&audio_dir)?;
    std::fs::create_dir_all(&scenes_dir)?;
    std::fs::create_dir_all(temp_root.join("ProjectSettings"))?;

    let ogg_guid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let mp3_guid = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let aiff_guid = "cccccccccccccccccccccccccccccccc";
    for (name, guid, data) in [
        ("clip.ogg", ogg_guid, &ogg_bytes[..]),
        ("clip.mp3", mp3_guid, &mp3_bytes[..]),
        ("clip.aiff", aiff_guid, aiff_bytes.as_slice()),
    ] {
        std::fs::write(audio_dir.join(name), data)?;
        std::fs::write(
            audio_dir.join(format!("{name}.meta")),
            format!("fileFormatVersion: 2\nguid: {guid}\n"),
        )?;
    }

    let scene_yaml = format!(
        "%YAML 1.1\n--- !u!1 &10\nGameObject:\n  m_Name: OggSrc\n--- !u!4 &11\nTransform:\n  m_GameObject: {{fileID: 10}}\n  m_Father: {{fileID: 0}}\n--- !u!82 &12\nAudioSource:\n  m_GameObject: {{fileID: 10}}\n  m_audioClip: {{fileID: 8300000, guid: {ogg_guid}, type: 3}}\n--- !u!1 &20\nGameObject:\n  m_Name: Mp3Src\n--- !u!4 &21\nTransform:\n  m_GameObject: {{fileID: 20}}\n  m_Father: {{fileID: 0}}\n--- !u!82 &22\nAudioSource:\n  m_GameObject: {{fileID: 20}}\n  m_audioClip: {{fileID: 8300000, guid: {mp3_guid}, type: 3}}\n--- !u!1 &30\nGameObject:\n  m_Name: AiffSrc\n--- !u!4 &31\nTransform:\n  m_GameObject: {{fileID: 30}}\n  m_Father: {{fileID: 0}}\n--- !u!82 &32\nAudioSource:\n  m_GameObject: {{fileID: 30}}\n  m_audioClip: {{fileID: 8300000, guid: {aiff_guid}, type: 3}}\n"
    );
    let scene_path = scenes_dir.join("MultiAudio.unity");
    std::fs::write(&scene_path, scene_yaml)?;

    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene_path)?;
    assert!(runtime.diagnostics().is_empty());
    let scene = runtime.scenes.first().ok_or("scene not loaded")?;
    assert_eq!(scene.audio_sources.len(), 3);
    for (entity, _) in &scene.audio_sources {
        let source = runtime
            .audio()
            .source(*entity)
            .ok_or("missing audio source")?;
        assert!(!source.unsupported_clip);
        let clip_id = source.clip.ok_or("missing clip id")?;
        let clip = runtime
            .audio()
            .clip(clip_id)
            .ok_or("missing decoded clip")?;
        assert!(clip.frame_count() > 0);
        assert!(clip.duration > 0.0);
    }
    std::fs::remove_dir_all(&temp_root)?;
    Ok(())
}
