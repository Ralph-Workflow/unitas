use unitas_core::EntityId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JointId3D(pub u64);

impl JointId3D {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointLimits3D {
    pub min: f64,
    pub max: f64,
}

impl Default for JointLimits3D {
    fn default() -> Self {
        Self { min: 0.0, max: 0.0 }
    }
}

impl JointLimits3D {
    pub const fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointSpring3D {
    pub spring: f64,
    pub damper: f64,
    pub target_position: f64,
}

impl Default for JointSpring3D {
    fn default() -> Self {
        Self {
            spring: 0.0,
            damper: 0.0,
            target_position: 0.0,
        }
    }
}

impl JointSpring3D {
    pub const fn new(spring: f64, damper: f64, target_position: f64) -> Self {
        Self {
            spring,
            damper,
            target_position,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointMotor3D {
    pub target_velocity: f64,
    pub force: f64,
    pub free_spin: bool,
}

impl Default for JointMotor3D {
    fn default() -> Self {
        Self {
            target_velocity: 0.0,
            force: 0.0,
            free_spin: false,
        }
    }
}

impl JointMotor3D {
    pub const fn new(target_velocity: f64, force: f64, free_spin: bool) -> Self {
        Self {
            target_velocity,
            force,
            free_spin,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ConfigurableJointMotion {
    #[default]
    Free,
    Limited,
    Locked,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointDrive3D {
    pub position_spring: f64,
    pub position_damper: f64,
    pub maximum_force: f64,
}

impl Default for JointDrive3D {
    fn default() -> Self {
        Self {
            position_spring: 0.0,
            position_damper: 0.0,
            maximum_force: f64::INFINITY,
        }
    }
}

impl JointDrive3D {
    pub const fn new(position_spring: f64, position_damper: f64, maximum_force: f64) -> Self {
        Self {
            position_spring,
            position_damper,
            maximum_force,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedJoint3D {
    pub body_a: EntityId,
    pub body_b: Option<EntityId>,
    pub anchor_a: [f64; 3],
    pub anchor_b: [f64; 3],
    pub break_force: f64,
    pub break_torque: f64,
}

impl FixedJoint3D {
    pub const fn new(body_a: EntityId, body_b: Option<EntityId>) -> Self {
        Self {
            body_a,
            body_b,
            anchor_a: [0.0; 3],
            anchor_b: [0.0; 3],
            break_force: f64::INFINITY,
            break_torque: f64::INFINITY,
        }
    }

    pub const fn with_anchors(mut self, anchor_a: [f64; 3], anchor_b: [f64; 3]) -> Self {
        self.anchor_a = anchor_a;
        self.anchor_b = anchor_b;
        self
    }

    pub const fn with_break_force(mut self, break_force: f64) -> Self {
        self.break_force = break_force;
        self
    }

    pub const fn with_break_torque(mut self, break_torque: f64) -> Self {
        self.break_torque = break_torque;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HingeJoint3D {
    pub body_a: EntityId,
    pub body_b: Option<EntityId>,
    pub anchor_a: [f64; 3],
    pub anchor_b: [f64; 3],
    pub axis: [f64; 3],
    pub use_limits: bool,
    pub limits: JointLimits3D,
    pub use_spring: bool,
    pub spring: JointSpring3D,
    pub use_motor: bool,
    pub motor: JointMotor3D,
    pub break_force: f64,
    pub break_torque: f64,
}

impl HingeJoint3D {
    pub const fn new(body_a: EntityId, body_b: Option<EntityId>, axis: [f64; 3]) -> Self {
        Self {
            body_a,
            body_b,
            anchor_a: [0.0; 3],
            anchor_b: [0.0; 3],
            axis,
            use_limits: false,
            limits: JointLimits3D { min: 0.0, max: 0.0 },
            use_spring: false,
            spring: JointSpring3D {
                spring: 0.0,
                damper: 0.0,
                target_position: 0.0,
            },
            use_motor: false,
            motor: JointMotor3D {
                target_velocity: 0.0,
                force: 0.0,
                free_spin: false,
            },
            break_force: f64::INFINITY,
            break_torque: f64::INFINITY,
        }
    }

    pub const fn with_anchors(mut self, anchor_a: [f64; 3], anchor_b: [f64; 3]) -> Self {
        self.anchor_a = anchor_a;
        self.anchor_b = anchor_b;
        self
    }

    pub const fn with_limits(mut self, min: f64, max: f64) -> Self {
        self.use_limits = true;
        self.limits = JointLimits3D { min, max };
        self
    }

    pub const fn with_spring(mut self, spring: f64, damper: f64, target_position: f64) -> Self {
        self.use_spring = true;
        self.spring = JointSpring3D {
            spring,
            damper,
            target_position,
        };
        self
    }

    pub const fn with_motor(mut self, target_velocity: f64, force: f64, free_spin: bool) -> Self {
        self.use_motor = true;
        self.motor = JointMotor3D {
            target_velocity,
            force,
            free_spin,
        };
        self
    }

    pub const fn with_break_force(mut self, break_force: f64) -> Self {
        self.break_force = break_force;
        self
    }

    pub const fn with_break_torque(mut self, break_torque: f64) -> Self {
        self.break_torque = break_torque;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringJoint3D {
    pub body_a: EntityId,
    pub body_b: Option<EntityId>,
    pub anchor_a: [f64; 3],
    pub anchor_b: [f64; 3],
    pub min_distance: f64,
    pub max_distance: f64,
    pub spring: f64,
    pub damper: f64,
    pub tolerance: f64,
    pub break_force: f64,
    pub break_torque: f64,
}

impl SpringJoint3D {
    pub const fn new(body_a: EntityId, body_b: Option<EntityId>, spring: f64, damper: f64) -> Self {
        Self {
            body_a,
            body_b,
            anchor_a: [0.0; 3],
            anchor_b: [0.0; 3],
            min_distance: 0.0,
            max_distance: 0.0,
            spring,
            damper,
            tolerance: 0.025,
            break_force: f64::INFINITY,
            break_torque: f64::INFINITY,
        }
    }

    pub const fn with_anchors(mut self, anchor_a: [f64; 3], anchor_b: [f64; 3]) -> Self {
        self.anchor_a = anchor_a;
        self.anchor_b = anchor_b;
        self
    }

    pub const fn with_distance_band(mut self, min_distance: f64, max_distance: f64) -> Self {
        self.min_distance = min_distance;
        self.max_distance = max_distance;
        self
    }

    pub const fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    pub const fn with_break_force(mut self, break_force: f64) -> Self {
        self.break_force = break_force;
        self
    }

    pub const fn with_break_torque(mut self, break_torque: f64) -> Self {
        self.break_torque = break_torque;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterJoint3D {
    pub body_a: EntityId,
    pub body_b: Option<EntityId>,
    pub anchor_a: [f64; 3],
    pub anchor_b: [f64; 3],
    pub axis: [f64; 3],
    pub swing_axis: [f64; 3],
    pub twist_limit_low: f64,
    pub twist_limit_high: f64,
    pub swing1_limit: f64,
    pub swing2_limit: f64,
    pub break_force: f64,
    pub break_torque: f64,
}

impl CharacterJoint3D {
    pub const fn new(body_a: EntityId, body_b: Option<EntityId>) -> Self {
        Self {
            body_a,
            body_b,
            anchor_a: [0.0; 3],
            anchor_b: [0.0; 3],
            axis: [1.0, 0.0, 0.0],
            swing_axis: [0.0, 1.0, 0.0],
            twist_limit_low: -std::f64::consts::FRAC_PI_4,
            twist_limit_high: std::f64::consts::FRAC_PI_4,
            swing1_limit: std::f64::consts::FRAC_PI_4,
            swing2_limit: std::f64::consts::FRAC_PI_4,
            break_force: f64::INFINITY,
            break_torque: f64::INFINITY,
        }
    }

    pub const fn with_anchors(mut self, anchor_a: [f64; 3], anchor_b: [f64; 3]) -> Self {
        self.anchor_a = anchor_a;
        self.anchor_b = anchor_b;
        self
    }

    pub const fn with_axes(mut self, axis: [f64; 3], swing_axis: [f64; 3]) -> Self {
        self.axis = axis;
        self.swing_axis = swing_axis;
        self
    }

    pub const fn with_twist_limits(mut self, low: f64, high: f64) -> Self {
        self.twist_limit_low = low;
        self.twist_limit_high = high;
        self
    }

    pub const fn with_swing_limits(mut self, swing1: f64, swing2: f64) -> Self {
        self.swing1_limit = swing1;
        self.swing2_limit = swing2;
        self
    }

    pub const fn with_break_force(mut self, break_force: f64) -> Self {
        self.break_force = break_force;
        self
    }

    pub const fn with_break_torque(mut self, break_torque: f64) -> Self {
        self.break_torque = break_torque;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfigurableJoint3D {
    pub body_a: EntityId,
    pub body_b: Option<EntityId>,
    pub anchor_a: [f64; 3],
    pub anchor_b: [f64; 3],
    pub axis: [f64; 3],
    pub secondary_axis: [f64; 3],
    pub x_motion: ConfigurableJointMotion,
    pub y_motion: ConfigurableJointMotion,
    pub z_motion: ConfigurableJointMotion,
    pub angular_x_motion: ConfigurableJointMotion,
    pub angular_y_motion: ConfigurableJointMotion,
    pub angular_z_motion: ConfigurableJointMotion,
    pub linear_limit: f64,
    pub angular_x_limit_low: f64,
    pub angular_x_limit_high: f64,
    pub angular_y_limit: f64,
    pub angular_z_limit: f64,
    pub x_drive: JointDrive3D,
    pub y_drive: JointDrive3D,
    pub z_drive: JointDrive3D,
    pub z_angular_drive: Option<JointDrive3D>,
    pub target_position: [f64; 3],
    pub target_velocity: [f64; 3],
    pub break_force: f64,
    pub break_torque: f64,
}

impl ConfigurableJoint3D {
    pub const fn new(body_a: EntityId, body_b: Option<EntityId>) -> Self {
        let default_drive = JointDrive3D {
            position_spring: 0.0,
            position_damper: 0.0,
            maximum_force: f64::INFINITY,
        };
        Self {
            body_a,
            body_b,
            anchor_a: [0.0; 3],
            anchor_b: [0.0; 3],
            axis: [1.0, 0.0, 0.0],
            secondary_axis: [0.0, 1.0, 0.0],
            x_motion: ConfigurableJointMotion::Free,
            y_motion: ConfigurableJointMotion::Free,
            z_motion: ConfigurableJointMotion::Free,
            angular_x_motion: ConfigurableJointMotion::Free,
            angular_y_motion: ConfigurableJointMotion::Free,
            angular_z_motion: ConfigurableJointMotion::Free,
            linear_limit: 0.0,
            angular_x_limit_low: 0.0,
            angular_x_limit_high: 0.0,
            angular_y_limit: 0.0,
            angular_z_limit: 0.0,
            x_drive: default_drive,
            y_drive: default_drive,
            z_drive: default_drive,
            z_angular_drive: None,
            target_position: [0.0; 3],
            target_velocity: [0.0; 3],
            break_force: f64::INFINITY,
            break_torque: f64::INFINITY,
        }
    }

    pub const fn with_anchors(mut self, anchor_a: [f64; 3], anchor_b: [f64; 3]) -> Self {
        self.anchor_a = anchor_a;
        self.anchor_b = anchor_b;
        self
    }

    pub const fn with_linear_motions(
        mut self,
        x_motion: ConfigurableJointMotion,
        y_motion: ConfigurableJointMotion,
        z_motion: ConfigurableJointMotion,
    ) -> Self {
        self.x_motion = x_motion;
        self.y_motion = y_motion;
        self.z_motion = z_motion;
        self
    }

    pub const fn with_angular_motions(
        mut self,
        angular_x_motion: ConfigurableJointMotion,
        angular_y_motion: ConfigurableJointMotion,
        angular_z_motion: ConfigurableJointMotion,
    ) -> Self {
        self.angular_x_motion = angular_x_motion;
        self.angular_y_motion = angular_y_motion;
        self.angular_z_motion = angular_z_motion;
        self
    }

    pub const fn with_linear_limit(mut self, linear_limit: f64) -> Self {
        self.linear_limit = linear_limit;
        self
    }

    pub const fn with_angular_limits(
        mut self,
        x_low: f64,
        x_high: f64,
        y_limit: f64,
        z_limit: f64,
    ) -> Self {
        self.angular_x_limit_low = x_low;
        self.angular_x_limit_high = x_high;
        self.angular_y_limit = y_limit;
        self.angular_z_limit = z_limit;
        self
    }

    pub const fn with_x_drive(mut self, drive: JointDrive3D) -> Self {
        self.x_drive = drive;
        self
    }

    pub const fn with_y_drive(mut self, drive: JointDrive3D) -> Self {
        self.y_drive = drive;
        self
    }

    pub const fn with_z_drive(mut self, drive: JointDrive3D) -> Self {
        self.z_drive = drive;
        self
    }

    pub const fn with_target_position(mut self, target_position: [f64; 3]) -> Self {
        self.target_position = target_position;
        self
    }

    pub const fn with_target_velocity(mut self, target_velocity: [f64; 3]) -> Self {
        self.target_velocity = target_velocity;
        self
    }

    pub const fn with_break_force(mut self, break_force: f64) -> Self {
        self.break_force = break_force;
        self
    }

    pub const fn with_break_torque(mut self, break_torque: f64) -> Self {
        self.break_torque = break_torque;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Joint3D {
    Fixed(FixedJoint3D),
    Hinge(HingeJoint3D),
    Spring(SpringJoint3D),
    Character(CharacterJoint3D),
    Configurable(ConfigurableJoint3D),
}

impl Joint3D {
    pub fn body_a(&self) -> EntityId {
        match self {
            Self::Fixed(j) => j.body_a,
            Self::Hinge(j) => j.body_a,
            Self::Spring(j) => j.body_a,
            Self::Character(j) => j.body_a,
            Self::Configurable(j) => j.body_a,
        }
    }

    pub fn body_b(&self) -> Option<EntityId> {
        match self {
            Self::Fixed(j) => j.body_b,
            Self::Hinge(j) => j.body_b,
            Self::Spring(j) => j.body_b,
            Self::Character(j) => j.body_b,
            Self::Configurable(j) => j.body_b,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrokenJointEvent3D {
    pub joint_id: JointId3D,
    pub entity: EntityId,
    pub break_force: f64,
}
