use unitas_core::EntityId;
use unitas_physics::two_d::body::{
    BodyDesc2D, BodyId2D, BodyType2D, DistanceJoint2D, FixedJoint2D, FrictionJoint2D, HingeJoint2D,
    Joint2D, JointLimits2D, JointMotor2D, RelativeJoint2D, SliderJoint2D, SpringJoint2D,
    TargetJoint2D, WheelJoint2D,
};

use crate::abi::with_runtime_state;
use crate::runtime::Runtime;
use crate::unitas_runtime_path_buffer;

macro_rules! joint2d_ffi_export {
    (pub extern "C" fn $name:ident($($arg:ident : $arg_ty:ty),* $(,)?) $(-> $ret:ty)? $body:block) => {
        #[unsafe(export_name = stringify!($name))]
        pub extern "C" fn $name($($arg : $arg_ty),*) $(-> $ret)? {
            #[used]
            static KEEP: extern "C" fn($($arg_ty),*) $(-> $ret)? = $name;
            $body
        }
    };
}

fn ensure_body_2d_exists(runtime: &mut Runtime, entity: EntityId, is_dynamic: bool) -> BodyId2D {
    if let Some(id) = runtime.physics.body_id_2d(entity) {
        return id;
    }
    let tf = runtime.entity_transform(entity);
    let pos = tf.map_or([0.0, 0.0], |t| [t.translation[0], t.translation[1]]);
    let desc = if is_dynamic {
        BodyDesc2D::dynamic(entity, pos)
    } else {
        BodyDesc2D::static_body(entity, pos)
    };
    runtime.physics_mut().add_body_2d(desc)
}

fn orient_bodies_2d(
    runtime: &mut Runtime,
    owner: EntityId,
    connected_id: u64,
    anchor: [f32; 2],
    conn_anchor: [f32; 2],
) -> (BodyId2D, Option<BodyId2D>, [f32; 2], [f32; 2]) {
    let owner_bid = ensure_body_2d_exists(runtime, owner, true);
    if connected_id == 0 || connected_id == owner.as_u64() {
        let world_anchor = runtime.physics_mut().ensure_world_anchor_2d();
        return (world_anchor, Some(owner_bid), conn_anchor, anchor);
    }
    let conn_entity = EntityId::new(connected_id);
    let conn_bid = ensure_body_2d_exists(runtime, conn_entity, false);
    let owner_dyn = runtime
        .physics
        .world_2d()
        .body(owner_bid)
        .is_some_and(|b| b.body_type == BodyType2D::Dynamic);
    let conn_dyn = runtime
        .physics
        .world_2d()
        .body(conn_bid)
        .is_some_and(|b| b.body_type == BodyType2D::Dynamic);
    if !owner_dyn && conn_dyn {
        (owner_bid, Some(conn_bid), anchor, conn_anchor)
    } else {
        (conn_bid, Some(owner_bid), conn_anchor, anchor)
    }
}

pub fn apply_joint_2d_params(runtime: &mut Runtime, entity: EntityId, v: &[f64]) -> bool {
    if v.len() < 24 {
        return false;
    }
    let kind = v[0] as u32;
    let connected_id = v[1] as u64;
    let anchor = [v[2] as f32, v[3] as f32];
    let conn_anchor = [v[4] as f32, v[5] as f32];
    let (body_a, body_b, anchor_a, anchor_b) =
        orient_bodies_2d(runtime, entity, connected_id, anchor, conn_anchor);
    let bf = if v[22] <= 0.0 {
        f32::INFINITY
    } else {
        v[22] as f32
    };
    let bt = if v[23] <= 0.0 {
        f32::INFINITY
    } else {
        v[23] as f32
    };
    let Some(joint) =
        build_joint_2d_from_params(kind, (body_a, body_b), (anchor_a, anchor_b), v, (bf, bt))
    else {
        return false;
    };
    runtime.physics_mut().set_joint_2d(entity, kind, joint);
    true
}

fn build_joint_2d_from_params(
    kind: u32,
    bodies: (BodyId2D, Option<BodyId2D>),
    anchors: ([f32; 2], [f32; 2]),
    v: &[f64],
    breaks: (f32, f32),
) -> Option<Joint2D> {
    if kind <= 4 {
        return build_joint_2d_primary(kind, bodies, anchors, v, breaks);
    }
    build_joint_2d_secondary(kind, bodies, anchors, v, breaks)
}

