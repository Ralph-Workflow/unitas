//! The scene's serialized `m_audioClip` reference has to reach the audio world
//! as the clip the asset importer produced, because the managed `AudioSource`
//! binds the native clip id the runtime reports for that source.

use std::path::Path;

use unitas_runtime::Runtime;

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
