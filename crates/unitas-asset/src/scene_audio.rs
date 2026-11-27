use std::collections::HashSet;
use std::path::PathBuf;

use crate::audio_import::is_supported_audio_path;
use crate::meta::GuidIndex;
use crate::unity_yaml::{Document, Node, Stream};
use unitas_core::EntityId;

use super::{
    reference_id, yaml_bool, yaml_f32, yaml_u8, AssetGuid, AudioBinding, AudioSourceComponent,
    GameObjectRecords, SceneError, AUDIO_CLIP_CLASS, AUDIO_SOURCE_CLASS, DEFAULT_AUDIO_ENABLED,
    DEFAULT_AUDIO_LOOPING, DEFAULT_AUDIO_MAX_DISTANCE, DEFAULT_AUDIO_MIN_DISTANCE,
    DEFAULT_AUDIO_MUTE, DEFAULT_AUDIO_PITCH, DEFAULT_AUDIO_PLAY_ON_AWAKE, DEFAULT_AUDIO_PRIORITY,
    DEFAULT_AUDIO_SPATIAL_BLEND, DEFAULT_AUDIO_VOLUME, ROOT_FILE_ID,
};

const LOW_PASS_FILTER_BODY: &str = "AudioLowPassFilter";
const HIGH_PASS_FILTER_BODY: &str = "AudioHighPassFilter";
const ECHO_FILTER_BODY: &str = "AudioEchoFilter";
const DEFAULT_FILTER_CUTOFF_HZ: f32 = 5000.0;
const DEFAULT_FILTER_RESONANCE_Q: f32 = 1.0;
const DEFAULT_ECHO_DELAY_SECONDS: f32 = 0.5;
const DEFAULT_ECHO_DRY_DB: f32 = 0.0;
const DEFAULT_ECHO_FEEDBACK_DB: f32 = -10.0;
const DEFAULT_ECHO_WET_DB: f32 = 0.0;
const MIN_FILTER_CUTOFF_HZ: f32 = 10.0;
const MAX_FILTER_CUTOFF_HZ: f32 = 22000.0;
const MIN_FILTER_RESONANCE_Q: f32 = 1.0;
const MAX_FILTER_RESONANCE_Q: f32 = 10.0;
const MIN_ECHO_DELAY_SECONDS: f32 = 0.02;
const MAX_ECHO_DELAY_SECONDS: f32 = 1.0;
const MIN_ECHO_DB: f32 = -60.0;
const MAX_ECHO_DRY_DB: f32 = 0.0;
const MAX_ECHO_FEEDBACK_DB: f32 = 20.0;
const MAX_ECHO_WET_DB: f32 = 0.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioFilterKind {
    LowPass,
    HighPass,
    Echo,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioFilterComponent {
    pub kind: AudioFilterKind,
    pub enabled: bool,
    pub cutoff_hz: f32,
    pub resonance_q: f32,
    pub delay_seconds: f32,
    pub dry_db: f32,
    pub feedback_db: f32,
    pub wet_db: f32,
}

pub fn collect_audio_filters(
    stream: &Stream,
    game_objects: &GameObjectRecords,
    instance_roots: &HashSet<i64>,
) -> Result<Vec<(EntityId, AudioFilterComponent)>, SceneError> {
    let mut bindings = Vec::new();
    for document in &stream.documents {
        if document.stripped {
            continue;
        }
        let Some((kind, body)) = filter_body(document) else {
            continue;
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
        bindings.push((record.entity, audio_filter_component(kind, body)));
    }
    Ok(bindings)
}

fn filter_body(document: &Document) -> Option<(AudioFilterKind, &Node)> {
    if let Some(body) = document.root.get(LOW_PASS_FILTER_BODY) {
        return Some((AudioFilterKind::LowPass, body));
    }
    if let Some(body) = document.root.get(HIGH_PASS_FILTER_BODY) {
        return Some((AudioFilterKind::HighPass, body));
    }
    if let Some(body) = document.root.get(ECHO_FILTER_BODY) {
        return Some((AudioFilterKind::Echo, body));
    }
    None
}

fn audio_filter_component(kind: AudioFilterKind, body: &Node) -> AudioFilterComponent {
    let enabled = yaml_bool(
        body.get("m_Enabled").or_else(|| body.get("m_enabled")),
        true,
    );
    let cutoff_hz = yaml_f32(
        body.get("m_CutoffFrequency")
            .or_else(|| body.get("CutoffFrequency"))
            .or_else(|| body.get("cutoffFrequency")),
        DEFAULT_FILTER_CUTOFF_HZ,
        |value| (MIN_FILTER_CUTOFF_HZ..=MAX_FILTER_CUTOFF_HZ).contains(&value),
    );
    let resonance_q = yaml_f32(
        body.get("m_LowpassResonanceQ")
            .or_else(|| body.get("m_HighpassResonanceQ"))
            .or_else(|| body.get("LowpassResonanceQ"))
            .or_else(|| body.get("HighpassResonanceQ"))
            .or_else(|| body.get("lowpassResonanceQ"))
            .or_else(|| body.get("highpassResonanceQ")),
        DEFAULT_FILTER_RESONANCE_Q,
        |value| (MIN_FILTER_RESONANCE_Q..=MAX_FILTER_RESONANCE_Q).contains(&value),
    );
    let delay_seconds = yaml_f32(
        body.get("m_EchoDelay")
            .or_else(|| body.get("EchoDelay"))
            .or_else(|| body.get("echoDelay")),
        DEFAULT_ECHO_DELAY_SECONDS,
        |value| (MIN_ECHO_DELAY_SECONDS..=MAX_ECHO_DELAY_SECONDS).contains(&value),
    );
    let dry_db = yaml_f32(
        body.get("m_EchoDryLevel")
            .or_else(|| body.get("EchoDryLevel"))
            .or_else(|| body.get("echoDryLevel")),
        DEFAULT_ECHO_DRY_DB,
        |value| (MIN_ECHO_DB..=MAX_ECHO_DRY_DB).contains(&value),
    );
    let feedback_db = yaml_f32(
        body.get("m_EchoFeedback")
            .or_else(|| body.get("EchoFeedback"))
            .or_else(|| body.get("echoFeedback")),
        DEFAULT_ECHO_FEEDBACK_DB,
        |value| (MIN_ECHO_DB..=MAX_ECHO_FEEDBACK_DB).contains(&value),
    );
    let wet_db = yaml_f32(
        body.get("m_EchoWetLevel")
            .or_else(|| body.get("EchoWetLevel"))
            .or_else(|| body.get("echoWetLevel")),
        DEFAULT_ECHO_WET_DB,
        |value| (MIN_ECHO_DB..=MAX_ECHO_WET_DB).contains(&value),
    );
    AudioFilterComponent {
        kind,
        enabled,
        cutoff_hz,
        resonance_q,
        delay_seconds,
        dry_db,
        feedback_db,
        wet_db,
    }
}

pub(super) fn collect_audio(
    stream: &Stream,
    game_objects: &GameObjectRecords,
    instance_roots: &HashSet<i64>,
    index: Option<&GuidIndex>,
) -> Result<Vec<AudioBinding>, SceneError> {
    let filters = collect_audio_filters(stream, game_objects, instance_roots)?;
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
        let mut component = audio_source_component(body, &record.name, index);
        component.filters = filters
            .iter()
            .filter(|(entity, _)| *entity == record.entity)
            .map(|(_, filter)| *filter)
            .collect();
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
        filters: Vec::new(),
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

    if !is_supported_audio_path(&path) {
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown");
        return (
            Some(guid.clone()),
            Some(path.clone()),
            Some(format!(
                "AudioSource on '{game_object_name}' AudioClip guid {guid} ({}): unsupported audio format .{ext} (supported: .wav, .aiff, .aif, .ogg, .mp3)",
                path.display()
            )),
        );
    }

    (Some(guid), Some(path), None)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};

    use crate::audio_import::{decode_audio_for_path, PcmClip};
    use crate::meta::GuidIndex;
    use crate::scene::{assemble_scene_bundle_with_guids, AudioSourceComponent, GameObjectRecord};
    use crate::unity_yaml;
    use unitas_core::EntityId;

    use super::{collect_audio_filters, AudioFilterKind, GameObjectRecords};

    const OGG_FIXTURE: &[u8] = include_bytes!("../tests/fixtures/sine_mono.ogg");
    const MP3_FIXTURE: &[u8] = include_bytes!("../tests/fixtures/sine_mono.mp3");

    fn build_aiff_16(samples: &[i16]) -> Vec<u8> {
        let mut comm = Vec::with_capacity(18);
        comm.extend_from_slice(&1u16.to_be_bytes());
        comm.extend_from_slice(&(samples.len() as u32).to_be_bytes());
        comm.extend_from_slice(&16u16.to_be_bytes());
        comm.extend_from_slice(&[0x40, 0x0e, 0xac, 0x44, 0, 0, 0, 0, 0, 0]);
        let mut ssnd = Vec::with_capacity(8 + samples.len() * 2);
        ssnd.extend_from_slice(&[0u8; 8]);
        for sample in samples {
            ssnd.extend_from_slice(&sample.to_be_bytes());
        }
        let mut form = Vec::new();
        form.extend_from_slice(b"AIFFCOMM");
        form.extend_from_slice(&(comm.len() as u32).to_be_bytes());
        form.extend_from_slice(&comm);
        form.extend_from_slice(b"SSND");
        form.extend_from_slice(&(ssnd.len() as u32).to_be_bytes());
        form.extend_from_slice(&ssnd);
        let mut out = Vec::with_capacity(8 + form.len());
        out.extend_from_slice(b"FORM");
        out.extend_from_slice(&(form.len() as u32).to_be_bytes());
        out.extend_from_slice(&form);
        out
    }

    fn build_wav_16(samples: &[i16]) -> Vec<u8> {
        let data_len = (samples.len() * 2) as u32;
        let riff_len = 4 + 24 + 8 + data_len;
        let mut out = Vec::with_capacity(8 + riff_len as usize);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&riff_len.to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&44100u32.to_le_bytes());
        out.extend_from_slice(&88200u32.to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            out.extend_from_slice(&sample.to_le_bytes());
        }
        out
    }

    fn single_source_scene(guid: &str) -> String {
        format!(
            "%YAML 1.1\n--- !u!1 &10\nGameObject:\n  m_Name: Emitter\n--- !u!4 &11\nTransform:\n  m_GameObject: {{fileID: 10}}\n  m_Father: {{fileID: 0}}\n--- !u!82 &12\nAudioSource:\n  m_GameObject: {{fileID: 10}}\n  m_audioClip: {{fileID: 8300000, guid: {guid}, type: 3}}\n"
        )
    }

    fn resolve_single_clip(guid: &str, path: &str) -> Result<AudioSourceComponent, String> {
        let mut index = GuidIndex::default();
        index
            .insert(guid.to_string(), PathBuf::from(path))
            .map_err(|err| err.to_string())?;
        let stream =
            unity_yaml::parse(&single_source_scene(guid)).map_err(|err| err.to_string())?;
        let assembled =
            assemble_scene_bundle_with_guids(&stream, &index).map_err(|err| err.to_string())?;
        assembled
            .audio_bindings
            .into_iter()
            .next()
            .map(|(_, comp)| comp)
            .ok_or_else(|| "missing audio binding".to_string())
    }

    fn decode_clip(bytes: &[u8], path: &str) -> Result<PcmClip, String> {
        decode_audio_for_path(bytes, Path::new(path)).map_err(|err| err.to_string())
    }

    #[test]
    fn scene_audio_accepts_ogg_mp3_aiff_and_wav_and_decodes_each() -> Result<(), String> {
        let cases: &[(&str, &str, Vec<u8>)] = &[
            (
                "11111111111111111111111111111111",
                "Assets/Audio/theme.ogg",
                OGG_FIXTURE.to_vec(),
            ),
            (
                "22222222222222222222222222222222",
                "Assets/Audio/voice.MP3",
                MP3_FIXTURE.to_vec(),
            ),
            (
                "33333333333333333333333333333333",
                "Assets/Audio/chime.aiff",
                build_aiff_16(&[0, 1200, -1200, 2400]),
            ),
            (
                "44444444444444444444444444444444",
                "Assets/Audio/short.AIF",
                build_aiff_16(&[500, -500]),
            ),
            (
                "55555555555555555555555555555555",
                "Assets/Audio/beep.wav",
                build_wav_16(&[1000, -1000]),
            ),
        ];
        for (guid, path, bytes) in cases {
            let component = resolve_single_clip(guid, path)?;
            assert_eq!(component.clip_guid.as_deref(), Some(*guid));
            assert_eq!(component.clip_path.as_deref(), Some(Path::new(*path)));
            assert_eq!(component.clip_diagnostic, None);
            let resolved = component
                .clip_path
                .as_deref()
                .ok_or_else(|| "missing clip_path".to_string())?;
            let decoded = decode_clip(bytes, &resolved.to_string_lossy())?;
            assert!(decoded.channels >= 1);
            assert!(decoded.sample_rate > 0);
            assert!(!decoded.samples.is_empty());
        }
        Ok(())
    }

    #[test]
    fn scene_audio_diagnoses_unknown_extensions_with_name_and_asset() -> Result<(), String> {
        for (guid, path, expected_ext) in [
            (
                "66666666666666666666666666666666",
                "Assets/Audio/lossless.flac",
                ".flac",
            ),
            (
                "77777777777777777777777777777777",
                "Assets/Audio/mystery.xyz",
                ".xyz",
            ),
        ] {
            let component = resolve_single_clip(guid, path)?;
            assert_eq!(component.clip_guid.as_deref(), Some(guid));
            assert_eq!(component.clip_path.as_deref(), Some(Path::new(path)));
            let diagnostic = component
                .clip_diagnostic
                .as_deref()
                .ok_or_else(|| "expected diagnostic for unsupported extension".to_string())?;
            assert!(diagnostic.contains("Emitter"));
            assert!(diagnostic.contains(guid));
            assert!(diagnostic.contains(path));
            assert!(diagnostic.contains(expected_ext));
            assert!(decode_clip(b"not-audio", path).is_err());
        }
        Ok(())
    }

    #[test]
    fn scene_audio_parses_filter_components_for_their_audio_source() -> Result<(), String> {
        let yaml = "%YAML 1.1\n--- !u!1 &10\nGameObject:\n  m_Name: Emitter\n--- !u!4 &11\nTransform:\n  m_GameObject: {fileID: 10}\n  m_Father: {fileID: 0}\n--- !u!82 &12\nAudioSource:\n  m_GameObject: {fileID: 10}\n--- !u!90 &13\nAudioLowPassFilter:\n  m_GameObject: {fileID: 10}\n  m_CutoffFrequency: 1200\n  m_LowpassResonanceQ: 2\n--- !u!91 &14\nAudioHighPassFilter:\n  m_GameObject: {fileID: 10}\n  m_CutoffFrequency: 300\n  m_HighpassResonanceQ: 3\n--- !u!92 &15\nAudioEchoFilter:\n  m_GameObject: {fileID: 10}\n  m_EchoDelay: 0.25\n  m_EchoDryLevel: -6\n  m_EchoFeedback: -12\n  m_EchoWetLevel: -3\n";
        let stream = unity_yaml::parse(yaml).map_err(|err| err.to_string())?;
        let mut records = GameObjectRecords::new();
        records.insert(
            10,
            GameObjectRecord {
                entity: EntityId::new(7),
                name: "Emitter".to_string(),
                layer: 0,
            },
        );
        let filters = collect_audio_filters(&stream, &records, &HashSet::new())
            .map_err(|err| err.to_string())?;
        assert_eq!(filters.len(), 3);

        let (entity, low_pass) = filters[0];
        assert_eq!(entity, EntityId::new(7));
        assert_eq!(low_pass.kind, AudioFilterKind::LowPass);
        assert!(low_pass.enabled);
        assert_eq!(low_pass.cutoff_hz, 1200.0);
        assert_eq!(low_pass.resonance_q, 2.0);

        let (_, high_pass) = filters[1];
        assert_eq!(high_pass.kind, AudioFilterKind::HighPass);
        assert_eq!(high_pass.cutoff_hz, 300.0);
        assert_eq!(high_pass.resonance_q, 3.0);

        let (_, echo) = filters[2];
        assert_eq!(echo.kind, AudioFilterKind::Echo);
        assert_eq!(echo.delay_seconds, 0.25);
        assert_eq!(echo.dry_db, -6.0);
        assert_eq!(echo.feedback_db, -12.0);
        assert_eq!(echo.wet_db, -3.0);
        Ok(())
    }

    #[test]
    fn scene_audio_clamps_out_of_range_filter_values() -> Result<(), String> {
        let yaml = "%YAML 1.1\n--- !u!1 &10\nGameObject:\n  m_Name: Emitter\n--- !u!90 &13\nAudioLowPassFilter:\n  m_GameObject: {fileID: 10}\n  m_CutoffFrequency: 999999\n  m_LowpassResonanceQ: 99\n  m_Enabled: 0\n";
        let stream = unity_yaml::parse(yaml).map_err(|err| err.to_string())?;
        let mut records = GameObjectRecords::new();
        records.insert(
            10,
            GameObjectRecord {
                entity: EntityId::new(3),
                name: "Emitter".to_string(),
                layer: 0,
            },
        );
        let filters = collect_audio_filters(&stream, &records, &HashSet::new())
            .map_err(|err| err.to_string())?;
        assert_eq!(filters.len(), 1);
        let (_, filter) = filters[0];
        assert!(!filter.enabled);
        assert_eq!(filter.cutoff_hz, 5000.0);
        assert_eq!(filter.resonance_q, 1.0);
        Ok(())
    }
}
