use std::collections::HashMap;
use std::f64::consts::PI;

use unitas_core::EntityId;

use crate::body::{
    add, dot, inverse_rotation, multiply_rotation, rotate_vector, subtract, to_local,
    vector_length, BodyRole,
};
use crate::world::World;

const DEFAULT_STIFFNESS: f64 = 100.0;
const DEFAULT_DAMPING: f64 = 20.0;
const AXIS_EPSILON: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ArticulationError {
    InvalidJoint,
    InvalidDeltaTime(f64),
    MissingBody(EntityId),
    UnsupportedBodyRole(EntityId),
}

impl std::fmt::Display for ArticulationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJoint => {
                write!(formatter, "the articulation joint configuration is invalid")
            }
            Self::InvalidDeltaTime(delta_seconds) => write!(
                formatter,
                "delta seconds {delta_seconds} is not a finite positive number"
            ),
            Self::MissingBody(entity) => {
                write!(formatter, "entity {entity} has no body in the world")
            }
            Self::UnsupportedBodyRole(entity) => write!(
                formatter,
                "entity {entity} cannot be driven by a joint because it is not kinematic"
            ),
        }
    }
}

impl std::error::Error for ArticulationError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JointKind {
    Revolute,
    Prismatic,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointLimits {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointDrive {
    pub target: f64,
    pub stiffness: f64,
    pub damping: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArticulationJoint {
    pub parent: EntityId,
    pub child: EntityId,
    pub kind: JointKind,
    pub anchor: [f64; 3],
    pub child_anchor: [f64; 3],
    pub axis: [f64; 3],
    pub limits: JointLimits,
    pub drive: JointDrive,
}

impl ArticulationJoint {
    pub fn revolute(parent: EntityId, child: EntityId) -> Self {
        Self {
            parent,
            child,
            kind: JointKind::Revolute,
            anchor: [0.0; 3],
            child_anchor: [0.0; 3],
            axis: [0.0, 0.0, 1.0],
            limits: JointLimits {
                lower: -PI,
                upper: PI,
            },
            drive: JointDrive {
                target: 0.0,
                stiffness: DEFAULT_STIFFNESS,
                damping: DEFAULT_DAMPING,
            },
        }
    }

    pub fn prismatic(parent: EntityId, child: EntityId) -> Self {
        Self {
            parent,
            child,
            kind: JointKind::Prismatic,
            anchor: [0.0; 3],
            child_anchor: [0.0; 3],
            axis: [1.0, 0.0, 0.0],
            limits: JointLimits {
                lower: -PI,
                upper: PI,
            },
            drive: JointDrive {
                target: 0.0,
                stiffness: DEFAULT_STIFFNESS,
                damping: DEFAULT_DAMPING,
            },
        }
    }

    pub fn with_anchor(mut self, anchor: [f64; 3]) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn with_child_anchor(mut self, child_anchor: [f64; 3]) -> Self {
        self.child_anchor = child_anchor;
        self
    }

    pub fn with_axis(mut self, axis: [f64; 3]) -> Self {
        self.axis = axis;
        self
    }

    pub fn with_limits(mut self, lower: f64, upper: f64) -> Self {
        self.limits = JointLimits { lower, upper };
        self
    }

    pub fn with_target(mut self, target: f64) -> Self {
        self.drive.target = target;
        self
    }

    pub fn with_drive(mut self, stiffness: f64, damping: f64) -> Self {
        self.drive.stiffness = stiffness;
        self.drive.damping = damping;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointState {
    pub value: f64,
    pub velocity: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct JointRuntime {
    value: f64,
    velocity: f64,
    rest_relative: [f64; 4],
    rest_angle: f64,
    anchor_rest: [f64; 3],
    perpendicular: [f64; 3],
}

pub struct ArticulationBody {
    joints: Vec<ArticulationJoint>,
    runtime: Vec<JointRuntime>,
    order: Vec<usize>,
    initialized: bool,
}

impl ArticulationBody {
    pub fn new(joints: Vec<ArticulationJoint>) -> Result<Self, ArticulationError> {
        for joint in &joints {
            if joint.parent == joint.child {
                return Err(ArticulationError::InvalidJoint);
            }
            if !joint.anchor.iter().all(|value| value.is_finite())
                || !joint.child_anchor.iter().all(|value| value.is_finite())
                || !joint.axis.iter().all(|value| value.is_finite())
            {
                return Err(ArticulationError::InvalidJoint);
            }
            if vector_length(joint.axis) <= AXIS_EPSILON {
                return Err(ArticulationError::InvalidJoint);
            }
            if !joint.limits.lower.is_finite()
                || !joint.limits.upper.is_finite()
                || joint.limits.lower > joint.limits.upper
            {
                return Err(ArticulationError::InvalidJoint);
            }
            if !joint.drive.target.is_finite()
                || !joint.drive.stiffness.is_finite()
                || !joint.drive.damping.is_finite()
            {
                return Err(ArticulationError::InvalidJoint);
            }
        }
        let order = dependency_order(&joints)?;
        let runtime = vec![
            JointRuntime {
                value: 0.0,
                velocity: 0.0,
                rest_relative: [0.0, 0.0, 0.0, 1.0],
                rest_angle: 0.0,
                anchor_rest: [0.0; 3],
                perpendicular: [0.0; 3],
            };
            joints.len()
        ];
        Ok(Self {
            joints,
            runtime,
            order,
            initialized: false,
        })
    }

    pub fn joints(&self) -> &[ArticulationJoint] {
        &self.joints
    }

    pub fn joint_state(&self, index: usize) -> Option<JointState> {
        self.runtime.get(index).map(|runtime| JointState {
            value: runtime.value,
            velocity: runtime.velocity,
        })
    }

    pub fn step(&mut self, world: &mut World, delta_seconds: f64) -> Result<(), ArticulationError> {
        if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return Err(ArticulationError::InvalidDeltaTime(delta_seconds));
        }
        if !self.initialized {
            self.initialize(world)?;
            self.initialized = true;
        }
        for index in self.order.clone() {
            self.drive_joint(world, index, delta_seconds)?;
        }
        Ok(())
    }

    fn initialize(&mut self, world: &World) -> Result<(), ArticulationError> {
        for index in self.order.clone() {
            let joint = self.joints[index];
            let parent_position = world
                .position(joint.parent)
                .ok_or(ArticulationError::MissingBody(joint.parent))?;
            let parent_rotation = world
                .rotation(joint.parent)
                .ok_or(ArticulationError::MissingBody(joint.parent))?;
            let child_position = world
                .position(joint.child)
                .ok_or(ArticulationError::MissingBody(joint.child))?;
            let child_rotation = world
                .rotation(joint.child)
                .ok_or(ArticulationError::MissingBody(joint.child))?;
            let axis = normalized_axis(joint.axis);
            let relative = multiply_rotation(inverse_rotation(parent_rotation), child_rotation);
            let child_anchor_world = add(
                child_position,
                rotate_vector(child_rotation, joint.child_anchor),
            );
            let child_anchor_local = to_local(parent_rotation, parent_position, child_anchor_world);
            let offset = subtract(child_anchor_local, joint.anchor);
            let (value, perpendicular) = match joint.kind {
                JointKind::Revolute => (revolute_angle(relative, axis), [0.0; 3]),
                JointKind::Prismatic => {
                    let along = dot(offset, axis);
                    (along, subtract(offset, scale(axis, along)))
                }
            };
            self.runtime[index] = JointRuntime {
                value,
                velocity: 0.0,
                rest_relative: relative,
                rest_angle: revolute_angle(relative, axis),
                anchor_rest: child_anchor_local,
                perpendicular,
            };
        }
        Ok(())
    }

    fn drive_joint(
        &mut self,
        world: &mut World,
        index: usize,
        delta_seconds: f64,
    ) -> Result<(), ArticulationError> {
        let joint = self.joints[index];
        let runtime = self.runtime[index];
        let role = world
            .body_role(joint.child)
            .ok_or(ArticulationError::MissingBody(joint.child))?;
        if role == BodyRole::Dynamic {
            return Err(ArticulationError::UnsupportedBodyRole(joint.child));
        }
        let parent_position = world
            .position(joint.parent)
            .ok_or(ArticulationError::MissingBody(joint.parent))?;
        let parent_rotation = world
            .rotation(joint.parent)
            .ok_or(ArticulationError::MissingBody(joint.parent))?;

        let drive = joint.drive;
        let error = drive.target - runtime.value;
        let acceleration = drive.stiffness * error - drive.damping * runtime.velocity;
        let mut velocity = runtime.velocity + acceleration * delta_seconds;
        let mut value = runtime.value + velocity * delta_seconds;
        if value < joint.limits.lower {
            value = joint.limits.lower;
            velocity = 0.0;
        } else if value > joint.limits.upper {
            value = joint.limits.upper;
            velocity = 0.0;
        }

        let axis = normalized_axis(joint.axis);
        let (relative_rotation, child_anchor_local) = match joint.kind {
            JointKind::Revolute => {
                let turn = axis_angle(axis, value - runtime.rest_angle);
                (
                    multiply_rotation(turn, runtime.rest_relative),
                    add(
                        joint.anchor,
                        rotate_vector(turn, subtract(runtime.anchor_rest, joint.anchor)),
                    ),
                )
            }
            JointKind::Prismatic => (
                runtime.rest_relative,
                add(add(joint.anchor, scale(axis, value)), runtime.perpendicular),
            ),
        };
        let child_rotation = multiply_rotation(parent_rotation, relative_rotation);
        let child_anchor_world = add(
            parent_position,
            rotate_vector(parent_rotation, child_anchor_local),
        );
        let child_position = subtract(
            child_anchor_world,
            rotate_vector(child_rotation, joint.child_anchor),
        );
        world
            .set_position(joint.child, child_position)
            .map_err(|_| ArticulationError::MissingBody(joint.child))?;
        world
            .set_rotation(joint.child, child_rotation)
            .map_err(|_| ArticulationError::MissingBody(joint.child))?;
        self.runtime[index].value = value;
        self.runtime[index].velocity = velocity;
        Ok(())
    }
}

fn dependency_order(joints: &[ArticulationJoint]) -> Result<Vec<usize>, ArticulationError> {
    let mut child_joint: HashMap<EntityId, usize> = HashMap::new();
    for (index, joint) in joints.iter().enumerate() {
        if child_joint.insert(joint.child, index).is_some() {
            return Err(ArticulationError::InvalidJoint);
        }
    }
    let mut depths = vec![0usize; joints.len()];
    for (index, depth) in depths.iter_mut().enumerate() {
        let mut current = index;
        let mut steps = 0usize;
        while let Some(parent_joint) = child_joint.get(&joints[current].parent) {
            current = *parent_joint;
            steps += 1;
            if steps > joints.len() {
                return Err(ArticulationError::InvalidJoint);
            }
        }
        *depth = steps;
    }
    let mut order: Vec<usize> = (0..joints.len()).collect();
    order.sort_by_key(|index| depths[*index]);
    Ok(order)
}

fn normalized_axis(axis: [f64; 3]) -> [f64; 3] {
    let length = vector_length(axis);
    if length <= AXIS_EPSILON {
        return [0.0; 3];
    }
    scale(axis, 1.0 / length)
}

fn axis_angle(axis: [f64; 3], angle: f64) -> [f64; 4] {
    let half = angle * 0.5;
    let sine = half.sin();
    [axis[0] * sine, axis[1] * sine, axis[2] * sine, half.cos()]
}

fn revolute_angle(relative: [f64; 4], axis: [f64; 3]) -> f64 {
    let sine = dot([relative[0], relative[1], relative[2]], axis);
    2.0 * sine.atan2(relative[3])
}

fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}
