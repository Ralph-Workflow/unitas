#[path = "abi_joints2d.rs"]
pub mod abi_joints2d;
pub use abi_joints2d::*;

use unitas_core::EntityId;
use unitas_physics::{
    BodyDesc, BodyRole, CharacterJoint3D, ColliderShape, CollisionDetectionMode3D,
    ConfigurableJoint3D, ConfigurableJointMotion, FixedJoint3D, HingeJoint3D, Joint3D,
    JointDrive3D, JointLimits3D, JointMotor3D, JointSpring3D, PhysicsError, Shape, SpringJoint3D,
};

use crate::physics::PhysicsRuntimeWorld;
use crate::runtime::Runtime;
use crate::unitas_runtime_path_buffer;

macro_rules! joint_ffi_export {
    (pub extern "C" fn $name:ident($($arg:ident : $arg_ty:ty),* $(,)?) $(-> $ret:ty)? $body:block) => {
        #[unsafe(export_name = stringify!($name))]
        pub extern "C" fn $name($($arg : $arg_ty),*) $(-> $ret)? {
            #[used]
            static KEEP: extern "C" fn($($arg_ty),*) $(-> $ret)? = $name;
            $body
        }
    };
}

pub(crate) fn read_f64_slice(handle: u64, in_buffer: u64, count: usize) -> Option<Vec<f64>> {
    let byte_len = count.checked_mul(8)?;
    let expected = unitas_runtime_path_buffer(handle, byte_len as u64);
    if expected == 0 || in_buffer != expected {
        return None;
    }
    super::with_runtime_state(handle, |state| {
        if state.path_buffer.len() < byte_len {
            return None;
        }
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            let start = i * 8;
            let bytes: [u8; 8] = state.path_buffer.get(start..start + 8)?.try_into().ok()?;
            out.push(f64::from_le_bytes(bytes));
        }
        Some(out)
    })
    .ok()
    .flatten()
}

fn ensure_body_3d_exists(runtime: &mut Runtime, entity: EntityId, is_dynamic: bool) {
    let desired_role = if is_dynamic {
        BodyRole::Dynamic
    } else {
        BodyRole::Static
    };
    let current_role = runtime.physics.world_3d().body_role(entity);
    if current_role == Some(desired_role) || (current_role.is_some() && !is_dynamic) {
        return;
    }
    let tf = runtime.entity_transform(entity);
    let pos = tf.as_ref().map_or([0.0; 3], |t| {
        [
            f64::from(t.translation[0]),
            f64::from(t.translation[1]),
            f64::from(t.translation[2]),
        ]
    });
    let rot = tf.as_ref().map_or([0.0, 0.0, 0.0, 1.0], |t| {
        [
            f64::from(t.rotation[0]),
            f64::from(t.rotation[1]),
            f64::from(t.rotation[2]),
            f64::from(t.rotation[3]),
        ]
    });
    runtime.physics_mut().world_3d_mut().remove_body(entity);
    let desc = BodyDesc {
        position: pos,
        rotation: rot,
        mass: if is_dynamic { 1.0 } else { 0.0 },
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: true,
        collider_shape: ColliderShape::Primitive(Shape::Sphere { radius: 0.25 }),
        layer: 1,
        role: desired_role,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: CollisionDetectionMode3D::Discrete,
    };
    let _ = runtime
        .physics_mut()
        .world_3d_mut()
        .add_body_with_default_material(entity, desc);
    if is_dynamic {
        register_dynamic_scene_body(runtime, entity);
    }
}

fn register_dynamic_scene_body(runtime: &mut Runtime, entity: EntityId) {
    for scene in &mut runtime.scenes {
        if scene.graph.contains(entity) {
            scene.rigid_bodies.insert(entity);
            if !scene.bodies.contains(&entity) {
                scene.bodies.push(entity);
            }
            if !scene.body_offsets.iter().any(|(id, _)| *id == entity) {
                scene.body_offsets.push((entity, [0.0; 3]));
            }
            break;
        }
    }
}

