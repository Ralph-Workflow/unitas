use std::error::Error;

use unitas_core::{EntityId, Transform};
use unitas_runtime::animation::AnimationWorld;
use unitas_runtime::cinemachine::{CameraBlend, CinemachineWorld, VirtualCamera};
use unitas_runtime::timeline::{
    PlaybackState, TimelineClip, TimelineClipPayload, TimelineDiagnostic, TimelineError,
    TimelineSequence, TimelineTrack, TimelineTrackKind, TimelineWorld, TimelineWrapMode,
};

fn sequence_with(tracks: Vec<TimelineTrack>) -> TimelineSequence {
    TimelineSequence {
        name: "cutscene".to_string(),
        tracks,
    }
}

fn signal_clip(
    signal: &str,
    receiver: &str,
    time: f64,
    retroactive: bool,
    emit_once: bool,
) -> TimelineClip {
    TimelineClip {
        name: signal.to_string(),
        start: time,
        duration: 0.0,
        payload: TimelineClipPayload::Signal {
            signal: signal.to_string(),
            receiver: receiver.to_string(),
            retroactive,
            emit_once,
        },
    }
}

fn signal_track(clips: Vec<TimelineClip>) -> TimelineTrack {
    TimelineTrack {
        kind: TimelineTrackKind::Signal,
        clips,
    }
}

fn audio_track(clip: &str, start: f64, duration: f64, volume: f32) -> TimelineTrack {
    TimelineTrack {
        kind: TimelineTrackKind::Audio,
        clips: vec![TimelineClip {
            name: clip.to_string(),
            start,
            duration,
            payload: TimelineClipPayload::Audio {
                clip: clip.to_string(),
                volume,
            },
        }],
    }
}

fn activation_track(entity: EntityId, start: f64, duration: f64, active: bool) -> TimelineTrack {
    TimelineTrack {
        kind: TimelineTrackKind::Activation,
        clips: vec![TimelineClip {
            name: "activate".to_string(),
            start,
            duration,
            payload: TimelineClipPayload::Activation { entity, active },
        }],
    }
}

fn animation_track(entity: EntityId, start: f64, duration: f64) -> TimelineTrack {
    TimelineTrack {
        kind: TimelineTrackKind::Animation,
        clips: vec![TimelineClip {
            name: "clip".to_string(),
            start,
            duration,
            payload: TimelineClipPayload::Animation {
                entity,
                clip: "clip".to_string(),
            },
        }],
    }
}

fn shot_track(
    camera: &str,
    start: f64,
    duration: f64,
    blend: Option<CameraBlend>,
) -> TimelineTrack {
    TimelineTrack {
        kind: TimelineTrackKind::CinemachineShot,
        clips: vec![TimelineClip {
            name: camera.to_string(),
            start,
            duration,
            payload: TimelineClipPayload::CinemachineShot {
                camera: camera.to_string(),
                blend,
            },
        }],
    }
}

fn timeline_of(
    tracks: Vec<TimelineTrack>,
    wrap_mode: TimelineWrapMode,
) -> Result<(TimelineWorld, u64), Box<dyn Error>> {
    let mut world = TimelineWorld::new();
    let sequence = world.register_sequence(sequence_with(tracks));
    let player = world.create_player(sequence, wrap_mode)?;
    Ok((world, player))
}

fn time_of(world: &TimelineWorld, player: u64) -> Result<f64, Box<dyn Error>> {
    Ok(world.player(player).ok_or("player missing")?.time())
}

fn state_of(world: &TimelineWorld, player: u64) -> Result<PlaybackState, Box<dyn Error>> {
    Ok(world.player(player).ok_or("player missing")?.state())
}

fn signal_count(world: &TimelineWorld, player: u64) -> Result<usize, Box<dyn Error>> {
    Ok(world
        .player(player)
        .ok_or("player missing")?
        .signal_events()
        .len())
}

