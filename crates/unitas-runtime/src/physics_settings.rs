use unitas_asset::project_settings::{PhysicsProjectSettings, PhysicsSolverType};
use unitas_asset::scene::{
    BounceCombine as SceneBounceCombine, FrictionCombine as SceneFrictionCombine, MaterialValues,
    RigidbodyCollisionDetection, RigidbodyInterpolation,
};
use unitas_physics::matrix::CollisionMatrix;
use unitas_physics::{
    BodyInterpolation, BounceCombine, CollisionDetectionMode3D, FrictionCombine, PhysicsMaterial3D,
    SolverSettings, SolverType,
};

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
    let mut matrix = CollisionMatrix::new();
    for layer in 0..32 {
        for other in 0..32 {
            let enabled = settings.layer_collision_matrix[layer] & (1u32 << other) != 0;
            matrix.set_enabled(layer, other, enabled);
        }
    }
    matrix
}
