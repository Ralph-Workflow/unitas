use unitas_core::EntityId;
use unitas_physics::two_d::{AreaEffector2D, BuoyancyEffector2D, PointEffector2D};

use crate::abi::with_runtime_state;

ffi_export! {
pub extern "C" fn unitas_runtime_collider2d_set_area_effector(
    handle: u64,
    entity: u64,
    force_angle: f32,
    force_magnitude: f32,
    force_variation: f32,
    drag: f32,
    angular_drag: f32,
    use_global_angle: u32,
    collider_mask: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let entity = EntityId::new(entity);
        let colliders = state.runtime.physics_mut().collider_ids_2d(entity).to_vec();
        let mut applied = false;
        for collider in colliders {
            let effector = AreaEffector2D {
                force_angle,
                force_magnitude,
                force_variation,
                drag,
                angular_drag,
                use_global_angle: use_global_angle != 0,
                collider_mask,
            };
            applied |= state
                .runtime
                .physics_mut()
                .set_area_effector_2d(collider, effector);
        }
        if !applied {
            state.set_error("collider not found");
            return -4;
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_collider2d_set_point_effector(
    handle: u64,
    entity: u64,
    force_magnitude: f32,
    force_variation: f32,
    distance_scale: f32,
    drag: f32,
    angular_drag: f32,
    collider_mask: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let entity = EntityId::new(entity);
        let colliders = state.runtime.physics_mut().collider_ids_2d(entity).to_vec();
        let mut applied = false;
        for collider in colliders {
            let effector = PointEffector2D {
                force_magnitude,
                force_variation,
                distance_scale,
                drag,
                angular_drag,
                collider_mask,
            };
            applied |= state
                .runtime
                .physics_mut()
                .set_point_effector_2d(collider, effector);
        }
        if !applied {
            state.set_error("collider not found");
            return -4;
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_collider2d_set_buoyancy_effector(
    handle: u64,
    entity: u64,
    surface_level: f32,
    density: f32,
    linear_drag: f32,
    angular_drag: f32,
    flow_angle: f32,
    flow_magnitude: f32,
    flow_variation: f32,
    collider_mask: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let entity = EntityId::new(entity);
        let colliders = state.runtime.physics_mut().collider_ids_2d(entity).to_vec();
        let mut applied = false;
        for collider in colliders {
            let effector = BuoyancyEffector2D {
                surface_level,
                density,
                linear_drag,
                angular_drag,
                flow_angle,
                flow_magnitude,
                flow_variation,
                collider_mask,
            };
            applied |= state
                .runtime
                .physics_mut()
                .set_buoyancy_effector_2d(collider, effector);
        }
        if !applied {
            state.set_error("collider not found");
            return -4;
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