#[test]
fn timeline_sequence_duration_is_the_latest_clip_end() -> Result<(), Box<dyn Error>> {
    let sequence = sequence_with(vec![
        animation_track(EntityId::new(1), 0.0, 2.0),
        audio_track("music", 1.0, 3.5, 0.8),
    ]);
    assert_eq!(sequence.duration(), 4.5);
    assert_eq!(sequence.clip_count(), 2);
    Ok(())
}

#[test]
fn timeline_play_advances_the_playhead_and_pause_holds_it() -> Result<(), Box<dyn Error>> {
    let (mut world, player) = timeline_of(
        vec![audio_track("music", 0.0, 4.0, 1.0)],
        TimelineWrapMode::Once,
    )?;
    world.player_mut(player).ok_or("player missing")?.play();
    world.update(player, 0.5)?;
    assert_eq!(time_of(&world, player)?, 0.5);
    world.player_mut(player).ok_or("player missing")?.pause();
    world.update(player, 0.5)?;
    assert_eq!(time_of(&world, player)?, 0.5);
    assert_eq!(state_of(&world, player)?, PlaybackState::Paused);
    Ok(())
}

#[test]
fn timeline_seek_positions_the_playhead_within_the_sequence() -> Result<(), Box<dyn Error>> {
    let (mut world, player) = timeline_of(
        vec![audio_track("music", 0.0, 4.0, 1.0)],
        TimelineWrapMode::Once,
    )?;
    world.player_mut(player).ok_or("player missing")?.seek(2.5);
    assert_eq!(time_of(&world, player)?, 2.5);
    world.player_mut(player).ok_or("player missing")?.seek(9.0);
    assert_eq!(time_of(&world, player)?, 4.0);
    world.player_mut(player).ok_or("player missing")?.seek(-3.0);
    assert_eq!(time_of(&world, player)?, 0.0);
    Ok(())
}

#[test]
fn timeline_once_wrap_mode_completes_and_raises_stopped() -> Result<(), Box<dyn Error>> {
    let (mut world, player) = timeline_of(
        vec![audio_track("music", 0.0, 4.0, 1.0)],
        TimelineWrapMode::Once,
    )?;
    world.player_mut(player).ok_or("player missing")?.play();
    world.update(player, 3.0)?;
    world.update(player, 2.0)?;
    let timeline = world.player(player).ok_or("player missing")?;
    assert_eq!(timeline.state(), PlaybackState::Complete);
    assert!(timeline.is_complete());
    assert!(timeline.at_end());
    assert_eq!(timeline.time(), 4.0);
    assert_eq!(timeline.stopped_count(), 1);
    world.update(player, 1.0)?;
    assert_eq!(
        world
            .player(player)
            .ok_or("player missing")?
            .stopped_count(),
        1
    );
    Ok(())
}

#[test]
fn timeline_hold_wrap_mode_keeps_the_last_frame_without_stopping() -> Result<(), Box<dyn Error>> {
    let (mut world, player) = timeline_of(
        vec![audio_track("music", 0.0, 4.0, 1.0)],
        TimelineWrapMode::Hold,
    )?;
    world.player_mut(player).ok_or("player missing")?.play();
    world.update(player, 3.0)?;
    world.update(player, 2.0)?;
    assert_eq!(time_of(&world, player)?, 4.0);
    assert!(world.player(player).ok_or("player missing")?.at_end());
    assert_eq!(
        world
            .player(player)
            .ok_or("player missing")?
            .stopped_count(),
        0
    );
    world.update(player, 1.0)?;
    assert_eq!(time_of(&world, player)?, 4.0);
    Ok(())
}

#[test]
fn timeline_loop_wrap_mode_restarts_from_zero() -> Result<(), Box<dyn Error>> {
    let (mut world, player) = timeline_of(
        vec![audio_track("music", 0.0, 4.0, 1.0)],
        TimelineWrapMode::Loop,
    )?;
    world.player_mut(player).ok_or("player missing")?.play();
    world.update(player, 3.0)?;
    world.update(player, 2.0)?;
    let timeline = world.player(player).ok_or("player missing")?;
    assert_eq!(timeline.time(), 1.0);
    assert_eq!(timeline.wraps(), 1);
    assert!(!timeline.is_complete());
    Ok(())
}