fn orient_bodies_3d(
    physics: &PhysicsRuntimeWorld,
    owner: EntityId,
    connected: Option<EntityId>,
    anchor: [f64; 3],
    connected_anchor: [f64; 3],
) -> (EntityId, Option<EntityId>, [f64; 3], [f64; 3]) {
    let Some(conn) = connected.filter(|&c| c != owner) else {
        return (owner, None, anchor, connected_anchor);
    };
    let owner_dyn = physics.world_3d().body_role(owner) == Some(BodyRole::Dynamic);
    let conn_dyn = physics.world_3d().body_role(conn) == Some(BodyRole::Dynamic);
    if !owner_dyn && conn_dyn {
        (owner, Some(conn), anchor, connected_anchor)
    } else {
        (conn, Some(owner), connected_anchor, anchor)
    }
}

fn decode_motion_3d(code: f64) -> ConfigurableJointMotion {
    match code as i32 {
        0 => ConfigurableJointMotion::Locked,
        1 => ConfigurableJointMotion::Limited,
        _ => ConfigurableJointMotion::Free,
    }
}

fn build_joint_3d_from_params(
    physics: &PhysicsRuntimeWorld,
    owner: EntityId,
    v: &[f64],
) -> Option<(u32, Joint3D)> {
    if v.len() < 42 {
        return None;
    }
    let kind = v[0] as u32;
    let connected = (v[1] as u64 != 0).then(|| EntityId::new(v[1] as u64));
    let (body_a, body_b, anchor_a, anchor_b) = orient_bodies_3d(
        physics,
        owner,
        connected,
        [v[2], v[3], v[4]],
        [v[5], v[6], v[7]],
    );
    let breaks = (
        if v[40] <= 0.0 { f64::INFINITY } else { v[40] },
        if v[41] <= 0.0 { f64::INFINITY } else { v[41] },
    );
    let joint = build_joint_3d_kind(
        kind,
        (body_a, body_b),
        (anchor_a, anchor_b),
        ([v[8], v[9], v[10]], [v[11], v[12], v[13]]),
        v,
        breaks,
    )?;
    Some((kind, joint))
}

fn build_joint_3d_kind(
    kind: u32,
    bodies: (EntityId, Option<EntityId>),
    anchors: ([f64; 3], [f64; 3]),
    axes: ([f64; 3], [f64; 3]),
    v: &[f64],
    breaks: (f64, f64),
) -> Option<Joint3D> {
    let (body_a, body_b) = bodies;
    let (anchor_a, anchor_b) = anchors;
    let (bf, bt) = breaks;
    match kind {
        0 => Some(Joint3D::Fixed(
            FixedJoint3D::new(body_a, body_b)
                .with_anchors(anchor_a, anchor_b)
                .with_break_force(bf)
                .with_break_torque(bt),
        )),
        1 => Some(Joint3D::Hinge(HingeJoint3D {
            body_a,
            body_b,
            anchor_a,
            anchor_b,
            axis: axes.0,
            use_limits: v[14] != 0.0,
            limits: JointLimits3D::new(v[15], v[16]),
            use_spring: v[17] != 0.0,
            spring: JointSpring3D::new(v[18], v[19], v[20]),
            use_motor: v[21] != 0.0,
            motor: JointMotor3D::new(v[22], v[23], v[24] != 0.0),
            break_force: bf,
            break_torque: bt,
        })),
        2 => Some(Joint3D::Spring(
            SpringJoint3D::new(body_a, body_b, v[18], v[19])
                .with_anchors(anchor_a, anchor_b)
                .with_distance_band(v[15], v[16])
                .with_tolerance(v[20])
                .with_break_force(bf)
                .with_break_torque(bt),
        )),
        3 => Some(Joint3D::Character(
            CharacterJoint3D::new(body_a, body_b)
                .with_anchors(anchor_a, anchor_b)
                .with_axes(axes.0, axes.1)
                .with_twist_limits(v[15], v[16])
                .with_swing_limits(v[18], v[19])
                .with_break_force(bf)
                .with_break_torque(bt),
        )),
        4 => Some(Joint3D::Configurable(build_cfg_joint_3d(
            bodies, anchors, axes, v, breaks,
        ))),
        _ => None,
    }
}