fn build_joint_2d_primary(
    kind: u32,
    bodies: (BodyId2D, Option<BodyId2D>),
    anchors: ([f32; 2], [f32; 2]),
    v: &[f64],
    breaks: (f32, f32),
) -> Option<Joint2D> {
    let (body_a, body_b) = bodies;
    let (anchor_a, anchor_b) = anchors;
    let (bf, bt) = breaks;
    match kind {
        0 => Some(Joint2D::Distance(DistanceJoint2D {
            body_a,
            body_b,
            anchor_a,
            anchor_b,
            distance: v[6] as f32,
            max_distance_only: v[7] != 0.0,
            break_force: bf,
        })),
        1 => Some(Joint2D::Fixed(FixedJoint2D {
            body_a,
            body_b,
            local_offset: anchor_a,
            reference_angle: v[10] as f32,
            damping_ratio: v[8] as f32,
            frequency: v[9] as f32,
            break_force: bf,
            break_torque: bt,
        })),
        2 => Some(Joint2D::Friction(FrictionJoint2D {
            body_a,
            body_b,
            max_force: v[11] as f32,
            max_torque: v[12] as f32,
            break_force: bf,
            break_torque: bt,
        })),
        3 => Some(Joint2D::Hinge(HingeJoint2D {
            body_a,
            body_b,
            anchor_a,
            anchor_b,
            use_motor: v[15] != 0.0,
            motor: JointMotor2D {
                motor_speed: v[16] as f32,
                max_motor_torque: v[17] as f32,
            },
            use_limits: v[18] != 0.0,
            limits: JointLimits2D {
                min: v[19] as f32,
                max: v[20] as f32,
            },
            break_force: bf,
            break_torque: bt,
        })),
        4 => Some(Joint2D::Relative(RelativeJoint2D {
            body_a,
            body_b,
            linear_offset: [v[6] as f32, v[7] as f32],
            angular_offset: v[10] as f32,
            max_force: v[11] as f32,
            max_torque: v[12] as f32,
            correction_scale: v[13] as f32,
            break_force: bf,
            break_torque: bt,
        })),
        _ => None,
    }
}

fn build_joint_2d_secondary(
    kind: u32,
    bodies: (BodyId2D, Option<BodyId2D>),
    anchors: ([f32; 2], [f32; 2]),
    v: &[f64],
    breaks: (f32, f32),
) -> Option<Joint2D> {
    let (body_a, body_b) = bodies;
    let (anchor_a, anchor_b) = anchors;
    let (bf, bt) = breaks;
    let motor = JointMotor2D {
        motor_speed: v[16] as f32,
        max_motor_torque: v[17] as f32,
    };
    let limits = JointLimits2D {
        min: v[19] as f32,
        max: v[20] as f32,
    };
    match kind {
        5 => Some(Joint2D::Slider(SliderJoint2D {
            body_a,
            body_b,
            anchor_a,
            anchor_b,
            angle_degrees: v[14] as f32,
            use_motor: v[15] != 0.0,
            motor,
            use_limits: v[18] != 0.0,
            limits,
            break_force: bf,
            break_torque: bt,
        })),
        6 => Some(Joint2D::Spring(SpringJoint2D {
            body_a,
            body_b,
            anchor_a,
            anchor_b,
            distance: v[6] as f32,
            damping_ratio: v[8] as f32,
            frequency: v[9] as f32,
            break_force: bf,
        })),
        7 => Some(Joint2D::Target(TargetJoint2D {
            body: body_b.unwrap_or(body_a),
            anchor: anchor_b,
            target: [v[6] as f32, v[7] as f32],
            max_force: v[11] as f32,
            frequency: v[9] as f32,
            damping_ratio: v[8] as f32,
            break_force: bf,
        })),
        8 => Some(Joint2D::Wheel(WheelJoint2D {
            body_a,
            body_b,
            anchor_a,
            anchor_b,
            suspension_angle_degrees: v[14] as f32,
            suspension_frequency: v[9] as f32,
            suspension_damping_ratio: v[8] as f32,
            use_motor: v[15] != 0.0,
            motor,
            break_force: bf,
            break_torque: bt,
        })),
        _ => None,
    }
}

joint2d_ffi_export! {
pub extern "C" fn unitas_runtime_joint2d_set(handle: u64, entity: u64, in_buffer: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    let Some(vals) = super::read_f64_slice(handle, in_buffer, 24) else {
        return -2;
    };
    with_runtime_state(handle, |state| {
        if !apply_joint_2d_params(&mut state.runtime, EntityId::new(entity), &vals) {
            state.set_error("invalid 2D joint configuration");
            return -4;
        }
        state.clear_error();
        0
    })
    .unwrap_or_else(|code| code)
}
}

joint2d_ffi_export! {
pub extern "C" fn unitas_runtime_joint2d_remove(handle: u64, entity: u64, kind: u32) -> i32 {
    if entity == 0 {
        return -2;
    }
    with_runtime_state(handle, |state| {
        state
            .runtime
            .physics_mut()
            .remove_joint_2d(EntityId::new(entity), kind);
        state.clear_error();
        0
    })
    .unwrap_or_else(|code| code)
}
}

joint2d_ffi_export! {
pub extern "C" fn unitas_runtime_body2d_position(handle: u64, entity: u64, out_buffer: u64) -> i32 {
    let expected = unitas_runtime_path_buffer(handle, 12);
    if entity == 0 || expected == 0 || out_buffer != expected {
        return -2;
    }
    with_runtime_state(handle, |state| {
        let Some(bid) = state.runtime.physics.body_id_2d(EntityId::new(entity)) else {
            state.set_error("2d body not found");
            return -4;
        };
        let Some(body) = state.runtime.physics.world_2d().body(bid) else {
            state.set_error("2d body not found");
            return -4;
        };
        state.path_buffer[0..4].copy_from_slice(&body.position[0].to_le_bytes());
        state.path_buffer[4..8].copy_from_slice(&body.position[1].to_le_bytes());
        state.path_buffer[8..12].copy_from_slice(&body.rotation.to_le_bytes());
        state.clear_error();
        0
    })
    .unwrap_or_else(|code| code)
}
}