#[test]
fn timeline_pingpong_wrap_mode_reverses_at_the_ends() -> Result<(), Box<dyn Error>> {
    let (mut world, player) = timeline_of(
        vec![audio_track("music", 0.0, 4.0, 1.0)],
        TimelineWrapMode::PingPong,
    )?;
    world.player_mut(player).ok_or("player missing")?.play();
    world.update(player, 3.0)?;
    assert_eq!(time_of(&world, player)?, 3.0);
    world.update(player, 1.0)?;
    let timeline = world.player(player).ok_or("player missing")?;
    assert_eq!(timeline.time(), 4.0);
    assert_eq!(timeline.direction(), -1.0);
    assert_eq!(timeline.wraps(), 1);
    world.update(player, 1.0)?;
    assert_eq!(time_of(&world, player)?, 3.0);
    world.update(player, 3.0)?;
    let timeline = world.player(player).ok_or("player missing")?;
    assert_eq!(timeline.time(), 0.0);
    assert_eq!(timeline.direction(), 1.0);
    assert_eq!(timeline.wraps(), 2);
    Ok(())
}

#[test]
fn timeline_signal_fires_once_per_crossing() -> Result<(), Box<dyn Error>> {
    let (mut world, player) = timeline_of(
        vec![
            audio_track("music", 0.0, 4.0, 1.0),
            signal_track(vec![signal_clip("open", "door", 1.0, false, false)]),
        ],
        TimelineWrapMode::Once,
    )?;
    world.player_mut(player).ok_or("player missing")?.play();
    world.update(player, 0.5)?;
    assert_eq!(signal_count(&world, player)?, 0);
    world.update(player, 1.0)?;
    assert_eq!(signal_count(&world, player)?, 1);
    let events = world
        .player(player)
        .ok_or("player missing")?
        .signal_events();
    assert_eq!(events[0].signal, "open");
    assert_eq!(events[0].receiver, "door");
    assert_eq!(events[0].time, 1.0);
    world.update(player, 1.0)?;
    assert_eq!(signal_count(&world, player)?, 1);
    Ok(())
}

#[test]
fn timeline_signal_fires_again_when_pingpong_crosses_it_backwards() -> Result<(), Box<dyn Error>> {
    let (mut world, player) = timeline_of(
        vec![
            audio_track("music", 0.0, 4.0, 1.0),
            signal_track(vec![signal_clip("open", "door", 1.0, false, false)]),
        ],
        TimelineWrapMode::PingPong,
    )?;
    world.player_mut(player).ok_or("player missing")?.play();
    world.update(player, 1.5)?;
    assert_eq!(signal_count(&world, player)?, 1);
    world.update(player, 2.5)?;
    world.update(player, 1.0)?;
    assert_eq!(signal_count(&world, player)?, 1);
    world.update(player, 2.0)?;
    assert_eq!(signal_count(&world, player)?, 2);
    Ok(())
}

#[test]
fn timeline_retroactive_signal_fires_when_playback_starts_after_it() -> Result<(), Box<dyn Error>> {
    let (mut world, player) = timeline_of(
        vec![
            audio_track("music", 0.0, 4.0, 1.0),
            signal_track(vec![signal_clip("open", "door", 1.0, true, false)]),
        ],
        TimelineWrapMode::Once,
    )?;
    world.player_mut(player).ok_or("player missing")?.seek(2.0);
    world.player_mut(player).ok_or("player missing")?.play();
    assert_eq!(signal_count(&world, player)?, 1);
    world.update(player, 1.0)?;
    assert_eq!(signal_count(&world, player)?, 1);
    Ok(())
}