fn build_cfg_joint_3d(
    bodies: (EntityId, Option<EntityId>),
    anchors: ([f64; 3], [f64; 3]),
    axes: ([f64; 3], [f64; 3]),
    v: &[f64],
    breaks: (f64, f64),
) -> ConfigurableJoint3D {
    let mut cfg = ConfigurableJoint3D::new(bodies.0, bodies.1)
        .with_anchors(anchors.0, anchors.1)
        .with_linear_motions(
            decode_motion_3d(v[14]),
            decode_motion_3d(v[15]),
            decode_motion_3d(v[16]),
        )
        .with_angular_motions(
            decode_motion_3d(v[17]),
            decode_motion_3d(v[18]),
            decode_motion_3d(v[19]),
        )
        .with_linear_limit(v[20])
        .with_angular_limits(v[21], v[22], v[23], v[24])
        .with_x_drive(JointDrive3D::new(v[25], v[26], v[27]))
        .with_y_drive(JointDrive3D::new(v[28], v[29], v[30]))
        .with_z_drive(JointDrive3D::new(v[31], v[32], v[33]))
        .with_target_position([v[34], v[35], v[36]])
        .with_target_velocity([v[37], v[38], v[39]])
        .with_break_force(breaks.0)
        .with_break_torque(breaks.1);
    cfg.axis = axes.0;
    cfg.secondary_axis = axes.1;
    cfg
}

pub fn apply_joint_3d_params(
    runtime: &mut Runtime,
    entity: EntityId,
    v: &[f64],
) -> Result<(), PhysicsError> {
    if v.len() < 42 {
        return Err(PhysicsError::InvalidBody(entity));
    }
    ensure_body_3d_exists(runtime, entity, true);
    let conn_id = v[1] as u64;
    if conn_id != 0 {
        ensure_body_3d_exists(runtime, EntityId::new(conn_id), false);
    }
    let Some((slot, joint)) = build_joint_3d_from_params(&runtime.physics, entity, v) else {
        return Err(PhysicsError::InvalidBody(entity));
    };
    runtime.physics_mut().set_joint_3d(entity, slot, joint)?;
    Ok(())
}

joint_ffi_export! {
pub extern "C" fn unitas_runtime_joint3d_set(handle: u64, entity: u64, in_buffer: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    let Some(vals) = read_f64_slice(handle, in_buffer, 42) else {
        return -2;
    };
    super::with_runtime_state(handle, |state| {
        match apply_joint_3d_params(&mut state.runtime, EntityId::new(entity), &vals) {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(err) => {
                state.set_error(&err.to_string());
                -4
            }
        }
    })
    .unwrap_or_else(|code| code)
}
}

joint_ffi_export! {
pub extern "C" fn unitas_runtime_joint3d_remove(handle: u64, entity: u64, kind: u32) -> i32 {
    if entity == 0 {
        return -2;
    }
    super::with_runtime_state(handle, |state| {
        state
            .runtime
            .physics_mut()
            .remove_joint_3d(EntityId::new(entity), kind);
        state.clear_error();
        0
    })
    .unwrap_or_else(|code| code)
}
}

joint_ffi_export! {
pub extern "C" fn unitas_runtime_body_rotation(handle: u64, entity: u64, out_buffer: u64) -> i32 {
    let expected = unitas_runtime_path_buffer(handle, 32);
    if entity == 0 || expected == 0 || out_buffer != expected {
        return -2;
    }
    super::with_runtime_state(handle, |state| {
        let Some(rot) = state
            .runtime
            .physics
            .world_3d()
            .rotation(EntityId::new(entity))
        else {
            state.set_error("body not found");
            return -4;
        };
        for (i, val) in rot.iter().enumerate() {
            state.path_buffer[i * 8..(i + 1) * 8].copy_from_slice(&val.to_le_bytes());
        }
        state.clear_error();
        0
    })
    .unwrap_or_else(|code| code)
}
}
