use std::sync::atomic::{AtomicU64, Ordering};

use crate::avatar_mask::AvatarMask;
use crate::clip::{AnimationClip, SampledBinding, WrapMode};
use crate::curve::AnimatedValue;

static NEXT_GRAPH_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectorUpdateMode {
    GameTime,
    UnscaledGameTime,
    DspClock,
    Manual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayState {
    Playing,
    Paused,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayableFrameData {
    pub delta_time: f32,
    pub weight: f32,
    pub evaluation_time: f64,
}

pub trait PlayableBehaviour {
    fn on_graph_start(&mut self) {}
    fn on_behaviour_play(&mut self) {}
    fn on_behaviour_pause(&mut self) {}
    fn prepare_frame(&mut self, _frame: PlayableFrameData) {}
    fn process_frame(&mut self, _frame: PlayableFrameData) {}
    fn on_graph_stop(&mut self) {}
    fn on_playable_destroy(&mut self) {}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlayableError {
    GraphDestroyed,
    UnknownPlayable,
    InvalidPort,
    InvalidLayer,
    CyclicConnection,
}

impl std::fmt::Display for PlayableError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GraphDestroyed => write!(formatter, "the playable graph is destroyed"),
            Self::UnknownPlayable => {
                write!(formatter, "the playable handle is not in the graph")
            }
            Self::InvalidPort => write!(formatter, "the playable port index is out of range"),
            Self::InvalidLayer => {
                write!(formatter, "the layer mixer layer index is out of range")
            }
            Self::CyclicConnection => write!(formatter, "the connection would create a cycle"),
        }
    }
}

