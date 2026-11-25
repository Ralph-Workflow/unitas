use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use unitas_animation::avatar_mask::AvatarMask;
use unitas_animation::playable::{
    AnimationClipPlayable, AnimationLayerMixerPlayable, AnimationMixerPlayable, AudioClipPlayable,
    AudioMixerPlayable, AudioPlayableOutput, AudioSourceBinding, DirectorUpdateMode, PlayState,
    PlayableBehaviour, PlayableError, PlayableFrameData, PlayableGraph, ScriptPlayable,
};
use unitas_animation::{
    AnimatedValue, AnimationClip, CurveBinding, CurveBindingTarget, FloatCurve, Keyframe,
    SampledBinding, WrapMode,
};

#[derive(Clone, Default)]
struct CallLog {
    calls: Rc<RefCell<Vec<String>>>,
}

impl CallLog {
    fn record(&self, call: &str) {
        self.calls.borrow_mut().push(call.to_string());
    }

    fn entries(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

struct RecordingBehaviour {
    log: CallLog,
    label: String,
}

impl RecordingBehaviour {
    fn new(log: CallLog, label: &str) -> Self {
        Self {
            log,
            label: label.to_string(),
        }
    }

    fn record(&self, callback: &str) {
        self.log.record(&format!("{}:{}", self.label, callback));
    }
}

impl PlayableBehaviour for RecordingBehaviour {
    fn on_graph_start(&mut self) {
        self.record("OnGraphStart");
    }

    fn on_behaviour_play(&mut self) {
        self.record("OnBehaviourPlay");
    }

    fn on_behaviour_pause(&mut self) {
        self.record("OnBehaviourPause");
    }

    fn prepare_frame(&mut self, _frame: PlayableFrameData) {
        self.record("PrepareFrame");
    }

    fn process_frame(&mut self, _frame: PlayableFrameData) {
        self.record("ProcessFrame");
    }

    fn on_graph_stop(&mut self) {
        self.record("OnGraphStop");
    }

    fn on_playable_destroy(&mut self) {
        self.record("OnPlayableDestroy");
    }
}

fn constant_position_clip(name: &str, value: f32) -> AnimationClip {
    AnimationClip::new(
        name,
        1.0,
        60.0,
        WrapMode::Loop,
        vec![CurveBinding::float(
            CurveBindingTarget::new("", "Transform", "localPosition.x"),
            FloatCurve::new(vec![Keyframe::linear(0.0, value)]),
        )],
        Vec::new(),
    )
}

fn layered_clip(name: &str, arm: f32, leg: f32) -> AnimationClip {
    AnimationClip::new(
        name,
        1.0,
        60.0,
        WrapMode::Loop,
        vec![
            CurveBinding::float(
                CurveBindingTarget::new("", "Transform", "localPosition.x"),
                FloatCurve::new(vec![Keyframe::linear(0.0, 0.0)]),
            ),
            CurveBinding::float(
                CurveBindingTarget::new("Arm", "Transform", "localPosition.x"),
                FloatCurve::new(vec![Keyframe::linear(0.0, arm)]),
            ),
            CurveBinding::float(
                CurveBindingTarget::new("Leg", "Transform", "localPosition.x"),
                FloatCurve::new(vec![Keyframe::linear(0.0, leg)]),
            ),
        ],
        Vec::new(),
    )
}

fn binding_value(pose: &[SampledBinding], object_path: &str) -> Option<AnimatedValue> {
    pose.iter()
        .find(|binding| binding.target.object_path == object_path)
        .map(|binding| binding.value)
}

#[test]
fn ani_002_outcome_32_mixer_output_is_the_weighted_blend_of_its_clip_inputs(
) -> Result<(), Box<dyn Error>> {
    let mut graph = PlayableGraph::new("Mix");
    let low = AnimationClipPlayable::create(&mut graph, constant_position_clip("Low", 1.0));
    let high = AnimationClipPlayable::create(&mut graph, constant_position_clip("High", 5.0));
    let mixer = AnimationMixerPlayable::create(&mut graph, 2);
    graph.connect(low.handle(), 0, mixer.handle(), 0, 0.25)?;
    graph.connect(high.handle(), 0, mixer.handle(), 1, 0.75)?;
    let output = graph.create_output("Animation", mixer.handle())?;

    graph.play();
    graph.evaluate(0.1);

    let pose = graph.output_pose(output);
    assert_eq!(pose.len(), 1);
    let AnimatedValue::Float(value) = binding_value(pose, "").ok_or("missing root binding")? else {
        return Err(String::from("root binding is not a float").into());
    };
    assert!(
        (value - 4.0).abs() < 1e-5,
        "ANI-002 outcome 32: the weighted pose of a 0.25/0.75 mixer was {value}"
    );

    assert_eq!(graph.input_weight(mixer.handle(), 0), Some(0.25));
    assert_eq!(graph.input_weight(mixer.handle(), 1), Some(0.75));
    graph.set_input_weight(mixer.handle(), 0, 0.5)?;
    assert_eq!(graph.input_weight(mixer.handle(), 0), Some(0.5));
    Ok(())
}

#[test]
fn ani_002_outcome_32_script_playable_callbacks_fire_in_the_order_the_specification_states(
) -> Result<(), Box<dyn Error>> {
    let log = CallLog::default();
    let mut graph = PlayableGraph::new("Script");
    let script = ScriptPlayable::create(
        &mut graph,
        Box::new(RecordingBehaviour::new(log.clone(), "only")),
    );
    let output = graph.create_output("Animation", script.handle())?;

    graph.play();
    graph.evaluate(0.1);
    graph.evaluate(0.1);
    graph.stop();
    graph.destroy();

    assert_eq!(graph.name(), "Script");
    assert_eq!(graph.node_count(), 1);
    assert_eq!(graph.output_count(), 1);
    assert_eq!(graph.output_name(output), Some("Animation"));
    assert_eq!(graph.state(), PlayState::Paused);
    assert!(!graph.is_playing());
    assert!(graph.is_destroyed());

    assert_eq!(
        log.entries(),
        vec![
            "only:OnGraphStart",
            "only:OnBehaviourPlay",
            "only:PrepareFrame",
            "only:ProcessFrame",
            "only:PrepareFrame",
            "only:ProcessFrame",
            "only:OnBehaviourPause",
            "only:OnGraphStop",
            "only:OnPlayableDestroy",
        ]
    );
    Ok(())
}

#[test]
fn every_prepare_frame_runs_before_any_process_frame_of_the_same_frame(
) -> Result<(), Box<dyn Error>> {
    let log = CallLog::default();
    let mut graph = PlayableGraph::new("Scripts");
    let first = ScriptPlayable::create(
        &mut graph,
        Box::new(RecordingBehaviour::new(log.clone(), "first")),
    );
    let second = ScriptPlayable::create(
        &mut graph,
        Box::new(RecordingBehaviour::new(log.clone(), "second")),
    );
    let mixer = AnimationMixerPlayable::create(&mut graph, 2);
    graph.connect(first.handle(), 0, mixer.handle(), 0, 1.0)?;
    graph.connect(second.handle(), 0, mixer.handle(), 1, 1.0)?;
    graph.create_output("Animation", mixer.handle())?;

    graph.play();
    graph.evaluate(0.1);

    assert_eq!(
        log.entries(),
        vec![
            "first:OnGraphStart",
            "second:OnGraphStart",
            "first:OnBehaviourPlay",
            "second:OnBehaviourPlay",
            "first:PrepareFrame",
            "second:PrepareFrame",
            "first:ProcessFrame",
            "second:ProcessFrame",
        ]
    );
    Ok(())
}

#[test]
fn ani_002_outcome_31_avatar_mask_restricts_a_playable_layer_to_the_masked_bindings(
) -> Result<(), Box<dyn Error>> {
    let mut graph = PlayableGraph::new("Layers");
    let base = AnimationClipPlayable::create(&mut graph, layered_clip("Base", 1.0, 2.0));
    let overlay = AnimationClipPlayable::create(&mut graph, layered_clip("Overlay", 10.0, 20.0));
    let layers = AnimationLayerMixerPlayable::create(&mut graph, 2);
    graph.connect(base.handle(), 0, layers.handle(), 0, 1.0)?;
    graph.connect(overlay.handle(), 0, layers.handle(), 1, 1.0)?;
    layers.set_layer_mask(&mut graph, 1, AvatarMask::new("Arm").with_path("Arm"))?;
    let output = graph.create_output("Animation", layers.handle())?;

    graph.play();
    graph.evaluate(0.1);

    let pose = graph.output_pose(output);
    assert_eq!(pose.len(), 3);
    assert_eq!(
        binding_value(pose, "Arm"),
        Some(AnimatedValue::Float(10.0)),
        "the masked arm binding takes the overlay value"
    );
    assert_eq!(
        binding_value(pose, "Leg"),
        Some(AnimatedValue::Float(2.0)),
        "the unmasked leg binding keeps the base value"
    );
    assert_eq!(binding_value(pose, ""), Some(AnimatedValue::Float(0.0)));
    Ok(())
}

#[test]
fn clip_playable_reports_time_speed_duration_and_done_state() -> Result<(), Box<dyn Error>> {
    let mut graph = PlayableGraph::new("Clip");
    let clip = AnimationClipPlayable::create(
        &mut graph,
        AnimationClip::new(
            "Once",
            1.0,
            60.0,
            WrapMode::Once,
            vec![CurveBinding::float(
                CurveBindingTarget::new("", "Transform", "localPosition.x"),
                FloatCurve::new(vec![Keyframe::linear(0.0, 0.0), Keyframe::linear(1.0, 1.0)]),
            )],
            Vec::new(),
        ),
    );
    let output = graph.create_output("Animation", clip.handle())?;
    graph.play();

    graph.set_time_update_mode(DirectorUpdateMode::Manual);
    assert_eq!(graph.time_update_mode(), DirectorUpdateMode::Manual);
    assert_eq!(graph.playable_duration(clip.handle()), Some(1.0));
    graph.evaluate(0.5);
    let time = graph
        .playable_time(clip.handle())
        .ok_or("missing playable time")?;
    assert!((time - 0.5).abs() < 1e-6, "playable time was {time}");
    assert_eq!(graph.playable_is_done(clip.handle()), Some(false));
    assert!(!graph.is_done());

    graph.set_playable_speed(clip.handle(), 2.0)?;
    assert_eq!(graph.playable_speed(clip.handle()), Some(2.0));
    graph.evaluate(0.5);
    let time = graph
        .playable_time(clip.handle())
        .ok_or("missing playable time")?;
    assert!((time - 1.5).abs() < 1e-6, "playable time was {time}");
    assert_eq!(graph.playable_is_done(clip.handle()), Some(true));
    assert!(graph.is_done());

    let pose = graph.output_pose(output);
    assert_eq!(pose.len(), 1);

    graph.set_time(0.25);
    assert_eq!(graph.time(), 0.25);
    assert_eq!(graph.playable_time(clip.handle()), Some(0.25));
    graph.set_playable_duration(clip.handle(), 0.75)?;
    assert_eq!(graph.playable_duration(clip.handle()), Some(0.75));
    Ok(())
}

#[test]
fn connecting_a_playable_to_itself_is_rejected() -> Result<(), Box<dyn Error>> {
    let mut graph = PlayableGraph::new("Cycle");
    let mixer = AnimationMixerPlayable::create(&mut graph, 1);
    let result = graph.connect(mixer.handle(), 0, mixer.handle(), 0, 1.0);
    let Err(error) = result else {
        return Err(String::from("a self connection was accepted").into());
    };
    assert_eq!(error.to_string(), "the connection would create a cycle");
    Ok(())
}

#[test]
fn a_handle_from_another_graph_is_never_accepted() -> Result<(), Box<dyn Error>> {
    let mut graph = PlayableGraph::new("Owner");
    let mixer = AnimationMixerPlayable::create(&mut graph, 1);
    let mut other = PlayableGraph::new("Foreign");
    let foreign = AnimationClipPlayable::create(&mut other, constant_position_clip("Foreign", 1.0));

    assert_ne!(foreign.handle(), mixer.handle());
    assert_eq!(
        graph.connect(foreign.handle(), 0, mixer.handle(), 0, 1.0),
        Err(PlayableError::UnknownPlayable)
    );
    assert_eq!(
        graph.set_playable_time(foreign.handle(), 0.5),
        Err(PlayableError::UnknownPlayable)
    );
    assert_eq!(
        graph.set_input_weight(foreign.handle(), 0, 1.0),
        Err(PlayableError::UnknownPlayable)
    );
    assert_eq!(
        graph.create_output("Foreign", foreign.handle()),
        Err(PlayableError::UnknownPlayable)
    );
    assert_eq!(graph.playable_time(foreign.handle()), None);
    assert_eq!(graph.playable_speed(foreign.handle()), None);
    assert_eq!(graph.playable_duration(foreign.handle()), None);
    assert_eq!(graph.playable_is_done(foreign.handle()), None);
    assert_eq!(graph.input_weight(foreign.handle(), 0), None);
    assert_eq!(
        graph.set_playable_speed(foreign.handle(), 2.0),
        Err(PlayableError::UnknownPlayable)
    );
    assert_eq!(
        graph.set_playable_duration(foreign.handle(), 1.0),
        Err(PlayableError::UnknownPlayable)
    );

    let foreign_output = other.create_output("Foreign", foreign.handle())?;
    assert_eq!(graph.output_name(foreign_output), None);
    assert!(graph.output_pose(foreign_output).is_empty());

    let foreign_layers = AnimationLayerMixerPlayable::create(&mut other, 1);
    assert_eq!(
        foreign_layers.set_layer_mask(&mut graph, 0, AvatarMask::new("Arm")),
        Err(PlayableError::UnknownPlayable)
    );
    Ok(())
}

#[test]
fn a_destroyed_graph_refuses_playable_mutations() -> Result<(), Box<dyn Error>> {
    let mut graph = PlayableGraph::new("Destroyed");
    let clip = AnimationClipPlayable::create(&mut graph, constant_position_clip("Clip", 1.0));
    graph.destroy();

    assert_eq!(
        graph.set_playable_time(clip.handle(), 1.0),
        Err(PlayableError::GraphDestroyed)
    );
    assert_eq!(
        graph.set_playable_speed(clip.handle(), 2.0),
        Err(PlayableError::GraphDestroyed)
    );
    assert_eq!(
        graph.set_playable_duration(clip.handle(), 3.0),
        Err(PlayableError::GraphDestroyed)
    );
    Ok(())
}

#[test]
fn invalid_connections_and_masks_are_rejected() -> Result<(), Box<dyn Error>> {
    let mut graph = PlayableGraph::new("Invalid");
    let mixer = AnimationMixerPlayable::create(&mut graph, 1);
    let layers = AnimationLayerMixerPlayable::create(&mut graph, 1);
    let clip = AnimationClipPlayable::create(&mut graph, constant_position_clip("Clip", 1.0));

    assert_eq!(
        graph.connect(clip.handle(), 1, mixer.handle(), 0, 1.0),
        Err(PlayableError::InvalidPort)
    );
    assert_eq!(
        graph.connect(clip.handle(), 0, mixer.handle(), 4, 1.0),
        Err(PlayableError::InvalidPort)
    );
    assert_eq!(
        graph.set_input_weight(mixer.handle(), 4, 1.0),
        Err(PlayableError::InvalidPort)
    );
    assert_eq!(
        layers.set_layer_mask(&mut graph, 3, AvatarMask::new("Arm")),
        Err(PlayableError::InvalidLayer)
    );

    let mut other = PlayableGraph::new("Other");
    let foreign = AnimationClipPlayable::create(&mut other, constant_position_clip("Foreign", 1.0));
    assert_eq!(
        graph.connect(foreign.handle(), 0, mixer.handle(), 0, 1.0),
        Err(PlayableError::UnknownPlayable)
    );

    graph.destroy();
    assert_eq!(
        graph.connect(clip.handle(), 0, mixer.handle(), 0, 1.0),
        Err(PlayableError::GraphDestroyed)
    );
    Ok(())
}

#[test]
fn a_handle_from_another_graph_is_not_usable_in_this_graph() -> Result<(), Box<dyn Error>> {
    let mut graph = PlayableGraph::new("Owner");
    let mixer = AnimationMixerPlayable::create(&mut graph, 1);
    let mut other = PlayableGraph::new("Other");
    let foreign = AnimationClipPlayable::create(&mut other, constant_position_clip("Foreign", 1.0));

    assert_eq!(graph.set_input_weight(mixer.handle(), 0, 1.0), Ok(()));
    assert_eq!(
        graph.set_input_weight(foreign.handle(), 0, 1.0),
        Err(PlayableError::UnknownPlayable)
    );
    assert_eq!(
        graph.set_playable_time(foreign.handle(), 0.5),
        Err(PlayableError::UnknownPlayable)
    );
    assert_eq!(
        graph.set_playable_speed(foreign.handle(), 2.0),
        Err(PlayableError::UnknownPlayable)
    );
    assert_eq!(
        graph.set_playable_duration(foreign.handle(), 1.0),
        Err(PlayableError::UnknownPlayable)
    );
    assert_eq!(
        graph.create_output("Foreign", foreign.handle()),
        Err(PlayableError::UnknownPlayable)
    );
    assert_eq!(graph.playable_time(foreign.handle()), None);
    assert_eq!(graph.playable_speed(foreign.handle()), None);
    assert_eq!(graph.playable_duration(foreign.handle()), None);
    assert_eq!(graph.playable_is_done(foreign.handle()), None);
    assert_eq!(graph.input_weight(foreign.handle(), 0), None);
    Ok(())
}

#[test]
fn playable_time_update_mode_round_trips_and_controls_stepping() -> Result<(), Box<dyn Error>> {
    let mut graph = PlayableGraph::new("UpdateMode");
    assert_eq!(graph.time_update_mode(), DirectorUpdateMode::GameTime);

    for mode in [
        DirectorUpdateMode::GameTime,
        DirectorUpdateMode::UnscaledGameTime,
        DirectorUpdateMode::DspClock,
        DirectorUpdateMode::Manual,
    ] {
        graph.set_time_update_mode(mode);
        assert_eq!(graph.time_update_mode(), mode);
    }

    let clip = AnimationClipPlayable::create(
        &mut graph,
        AnimationClip::new(
            "Ramp",
            1.0,
            60.0,
            WrapMode::Once,
            vec![CurveBinding::float(
                CurveBindingTarget::new("", "Transform", "localPosition.x"),
                FloatCurve::new(vec![
                    Keyframe::linear(0.0, 0.0),
                    Keyframe::linear(1.0, 10.0),
                ]),
            )],
            Vec::new(),
        ),
    );
    let output = graph.create_output("Animation", clip.handle())?;

    graph.evaluate(0.25);
    assert_eq!(graph.time(), 0.0);
    assert_eq!(graph.playable_time(clip.handle()), Some(0.0));
    assert_eq!(
        binding_value(graph.output_pose(output), ""),
        Some(AnimatedValue::Float(0.0))
    );

    graph.set_time(0.4);
    graph.evaluate(0.0);
    assert_eq!(graph.time(), 0.4);
    assert_eq!(graph.playable_time(clip.handle()), Some(0.4));
    let AnimatedValue::Float(manual_value) =
        binding_value(graph.output_pose(output), "").ok_or("missing root binding")?
    else {
        return Err(String::from("root binding is not a float").into());
    };
    assert!(
        (manual_value - 4.0).abs() < 1e-5,
        "manual pose was {manual_value}"
    );

    graph.set_time_update_mode(DirectorUpdateMode::GameTime);
    graph.play();
    graph.evaluate(0.2);
    let time = graph
        .playable_time(clip.handle())
        .ok_or("missing playable time")?;
    assert!((time - 0.6).abs() < 1e-6, "playable time was {time}");
    let AnimatedValue::Float(stepped_value) =
        binding_value(graph.output_pose(output), "").ok_or("missing root binding")?
    else {
        return Err(String::from("root binding is not a float").into());
    };
    assert!(
        (stepped_value - 6.0).abs() < 1e-5,
        "stepped pose was {stepped_value}"
    );
    Ok(())
}

#[test]
fn playable_speed_scales_evaluated_time() -> Result<(), Box<dyn Error>> {
    let make_ramp_clip = |name: &str| {
        AnimationClip::new(
            name,
            2.0,
            60.0,
            WrapMode::Once,
            vec![CurveBinding::float(
                CurveBindingTarget::new("", "Transform", "localPosition.x"),
                FloatCurve::new(vec![
                    Keyframe::linear(0.0, 0.0),
                    Keyframe::linear(2.0, 20.0),
                ]),
            )],
            Vec::new(),
        )
    };

    let mut graph = PlayableGraph::new("Speed");
    let half = AnimationClipPlayable::create(&mut graph, make_ramp_clip("Half"));
    let normal = AnimationClipPlayable::create(&mut graph, make_ramp_clip("Normal"));
    let double = AnimationClipPlayable::create(&mut graph, make_ramp_clip("Double"));

    graph.set_playable_speed(half.handle(), 0.5)?;
    graph.set_playable_speed(normal.handle(), 1.0)?;
    graph.set_playable_speed(double.handle(), 2.0)?;

    assert_eq!(graph.playable_speed(half.handle()), Some(0.5));
    assert_eq!(graph.playable_speed(normal.handle()), Some(1.0));
    assert_eq!(graph.playable_speed(double.handle()), Some(2.0));

    let half_output = graph.create_output("HalfOutput", half.handle())?;
    let normal_output = graph.create_output("NormalOutput", normal.handle())?;
    let double_output = graph.create_output("DoubleOutput", double.handle())?;

    graph.play();
    graph.evaluate(0.4);

    let half_time = graph
        .playable_time(half.handle())
        .ok_or("missing half time")?;
    let normal_time = graph
        .playable_time(normal.handle())
        .ok_or("missing normal time")?;
    let double_time = graph
        .playable_time(double.handle())
        .ok_or("missing double time")?;

    assert!((half_time - 0.2).abs() < 1e-6, "half time was {half_time}");
    assert!(
        (normal_time - 0.4).abs() < 1e-6,
        "normal time was {normal_time}"
    );
    assert!(
        (double_time - 0.8).abs() < 1e-6,
        "double time was {double_time}"
    );

    let AnimatedValue::Float(half_pose) =
        binding_value(graph.output_pose(half_output), "").ok_or("missing half binding")?
    else {
        return Err(String::from("half binding is not a float").into());
    };
    let AnimatedValue::Float(normal_pose) =
        binding_value(graph.output_pose(normal_output), "").ok_or("missing normal binding")?
    else {
        return Err(String::from("normal binding is not a float").into());
    };
    let AnimatedValue::Float(double_pose) =
        binding_value(graph.output_pose(double_output), "").ok_or("missing double binding")?
    else {
        return Err(String::from("double binding is not a float").into());
    };

    assert!((half_pose - 2.0).abs() < 1e-5, "half pose was {half_pose}");
    assert!(
        (normal_pose - 4.0).abs() < 1e-5,
        "normal pose was {normal_pose}"
    );
    assert!(
        (double_pose - 8.0).abs() < 1e-5,
        "double pose was {double_pose}"
    );
    Ok(())
}

#[test]
fn playable_output_has_inspectable_name() -> Result<(), Box<dyn Error>> {
    let mut graph = PlayableGraph::new("Outputs");
    let primary_clip =
        AnimationClipPlayable::create(&mut graph, constant_position_clip("PrimaryClip", 3.5));
    let secondary_clip =
        AnimationClipPlayable::create(&mut graph, constant_position_clip("SecondaryClip", 7.25));

    let primary_output = graph.create_output("PrimaryPose", primary_clip.handle())?;
    let secondary_output = graph.create_output("SecondaryPose", secondary_clip.handle())?;

    assert_eq!(graph.output_count(), 2);
    assert_eq!(graph.output_name(primary_output), Some("PrimaryPose"));
    assert_eq!(graph.output_name(secondary_output), Some("SecondaryPose"));

    let mut foreign_graph = PlayableGraph::new("Foreign");
    let foreign_clip =
        AnimationClipPlayable::create(&mut foreign_graph, constant_position_clip("Foreign", 1.0));
    let foreign_output = foreign_graph.create_output("ForeignPose", foreign_clip.handle())?;
    assert_eq!(graph.output_name(foreign_output), None);
    assert!(graph.output_pose(foreign_output).is_empty());

    graph.play();
    graph.evaluate(0.1);

    assert_eq!(
        binding_value(graph.output_pose(primary_output), ""),
        Some(AnimatedValue::Float(3.5))
    );
    assert_eq!(
        binding_value(graph.output_pose(secondary_output), ""),
        Some(AnimatedValue::Float(7.25))
    );
    Ok(())
}

#[test]
fn audio_playable_output_plays_through_audio_source_and_mixer_group() -> Result<(), Box<dyn Error>>
{
    let mut graph = PlayableGraph::new("AudioGraph");
    let voice_clip =
        AudioClipPlayable::create(&mut graph, "Assets/Audio/VoiceLine.ogg", 2.0, false);
    let music_clip = AudioClipPlayable::create(&mut graph, "Assets/Audio/Theme.ogg", 4.0, true);
    let audio_mixer = AudioMixerPlayable::create(&mut graph, 2);

    graph.connect(voice_clip.handle(), 0, audio_mixer.handle(), 0, 0.25)?;
    graph.connect(music_clip.handle(), 0, audio_mixer.handle(), 1, 0.75)?;

    let source_binding = AudioSourceBinding::new("DialogAudioSource", "Master/Dialog");
    let audio_output = AudioPlayableOutput::create(
        &mut graph,
        "DialogAudio",
        audio_mixer.handle(),
        source_binding,
    )?;

    assert_eq!(
        graph.output_name(audio_output.handle()),
        Some("DialogAudio")
    );

    let before_play = audio_output
        .state(&graph)
        .ok_or("missing audio output state before play")?;
    assert!(!before_play.is_playing);
    assert_eq!(before_play.audio_source.source_name, "DialogAudioSource");
    assert_eq!(before_play.audio_source.mixer_group, "Master/Dialog");

    graph.play();
    graph.evaluate(0.5);

    let state = audio_output
        .state(&graph)
        .ok_or("missing audio output state during play")?;
    assert!(state.is_playing);
    assert_eq!(state.audio_source.source_name, "DialogAudioSource");
    assert_eq!(state.audio_source.mixer_group, "Master/Dialog");
    assert_eq!(
        state.audio_source.assigned_clip.as_deref(),
        Some("Assets/Audio/Theme.ogg")
    );
    assert_eq!(state.active_clips.len(), 2);
    assert_eq!(state.active_clips[0].clip_id, "Assets/Audio/VoiceLine.ogg");
    assert!((state.active_clips[0].weight - 0.25).abs() < 1e-5);
    assert!((state.active_clips[0].time - 0.5).abs() < 1e-6);
    assert!(!state.active_clips[0].looping);
    assert_eq!(state.active_clips[1].clip_id, "Assets/Audio/Theme.ogg");
    assert!((state.active_clips[1].weight - 0.75).abs() < 1e-5);
    assert!((state.active_clips[1].time - 0.5).abs() < 1e-6);
    assert!(state.active_clips[1].looping);
    assert!((state.effective_volume - 1.0).abs() < 1e-5);

    graph.set_input_weight(audio_mixer.handle(), 0, 0.8)?;
    graph.set_input_weight(audio_mixer.handle(), 1, 0.2)?;
    audio_output.set_target(
        &mut graph,
        AudioSourceBinding::new("DialogAudioSource", "Master/Voiceover"),
    )?;
    graph.evaluate(0.25);

    let switched = audio_output
        .state(&graph)
        .ok_or("missing audio output state after weight switch")?;
    assert_eq!(switched.audio_source.mixer_group, "Master/Voiceover");
    assert_eq!(
        switched.audio_source.assigned_clip.as_deref(),
        Some("Assets/Audio/VoiceLine.ogg")
    );
    assert!((switched.active_clips[0].time - 0.75).abs() < 1e-6);

    assert_eq!(graph.playable_is_done(voice_clip.handle()), Some(false));
    assert_eq!(graph.playable_is_done(music_clip.handle()), Some(false));
    graph.evaluate(1.5);
    assert_eq!(graph.playable_is_done(voice_clip.handle()), Some(true));
    assert_eq!(graph.playable_is_done(music_clip.handle()), Some(false));

    graph.stop();
    let stopped = audio_output
        .state(&graph)
        .ok_or("missing audio output state after stop")?;
    assert!(!stopped.is_playing);

    let anim_clip = AnimationClipPlayable::create(&mut graph, constant_position_clip("Anim", 1.0));
    let anim_output = graph.create_output("AnimOut", anim_clip.handle())?;
    assert_eq!(
        graph.set_audio_output_target(
            anim_output,
            AudioSourceBinding::new("DialogAudioSource", "Master/Voiceover")
        ),
        Err(PlayableError::UnknownOutput)
    );

    let mut foreign_graph = PlayableGraph::new("ForeignAudio");
    let foreign_clip =
        AudioClipPlayable::create(&mut foreign_graph, "Assets/Audio/Foreign.ogg", 1.0, false);
    assert_eq!(
        AudioPlayableOutput::create(
            &mut graph,
            "ForeignOut",
            foreign_clip.handle(),
            AudioSourceBinding::new("ForeignSource", "Master")
        ),
        Err(PlayableError::UnknownPlayable)
    );
    assert_eq!(
        audio_output.set_target(
            &mut foreign_graph,
            AudioSourceBinding::new("ForeignSource", "Master")
        ),
        Err(PlayableError::UnknownOutput)
    );
    assert_eq!(
        PlayableError::UnknownOutput.to_string(),
        "the playable output handle is not in the graph"
    );

    graph.destroy();
    assert_eq!(
        audio_output.set_target(
            &mut graph,
            AudioSourceBinding::new("DialogAudioSource", "Master/Muted")
        ),
        Err(PlayableError::GraphDestroyed)
    );
    assert_eq!(
        AudioPlayableOutput::create(
            &mut graph,
            "DestroyedAudio",
            audio_mixer.handle(),
            AudioSourceBinding::new("DialogAudioSource", "Master/Muted")
        ),
        Err(PlayableError::GraphDestroyed)
    );
    Ok(())
}
