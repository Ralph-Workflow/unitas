use unitas_core::EntityId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BodyId2D(u64);

impl BodyId2D {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl From<u64> for BodyId2D {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<BodyId2D> for u64 {
    fn from(id: BodyId2D) -> Self {
        id.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BodyType2D {
    #[default]
    Dynamic,
    Kinematic,
    Static,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CollisionDetectionMode2D {
    #[default]
    Discrete,
    Continuous,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RigidbodySleepMode2D {
    NeverSleep,
    #[default]
    StartAwake,
    StartAsleep,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RigidbodyInterpolation2D {
    #[default]
    None,
    Interpolate,
    Extrapolate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BodyDesc2D {
    pub body_type: BodyType2D,
    pub position: [f32; 2],
    pub rotation: f32,
    pub linear_velocity: [f32; 2],
    pub angular_velocity: f32,
    pub gravity_scale: f32,
    pub mass: f32,
    pub linear_drag: f32,
    pub angular_drag: f32,
    pub simulated: bool,
    pub sleeping: bool,
    pub collision_detection_mode: CollisionDetectionMode2D,
    pub sleep_mode: RigidbodySleepMode2D,
    pub interpolation: RigidbodyInterpolation2D,
    pub use_full_kinematic_contacts: bool,
    pub game_object: EntityId,
    pub layer: u8,
}

impl Default for BodyDesc2D {
    fn default() -> Self {
        Self {
            body_type: BodyType2D::Dynamic,
            position: [0.0, 0.0],
            rotation: 0.0,
            linear_velocity: [0.0, 0.0],
            angular_velocity: 0.0,
            gravity_scale: 1.0,
            mass: 1.0,
            linear_drag: 0.0,
            angular_drag: 0.05,
            simulated: true,
            sleeping: false,
            collision_detection_mode: CollisionDetectionMode2D::Discrete,
            sleep_mode: RigidbodySleepMode2D::StartAwake,
            interpolation: RigidbodyInterpolation2D::None,
            use_full_kinematic_contacts: false,
            game_object: EntityId::new(0),
            layer: 0,
        }
    }
}

impl BodyDesc2D {
    pub fn dynamic(game_object: EntityId, position: [f32; 2]) -> Self {
        Self {
            body_type: BodyType2D::Dynamic,
            position,
            game_object,
            ..Self::default()
        }
    }

    pub fn kinematic(game_object: EntityId, position: [f32; 2]) -> Self {
        Self {
            body_type: BodyType2D::Kinematic,
            position,
            gravity_scale: 0.0,
            game_object,
            ..Self::default()
        }
    }

    pub fn static_body(game_object: EntityId, position: [f32; 2]) -> Self {
        Self {
            body_type: BodyType2D::Static,
            position,
            gravity_scale: 0.0,
            mass: 0.0,
            game_object,
            ..Self::default()
        }
    }

    pub fn with_layer(mut self, layer: u8) -> Self {
        self.layer = layer;
        self
    }

    pub fn with_velocity(mut self, linear_velocity: [f32; 2]) -> Self {
        self.linear_velocity = linear_velocity;
        self
    }

    pub fn with_gravity_scale(mut self, gravity_scale: f32) -> Self {
        self.gravity_scale = gravity_scale;
        self
    }

    pub fn with_mass(mut self, mass: f32) -> Self {
        self.mass = mass;
        self
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_simulated(mut self, simulated: bool) -> Self {
        self.simulated = simulated;
        self
    }

    pub fn with_collision_detection_mode(mut self, mode: CollisionDetectionMode2D) -> Self {
        self.collision_detection_mode = mode;
        self
    }

    pub fn with_sleep_mode(mut self, sleep_mode: RigidbodySleepMode2D) -> Self {
        self.sleep_mode = sleep_mode;
        match sleep_mode {
            RigidbodySleepMode2D::NeverSleep | RigidbodySleepMode2D::StartAwake => {
                self.sleeping = false;
            }
            RigidbodySleepMode2D::StartAsleep => {
                self.sleeping = true;
            }
        }
        self
    }

    pub fn with_interpolation(mut self, interpolation: RigidbodyInterpolation2D) -> Self {
        self.interpolation = interpolation;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rigidbody2D {
    pub id: BodyId2D,
    pub body_type: BodyType2D,
    pub position: [f32; 2],
    pub rotation: f32,
    pub linear_velocity: [f32; 2],
    pub angular_velocity: f32,
    pub gravity_scale: f32,
    pub mass: f32,
    pub linear_drag: f32,
    pub angular_drag: f32,
    pub simulated: bool,
    pub sleeping: bool,
    pub collision_detection_mode: CollisionDetectionMode2D,
    pub sleep_mode: RigidbodySleepMode2D,
    pub interpolation: RigidbodyInterpolation2D,
    pub use_full_kinematic_contacts: bool,
    pub game_object: EntityId,
    pub layer: u8,
    pub(crate) accumulated_force: [f32; 2],
    pub(crate) accumulated_torque: f32,
    pub(crate) sleep_timer: f32,
    pub(crate) previous_position: [f32; 2],
    pub(crate) previous_rotation: f32,
    pub(crate) move_target_position: Option<[f32; 2]>,
    pub(crate) move_target_rotation: Option<f32>,
}

impl Rigidbody2D {
    pub fn from_desc(id: BodyId2D, desc: BodyDesc2D) -> Self {
        let initial_sleeping = match desc.sleep_mode {
            RigidbodySleepMode2D::NeverSleep => false,
            RigidbodySleepMode2D::StartAsleep => true,
            RigidbodySleepMode2D::StartAwake => desc.sleeping,
        };
        Self {
            id,
            body_type: desc.body_type,
            position: desc.position,
            rotation: desc.rotation,
            linear_velocity: desc.linear_velocity,
            angular_velocity: desc.angular_velocity,
            gravity_scale: desc.gravity_scale,
            mass: desc.mass,
            linear_drag: desc.linear_drag,
            angular_drag: desc.angular_drag,
            simulated: desc.simulated,
            sleeping: initial_sleeping,
            collision_detection_mode: desc.collision_detection_mode,
            sleep_mode: desc.sleep_mode,
            interpolation: desc.interpolation,
            use_full_kinematic_contacts: desc.use_full_kinematic_contacts,
            game_object: desc.game_object,
            layer: desc.layer,
            accumulated_force: [0.0, 0.0],
            accumulated_torque: 0.0,
            sleep_timer: 0.0,
            previous_position: desc.position,
            previous_rotation: desc.rotation,
            move_target_position: None,
            move_target_rotation: None,
        }
    }

    pub fn is_dynamic(&self) -> bool {
        self.body_type == BodyType2D::Dynamic
    }

    pub fn is_kinematic(&self) -> bool {
        self.body_type == BodyType2D::Kinematic
    }

    pub fn is_static(&self) -> bool {
        self.body_type == BodyType2D::Static
    }

    pub fn is_sleeping(&self) -> bool {
        self.sleeping
    }

    pub fn set_sleep_mode(&mut self, sleep_mode: RigidbodySleepMode2D) {
        self.sleep_mode = sleep_mode;
        match sleep_mode {
            RigidbodySleepMode2D::NeverSleep => self.wake_up(),
            RigidbodySleepMode2D::StartAsleep => self.sleep(),
            RigidbodySleepMode2D::StartAwake => {}
        }
    }

    pub fn sleep(&mut self) {
        if self.sleep_mode == RigidbodySleepMode2D::NeverSleep {
            return;
        }
        self.sleeping = true;
        self.linear_velocity = [0.0, 0.0];
        self.angular_velocity = 0.0;
    }

    pub fn wake_up(&mut self) {
        self.sleeping = false;
        self.sleep_timer = 0.0;
    }

    pub fn interpolated_pose(&self, alpha: f32, fixed_dt: f32) -> ([f32; 2], f32) {
        let a = alpha.clamp(0.0, 1.0);
        match self.interpolation {
            RigidbodyInterpolation2D::None => (self.position, self.rotation),
            RigidbodyInterpolation2D::Interpolate => (
                [
                    self.previous_position[0] + (self.position[0] - self.previous_position[0]) * a,
                    self.previous_position[1] + (self.position[1] - self.previous_position[1]) * a,
                ],
                self.previous_rotation + (self.rotation - self.previous_rotation) * a,
            ),
            RigidbodyInterpolation2D::Extrapolate => {
                let dt_rem = a * fixed_dt.max(0.0);
                (
                    [
                        self.position[0] + self.linear_velocity[0] * dt_rem,
                        self.position[1] + self.linear_velocity[1] * dt_rem,
                    ],
                    self.rotation + self.angular_velocity * dt_rem,
                )
            }
        }
    }

    pub fn add_force(&mut self, force: [f32; 2]) {
        if self.body_type == BodyType2D::Dynamic {
            self.accumulated_force[0] += force[0];
            self.accumulated_force[1] += force[1];
            if force[0] != 0.0 || force[1] != 0.0 {
                self.wake_up();
            }
        }
    }

    pub fn add_impulse(&mut self, impulse: [f32; 2]) {
        if self.body_type == BodyType2D::Dynamic && self.mass > 0.0 {
            self.linear_velocity[0] += impulse[0] / self.mass;
            self.linear_velocity[1] += impulse[1] / self.mass;
            if impulse[0] != 0.0 || impulse[1] != 0.0 {
                self.wake_up();
            }
        }
    }

    pub fn add_torque(&mut self, torque: f32) {
        if self.body_type == BodyType2D::Dynamic {
            self.accumulated_torque += torque;
            if torque != 0.0 {
                self.wake_up();
            }
        }
    }

    pub fn move_position(&mut self, target: [f32; 2]) {
        self.move_target_position = Some(target);
        self.wake_up();
    }

    pub fn move_rotation(&mut self, angle_radians: f32) {
        self.move_target_rotation = Some(angle_radians);
        self.wake_up();
    }

    pub(crate) fn inverse_mass(&self) -> f32 {
        match self.body_type {
            BodyType2D::Dynamic if self.mass > 0.0 => 1.0 / self.mass,
            BodyType2D::Dynamic | BodyType2D::Kinematic | BodyType2D::Static => 0.0,
        }
    }
}