impl std::error::Error for PlayableError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayableHandle {
    index: usize,
    graph_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayableOutputHandle {
    index: usize,
    graph_id: u64,
}

#[derive(Clone, Copy, Debug)]
struct PlayableState {
    time: f64,
    speed: f64,
    duration: f64,
    input_weight: f32,
}

impl Default for PlayableState {
    fn default() -> Self {
        Self {
            time: 0.0,
            speed: 1.0,
            duration: 0.0,
            input_weight: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
struct MixerInput {
    source: Option<PlayableHandle>,
    weight: f32,
}

#[derive(Clone, Debug)]
struct LayerMixerLayer {
    source: Option<PlayableHandle>,
    weight: f32,
    mask: Option<AvatarMask>,
}

enum PlayableNode {
    Clip {
        clip: AnimationClip,
        previous_time: f64,
        state: PlayableState,
    },
    Mixer {
        inputs: Vec<MixerInput>,
        state: PlayableState,
    },
    LayerMixer {
        layers: Vec<LayerMixerLayer>,
        state: PlayableState,
    },
    Script {
        behaviour: Box<dyn PlayableBehaviour>,
        state: PlayableState,
    },
}

impl PlayableNode {
    fn state(&self) -> &PlayableState {
        match self {
            Self::Clip { state, .. }
            | Self::Mixer { state, .. }
            | Self::LayerMixer { state, .. }
            | Self::Script { state, .. } => state,
        }
    }

    fn state_mut(&mut self) -> &mut PlayableState {
        match self {
            Self::Clip { state, .. }
            | Self::Mixer { state, .. }
            | Self::LayerMixer { state, .. }
            | Self::Script { state, .. } => state,
        }
    }

    fn inputs(&self) -> Vec<PlayableHandle> {
        match self {
            Self::Clip { .. } | Self::Script { .. } => Vec::new(),
            Self::Mixer { inputs, .. } => inputs
                .iter()
                .filter_map(|input| input.source)
                .collect::<Vec<PlayableHandle>>(),
            Self::LayerMixer { layers, .. } => layers
                .iter()
                .filter_map(|layer| layer.source)
                .collect::<Vec<PlayableHandle>>(),
        }
    }

    fn set_time(&mut self, time: f64) {
        if let Self::Clip {
            previous_time,
            state,
            ..
        } = self
        {
            *previous_time = time;
            state.time = time;
            return;
        }
        self.state_mut().time = time;
    }
}

struct PlayableOutput {
    name: String,
    source: Option<PlayableHandle>,
    pose: Vec<SampledBinding>,
}

pub struct PlayableGraph {
    name: String,
    id: u64,
    nodes: Vec<PlayableNode>,
    outputs: Vec<PlayableOutput>,
    state: PlayState,
    destroyed: bool,
    time: f64,
    update_mode: DirectorUpdateMode,
}

impl PlayableGraph {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            id: NEXT_GRAPH_ID.fetch_add(1, Ordering::Relaxed),
            nodes: Vec::new(),
            outputs: Vec::new(),
            state: PlayState::Paused,
            destroyed: false,
            time: 0.0,
            update_mode: DirectorUpdateMode::GameTime,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn state(&self) -> PlayState {
        self.state
    }

    pub fn is_playing(&self) -> bool {
        self.state == PlayState::Playing
    }

    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn output_count(&self) -> usize {
        self.outputs.len()
    }

    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn set_time(&mut self, time: f64) {
        let time = if time.is_finite() { time.max(0.0) } else { 0.0 };
        self.time = time;
        for node in &mut self.nodes {
            node.set_time(time);
        }
    }

    pub fn time_update_mode(&self) -> DirectorUpdateMode {
        self.update_mode
    }

    pub fn set_time_update_mode(&mut self, update_mode: DirectorUpdateMode) {
        self.update_mode = update_mode;
    }

    pub fn connect(
        &mut self,
        source: PlayableHandle,
        source_port: u32,
        destination: PlayableHandle,
        destination_port: u32,
        weight: f32,
    ) -> Result<(), PlayableError> {
        if self.destroyed {
            return Err(PlayableError::GraphDestroyed);
        }
        if self.node(source).is_none() || self.node(destination).is_none() {
            return Err(PlayableError::UnknownPlayable);
        }
        if source_port != 0 {
            return Err(PlayableError::InvalidPort);
        }
        if self.depends_on(destination, source) {
            return Err(PlayableError::CyclicConnection);
        }
        let weight = sanitize_weight(weight);
        let Some(node) = self.node_mut(destination) else {
            return Err(PlayableError::UnknownPlayable);
        };
        match node {
            PlayableNode::Mixer { inputs, .. } => {
                let Some(input) = inputs.get_mut(destination_port as usize) else {
                    return Err(PlayableError::InvalidPort);
                };
                input.source = Some(source);
                input.weight = weight;
            }
            PlayableNode::LayerMixer { layers, .. } => {
                let Some(layer) = layers.get_mut(destination_port as usize) else {
                    return Err(PlayableError::InvalidLayer);
                };
                layer.source = Some(source);
                layer.weight = weight;
            }
            _ => return Err(PlayableError::InvalidPort),
        }
        node.state_mut().input_weight = weight;
        Ok(())
    }

    pub fn set_input_weight(
        &mut self,
        destination: PlayableHandle,
        destination_port: u32,
        weight: f32,
    ) -> Result<(), PlayableError> {
        if self.destroyed {
            return Err(PlayableError::GraphDestroyed);
        }
        let weight = sanitize_weight(weight);
        let Some(node) = self.node_mut(destination) else {
            return Err(PlayableError::UnknownPlayable);
        };
        match node {
            PlayableNode::Mixer { inputs, .. } => {
                let Some(input) = inputs.get_mut(destination_port as usize) else {
                    return Err(PlayableError::InvalidPort);
                };
                input.weight = weight;
            }
            PlayableNode::LayerMixer { layers, .. } => {
                let Some(layer) = layers.get_mut(destination_port as usize) else {
                    return Err(PlayableError::InvalidLayer);
                };
                layer.weight = weight;
            }
            _ => return Err(PlayableError::InvalidPort),
        }
        node.state_mut().input_weight = weight;
        Ok(())
    }

    pub fn input_weight(&self, destination: PlayableHandle, destination_port: u32) -> Option<f32> {
        match self.node(destination)? {
            PlayableNode::Mixer { inputs, .. } => inputs
                .get(destination_port as usize)
                .map(|input| input.weight),
            PlayableNode::LayerMixer { layers, .. } => layers
                .get(destination_port as usize)
                .map(|layer| layer.weight),
            _ => None,
        }
    }

    pub fn create_output(
        &mut self,
        name: impl Into<String>,
        source: PlayableHandle,
    ) -> Result<PlayableOutputHandle, PlayableError> {
        if self.destroyed {
            return Err(PlayableError::GraphDestroyed);
        }
        if self.node(source).is_none() {
            return Err(PlayableError::UnknownPlayable);
        }
        self.outputs.push(PlayableOutput {
            name: name.into(),
            source: Some(source),
            pose: Vec::new(),
        });
        Ok(PlayableOutputHandle {
            index: self.outputs.len() - 1,
            graph_id: self.id,
        })
    }

    pub fn output_name(&self, output: PlayableOutputHandle) -> Option<&str> {
        self.output(output).map(|entry| entry.name.as_str())
    }

    pub fn output_pose(&self, output: PlayableOutputHandle) -> &[SampledBinding] {
        self.output(output)
            .map(|entry| entry.pose.as_slice())
            .unwrap_or(&[])
    }

    pub fn play(&mut self) {
        if self.destroyed || self.state == PlayState::Playing {
            return;
        }
        self.state = PlayState::Playing;
        let reachable = self.reachable_nodes();
        for (index, node) in self.nodes.iter_mut().enumerate() {
            if !reachable[index] {
                continue;
            }
            if let PlayableNode::Script { behaviour, .. } = node {
                behaviour.on_graph_start();
            }
        }
        for (index, node) in self.nodes.iter_mut().enumerate() {
            if !reachable[index] {
                continue;
            }
            if let PlayableNode::Script { behaviour, .. } = node {
                behaviour.on_behaviour_play();
            }
        }
    }

    pub fn stop(&mut self) {
        if self.destroyed || self.state != PlayState::Playing {
            return;
        }
        self.state = PlayState::Paused;
        let reachable = self.reachable_nodes();
        for (index, node) in self.nodes.iter_mut().enumerate() {
            if !reachable[index] {
                continue;
            }
            if let PlayableNode::Script { behaviour, .. } = node {
                behaviour.on_behaviour_pause();
            }
        }
        for (index, node) in self.nodes.iter_mut().enumerate() {
            if !reachable[index] {
                continue;
            }
            if let PlayableNode::Script { behaviour, .. } = node {
                behaviour.on_graph_stop();
            }
        }
    }

    pub fn destroy(&mut self) {
        if self.destroyed {
            return;
        }
        self.stop();
        let reachable = self.reachable_nodes();
        for (index, node) in self.nodes.iter_mut().enumerate() {
            if !reachable[index] {
                continue;
            }
            if let PlayableNode::Script { behaviour, .. } = node {
                behaviour.on_playable_destroy();
            }
        }
        self.destroyed = true;
    }

    pub fn evaluate(&mut self, delta_seconds: f64) {
        if self.destroyed {
            return;
        }
        if self.state == PlayState::Playing {
            let delta = if delta_seconds.is_finite() {
                delta_seconds
            } else {
                0.0
            };
            self.time += delta;
            self.advance(delta);
            self.run_frame_callbacks(delta);
        }
        self.refresh_output_poses();
    }

    pub fn is_done(&self) -> bool {
        if self.outputs.is_empty() {
            return false;
        }
        self.outputs.iter().all(|output| {
            output
                .source
                .is_some_and(|source| self.node_is_done(source))
        })
    }

    pub fn playable_time(&self, handle: PlayableHandle) -> Option<f64> {
        self.node(handle).map(|node| node.state().time)
    }

    pub fn set_playable_time(
        &mut self,
        handle: PlayableHandle,
        time: f64,
    ) -> Result<(), PlayableError> {
        if self.destroyed {
            return Err(PlayableError::GraphDestroyed);
        }
        let Some(node) = self.node_mut(handle) else {
            return Err(PlayableError::UnknownPlayable);
        };
        let time = if time.is_finite() { time.max(0.0) } else { 0.0 };
        node.set_time(time);
        Ok(())
    }

    pub fn playable_speed(&self, handle: PlayableHandle) -> Option<f64> {
        self.node(handle).map(|node| node.state().speed)
    }

    pub fn set_playable_speed(
        &mut self,
        handle: PlayableHandle,
        speed: f64,
    ) -> Result<(), PlayableError> {
        if self.destroyed {
            return Err(PlayableError::GraphDestroyed);
        }
        let Some(node) = self.node_mut(handle) else {
            return Err(PlayableError::UnknownPlayable);
        };
        node.state_mut().speed = if speed.is_finite() { speed } else { 1.0 };
        Ok(())
    }

    pub fn playable_duration(&self, handle: PlayableHandle) -> Option<f64> {
        self.node(handle).map(|node| node.state().duration)
    }

    pub fn set_playable_duration(
        &mut self,
        handle: PlayableHandle,
        duration: f64,
    ) -> Result<(), PlayableError> {
        if self.destroyed {
            return Err(PlayableError::GraphDestroyed);
        }
        let Some(node) = self.node_mut(handle) else {
            return Err(PlayableError::UnknownPlayable);
        };
        node.state_mut().duration = if duration.is_finite() {
            duration.max(0.0)
        } else {
            0.0
        };
        Ok(())
    }

    pub fn playable_is_done(&self, handle: PlayableHandle) -> Option<bool> {
        self.node(handle).map(|node| self.node_is_done_handle(node))
    }

    fn node(&self, handle: PlayableHandle) -> Option<&PlayableNode> {
        if handle.graph_id != self.id {
            return None;
        }
        self.nodes.get(handle.index)
    }

    fn node_mut(&mut self, handle: PlayableHandle) -> Option<&mut PlayableNode> {
        if handle.graph_id != self.id {
            return None;
        }
        self.nodes.get_mut(handle.index)
    }

    fn output(&self, handle: PlayableOutputHandle) -> Option<&PlayableOutput> {
        if handle.graph_id != self.id {
            return None;
        }
        self.outputs.get(handle.index)
    }

    fn add_node(&mut self, node: PlayableNode) -> PlayableHandle {
        self.nodes.push(node);
        PlayableHandle {
            index: self.nodes.len() - 1,
            graph_id: self.id,
        }
    }

    fn set_layer_mask(
        &mut self,
        layer: PlayableHandle,
        layer_index: u32,
        mask: AvatarMask,
    ) -> Result<(), PlayableError> {
        if self.destroyed {
            return Err(PlayableError::GraphDestroyed);
        }
        let Some(node) = self.node_mut(layer) else {
            return Err(PlayableError::UnknownPlayable);
        };
        let PlayableNode::LayerMixer { layers, .. } = node else {
            return Err(PlayableError::InvalidLayer);
        };
        let Some(entry) = layers.get_mut(layer_index as usize) else {
            return Err(PlayableError::InvalidLayer);
        };
        entry.mask = Some(mask);
        Ok(())
    }

    fn depends_on(&self, node: PlayableHandle, target: PlayableHandle) -> bool {
        let mut pending = vec![node];
        let mut visited: Vec<PlayableHandle> = Vec::new();
        while let Some(current) = pending.pop() {
            if current == target {
                return true;
            }
            if visited.contains(&current) {
                continue;
            }
            visited.push(current);
            if let Some(playable) = self.nodes.get(current.index) {
                pending.extend(playable.inputs());
            }
        }
        false
    }

    fn reachable_nodes(&self) -> Vec<bool> {
        let mut reachable = vec![false; self.nodes.len()];
        let mut pending: Vec<usize> = self
            .outputs
            .iter()
            .filter_map(|output| output.source)
            .map(|source| source.index)
            .collect();
        while let Some(index) = pending.pop() {
            if index >= self.nodes.len() || reachable[index] {
                continue;
            }
            reachable[index] = true;
            pending.extend(self.nodes[index].inputs().iter().map(|input| input.index));
        }
        reachable
    }

    fn advance(&mut self, delta: f64) {
        for node in &mut self.nodes {
            let advanced = delta * node.state().speed;
            match node {
                PlayableNode::Clip {
                    previous_time,
                    state,
                    ..
                } => {
                    *previous_time = state.time;
                    state.time += advanced;
                }
                PlayableNode::Mixer { state, .. }
                | PlayableNode::LayerMixer { state, .. }
                | PlayableNode::Script { state, .. } => {
                    state.time += advanced;
                }
            }
        }
    }

    fn run_frame_callbacks(&mut self, delta: f64) {
        let reachable = self.reachable_nodes();
        let evaluation_time = self.time;
        for (index, node) in self.nodes.iter_mut().enumerate() {
            if !reachable[index] {
                continue;
            }
            if let PlayableNode::Script { behaviour, state } = node {
                behaviour.prepare_frame(PlayableFrameData {
                    delta_time: delta as f32,
                    weight: state.input_weight,
                    evaluation_time,
                });
            }
        }
        for (index, node) in self.nodes.iter_mut().enumerate() {
            if !reachable[index] {
                continue;
            }
            if let PlayableNode::Script { behaviour, state } = node {
                behaviour.process_frame(PlayableFrameData {
                    delta_time: delta as f32,
                    weight: state.input_weight,
                    evaluation_time,
                });
            }
        }
    }

    fn refresh_output_poses(&mut self) {
        let sources: Vec<Option<PlayableHandle>> =
            self.outputs.iter().map(|output| output.source).collect();
        for (index, source) in sources.into_iter().enumerate() {
            let pose = match source {
                Some(handle) => self.evaluate_node(handle),
                None => Vec::new(),
            };
            self.outputs[index].pose = pose;
        }
    }

    fn evaluate_node(&self, handle: PlayableHandle) -> Vec<SampledBinding> {
        let Some(node) = self.nodes.get(handle.index) else {
            return Vec::new();
        };
        match node {
            PlayableNode::Clip {
                clip,
                previous_time,
                state,
            } => {
                clip.sample(*previous_time as f32, state.time as f32)
                    .bindings
            }
            PlayableNode::Mixer { inputs, .. } => {
                let mut pose: Vec<SampledBinding> = Vec::new();
                let mut accumulated = 0.0_f32;
                for input in inputs {
                    let Some(source) = input.source else {
                        continue;
                    };
                    if input.weight <= 0.0 {
                        continue;
                    }
                    let sample = self.evaluate_node(source);
                    let fraction = input.weight / (accumulated + input.weight);
                    blend_into(&mut pose, &sample, fraction);
                    accumulated += input.weight;
                }
                pose
            }
            PlayableNode::LayerMixer { layers, .. } => {
                let mut pose: Vec<SampledBinding> = Vec::new();
                for (index, layer) in layers.iter().enumerate() {
                    let Some(source) = layer.source else {
                        continue;
                    };
                    let sample = self.evaluate_node(source);
                    let masked = match &layer.mask {
                        Some(mask) => mask.filter_bindings(&sample),
                        None => sample,
                    };
                    let weight = layer.weight.clamp(0.0, 1.0);
                    if index == 0 {
                        pose = masked
                            .into_iter()
                            .map(|item| SampledBinding {
                                target: item.target.clone(),
                                value: default_value(&item.value).blend(&item.value, weight),
                            })
                            .collect();
                    } else {
                        blend_into(&mut pose, &masked, weight);
                    }
                }
                pose
            }
            PlayableNode::Script { .. } => Vec::new(),
        }
    }

    fn node_is_done(&self, handle: PlayableHandle) -> bool {
        match self.nodes.get(handle.index) {
            Some(node) => self.node_is_done_handle(node),
            None => false,
        }
    }

    fn node_is_done_handle(&self, node: &PlayableNode) -> bool {
        match node {
            PlayableNode::Clip { clip, state, .. } => {
                !matches!(clip.wrap_mode, WrapMode::Loop | WrapMode::PingPong)
                    && state.time >= state.duration
            }
            PlayableNode::Script { state, .. } => {
                state.duration > 0.0 && state.time >= state.duration
            }
            PlayableNode::Mixer { inputs, .. } => {
                !inputs.is_empty()
                    && inputs
                        .iter()
                        .all(|input| input.source.is_none_or(|source| self.node_is_done(source)))
            }
            PlayableNode::LayerMixer { layers, .. } => {
                !layers.is_empty()
                    && layers
                        .iter()
                        .all(|layer| layer.source.is_none_or(|source| self.node_is_done(source)))
            }
        }
    }
}

fn sanitize_weight(weight: f32) -> f32 {
    if weight.is_finite() {
        weight.max(0.0)
    } else {
        0.0
    }
}

fn default_value(value: &AnimatedValue) -> AnimatedValue {
    match value {
        AnimatedValue::Float(_) => AnimatedValue::Float(0.0),
        AnimatedValue::Vector3(_) => AnimatedValue::Vector3([0.0; 3]),
        AnimatedValue::Quaternion(_) => AnimatedValue::Quaternion([0.0, 0.0, 0.0, 1.0]),
        AnimatedValue::Color(_) => AnimatedValue::Color([0.0; 4]),
    }
}

fn blend_into(pose: &mut Vec<SampledBinding>, incoming: &[SampledBinding], fraction: f32) {
    for item in incoming {
        if let Some(existing) = pose.iter_mut().find(|entry| entry.target == item.target) {
            existing.value = existing.value.blend(&item.value, fraction);
        } else {
            pose.push(SampledBinding {
                target: item.target.clone(),
                value: default_value(&item.value).blend(&item.value, fraction),
            });
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationClipPlayable {
    handle: PlayableHandle,
}

impl AnimationClipPlayable {
    pub fn create(graph: &mut PlayableGraph, clip: AnimationClip) -> Self {
        let duration = clip.effective_duration() as f64;
        let handle = graph.add_node(PlayableNode::Clip {
            clip,
            previous_time: 0.0,
            state: PlayableState {
                duration,
                ..PlayableState::default()
            },
        });
        Self { handle }
    }

    pub fn handle(&self) -> PlayableHandle {
        self.handle
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationMixerPlayable {
    handle: PlayableHandle,
}

impl AnimationMixerPlayable {
    pub fn create(graph: &mut PlayableGraph, input_count: usize) -> Self {
        let inputs = (0..input_count)
            .map(|_| MixerInput {
                source: None,
                weight: 1.0,
            })
            .collect();
        let handle = graph.add_node(PlayableNode::Mixer {
            inputs,
            state: PlayableState::default(),
        });
        Self { handle }
    }

    pub fn handle(&self) -> PlayableHandle {
        self.handle
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationLayerMixerPlayable {
    handle: PlayableHandle,
}

impl AnimationLayerMixerPlayable {
    pub fn create(graph: &mut PlayableGraph, layer_count: usize) -> Self {
        let layers = (0..layer_count)
            .map(|_| LayerMixerLayer {
                source: None,
                weight: 1.0,
                mask: None,
            })
            .collect();
        let handle = graph.add_node(PlayableNode::LayerMixer {
            layers,
            state: PlayableState::default(),
        });
        Self { handle }
    }

    pub fn handle(&self) -> PlayableHandle {
        self.handle
    }

    pub fn set_layer_mask(
        &self,
        graph: &mut PlayableGraph,
        layer_index: u32,
        mask: AvatarMask,
    ) -> Result<(), PlayableError> {
        graph.set_layer_mask(self.handle, layer_index, mask)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptPlayable {
    handle: PlayableHandle,
}

impl ScriptPlayable {
    pub fn create(graph: &mut PlayableGraph, behaviour: Box<dyn PlayableBehaviour>) -> Self {
        let handle = graph.add_node(PlayableNode::Script {
            behaviour,
            state: PlayableState::default(),
        });
        Self { handle }
    }

    pub fn handle(&self) -> PlayableHandle {
        self.handle
    }
}
