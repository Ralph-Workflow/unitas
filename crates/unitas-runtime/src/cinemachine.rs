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
        }
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
