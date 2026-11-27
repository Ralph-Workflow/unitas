use unitas_asset::project_settings::{
    Physics2DProjectSettings, PhysicsProjectSettings, PhysicsSolverType,
};
use unitas_asset::scene::{
    BounceCombine as SceneBounceCombine, FrictionCombine as SceneFrictionCombine, MaterialValues,
    RigidbodyCollisionDetection, RigidbodyInterpolation,
};
use unitas_physics::matrix::CollisionMatrix;
use unitas_physics::two_d::World2D;
use unitas_physics::{
    BodyInterpolation, BounceCombine, CollisionDetectionMode3D, FrictionCombine, PhysicsMaterial3D,
    SolverSettings, SolverType,
};

use crate::abi::RuntimeState;

pub(crate) fn material_of(values: &MaterialValues) -> PhysicsMaterial3D {
    PhysicsMaterial3D {
        friction: values.friction,
        restitution: values.restitution,
        friction_combine: friction_combine_of(values.friction_combine),
        restitution_combine: bounce_combine_of(values.bounce_combine),
    }
}

pub(crate) fn friction_combine_of(value: SceneFrictionCombine) -> FrictionCombine {
    match value {
        SceneFrictionCombine::Average => FrictionCombine::Average,
        SceneFrictionCombine::Minimum => FrictionCombine::Minimum,
        SceneFrictionCombine::Multiply => FrictionCombine::Multiply,
        SceneFrictionCombine::Maximum => FrictionCombine::Maximum,
    }
}

pub(crate) fn bounce_combine_of(value: SceneBounceCombine) -> BounceCombine {
    match value {
        SceneBounceCombine::Average => BounceCombine::Average,
        SceneBounceCombine::Minimum => BounceCombine::Minimum,
        SceneBounceCombine::Maximum => BounceCombine::Maximum,
        SceneBounceCombine::Multiply => BounceCombine::Multiply,
    }
}

pub(crate) fn interpolation_of(value: RigidbodyInterpolation) -> BodyInterpolation {
    match value {
        RigidbodyInterpolation::None => BodyInterpolation::None,
        RigidbodyInterpolation::Interpolate => BodyInterpolation::Interpolate,
        RigidbodyInterpolation::Extrapolate => BodyInterpolation::Extrapolate,
    }
}

pub(crate) fn collision_detection_of(
    value: RigidbodyCollisionDetection,
) -> CollisionDetectionMode3D {
    match value {
        RigidbodyCollisionDetection::Discrete => CollisionDetectionMode3D::Discrete,
        RigidbodyCollisionDetection::Continuous => CollisionDetectionMode3D::Continuous,
        RigidbodyCollisionDetection::ContinuousDynamic => {
            CollisionDetectionMode3D::ContinuousDynamic
        }
        RigidbodyCollisionDetection::ContinuousSpeculative => {
            CollisionDetectionMode3D::ContinuousSpeculative
        }
    }
}

pub(crate) fn solver_type_of(value: PhysicsSolverType) -> SolverType {
    match value {
        PhysicsSolverType::Default => SolverType::Default,
        PhysicsSolverType::Tgs => SolverType::Tgs,
        PhysicsSolverType::TgsSoft => SolverType::TgsSoft,
    }
}

pub(crate) fn solver_settings_of(settings: &PhysicsProjectSettings) -> SolverSettings {
    let defaults = SolverSettings::default();
    SolverSettings {
        num_solver_iterations: settings.default_solver_iterations.max(1) as usize,
        num_internal_pgs_iterations: settings.default_solver_velocity_iterations.max(1) as usize,
        num_additional_friction_iterations: defaults.num_additional_friction_iterations,
        contact_offset: settings.default_contact_offset,
        allowed_linear_error: defaults.allowed_linear_error,
        max_depenetration_velocity: defaults.max_depenetration_velocity,
        deterministic_accumulation: settings.enhanced_determinism,
        solver_type: solver_type_of(settings.solver_type),
    }
}

pub(crate) fn layer_matrix_of(settings: &PhysicsProjectSettings) -> CollisionMatrix {
    layer_matrix_from_bits(&settings.layer_collision_matrix)
}

pub(crate) fn layer_matrix_2d_of(settings: &Physics2DProjectSettings) -> CollisionMatrix {
    layer_matrix_from_bits(&settings.layer_collision_matrix)
}

fn layer_matrix_from_bits(bits: &[u32; 32]) -> CollisionMatrix {
    let mut matrix = CollisionMatrix::new();
    for (layer, &mask) in bits.iter().enumerate() {
        for other in 0..32 {
            let enabled = mask & (1u32 << other) != 0;
            matrix.set_enabled(layer, other, enabled);
        }
    }
    matrix
}

pub(crate) fn apply_physics2d_project_settings(
    world: &mut World2D,
    settings: &Physics2DProjectSettings,
) {
    world.set_gravity([settings.gravity[0] as f32, settings.gravity[1] as f32]);
    world.set_velocity_iterations(settings.velocity_iterations.max(1) as usize);
    world.set_position_iterations(settings.position_iterations.max(1) as usize);
    world.set_queries_hit_triggers(settings.queries_hit_triggers);
    world.set_queries_start_in_colliders(settings.queries_start_in_colliders);
    world.set_callbacks_on_disable(settings.callbacks_on_disable);
    world.set_auto_sync_transforms(settings.auto_sync_transforms);
    world.set_collision_matrix(layer_matrix_2d_of(settings));
}

pub(crate) fn apply_physics2d_runtime_settings(
    state: &mut RuntimeState,
    gravity_x: f32,
    gravity_y: f32,
    velocity_iterations: u32,
    position_iterations: u32,
    queries_hit_triggers: u32,
) -> i32 {
    if !gravity_x.is_finite() || !gravity_y.is_finite() {
        state.set_error("invalid 2D gravity values");
        return -2;
    }
    let world = state.runtime.physics_mut().world_2d_mut();
    world.set_gravity([gravity_x, gravity_y]);
    world.set_velocity_iterations(velocity_iterations.max(1) as usize);
    world.set_position_iterations(position_iterations.max(1) as usize);
    world.set_queries_hit_triggers(queries_hit_triggers != 0);
    state.clear_error();
    0
}

pub(crate) fn set_physics2d_layer_collision(
    state: &mut RuntimeState,
    layer_a: u32,
    layer_b: u32,
    enabled: u32,
) -> i32 {
    let (Ok(a), Ok(b)) = (u8::try_from(layer_a), u8::try_from(layer_b)) else {
        state.set_error("layer index out of range");
        return -2;
    };
    if a >= 32 || b >= 32 {
        state.set_error("layer index out of range");
        return -2;
    }
    state
        .runtime
        .physics_mut()
        .ignore_layer_collision_2d(a, b, enabled == 0);
    state.clear_error();
    0
}

pub(crate) fn get_physics2d_layer_collision(
    state: &mut RuntimeState,
    layer_a: u32,
    layer_b: u32,
) -> i32 {
    let (Ok(a), Ok(b)) = (u8::try_from(layer_a), u8::try_from(layer_b)) else {
        state.set_error("layer index out of range");
        return -2;
    };
    if a >= 32 || b >= 32 {
        state.set_error("layer index out of range");
        return -2;
    }
    let ignored = state
        .runtime
        .physics_mut()
        .world_2d()
        .get_ignore_layer_collision(a, b);
    state.clear_error();
    i32::from(!ignored)
}
