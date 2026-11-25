use unitas_core::EntityId;
use unitas_physics::simulate::{Simulation, SimulationError, SimulationMode};
use unitas_physics::{BodyDesc, BodyRole, PhysicsError, Shape, World};

fn moving_box(position: [f64; 3]) -> BodyDesc {
    BodyDesc {
        role: BodyRole::Dynamic,
        shape: Shape::Box {
            half_extents: [0.5, 0.5, 0.5],
        },
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity: [2.0, 0.0, 0.0],
        use_gravity: false,
        is_trigger: false,
        layer: 0,
        friction: 0.5,
        restitution: 0.0,
    }
}

#[test]
fn a_script_driven_scene_moves_only_when_simulate_is_called() -> Result<(), SimulationError> {
    let mut simulation = Simulation::new(World::new());
    simulation.set_mode(SimulationMode::Script);
    let body = EntityId::new(1);
    simulation
        .world_mut()
        .add_body(body, moving_box([0.0, 0.0, 0.0]))?;

    assert!(
        simulation.advance(0.5)?.is_none(),
        "an engine tick must not advance a script-driven scene"
    );
    assert_eq!(simulation.world().position(body), Some([0.0, 0.0, 0.0]));

    let result = simulation.simulate(0.5)?;
    assert_eq!(result.delta_seconds, 0.5);
    assert_eq!(
        simulation.world().position(body),
        Some([1.0, 0.0, 0.0]),
        "a simulate call advances the world by exactly the requested step"
    );
    Ok(())
}

#[test]
fn simulate_outside_script_mode_reports_an_error_and_does_not_advance(
) -> Result<(), SimulationError> {
    let mut simulation = Simulation::new(World::new());
    let body = EntityId::new(1);
    simulation
        .world_mut()
        .add_body(body, moving_box([0.0, 0.0, 0.0]))?;

    assert_eq!(simulation.mode(), SimulationMode::FixedUpdate);
    assert_eq!(
        simulation.simulate(0.5),
        Err(SimulationError::NotScriptMode(SimulationMode::FixedUpdate))
    );
    assert_eq!(simulation.world().position(body), Some([0.0, 0.0, 0.0]));
    Ok(())
}

#[test]
fn a_local_scene_advances_only_through_simulate() -> Result<(), SimulationError> {
    let mut local_scene = Simulation::local(World::new());
    assert_eq!(local_scene.mode(), SimulationMode::Script);
    assert!(!local_scene.auto_simulation());

    let body = EntityId::new(1);
    local_scene
        .world_mut()
        .add_body(body, moving_box([0.0, 0.0, 0.0]))?;
    assert!(local_scene.advance(0.5)?.is_none());
    assert_eq!(local_scene.world().position(body), Some([0.0, 0.0, 0.0]));

    local_scene.simulate(0.5)?;
    assert_eq!(local_scene.world().position(body), Some([1.0, 0.0, 0.0]));
    Ok(())
}

#[test]
fn a_local_scene_simulates_independently_of_the_default_scene() -> Result<(), SimulationError> {
    let mut default_scene = Simulation::new(World::new());
    default_scene.set_mode(SimulationMode::Script);
    let mut local_scene = Simulation::local(World::new());
    let body = EntityId::new(1);
    default_scene
        .world_mut()
        .add_body(body, moving_box([0.0, 0.0, 0.0]))?;
    local_scene
        .world_mut()
        .add_body(body, moving_box([0.0, 0.0, 0.0]))?;

    local_scene.simulate(0.5)?;
    assert_eq!(local_scene.world().position(body), Some([1.0, 0.0, 0.0]));
    assert_eq!(
        default_scene.world().position(body),
        Some([0.0, 0.0, 0.0]),
        "stepping a local scene must not advance the default scene"
    );

    default_scene.simulate(0.5)?;
    assert_eq!(default_scene.world().position(body), Some([1.0, 0.0, 0.0]));
    assert_eq!(
        local_scene.world().position(body),
        Some([1.0, 0.0, 0.0]),
        "stepping the default scene must not advance a local scene"
    );
    Ok(())
}

#[test]
fn a_local_scene_body_is_absent_from_the_default_scene_queries() -> Result<(), SimulationError> {
    let default_scene = Simulation::new(World::new());
    let mut local_scene = Simulation::local(World::new());
    let local_body = EntityId::new(1);
    local_scene
        .world_mut()
        .add_body(local_body, moving_box([0.0, 0.0, 0.0]))?;

    assert!(default_scene
        .world()
        .overlap_sphere_default_layer([0.0, 0.0, 0.0], 5.0, 0)
        .is_empty());
    assert_eq!(
        local_scene
            .world()
            .overlap_sphere_default_layer([0.0, 0.0, 0.0], 5.0, 0),
        vec![local_body]
    );
    Ok(())
}

