use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use unitas_animation::avatar_mask::AvatarMask;
use unitas_animation::playable::{
    AnimationClipPlayable, AnimationLayerMixerPlayable, AnimationMixerPlayable,
    DirectorUpdateMode, PlayableBehaviour, PlayableError, PlayableFrameData, PlayableGraph,
    PlayState, ScriptPlayable,
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
fn mixer_output_is_the_weighted_blend_of_its_clip_inputs() -> Result<(), Box<dyn Error>> {
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
    assert!((value - 4.0).abs() < 1e-5, "weighted pose was {value}");

    assert_eq!(graph.input_weight(mixer.handle(), 0), Some(0.25));
    assert_eq!(graph.input_weight(mixer.handle(), 1), Some(0.75));
    graph.set_input_weight(mixer.handle(), 0, 0.5)?;
    assert_eq!(graph.input_weight(mixer.handle(), 0), Some(0.5));
    Ok(())
}

#[test]
fn script_playable_callbacks_fire_in_the_order_the_specification_states(
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
fn avatar_mask_restricts_a_layer_to_the_masked_bindings() -> Result<(), Box<dyn Error>> {
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

    // Both graphs mint their first node at index 0, so only graph identity can
    // tell the foreign handle apart from the local mixer.
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
