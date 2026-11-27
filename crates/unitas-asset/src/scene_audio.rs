use std::collections::HashSet;
use std::path::PathBuf;

use crate::meta::GuidIndex;
use crate::unity_yaml::{Node, Stream};

use super::{
    reference_id, yaml_bool, yaml_f32, yaml_u8, AssetGuid, AudioBinding, AudioSourceComponent,
    GameObjectRecords, SceneError, AUDIO_CLIP_CLASS, AUDIO_SOURCE_CLASS, DEFAULT_AUDIO_ENABLED,
    DEFAULT_AUDIO_LOOPING, DEFAULT_AUDIO_MAX_DISTANCE, DEFAULT_AUDIO_MIN_DISTANCE,
    DEFAULT_AUDIO_MUTE, DEFAULT_AUDIO_PITCH, DEFAULT_AUDIO_PLAY_ON_AWAKE, DEFAULT_AUDIO_PRIORITY,
    DEFAULT_AUDIO_SPATIAL_BLEND, DEFAULT_AUDIO_VOLUME, ROOT_FILE_ID,
};

pub(super) fn collect_audio(
    stream: &Stream,
    game_objects: &GameObjectRecords,
    instance_roots: &HashSet<i64>,
    index: Option<&GuidIndex>,
) -> Result<Vec<AudioBinding>, SceneError> {
    let mut bindings = Vec::new();
    for document in &stream.documents {
        if document.stripped {
            continue;
        }
        if document.class_id == AUDIO_CLIP_CLASS {
            continue;
        }
        if document.class_id != AUDIO_SOURCE_CLASS {
            continue;
        }
        let body = match document.root.get("AudioSource") {
            Some(body) => body,
            None => continue,
        };
        let game_object_file_id = match reference_id(body.get("m_GameObject"))? {
            Some(file_id) => file_id,
            None => {
                return Err(SceneError::MissingGameObject {
                    transform_file_id: document.file_id,
                    game_object_file_id: ROOT_FILE_ID,
                })
            }
        };
        let record = match game_objects.get(&game_object_file_id) {
            Some(record) => record,
            None => {
                if instance_roots.contains(&game_object_file_id) {
                    continue;
                }
                return Err(SceneError::MissingGameObject {
                    transform_file_id: document.file_id,
                    game_object_file_id,
                });
            }
        };
        let component = audio_source_component(body, &record.name, index);
        bindings.push((record.entity, component));
    }
    Ok(bindings)
}

fn audio_source_component(
    body: &Node,
    game_object_name: &str,
    index: Option<&GuidIndex>,
) -> AudioSourceComponent {
    let (clip_guid, clip_path, clip_diagnostic) = resolve_audio_clip(body, game_object_name, index);
    let volume = yaml_f32(body.get("m_Volume"), DEFAULT_AUDIO_VOLUME, |value| {
        (0.0..=1.0).contains(&value)
    });
    let pitch = yaml_f32(body.get("m_Pitch"), DEFAULT_AUDIO_PITCH, |value| {
        value.is_finite()
    });
    let looping = yaml_bool(
        body.get("m_Loop").or_else(|| body.get("Loop")),
        DEFAULT_AUDIO_LOOPING,
    );
    let play_on_awake = yaml_bool(body.get("m_PlayOnAwake"), DEFAULT_AUDIO_PLAY_ON_AWAKE);
    let priority = yaml_u8(
        body.get("m_Priority").or_else(|| body.get("Priority")),
        DEFAULT_AUDIO_PRIORITY,
    );
    let spatial_blend_node = body.get("m_SpatialBlend").or_else(|| {
        body.get("panLevelCustomCurve").and_then(|curve| {
            curve
                .get("m_Curve")
                .and_then(Node::as_sequence)
                .and_then(|seq| seq.first())
                .and_then(|point| point.get("value"))
                .or(Some(curve))
        })
    });
    let spatial_blend = yaml_f32(spatial_blend_node, DEFAULT_AUDIO_SPATIAL_BLEND, |value| {
        (0.0..=1.0).contains(&value)
    });
    let min_distance = yaml_f32(
        body.get("m_MinDistance")
            .or_else(|| body.get("MinDistance")),
        DEFAULT_AUDIO_MIN_DISTANCE,
        |value| value >= 0.0,
    );
    let max_distance = yaml_f32(
        body.get("m_MaxDistance")
            .or_else(|| body.get("MaxDistance")),
        DEFAULT_AUDIO_MAX_DISTANCE,
        |value| value > 0.0,
    );
    let mute = yaml_bool(
        body.get("m_Mute").or_else(|| body.get("Mute")),
        DEFAULT_AUDIO_MUTE,
    );
    AudioSourceComponent {
        enabled: yaml_bool(body.get("m_Enabled"), DEFAULT_AUDIO_ENABLED),
        clip_guid,
        clip_path,
        clip_diagnostic,
        volume,
        pitch,
        looping,
        play_on_awake,
        priority,
        spatial_blend,
        min_distance,
        max_distance,
        mute,
    }
}

fn resolve_audio_clip(
    body: &Node,
    game_object_name: &str,
    index: Option<&GuidIndex>,
) -> (Option<AssetGuid>, Option<PathBuf>, Option<String>) {
    let clip_node = match body.get("m_audioClip").or_else(|| body.get("m_AudioClip")) {
        Some(node) => node,
        None => return (None, None, None),
    };
    let file_id = clip_node.get("fileID").and_then(Node::as_i64);
    let guid = clip_node
        .get("guid")
        .and_then(Node::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let guid = match (file_id, guid) {
        (Some(0) | None, None) => return (None, None, None),
        (Some(id), None) => {
            return (
                None,
                None,
                Some(format!(
                    "AudioSource on '{game_object_name}' references m_audioClip fileID {id} without a guid"
                )),
            )
        }
        (None | Some(_), Some(guid)) => guid.to_string(),
    };

    let path = match index.and_then(|guids| guids.get(&guid)) {
        Some(path) => path.to_path_buf(),
        None => {
            return (
                Some(guid.clone()),
                None,
                Some(format!(
                "AudioSource on '{game_object_name}' references unresolved AudioClip guid {guid}"
            )),
            )
        }
    };

    let supported = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("wav"))
        .unwrap_or(false);

    if !supported {
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown");
        return (
            Some(guid.clone()),
            Some(path.clone()),
            Some(format!(
                "AudioSource on '{game_object_name}' AudioClip guid {guid} ({}): unsupported audio format .{ext} (only .wav is supported)",
                path.display()
            )),
        );
    }

    (Some(guid), Some(path), None)
}