#[test]
fn auto_simulation_selects_the_fixed_update_and_script_modes() {
    let mut simulation = Simulation::new(World::new());
    assert_eq!(simulation.mode(), SimulationMode::FixedUpdate);
    assert!(simulation.auto_simulation());

    simulation.set_auto_simulation(false);
    assert_eq!(simulation.mode(), SimulationMode::Script);
    assert!(!simulation.auto_simulation());

    simulation.set_auto_simulation(true);
    assert_eq!(simulation.mode(), SimulationMode::FixedUpdate);
    assert!(simulation.auto_simulation());
}

#[test]
fn the_update_mode_advances_by_the_frame_delta_and_the_fixed_update_mode_by_the_fixed_timestep(
) -> Result<(), SimulationError> {
    let mut simulation = Simulation::new(World::new());
    let body = EntityId::new(1);
    simulation
        .world_mut()
        .add_body(body, moving_box([0.0, 0.0, 0.0]))?;

    simulation.set_mode(SimulationMode::Update);
    let update_step = simulation.advance(0.5)?;
    assert_eq!(update_step.map(|step| step.delta_seconds), Some(0.5));
    assert_eq!(simulation.world().position(body), Some([1.0, 0.0, 0.0]));

    simulation.set_mode(SimulationMode::FixedUpdate);
    simulation.set_fixed_timestep(0.25)?;
    let fixed_step = simulation.advance(0.5)?;
    assert_eq!(fixed_step.map(|step| step.delta_seconds), Some(0.25));
    assert_eq!(simulation.world().position(body), Some([1.5, 0.0, 0.0]));
    Ok(())
}

#[test]
fn a_simulate_step_reports_the_contacts_of_that_step() -> Result<(), SimulationError> {
    let mut simulation = Simulation::local(World::new());
    let first = EntityId::new(1);
    let second = EntityId::new(2);
    simulation
        .world_mut()
        .add_body(first, moving_box([0.0, 0.0, 0.0]))?;
    simulation
        .world_mut()
        .add_body(second, moving_box([0.5, 0.0, 0.0]))?;

    let result = simulation.simulate(0.5)?;
    assert!(
        result
            .collision_events
            .iter()
            .any(|event| event.a == first && event.b == second),
        "the contacts of the step are reported during the call"
    );
    Ok(())
}

#[test]
fn simulate_rejects_a_step_the_world_cannot_take() -> Result<(), SimulationError> {
    let mut simulation = Simulation::local(World::new());
    let body = EntityId::new(1);
    simulation
        .world_mut()
        .add_body(body, moving_box([0.0, 0.0, 0.0]))?;

    assert_eq!(
        simulation.simulate(-0.5),
        Err(SimulationError::World(PhysicsError::InvalidDeltaTime(-0.5)))
    );
    assert_eq!(simulation.world().position(body), Some([0.0, 0.0, 0.0]));
    Ok(())
}

#[test]
fn a_local_scene_body_does_not_collide_with_the_default_scene_bodies() -> Result<(), SimulationError>
{
    let mut default_scene = Simulation::new(World::new());
    default_scene.set_mode(SimulationMode::Script);
    let mut local_scene = Simulation::local(World::new());
    let first = EntityId::new(1);
    let second = EntityId::new(2);
    default_scene
        .world_mut()
        .add_body(first, moving_box([0.0, 0.0, 0.0]))?;
    default_scene
        .world_mut()
        .add_body(second, moving_box([0.5, 0.0, 0.0]))?;
    local_scene
        .world_mut()
        .add_body(EntityId::new(3), moving_box([0.0, 0.0, 0.0]))?;

    let local_step = local_scene.simulate(0.5)?;
    assert!(
        local_step.collision_events.is_empty(),
        "a local scene body must not collide with the default scene bodies"
    );

    let default_step = default_scene.simulate(0.5)?;
    assert!(
        default_step
            .collision_events
            .iter()
            .any(|event| event.a == first && event.b == second),
        "the default scene bodies still collide with each other"
    );
    Ok(())
}

#[test]
fn a_fixed_timestep_that_is_not_a_positive_number_is_rejected() {
    let mut simulation = Simulation::new(World::new());
    assert_eq!(
        simulation.set_fixed_timestep(0.0),
        Err(SimulationError::InvalidFixedTimestep(0.0))
    );
    assert!(matches!(
        simulation.set_fixed_timestep(f64::NAN),
        Err(SimulationError::InvalidFixedTimestep(value)) if value.is_nan()
    ));
    assert_eq!(simulation.fixed_timestep(), 0.02);
}
