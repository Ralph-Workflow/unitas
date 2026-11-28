//! The scene's serialized `m_audioClip` reference has to reach the audio world
//! as the clip the asset importer produced, because the managed `AudioSource`
//! binds the native clip id the runtime reports for that source.

use std::path::Path;

use unitas_runtime::{
    Runtime, test_read_path_buffer, test_write_path_buffer, unitas_runtime_audio_mix,
    unitas_runtime_audio_play, unitas_runtime_audio_set_listener,
    unitas_runtime_audio_set_position, unitas_runtime_audio_source_config, unitas_runtime_destroy,
    unitas_runtime_load_scene, unitas_runtime_new, unitas_runtime_path_buffer,
};

fn scene_audio_clip_guid(scene_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let scene_text = std::fs::read_to_string(scene_path)?;
    let guid = scene_text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("m_audioClip: "))
        .find_map(|value| {
            value.split_once("guid: ").map(|(_, guid)| {
                guid.split(',')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            })
        })
        .ok_or("scene audio clip reference not found")?;
    Ok(guid)
}

#[test]
fn marble_rush_collect_chime_binds_the_imported_clip() -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = Runtime::new()?;
    let scene_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples/marble-rush/Assets/Scenes/MarbleRush.unity");
    let expected_guid = scene_audio_clip_guid(&scene_path)?;
    runtime.load_scene(&scene_path)?;

    let scene = runtime.scene(0).ok_or("scene not loaded")?;
    let (entity, config) = scene
        .audio_sources()
        .iter()
        .find(|(_, config)| config.clip_guid.as_deref() == Some(expected_guid.as_str()))
        .ok_or("scene audio source with the serialized clip guid not found")?;
    let entity = *entity;
    assert!(config.clip_diagnostic.is_none(), "clip diagnostic present");

    let source = runtime
        .audio()
        .source(entity)
        .ok_or("audio source missing")?;
    let clip_id = source.clip.ok_or("audio source clip unresolved")?;
    let clip = runtime.audio().clip(clip_id).ok_or("clip not loaded")?;

    // CollectChime.wav is 8000 Hz mono with 2000 frames; a stand-in clip would
    // carry the default 44100 Hz rate and no frames of the asset.
    assert_eq!(clip.sample_rate, 8000);
    assert_eq!(clip.channels, 1);
    assert_eq!(clip.samples.len(), 2000);
    assert!(clip.duration > 0.24 && clip.duration < 0.26);
    Ok(())
}

/// The PhysicsLab scene's `PositionalChime` source is fully spatial
/// (`m_SpatialBlend: 1`). Through the headless mix ABI its stereo image must
/// pan toward the ear on its side and its level must fall with distance.
#[test]
fn physics_lab_positional_source_pans_and_attenuates_through_the_mix_abi()
-> Result<(), Box<dyn std::error::Error>> {
    const FRAMES: usize = 512;

    let handle = unitas_runtime_new();
    assert_ne!(handle, 0, "runtime handle creation failed");

    let scene_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples/marble-rush/Assets/Scenes/PhysicsLab.unity");
    let scene_bytes = scene_path.to_string_lossy().into_owned().into_bytes();
    let address = unitas_runtime_path_buffer(handle, scene_bytes.len() as u64);
    assert_ne!(address, 0);
    test_write_path_buffer(handle, &scene_bytes)?;
    assert_eq!(
        unitas_runtime_load_scene(handle, address, scene_bytes.len() as u64),
        0
    );

    // The native scene keys an AudioSource by its GameObject entity, which is
    // not known on the ABI, so discover the positional source by its unique
    // full spatial blend.
    let config_addr = unitas_runtime_path_buffer(handle, 44);
    assert_ne!(config_addr, 0);
    let mut entity = 0u64;
    for candidate in 1..=1024u64 {
        if unitas_runtime_audio_source_config(handle, candidate, config_addr) != 0 {
            continue;
        }
        let mut bytes = [0u8; 44];
        test_read_path_buffer(handle, &mut bytes)?;
        let spatial_blend = f32::from_le_bytes(bytes[8..12].try_into()?);
        let max_distance = f32::from_le_bytes(bytes[16..20].try_into()?);
        if spatial_blend == 1.0 && max_distance == 60.0 {
            entity = candidate;
            break;
        }
    }
    assert_ne!(entity, 0, "positional scene audio source not found");

    assert_eq!(
        unitas_runtime_audio_set_listener(handle, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0),
        0
    );
    assert_eq!(unitas_runtime_audio_play(handle, entity, 0), 0);

    let mix_addr = unitas_runtime_path_buffer(handle, (FRAMES * 8) as u64);
    assert_ne!(mix_addr, 0);
    let channel_energy = |handle: u64| -> Result<(f64, f64), Box<dyn std::error::Error>> {
        assert_eq!(
            unitas_runtime_audio_mix(handle, FRAMES as u64, mix_addr),
            FRAMES as u64
        );
        let mut bytes = vec![0u8; FRAMES * 8];
        test_read_path_buffer(handle, &mut bytes)?;
        let mut left = 0.0f64;
        let mut right = 0.0f64;
        for frame in 0..FRAMES {
            let l = f32::from_le_bytes(bytes[frame * 8..frame * 8 + 4].try_into()?);
            let r = f32::from_le_bytes(bytes[frame * 8 + 4..frame * 8 + 8].try_into()?);
            left += (l as f64) * (l as f64);
            right += (r as f64) * (r as f64);
        }
        Ok((left, right))
    };

    // Scene places the source at (4, 0, 2), right of a +Z-facing listener:
    // the right ear must dominate.
    let (near_left, near_right) = channel_energy(handle)?;
    assert!(
        near_right > near_left * 8.0,
        "expected a right-side pan, left={near_left} right={near_right}"
    );

    // Moving the source away (still inside maxDistance=60) must attenuate it.
    assert_eq!(
        unitas_runtime_audio_set_position(handle, entity, 4.0, 0.0, 40.0),
        0
    );
    let (far_left, far_right) = channel_energy(handle)?;
    assert!(
        far_left + far_right < (near_left + near_right) / 16.0,
        "expected distance attenuation, near={} far={}",
        near_left + near_right,
        far_left + far_right
    );

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}