#[test]
fn timeline_emit_once_signal_fires_only_on_its_first_crossing() -> Result<(), Box<dyn Error>> {
    let (mut world, player) = timeline_of(
        vec![
            audio_track("music", 0.0, 4.0, 1.0),
            signal_track(vec![signal_clip("open", "door", 1.0, false, true)]),
        ],
        TimelineWrapMode::Loop,
    )?;
    world.player_mut(player).ok_or("player missing")?.play();
    world.update(player, 1.5)?;
    assert_eq!(signal_count(&world, player)?, 1);
    world.update(player, 3.0)?;
    world.update(player, 1.0)?;
    assert_eq!(signal_count(&world, player)?, 1);
    Ok(())
}

#[test]
fn timeline_activation_track_activates_and_deactivates_its_target() -> Result<(), Box<dyn Error>> {
    let entity = EntityId::new(7);
    let (mut world, player) = timeline_of(
        vec![activation_track(entity, 1.0, 1.0, false)],
        TimelineWrapMode::Once,
    )?;
    world.register_entity(entity);
    world.player_mut(player).ok_or("player missing")?.play();
    world.update(player, 0.5)?;
    assert!(world
        .player(player)
        .ok_or("player missing")?
        .active_activation_clips()
        .is_empty());
    world.update(player, 1.0)?;
    assert_eq!(
        world
            .player(player)
            .ok_or("player missing")?
            .active_activation_clips(),
        vec![(entity, false)]
    );
    world.update(player, 1.0)?;
    assert!(world
        .player(player)
        .ok_or("player missing")?
        .active_activation_clips()
        .is_empty());
    Ok(())
}

#[test]
fn timeline_audio_track_reports_the_playing_clip_and_volume() -> Result<(), Box<dyn Error>> {
    let (mut world, player) = timeline_of(
        vec![audio_track("ambience", 0.0, 2.0, 0.5)],
        TimelineWrapMode::Once,
    )?;
    world.player_mut(player).ok_or("player missing")?.play();
    world.update(player, 0.5)?;
    let audio = world
        .player(player)
        .ok_or("player missing")?
        .active_audio_clips();
    assert_eq!(audio.len(), 1);
    assert_eq!(audio[0].clip, "ambience");
    assert_eq!(audio[0].volume, 0.5);
    world.update(player, 2.0)?;
    assert!(world
        .player(player)
        .ok_or("player missing")?
        .active_audio_clips()
        .is_empty());
    Ok(())
}

#[test]
fn timeline_animation_track_gates_the_bound_animator() -> Result<(), Box<dyn Error>> {
    let entity = EntityId::new(11);
    let (mut world, player) = timeline_of(
        vec![animation_track(entity, 1.0, 1.0)],
        TimelineWrapMode::Once,
    )?;
    let mut animations = AnimationWorld::new();
    animations.ensure_animator(entity, Transform::default());
    world.drive_animation(player, &mut animations)?;
    assert_eq!(animations.speed(entity), Some(0.0));
    world.player_mut(player).ok_or("player missing")?.play();
    world.update(player, 1.5)?;
    world.drive_animation(player, &mut animations)?;
    assert_eq!(animations.speed(entity), Some(1.0));
    let states = world
        .player(player)
        .ok_or("player missing")?
        .active_animation_clips();
    assert_eq!(states.len(), 1);
    assert_eq!(states[0].entity, entity);
    assert_eq!(states[0].local_time, 0.5);
    Ok(())
}

#[test]
fn timeline_cinemachine_shot_track_drives_the_live_camera() -> Result<(), Box<dyn Error>> {
    let (mut world, player) = timeline_of(
        vec![shot_track("Shot", 0.0, 2.0, Some(CameraBlend::cut()))],
        TimelineWrapMode::Once,
    )?;
    let mut cameras = CinemachineWorld::new();
    let main = cameras.add_camera(VirtualCamera::new("Main", 10));
    cameras.add_camera(VirtualCamera::new("Shot", 0));
    cameras.set_active(main, true);
    cameras.update(0.0);
    assert_eq!(
        cameras.live_camera().map(|camera| camera.name.as_str()),
        Some("Main")
    );
    world.player_mut(player).ok_or("player missing")?.play();
    world.drive_cinemachine(player, &mut cameras)?;
    assert_eq!(
        cameras.live_camera().map(|camera| camera.name.as_str()),
        Some("Shot")
    );
    world.update(player, 3.0)?;
    world.drive_cinemachine(player, &mut cameras)?;
    assert_eq!(
        cameras.live_camera().map(|camera| camera.name.as_str()),
        Some("Main")
    );
    Ok(())
}

