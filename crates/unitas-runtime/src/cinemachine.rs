macro_rules! ffi_export {
    (pub extern "C" fn $name:ident($($arg:ident : $arg_ty:ty),* $(,)?) $(-> $ret:ty)? $body:block) => {
        #[unsafe(export_name = stringify!($name))]
        pub extern "C" fn $name($($arg : $arg_ty),*) $(-> $ret)? {
            #[used]
            static KEEP: extern "C" fn($($arg_ty),*) $(-> $ret)? = $name;
            $body
        }
    };
}

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use unitas_core::EntityId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendStyle {
    Cut,
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraBlend {
    pub style: BlendStyle,
    pub duration: f64,
}

impl CameraBlend {
    pub fn cut() -> Self {
        Self {
            style: BlendStyle::Cut,
            duration: 0.0,
        }
    }

    pub fn ease_in_out(duration: f64) -> Self {
        Self {
            style: BlendStyle::EaseInOut,
            duration,
        }
    }

    pub fn is_cut(&self) -> bool {
        self.style == BlendStyle::Cut || self.duration <= 0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LensSettings {
    Perspective { field_of_view: f32 },
    Orthographic { size: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualCamera {
    pub name: String,
    pub priority: i32,
    pub follow: Option<EntityId>,
    pub look_at: Option<EntityId>,
    pub active: bool,
    pub lens: LensSettings,
    pub position: [f64; 3],
    pub rotation: [f64; 4],
    pub follow_offset: [f64; 3],
    pub follow_damping: f64,
    pub look_at_damping: f64,
}

impl VirtualCamera {
    pub fn new(name: impl Into<String>, priority: i32) -> Self {
        Self {
            name: name.into(),
            priority,
            follow: None,
            look_at: None,
            active: false,
            lens: LensSettings::Perspective {
                field_of_view: 60.0,
            },
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            follow_offset: [0.0; 3],
            follow_damping: 0.0,
            look_at_damping: 0.0,
        }
    }

    pub fn with_follow(mut self, target: EntityId) -> Self {
        self.follow = Some(target);
        self
    }

    pub fn with_look_at(mut self, target: EntityId) -> Self {
        self.look_at = Some(target);
        self
    }

    pub fn with_follow_offset(mut self, offset: [f64; 3]) -> Self {
        self.follow_offset = offset;
        self
    }

    pub fn with_damping(mut self, follow_damping: f64, look_at_damping: f64) -> Self {
        self.follow_damping = follow_damping;
        self.look_at_damping = look_at_damping;
        self
    }

    pub fn with_position(mut self, position: [f64; 3]) -> Self {
        self.position = position;
        self
    }

    pub fn set_follow(&mut self, target: Option<EntityId>) {
        self.follow = target;
    }

    pub fn set_look_at(&mut self, target: Option<EntityId>) {
        self.look_at = target;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraBlendState {
    pub from: String,
    pub to: String,
    pub style: BlendStyle,
    pub duration: f64,
    pub elapsed: f64,
}

impl CameraBlendState {
    pub fn progress(&self) -> f64 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    pub fn weight(&self) -> f64 {
        let progress = self.progress();
        match self.style {
            BlendStyle::Cut => 1.0,
            BlendStyle::Linear => progress,
            BlendStyle::EaseIn => progress * progress,
            BlendStyle::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),
            BlendStyle::EaseInOut => progress * progress * (3.0 - 2.0 * progress),
        }
    }

    pub fn is_complete(&self) -> bool {
        self.elapsed >= self.duration
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraNotLiveReason {
    Inactive,
    LowerPriority,
    EqualPriority,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrainUpdateMethod {
    Smart,
    Fixed,
    Late,
    Manual,
}

#[derive(Clone, Debug)]
pub struct CinemachineWorld {
    cameras: Vec<VirtualCamera>,
    activation: Vec<u64>,
    next_activation: u64,
    live: Option<usize>,
    shot: Option<usize>,
    blend: Option<CameraBlendState>,
    default_blend: CameraBlend,
    custom_blends: Vec<(String, String, CameraBlend)>,
    update_method: BrainUpdateMethod,
    targets: HashMap<EntityId, [f64; 3]>,
}

impl Default for CinemachineWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl CinemachineWorld {
    pub fn new() -> Self {
        Self {
            cameras: Vec::new(),
            activation: Vec::new(),
            next_activation: 0,
            live: None,
            shot: None,
            blend: None,
            default_blend: CameraBlend::ease_in_out(2.0),
            custom_blends: Vec::new(),
            update_method: BrainUpdateMethod::Smart,
            targets: HashMap::new(),
        }
    }

    pub fn add_camera(&mut self, camera: VirtualCamera) -> usize {
        let index = self.cameras.len();
        self.cameras.push(camera);
        self.activation.push(0);
        index
    }

    pub fn camera(&self, index: usize) -> Option<&VirtualCamera> {
        self.cameras.get(index)
    }

    pub fn camera_mut(&mut self, index: usize) -> Option<&mut VirtualCamera> {
        self.cameras.get_mut(index)
    }

    pub fn camera_by_name(&self, name: &str) -> Option<&VirtualCamera> {
        self.cameras.iter().find(|camera| camera.name == name)
    }

    pub fn camera_index(&self, name: &str) -> Option<usize> {
        self.cameras.iter().position(|camera| camera.name == name)
    }

    pub fn camera_count(&self) -> usize {
        self.cameras.len()
    }

    pub fn set_active(&mut self, index: usize, active: bool) -> bool {
        let Some(camera) = self.cameras.get_mut(index) else {
            return false;
        };
        camera.active = active;
        if active {
            self.next_activation += 1;
            self.activation[index] = self.next_activation;
        }
        self.refresh_live();
        true
    }

    pub fn activate(&mut self, index: usize) -> bool {
        self.set_active(index, true)
    }

    pub fn deactivate(&mut self, index: usize) -> bool {
        self.set_active(index, false)
    }

    pub fn set_priority(&mut self, index: usize, priority: i32) -> bool {
        let Some(camera) = self.cameras.get_mut(index) else {
            return false;
        };
        camera.priority = priority;
        self.refresh_live();
        true
    }

    pub fn set_default_blend(&mut self, blend: CameraBlend) {
        self.default_blend = blend;
    }

    pub fn default_blend(&self) -> CameraBlend {
        self.default_blend
    }

    pub fn set_custom_blend(&mut self, from: &str, to: &str, blend: CameraBlend) {
        self.custom_blends
            .retain(|(start, end, _)| !(start == from && end == to));
        self.custom_blends
            .push((from.to_string(), to.to_string(), blend));
    }

    pub fn custom_blend(&self, from: &str, to: &str) -> Option<CameraBlend> {
        self.custom_blends
            .iter()
            .find(|(start, end, _)| start == from && end == to)
            .map(|(_, _, blend)| *blend)
    }

    pub fn set_update_method(&mut self, method: BrainUpdateMethod) {
        self.update_method = method;
    }

    pub fn update_method(&self) -> BrainUpdateMethod {
        self.update_method
    }

    pub fn set_target_position(&mut self, entity: EntityId, position: [f64; 3]) {
        self.targets.insert(entity, position);
    }

    pub fn target_position(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.targets.get(&entity).copied()
    }

    pub fn update_with_targets(&mut self, delta: f64, targets: &[(EntityId, [f64; 3])]) {
        for (entity, position) in targets {
            self.set_target_position(*entity, *position);
        }
        self.update(delta);
    }

    pub fn presented_pose(&self) -> Option<([f64; 3], [f64; 4])> {
        if let Some(blend) = &self.blend {
            let from = self.camera_by_name(&blend.from);
            let to = self.camera_by_name(&blend.to);
            if let (Some(from), Some(to)) = (from, to) {
                let weight = blend.weight();
                return Some((
                    lerp_vector(from.position, to.position, weight),
                    slerp_rotation(from.rotation, to.rotation, weight),
                ));
            }
        }
        let camera = self.live_camera()?;
        Some((camera.position, camera.rotation))
    }

    pub fn live_index(&self) -> Option<usize> {
        self.live
    }

    pub fn live_camera(&self) -> Option<&VirtualCamera> {
        self.live.and_then(|index| self.cameras.get(index))
    }

    pub fn active_blend(&self) -> Option<&CameraBlendState> {
        self.blend.as_ref()
    }

    pub fn shot_index(&self) -> Option<usize> {
        self.shot
    }

    pub fn not_live_reason(&self, index: usize) -> Option<CameraNotLiveReason> {
        let camera = self.cameras.get(index)?;
        if self.live == Some(index) {
            return None;
        }
        if !camera.active {
            return Some(CameraNotLiveReason::Inactive);
        }
        let live = self.live?;
        let live_camera = &self.cameras[live];
        if live_camera.priority > camera.priority {
            return Some(CameraNotLiveReason::LowerPriority);
        }
        Some(CameraNotLiveReason::EqualPriority)
    }

    pub fn set_shot(&mut self, camera: Option<&str>, blend: Option<CameraBlend>) -> bool {
        match camera {
            Some(name) => {
                let Some(index) = self.camera_index(name) else {
                    return false;
                };
                if self.shot == Some(index) {
                    return true;
                }
                self.shot = Some(index);
                self.transition(Some(index), blend);
                true
            }
            None => {
                if self.shot.is_none() {
                    return true;
                }
                self.shot = None;
                let desired = self.priority_live();
                self.transition(desired, None);
                true
            }
        }
    }

    pub fn update(&mut self, delta: f64) {
        self.refresh_live();
        self.advance_blend(delta);
        self.update_cameras(delta);
    }

    fn update_cameras(&mut self, delta: f64) {
        let mut indices: Vec<usize> = Vec::new();
        if let Some(live) = self.live {
            indices.push(live);
        }
        if let Some(blend) = &self.blend {
            if let Some(index) = self.camera_index(&blend.from)
                && !indices.contains(&index)
            {
                indices.push(index);
            }
            if let Some(index) = self.camera_index(&blend.to)
                && !indices.contains(&index)
            {
                indices.push(index);
            }
        }
        for index in indices {
            self.update_camera(index, delta);
        }
    }

    fn update_camera(&mut self, index: usize, delta: f64) {
        let (follow_position, look_position) = {
            let Some(camera) = self.cameras.get(index) else {
                return;
            };
            (
                camera
                    .follow
                    .and_then(|entity| self.targets.get(&entity).copied()),
                camera
                    .look_at
                    .and_then(|entity| self.targets.get(&entity).copied()),
            )
        };
        let Some(camera) = self.cameras.get_mut(index) else {
            return;
        };
        if let Some(target) = follow_position {
            let desired = [
                target[0] + camera.follow_offset[0],
                target[1] + camera.follow_offset[1],
                target[2] + camera.follow_offset[2],
            ];
            camera.position = damp_vector(camera.position, desired, camera.follow_damping, delta);
        }
        if let Some(target) = look_position {
            let desired = look_rotation(camera.position, target);
            camera.rotation =
                damp_rotation(camera.rotation, desired, camera.look_at_damping, delta);
        }
    }

    fn refresh_live(&mut self) {
        if self.shot.is_some() {
            return;
        }
        let desired = self.priority_live();
        self.transition(desired, None);
    }

    fn priority_live(&self) -> Option<usize> {
        let mut best: Option<usize> = None;
        for (index, camera) in self.cameras.iter().enumerate() {
            if !camera.active {
                continue;
            }
            let better = match best {
                None => true,
                Some(current) => {
                    let current_camera = &self.cameras[current];
                    camera.priority > current_camera.priority
                        || (camera.priority == current_camera.priority
                            && self.activation[index] > self.activation[current])
                }
            };
            if better {
                best = Some(index);
            }
        }
        best
    }

    fn transition(&mut self, desired: Option<usize>, blend_spec: Option<CameraBlend>) {
        if desired == self.live {
            return;
        }
        let previous = self.live;
        self.live = desired;
        let from_name = previous
            .and_then(|index| self.cameras.get(index))
            .map(|camera| camera.name.clone())
            .unwrap_or_default();
        let to_name = desired
            .and_then(|index| self.cameras.get(index))
            .map(|camera| camera.name.clone())
            .unwrap_or_default();
        let spec = match blend_spec {
            Some(spec) => spec,
            None => self
                .custom_blend(&from_name, &to_name)
                .unwrap_or(self.default_blend),
        };
        self.blend = if previous.is_none() || spec.is_cut() {
            None
        } else {
            Some(CameraBlendState {
                from: from_name,
                to: to_name,
                style: spec.style,
                duration: spec.duration,
                elapsed: 0.0,
            })
        };
    }

    fn advance_blend(&mut self, delta: f64) {
        if !delta.is_finite() || delta <= 0.0 {
            return;
        }
        let Some(mut blend) = self.blend.take() else {
            return;
        };
        blend.elapsed += delta;
        if !blend.is_complete() {
            self.blend = Some(blend);
        }
    }
}

fn damp_vector(current: [f64; 3], desired: [f64; 3], damping: f64, delta: f64) -> [f64; 3] {
    if delta > 0.0 && damping > 0.0 {
        let alpha = 1.0 - (-damping * delta).exp();
        lerp_vector(current, desired, alpha)
    } else {
        desired
    }
}

fn damp_rotation(current: [f64; 4], desired: [f64; 4], damping: f64, delta: f64) -> [f64; 4] {
    if delta > 0.0 && damping > 0.0 {
        let alpha = 1.0 - (-damping * delta).exp();
        slerp_rotation(current, desired, alpha)
    } else {
        normalize_rotation(desired)
    }
}

fn lerp_vector(first: [f64; 3], second: [f64; 3], alpha: f64) -> [f64; 3] {
    [
        first[0] + (second[0] - first[0]) * alpha,
        first[1] + (second[1] - first[1]) * alpha,
        first[2] + (second[2] - first[2]) * alpha,
    ]
}

fn slerp_rotation(first: [f64; 4], second: [f64; 4], alpha: f64) -> [f64; 4] {
    let start = normalize_rotation(first);
    let end = normalize_rotation(second);
    let dot = start[0] * end[0] + start[1] * end[1] + start[2] * end[2] + start[3] * end[3];
    let (start, end, cosine) = if dot < 0.0 {
        (start, [-end[0], -end[1], -end[2], -end[3]], -dot)
    } else {
        (start, end, dot)
    };
    if cosine >= 0.9995 {
        return normalize_rotation([
            start[0] + (end[0] - start[0]) * alpha,
            start[1] + (end[1] - start[1]) * alpha,
            start[2] + (end[2] - start[2]) * alpha,
            start[3] + (end[3] - start[3]) * alpha,
        ]);
    }
    let angle = cosine.clamp(-1.0, 1.0).acos();
    let sine = angle.sin();
    if sine.abs() <= 1e-9 {
        return start;
    }
    let start_weight = ((1.0 - alpha) * angle).sin() / sine;
    let end_weight = (alpha * angle).sin() / sine;
    normalize_rotation([
        start[0] * start_weight + end[0] * end_weight,
        start[1] * start_weight + end[1] * end_weight,
        start[2] * start_weight + end[2] * end_weight,
        start[3] * start_weight + end[3] * end_weight,
    ])
}

fn normalize_rotation(rotation: [f64; 4]) -> [f64; 4] {
    let length = (rotation[0] * rotation[0]
        + rotation[1] * rotation[1]
        + rotation[2] * rotation[2]
        + rotation[3] * rotation[3])
        .sqrt();
    if !length.is_finite() || length <= 1e-9 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    [
        rotation[0] / length,
        rotation[1] / length,
        rotation[2] / length,
        rotation[3] / length,
    ]
}

fn look_rotation(from: [f64; 3], to: [f64; 3]) -> [f64; 4] {
    let delta = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let Some(forward) = normalize_vector(delta) else {
        return [0.0, 0.0, 0.0, 1.0];
    };
    let right = normalize_vector(cross_product([0.0, 1.0, 0.0], forward))
        .or_else(|| normalize_vector(cross_product([0.0, 0.0, 1.0], forward)));
    let Some(right) = right else {
        return [0.0, 0.0, 0.0, 1.0];
    };
    let up = cross_product(forward, right);
    rotation_from_basis(right, up, forward)
}

fn rotation_from_basis(right: [f64; 3], up: [f64; 3], forward: [f64; 3]) -> [f64; 4] {
    let m00 = right[0];
    let m01 = up[0];
    let m02 = forward[0];
    let m10 = right[1];
    let m11 = up[1];
    let m12 = forward[1];
    let m20 = right[2];
    let m21 = up[2];
    let m22 = forward[2];
    let trace = m00 + m11 + m22;
    if trace > 0.0 {
        let scale = 2.0 * (trace + 1.0).sqrt();
        [
            (m21 - m12) / scale,
            (m02 - m20) / scale,
            (m10 - m01) / scale,
            0.25 * scale,
        ]
    } else if m00 > m11 && m00 > m22 {
        let scale = 2.0 * (1.0 + m00 - m11 - m22).sqrt();
        [
            0.25 * scale,
            (m01 + m10) / scale,
            (m02 + m20) / scale,
            (m21 - m12) / scale,
        ]
    } else if m11 > m22 {
        let scale = 2.0 * (1.0 + m11 - m00 - m22).sqrt();
        [
            (m01 + m10) / scale,
            0.25 * scale,
            (m12 + m21) / scale,
            (m02 - m20) / scale,
        ]
    } else {
        let scale = 2.0 * (1.0 + m22 - m00 - m11).sqrt();
        [
            (m02 + m20) / scale,
            (m12 + m21) / scale,
            0.25 * scale,
            (m10 - m01) / scale,
        ]
    }
}

fn cross_product(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ]
}

fn normalize_vector(vector: [f64; 3]) -> Option<[f64; 3]> {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if !length.is_finite() || length <= 1e-9 {
        return None;
    }
    Some([vector[0] / length, vector[1] / length, vector[2] / length])
}

const CINEMACHINE_STATE_SIZE: usize = 264;
const CINEMACHINE_LIVE_NAME: usize = 0;
const CINEMACHINE_LIVE_POSE: usize = 64;
const CINEMACHINE_BLEND_FROM: usize = 120;
const CINEMACHINE_BLEND_TO: usize = 184;
const CINEMACHINE_BLEND_WEIGHT: usize = 248;
const CINEMACHINE_LIVE_FLAG: usize = 256;
const CINEMACHINE_BLEND_FLAG: usize = 260;

struct CinemachineState {
    world: CinemachineWorld,
    buffer: Vec<u8>,
}

impl CinemachineState {
    fn new() -> Self {
        Self {
            world: CinemachineWorld::new(),
            buffer: Vec::new(),
        }
    }
}

static CINEMACHINE_STATES: Mutex<BTreeMap<u64, CinemachineState>> = Mutex::new(BTreeMap::new());
static NEXT_CINEMACHINE_WORLD: AtomicU64 = AtomicU64::new(1);

fn with_cinemachine_state<T>(
    world: u64,
    action: impl FnOnce(&mut CinemachineState) -> T,
) -> Option<T> {
    let mut states = match CINEMACHINE_STATES.lock() {
        Ok(guard) => guard,
        Err(_) => return None,
    };
    let state = states.get_mut(&world)?;
    Some(action(state))
}

fn cinemachine_camera_index(handle: u64) -> Option<usize> {
    if handle == 0 {
        return None;
    }
    usize::try_from(handle - 1).ok()
}

fn cinemachine_read_name(state: &CinemachineState, address: u64, length: usize) -> Option<String> {
    let base = state.buffer.as_ptr() as usize as u64;
    if address < base {
        return None;
    }
    let offset = usize::try_from(address - base).ok()?;
    let end = offset.checked_add(length)?;
    let bytes = state.buffer.get(offset..end)?;
    let text = std::str::from_utf8(bytes).ok()?;
    Some(text.trim_end_matches('\0').to_string())
}

fn cinemachine_blend_style(value: i32) -> BlendStyle {
    match value {
        0 => BlendStyle::Cut,
        1 => BlendStyle::Linear,
        2 => BlendStyle::EaseIn,
        3 => BlendStyle::EaseOut,
        _ => BlendStyle::EaseInOut,
    }
}

fn cinemachine_write_name(slot: &mut [u8], name: &str) {
    let bytes = name.as_bytes();
    let count = bytes.len().min(slot.len().saturating_sub(1));
    slot[..count].copy_from_slice(&bytes[..count]);
}

fn cinemachine_write_f64(slot: &mut [u8], value: f64) {
    slot.copy_from_slice(&value.to_le_bytes());
}

ffi_export! {
pub extern "C" fn unitas_cinemachine_world_create() -> u64 {
    let mut states = match CINEMACHINE_STATES.lock() {
        Ok(guard) => guard,
        Err(_) => return 0,
    };
    let handle = NEXT_CINEMACHINE_WORLD.fetch_add(1, Ordering::Relaxed);
    states.insert(handle, CinemachineState::new());
    handle
}
}

ffi_export! {
pub extern "C" fn unitas_cinemachine_world_destroy(world: u64) -> i32 {
    let mut states = match CINEMACHINE_STATES.lock() {
        Ok(guard) => guard,
        Err(_) => return -1,
    };
    if states.remove(&world).is_some() {
        0
    } else {
        -1
    }
}
}

ffi_export! {
pub extern "C" fn unitas_cinemachine_world_buffer(world: u64, capacity: u64) -> u64 {
    let cap = match usize::try_from(capacity) {
        Ok(value) => value,
        Err(_) => return 0,
    };
    if cap == 0 {
        return 0;
    }
    with_cinemachine_state(world, |state| {
        if state.buffer.len() < cap {
            state.buffer.resize(cap, 0);
        }
        state.buffer.as_ptr() as usize as u64
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_cinemachine_world_add_camera(
    world: u64,
    name_address: u64,
    name_length: u64,
    priority: i32,
) -> u64 {
    let length = match usize::try_from(name_length) {
        Ok(value) => value,
        Err(_) => return 0,
    };
    with_cinemachine_state(world, |state| {
        let Some(name) = cinemachine_read_name(state, name_address, length) else {
            return 0;
        };
        let index = state.world.add_camera(VirtualCamera::new(name, priority));
        match u64::try_from(index) {
            Ok(value) => value + 1,
            Err(_) => 0,
        }
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_cinemachine_world_set_priority(
    world: u64,
    camera: u64,
    priority: i32,
) -> i32 {
    with_cinemachine_state(world, |state| {
        let Some(index) = cinemachine_camera_index(camera) else {
            return -1;
        };
        if state.world.set_priority(index, priority) {
            0
        } else {
            -1
        }
    })
    .unwrap_or(-1)
}
}

ffi_export! {
pub extern "C" fn unitas_cinemachine_world_set_active(world: u64, camera: u64, active: u8) -> i32 {
    with_cinemachine_state(world, |state| {
        let Some(index) = cinemachine_camera_index(camera) else {
            return -1;
        };
        if state.world.set_active(index, active != 0) {
            0
        } else {
            -1
        }
    })
    .unwrap_or(-1)
}
}

ffi_export! {
pub extern "C" fn unitas_cinemachine_world_set_follow(
    world: u64,
    camera: u64,
    has_follow: u8,
    entity: u64,
    offset_x: f64,
    offset_y: f64,
    offset_z: f64,
    damping: f64,
) -> i32 {
    with_cinemachine_state(world, |state| {
        let Some(index) = cinemachine_camera_index(camera) else {
            return -1;
        };
        let Some(camera) = state.world.camera_mut(index) else {
            return -1;
        };
        camera.follow = if has_follow != 0 {
            Some(EntityId::new(entity))
        } else {
            None
        };
        camera.follow_offset = [offset_x, offset_y, offset_z];
        camera.follow_damping = damping;
        0
    })
    .unwrap_or(-1)
}
}

ffi_export! {
pub extern "C" fn unitas_cinemachine_world_set_look_at(
    world: u64,
    camera: u64,
    has_look_at: u8,
    entity: u64,
    damping: f64,
) -> i32 {
    with_cinemachine_state(world, |state| {
        let Some(index) = cinemachine_camera_index(camera) else {
            return -1;
        };
        let Some(camera) = state.world.camera_mut(index) else {
            return -1;
        };
        camera.look_at = if has_look_at != 0 {
            Some(EntityId::new(entity))
        } else {
            None
        };
        camera.look_at_damping = damping;
        0
    })
    .unwrap_or(-1)
}
}

ffi_export! {
pub extern "C" fn unitas_cinemachine_world_set_target_position(
    world: u64,
    entity: u64,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    with_cinemachine_state(world, |state| {
        state
            .world
            .set_target_position(EntityId::new(entity), [x, y, z]);
        0
    })
    .unwrap_or(-1)
}
}

ffi_export! {
pub extern "C" fn unitas_cinemachine_world_set_default_blend(
    world: u64,
    style: i32,
    duration: f64,
) -> i32 {
    with_cinemachine_state(world, |state| {
        state.world.set_default_blend(CameraBlend {
            style: cinemachine_blend_style(style),
            duration,
        });
        0
    })
    .unwrap_or(-1)
}
}

ffi_export! {
pub extern "C" fn unitas_cinemachine_world_set_custom_blend(
    world: u64,
    from_address: u64,
    from_length: u64,
    to_address: u64,
    to_length: u64,
    style: i32,
    duration: f64,
) -> i32 {
    let from_size = match usize::try_from(from_length) {
        Ok(value) => value,
        Err(_) => return -1,
    };
    let to_size = match usize::try_from(to_length) {
        Ok(value) => value,
        Err(_) => return -1,
    };
    with_cinemachine_state(world, |state| {
        let Some(from) = cinemachine_read_name(state, from_address, from_size) else {
            return -1;
        };
        let Some(to) = cinemachine_read_name(state, to_address, to_size) else {
            return -1;
        };
        state.world.set_custom_blend(
            &from,
            &to,
            CameraBlend {
                style: cinemachine_blend_style(style),
                duration,
            },
        );
        0
    })
    .unwrap_or(-1)
}
}

ffi_export! {
pub extern "C" fn unitas_cinemachine_world_update(world: u64, delta: f64) -> i32 {
    with_cinemachine_state(world, |state| {
        state.world.update(delta);
        0
    })
    .unwrap_or(-1)
}
}

ffi_export! {
pub extern "C" fn unitas_cinemachine_world_state(
    world: u64,
    address: u64,
    capacity: u64,
) -> i32 {
    let cap = match usize::try_from(capacity) {
        Ok(value) => value,
        Err(_) => return -1,
    };
    if cap < CINEMACHINE_STATE_SIZE {
        return -1;
    }
    with_cinemachine_state(world, |state| {
        let base = state.buffer.as_ptr() as usize as u64;
        if address < base {
            return -1;
        }
        let offset = match usize::try_from(address - base) {
            Ok(value) => value,
            Err(_) => return -1,
        };
        let end = match offset.checked_add(CINEMACHINE_STATE_SIZE) {
            Some(value) => value,
            None => return -1,
        };
        if end > state.buffer.len() {
            return -1;
        }
        let mut record = [0u8; CINEMACHINE_STATE_SIZE];
        if let Some(camera) = state.world.live_camera() {
            cinemachine_write_name(
                &mut record[CINEMACHINE_LIVE_NAME..CINEMACHINE_LIVE_POSE],
                &camera.name,
            );
            cinemachine_write_f64(
                &mut record[CINEMACHINE_LIVE_POSE..CINEMACHINE_LIVE_POSE + 8],
                camera.position[0],
            );
            cinemachine_write_f64(
                &mut record[CINEMACHINE_LIVE_POSE + 8..CINEMACHINE_LIVE_POSE + 16],
                camera.position[1],
            );
            cinemachine_write_f64(
                &mut record[CINEMACHINE_LIVE_POSE + 16..CINEMACHINE_LIVE_POSE + 24],
                camera.position[2],
            );
            cinemachine_write_f64(
                &mut record[CINEMACHINE_LIVE_POSE + 24..CINEMACHINE_LIVE_POSE + 32],
                camera.rotation[0],
            );
            cinemachine_write_f64(
                &mut record[CINEMACHINE_LIVE_POSE + 32..CINEMACHINE_LIVE_POSE + 40],
                camera.rotation[1],
            );
            cinemachine_write_f64(
                &mut record[CINEMACHINE_LIVE_POSE + 40..CINEMACHINE_LIVE_POSE + 48],
                camera.rotation[2],
            );
            cinemachine_write_f64(
                &mut record[CINEMACHINE_LIVE_POSE + 48..CINEMACHINE_LIVE_POSE + 56],
                camera.rotation[3],
            );
            record[CINEMACHINE_LIVE_FLAG..CINEMACHINE_LIVE_FLAG + 4]
                .copy_from_slice(&1i32.to_le_bytes());
        }
        if let Some(blend) = state.world.active_blend() {
            cinemachine_write_name(
                &mut record[CINEMACHINE_BLEND_FROM..CINEMACHINE_BLEND_TO],
                &blend.from,
            );
            cinemachine_write_name(
                &mut record[CINEMACHINE_BLEND_TO..CINEMACHINE_BLEND_WEIGHT],
                &blend.to,
            );
            cinemachine_write_f64(
                &mut record[CINEMACHINE_BLEND_WEIGHT..CINEMACHINE_BLEND_WEIGHT + 8],
                blend.weight(),
            );
            record[CINEMACHINE_BLEND_FLAG..CINEMACHINE_BLEND_FLAG + 4]
                .copy_from_slice(&1i32.to_le_bytes());
        }
        state.buffer[offset..end].copy_from_slice(&record);
        0
    })
    .unwrap_or(-1)
}
}