#[test]
fn timeline_missing_bindings_identify_the_track_and_the_target() -> Result<(), Box<dyn Error>> {
    let (mut world, player) = timeline_of(
        vec![
            activation_track(EntityId::new(5), 0.0, 1.0, true),
            animation_track(EntityId::new(6), 0.0, 1.0),
            shot_track("Ghost", 0.0, 1.0, None),
        ],
        TimelineWrapMode::Once,
    )?;
    let mut animations = AnimationWorld::new();
    let mut cameras = CinemachineWorld::new();
    world.update(player, 0.1)?;
    world.drive_animation(player, &mut animations)?;
    world.drive_cinemachine(player, &mut cameras)?;
    let diagnostics = world.diagnostics();
    assert_eq!(diagnostics.len(), 3);
    let mut found: Vec<(TimelineTrackKind, usize)> = Vec::new();
    for diagnostic in diagnostics {
        match diagnostic {
            TimelineDiagnostic::MissingBinding {
                track,
                track_index,
                target,
            } => {
                assert!(!target.is_empty());
                found.push((*track, *track_index));
            }
        }
    }
    assert!(found.contains(&(TimelineTrackKind::Activation, 0)));
    assert!(found.contains(&(TimelineTrackKind::Animation, 1)));
    assert!(found.contains(&(TimelineTrackKind::CinemachineShot, 2)));
    Ok(())
}

#[test]
fn timeline_unknown_sequence_and_player_are_rejected() -> Result<(), Box<dyn Error>> {
    let mut world = TimelineWorld::new();
    match world.create_player(7, TimelineWrapMode::Once) {
        Err(TimelineError::UnknownSequence(7)) => {}
        Err(other) => return Err(format!("unexpected error {other:?}").into()),
        Ok(_) => return Err("an unknown sequence must not create a player".into()),
    }
    match world.update(9, 0.1) {
        Err(TimelineError::UnknownPlayer(9)) => {}
        Err(other) => return Err(format!("unexpected error {other:?}").into()),
        Ok(()) => return Err("an unknown player must not advance".into()),
    }
    Ok(())
}

#[test]
fn cin_001_outcome_33_combined_five_track_timeline_plays_pauses_seeks_completes_under_hold_loop_none_and_inspects_camera_blend(
) -> Result<(), Box<dyn Error>> {
    use unitas_runtime::cinemachine::BlendStyle;

    let anim_entity = EntityId::new(101);
    let act_entity = EntityId::new(102);

    let tracks = vec![
        animation_track(anim_entity, 0.0, 2.0),
        audio_track("theme", 0.0, 2.0, 0.75),
        activation_track(act_entity, 0.0, 1.5, true),
        signal_track(vec![signal_clip(" cue_mid", "director", 1.0, false, false)]),
        TimelineTrack {
            kind: TimelineTrackKind::CinemachineShot,
            clips: vec![
                TimelineClip {
                    name: "CamA".to_string(),
                    start: 0.0,
                    duration: 1.0,
                    payload: TimelineClipPayload::CinemachineShot {
                        camera: "CamA".to_string(),
                        blend: Some(CameraBlend::cut()),
                    },
                },
                TimelineClip {
                    name: "CamB".to_string(),
                    start: 1.0,
                    duration: 1.0,
                    payload: TimelineClipPayload::CinemachineShot {
                        camera: "CamB".to_string(),
                        blend: Some(CameraBlend {
                            style: BlendStyle::Linear,
                            duration: 0.5,
                        }),
                    },
                },
            ],
        },
    ];

    for mode in [
        TimelineWrapMode::Hold,
        TimelineWrapMode::Loop,
        TimelineWrapMode::None,
    ] {
        let (mut world, player) = timeline_of(tracks.clone(), mode)?;
        world.register_entity(act_entity);
        let mut animations = AnimationWorld::new();
        animations.ensure_animator(anim_entity, Transform::default());
        let mut cameras = CinemachineWorld::new();
        let cam_a = cameras.add_camera(VirtualCamera::new("CamA", 5));
        let cam_b = cameras.add_camera(VirtualCamera::new("CamB", 5));
        cameras.set_active(cam_a, true);
        cameras.set_active(cam_b, true);

        world.player_mut(player).ok_or("player missing")?.play();
        world.update(player, 0.5)?;
        world.drive_animation(player, &mut animations)?;
        world.drive_cinemachine(player, &mut cameras)?;
        cameras.update(0.5);

        assert_eq!(animations.speed(anim_entity), Some(1.0));
        assert_eq!(
            world
                .player(player)
                .ok_or("player missing")?
                .active_audio_clips()[0]
                .clip,
            "theme"
        );
        assert_eq!(
            world
                .player(player)
                .ok_or("player missing")?
                .active_activation_clips(),
            vec![(act_entity, true)]
        );
        assert_eq!(
            cameras.live_camera().map(|c| c.name.as_str()),
            Some("CamA"),
            "CIN-001 outcome 33: live virtual camera must be CamA in first shot"
        );
        assert_eq!(signal_count(&world, player)?, 0);

        world.player_mut(player).ok_or("player missing")?.pause();
        world.update(player, 0.5)?;
        assert_eq!(
            time_of(&world, player)?,
            0.5,
            "CIN-001 outcome 33: paused timeline must hold time"
        );

        world.player_mut(player).ok_or("player missing")?.play();
        world.update(player, 0.75)?;
        world.drive_cinemachine(player, &mut cameras)?;
        cameras.update(0.25);

        assert_eq!(
            signal_count(&world, player)?,
            1,
            "CIN-001 outcome 33: signal must fire once on crossing t=1.0"
        );
        assert_eq!(
            cameras.live_camera().map(|c| c.name.as_str()),
            Some("CamB"),
            "CIN-001 outcome 33: live virtual camera must switch to CamB in second shot"
        );
        let active_blend = cameras
            .active_blend()
            .ok_or("CIN-001 outcome 33: camera transition must have inspectable active blend")?;
        assert!(
            (active_blend.progress() - 0.5).abs() < 1e-6,
            "CIN-001 outcome 33: active blend weight/progress must be 0.5 at 0.25s of 0.5s blend"
        );

        world.player_mut(player).ok_or("player missing")?.seek(0.25);
        assert_eq!(time_of(&world, player)?, 0.25);

        world.player_mut(player).ok_or("player missing")?.seek(1.5);
        world.update(player, 1.0)?;

        let tl = world.player(player).ok_or("player missing")?;
        match mode {
            TimelineWrapMode::Hold => {
                assert_eq!(tl.time(), 2.0);
                assert_eq!(tl.state(), PlaybackState::Playing);
                assert_eq!(tl.stopped_count(), 0);
            }
            TimelineWrapMode::Loop => {
                assert!((tl.time() - 0.5).abs() < 1e-6);
                assert_eq!(tl.wraps(), 1);
                assert_eq!(tl.state(), PlaybackState::Playing);
            }
            TimelineWrapMode::None | TimelineWrapMode::Once => {
                assert_eq!(tl.time(), 2.0);
                assert_eq!(tl.state(), PlaybackState::Complete);
                assert_eq!(tl.stopped_count(), 1);
            }
            TimelineWrapMode::PingPong => {}
        }
    }
    Ok(())
}
