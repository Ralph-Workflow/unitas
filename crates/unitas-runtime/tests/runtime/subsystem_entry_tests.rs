use std::sync::Mutex;

use unitas_core::EntityId;
use unitas_physics::matrix::{CollisionMessage, PhysicsObject};
use unitas_physics::navigation::{NavMesh, NavMeshTriangle, NavigationError};
use unitas_physics::particles::{
    Curve, CustomParticleDataStream, EmitParams, ForceFieldShape, ParticlePropertyCurves,
    ParticleShape, ParticleSystemForceField, ParticleSystemState, StopAction,
};
use unitas_physics::two_d::{
    BodyDesc2D, ColliderDesc2D, ContactFilter2D, MessageKind2D, PlatformEffector2D,
};
use unitas_physics::{BodyDesc, BodyRole, PhysicsError, Shape};
use unitas_runtime::particles::ParticleRuntimeWorld;
use unitas_runtime::physics::PhysicsRuntimeWorld;
use unitas_runtime::{Callbacks, Runtime};

#[test]
fn subsystem_entry_physics_3d_inspects_steps_and_routes_matrix_messages() -> Result<(), PhysicsError>
{
    let mut runtime_physics = PhysicsRuntimeWorld::new();
    let floor = EntityId::new(1);
    let crate_box = EntityId::new(2);

    runtime_physics.add_body_3d(
        floor,
        BodyDesc {
            role: BodyRole::Static,
            shape: Shape::Box {
                half_extents: [5.0, 0.5, 5.0],
            },
            position: [0.0, -0.5, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0, 0.0, 0.0],
            use_gravity: false,
            is_trigger: false,
            layer: 1,
            friction: 0.5,
            restitution: 0.0,
        },
    )?;
    runtime_physics.add_body_3d(
        crate_box,
        BodyDesc {
            role: BodyRole::Dynamic,
            shape: Shape::Box {
                half_extents: [0.5, 0.5, 0.5],
            },
            position: [0.0, 0.6, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 1.0,
            velocity: [0.0, -2.0, 0.0],
            use_gravity: true,
            is_trigger: false,
            layer: 1,
            friction: 0.5,
            restitution: 0.0,
        },
    )?;

    let summary = runtime_physics.summary();
    assert_eq!(summary.body_count_3d, 2);

    let before = runtime_physics.inspect_body_3d(crate_box);
    assert!(before.is_some());

    let (collisions, _triggers) = runtime_physics.step_3d(0.2)?;
    assert!(!collisions.is_empty());

    let hits = runtime_physics.raycast_3d([0.0, 5.0, 0.0], [0.0, -1.0, 0.0], 10.0, u32::MAX);
    assert!(!hits.is_empty());

    let mut dynamic_obj = PhysicsObject::new(20, 1);
    dynamic_obj.has_rigidbody = true;
    dynamic_obj.rigidbody_object_id = Some(200);
    let mut static_obj = PhysicsObject::new(10, 0);
    static_obj.is_static = true;

    let deliveries = runtime_physics.route_3d_message(
        CollisionMessage::OnCollisionEnter,
        &dynamic_obj,
        &static_obj,
    );
    assert_eq!(deliveries.len(), 2);

    runtime_physics.set_layer_collision_3d(0, 1, false);
    assert!(!runtime_physics.layer_collision_enabled_3d(0, 1));
    let blocked = runtime_physics.route_3d_message(
        CollisionMessage::OnCollisionEnter,
        &dynamic_obj,
        &static_obj,
    );
    assert!(blocked.is_empty());
    Ok(())
}

#[test]
fn subsystem_entry_physics_3d_stepping_and_message_routing_agree_on_the_collision_action_matrix(
) -> Result<(), PhysicsError> {
    let mut runtime_physics = PhysicsRuntimeWorld::new();
    let floor = EntityId::new(1);
    let trigger_zone = EntityId::new(2);
    let crate_body = EntityId::new(3);
    let dynamic_body = EntityId::new(4);

    runtime_physics.add_body_3d(
        floor,
        BodyDesc {
            role: BodyRole::Static,
            shape: Shape::Box {
                half_extents: [2.0, 0.5, 2.0],
            },
            position: [0.0, -0.5, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0; 3],
            use_gravity: false,
            is_trigger: false,
            layer: 0,
            friction: 0.5,
            restitution: 0.0,
        },
    )?;
    runtime_physics.add_body_3d(
        trigger_zone,
        BodyDesc {
            role: BodyRole::Static,
            shape: Shape::Box {
                half_extents: [0.5, 0.5, 0.5],
            },
            position: [10.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0; 3],
            use_gravity: false,
            is_trigger: true,
            layer: 0,
            friction: 0.5,
            restitution: 0.0,
        },
    )?;
    runtime_physics.add_body_3d(
        crate_body,
        BodyDesc {
            role: BodyRole::Static,
            shape: Shape::Box {
                half_extents: [0.5, 0.5, 0.5],
            },
            position: [10.5, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0; 3],
            use_gravity: false,
            is_trigger: false,
            layer: 0,
            friction: 0.5,
            restitution: 0.0,
        },
    )?;
    runtime_physics.add_body_3d(
        dynamic_body,
        BodyDesc {
            role: BodyRole::Dynamic,
            shape: Shape::Sphere { radius: 0.5 },
            position: [0.0, 0.4, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 1.0,
            velocity: [0.0; 3],
            use_gravity: false,
            is_trigger: false,
            layer: 0,
            friction: 0.5,
            restitution: 0.0,
        },
    )?;

    let (collisions, triggers) = runtime_physics.step_3d(0.02)?;
    assert_eq!(
        collisions.len(),
        1,
        "PHY-001: the subsystem entry must report only the collision pair that carries a Rigidbody"
    );
    assert_eq!(collisions[0].a, floor);
    assert_eq!(collisions[0].b, dynamic_body);
    assert!(
        triggers.is_empty(),
        "PHY-001 story 13: the subsystem entry must report no trigger event for two colliders without a Rigidbody"
    );

    let mut static_obj = PhysicsObject::new(10, 0);
    static_obj.is_static = true;
    let mut dynamic_obj = PhysicsObject::new(11, 0);
    dynamic_obj.has_rigidbody = true;
    assert!(
        !runtime_physics
            .route_3d_message(CollisionMessage::OnCollisionEnter, &static_obj, &dynamic_obj)
            .is_empty(),
        "PHY-001: the subsystem entry message surface must route the collision pair that carries a Rigidbody"
    );
    let mut trigger_obj = PhysicsObject::new(12, 0);
    trigger_obj.is_trigger = true;
    assert!(
        runtime_physics
            .route_3d_message(CollisionMessage::OnTriggerEnter, &trigger_obj, &static_obj)
            .is_empty(),
        "PHY-001 story 13: the subsystem entry message surface must route no trigger message for two colliders without a Rigidbody"
    );
    Ok(())
}

#[test]
fn subsystem_entry_physics_2d_steps_and_enforces_one_way_platform_effector() {
    let mut runtime_physics = PhysicsRuntimeWorld::new();
    let platform_entity = EntityId::new(10);
    let actor_entity = EntityId::new(11);

    let platform_body =
        runtime_physics.add_body_2d(BodyDesc2D::static_body(platform_entity, [0.0, 0.0]));
    let mut platform_collider_desc = ColliderDesc2D::box_collider(platform_entity, [2.0, 0.2]);
    platform_collider_desc.attached_body = Some(platform_body);
    let platform_collider = runtime_physics.add_collider_2d(platform_collider_desc);
    assert!(runtime_physics
        .set_platform_effector_2d(platform_collider, PlatformEffector2D::new(180.0),));

    let mut upward_actor = BodyDesc2D::dynamic(actor_entity, [0.0, -0.5]);
    upward_actor.linear_velocity = [0.0, 8.0];
    upward_actor.gravity_scale = 0.0;
    let actor_body = runtime_physics.add_body_2d(upward_actor);
    let mut actor_collider_desc = ColliderDesc2D::circle_collider(actor_entity, 0.2);
    actor_collider_desc.attached_body = Some(actor_body);
    let _actor_collider = runtime_physics.add_collider_2d(actor_collider_desc);

    let pass_messages = runtime_physics.step_2d(0.15);
    assert!(pass_messages.is_empty());

    if let Some(body) = runtime_physics.world_2d_mut().body_mut(actor_body) {
        body.position = [0.0, 0.5];
        body.linear_velocity = [0.0, -4.0];
    }
    let land_messages = runtime_physics.step_2d(0.15);
    assert!(land_messages
        .iter()
        .any(|msg| msg.kind == MessageKind2D::CollisionEnter2D && msg.recipient == actor_entity));

    let hits = runtime_physics.raycast_2d([0.0, 2.0], [0.0, -1.0], 5.0, ContactFilter2D::default());
    assert!(!hits.is_empty());
}

#[test]
fn subsystem_entry_particles_control_systems_and_collide_with_3d_world() -> Result<(), PhysicsError>
{
    let mut physics = PhysicsRuntimeWorld::new();
    let floor = EntityId::new(1);
    physics.add_body_3d(
        floor,
        BodyDesc {
            role: BodyRole::Static,
            shape: Shape::Sphere { radius: 1.0 },
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0, 0.0, 0.0],
            use_gravity: false,
            is_trigger: false,
            layer: 1,
            friction: 0.5,
            restitution: 0.0,
        },
    )?;

    let mut particles = ParticleRuntimeWorld::new();
    let emitter = EntityId::new(50);
    {
        let system = particles.register_system(emitter, 42);
        system.set_shape(ParticleShape::Point);
        system.set_emit_position([0.0, 0.2, 0.0]);
        system.set_emit_params(EmitParams {
            start_speed: 0.0,
            start_size: 0.2,
            start_lifetime: 2.0,
            start_color: [1.0, 1.0, 1.0, 1.0],
        });
    }
    assert!(particles.set_curves(
        emitter,
        ParticlePropertyCurves {
            velocity: Some(Curve::new([0.0, -1.0, 0.0], [0.0, -1.0, 0.0])),
            force: None,
            color: None,
            size: None,
        },
    ));
    assert!(particles.emit(emitter, 4));
    particles.step_all(0.1, Some(physics.world_3d()));

    let snapshot = particles.snapshot(emitter);
    assert!(snapshot.is_some());
    if let Some(snap) = snapshot {
        assert_eq!(snap.live_particle_count, 4);
        assert_eq!(snap.state, ParticleSystemState::Playing);
    }

    let collisions = particles.collision_events(emitter).unwrap_or(&[]);
    assert_eq!(collisions.len(), 4);

    assert!(particles.pause(emitter));
    assert_eq!(
        particles.snapshot(emitter).map(|s| s.state),
        Some(ParticleSystemState::Paused)
    );
    assert!(particles.play(emitter));
    assert!(particles.stop(emitter, StopAction::Destroy));
    assert!(particles.clear(emitter));
    particles.step_all(0.05, None);
    assert_eq!(particles.system_count(), 0);
    Ok(())
}

#[test]
fn subsystem_entry_teleports_a_dynamic_body_and_syncs_its_transform(
) -> Result<(), Box<dyn std::error::Error>> {
    const FIXTURE: &str = "tests/fixtures/scene_physics_collision_no_script.unity";
    let mut runtime = unitas_runtime::Runtime::new()?;
    runtime.load_scene(std::path::Path::new(FIXTURE))?;
    let ball = runtime
        .find_entity_by_name("Ball")
        .ok_or("missing Ball entity")?;
    let before = runtime
        .body_position(ball)
        .ok_or("missing Ball physics body")?;
    let target = [before[0] + 3.0, before[1] + 2.0, before[2] - 1.0];

    runtime.teleport_body_position(ball, target)?;
    assert_eq!(runtime.body_position(ball), Some(target));

    let transform = runtime
        .world_transform_of(ball)
        .ok_or("missing Ball transform")?;
    assert!((f64::from(transform.translation[0]) - target[0]).abs() < 1e-4);
    assert!((f64::from(transform.translation[1]) - target[1]).abs() < 1e-4);
    assert!((f64::from(transform.translation[2]) - target[2]).abs() < 1e-4);
    Ok(())
}

#[test]
fn subsystem_entry_particles_custom_data_and_force_field_and_trigger_events_in_runtime_world(
) -> Result<(), PhysicsError> {
    let mut physics = PhysicsRuntimeWorld::new();
    let trigger_zone = EntityId::new(77);
    physics.add_body_3d(
        trigger_zone,
        BodyDesc {
            role: BodyRole::Static,
            shape: Shape::Sphere { radius: 2.0 },
            position: [1.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0, 0.0, 0.0],
            use_gravity: false,
            is_trigger: true,
            layer: 1,
            friction: 0.5,
            restitution: 0.0,
        },
    )?;

    let mut particles = ParticleRuntimeWorld::new();
    let emitter = EntityId::new(88);
    {
        let system = particles.register_system(emitter, 314);
        system.set_shape(ParticleShape::Point);
        system.set_emit_position([0.0, 0.0, 0.0]);
        system.set_emit_params(EmitParams {
            start_speed: 0.0,
            start_size: 0.5,
            start_lifetime: 2.0,
            start_color: [1.0, 1.0, 1.0, 1.0],
        });
        system.set_external_forces_enabled(true);
    }
    assert!(particles.emit_with_params(
        emitter,
        EmitParams {
            start_speed: 0.0,
            start_size: 0.5,
            start_lifetime: 2.0,
            start_color: [0.25, 0.5, 0.75, 1.0],
        },
        2,
    ));
    let stream_data = vec![[1.0, 2.0, 3.0, 4.0], [5.0, 6.0, 7.0, 8.0]];
    assert!(
        particles.set_custom_particle_data(
            emitter,
            CustomParticleDataStream::Custom1,
            &stream_data
        ),
        "REN-004 outcome 16: ParticleRuntimeWorld must accept custom particle data stream"
    );
    assert_eq!(
        particles.get_custom_particle_data(emitter, CustomParticleDataStream::Custom1),
        Some(stream_data.as_slice()),
        "REN-004 outcome 16: ParticleRuntimeWorld must read back custom particle data stream"
    );

    assert!(
        particles.add_force_field(
            emitter,
            ParticleSystemForceField {
                position: [0.0, 0.0, 0.0],
                shape: ForceFieldShape::Sphere { radius: 5.0 },
                direction: [4.0, 0.0, 0.0],
                gravity: [0.0, 0.0, 0.0],
                gravity_focus: 0.0,
                drag: 0.0,
                layer: 0,
            }
        ),
        "REN-004 outcome 16: ParticleRuntimeWorld must register ParticleSystemForceField"
    );

    particles.step_all(0.5, Some(physics.world_3d()));
    let live = particles.particles(emitter).unwrap_or(&[]);
    assert_eq!(live.len(), 2);
    assert!(
        (live[0].position[0] - 1.0).abs() < 1e-4,
        "REN-004 outcome 16: force field in runtime world must move particle into trigger zone"
    );
    let triggers = particles.trigger_events(emitter).unwrap_or(&[]);
    assert_eq!(
        triggers.len(),
        2,
        "REN-004 outcome 16: runtime particle world must report OnParticleTrigger contacts"
    );
    assert_eq!(triggers[0].entity, trigger_zone);
    Ok(())
}

static SUBSYSTEM_ENTRY_COLLISIONS: Mutex<Vec<(u64, u64, u32)>> = Mutex::new(Vec::new());
static SUBSYSTEM_ENTRY_TRIGGERS: Mutex<Vec<(u64, u64, u32)>> = Mutex::new(Vec::new());

extern "C" fn subsystem_entry_create_script(_entity: u64, _hi: u64, _lo: u64) -> u64 {
    1
}

extern "C" fn subsystem_entry_noop_handle(_handle: u64) {}

extern "C" fn subsystem_entry_noop_step(_handle: u64, _delta: f64) {}

extern "C" fn subsystem_entry_collision(a: u64, b: u64, phase: u32) {
    if let Ok(mut events) = SUBSYSTEM_ENTRY_COLLISIONS.lock() {
        events.push((a, b, phase));
    }
}

extern "C" fn subsystem_entry_trigger(a: u64, b: u64, phase: u32) {
    if let Ok(mut events) = SUBSYSTEM_ENTRY_TRIGGERS.lock() {
        events.push((a, b, phase));
    }
}

fn subsystem_entry_callbacks() -> Callbacks {
    Callbacks {
        create_script: subsystem_entry_create_script,
        awake: subsystem_entry_noop_handle,
        on_enable: subsystem_entry_noop_handle,
        start: subsystem_entry_noop_handle,
        fixed_update: subsystem_entry_noop_step,
        update: subsystem_entry_noop_step,
        late_update: subsystem_entry_noop_step,
        on_disable: subsystem_entry_noop_handle,
        destroy: subsystem_entry_noop_handle,
        collision: subsystem_entry_collision,
        trigger: subsystem_entry_trigger,
    }
}

fn subsystem_entry_collision_events() -> Result<Vec<(u64, u64, u32)>, String> {
    match SUBSYSTEM_ENTRY_COLLISIONS.lock() {
        Ok(guard) => Ok(guard.clone()),
        Err(_) => Err(String::from("mutex poisoned")),
    }
}

fn subsystem_entry_trigger_events() -> Result<Vec<(u64, u64, u32)>, String> {
    match SUBSYSTEM_ENTRY_TRIGGERS.lock() {
        Ok(guard) => Ok(guard.clone()),
        Err(_) => Err(String::from("mutex poisoned")),
    }
}

fn subsystem_entry_navmesh() -> Result<NavMesh, NavigationError> {
    let mut triangles = Vec::new();
    for index in 0..4 {
        let x = f64::from(index);
        triangles.push(NavMeshTriangle::new(
            [[x, 0.0, 0.0], [x, 0.0, 1.0], [x + 1.0, 0.0, 0.0]],
            0,
        ));
        triangles.push(NavMeshTriangle::new(
            [[x + 1.0, 0.0, 0.0], [x, 0.0, 1.0], [x + 1.0, 0.0, 1.0]],
            0,
        ));
    }
    NavMesh::build(&triangles, &[])
}

fn subsystem_entry_dynamic_sphere(position: [f64; 3], radius: f64) -> BodyDesc {
    BodyDesc {
        role: BodyRole::Dynamic,
        shape: Shape::Sphere { radius },
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity: [0.0; 3],
        use_gravity: true,
        is_trigger: false,
        layer: 0,
        friction: 0.6,
        restitution: 0.0,
    }
}

#[test]
fn subsystem_entry_runtime_tick_steps_2d_physics_and_dispatches_the_collision_callback(
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(mut events) = SUBSYSTEM_ENTRY_COLLISIONS.lock() {
        events.clear();
    }
    let mut runtime = Runtime::new()?;
    let floor_entity = EntityId::new(31);
    let ball_entity = EntityId::new(32);
    let ball_body = {
        let physics = runtime.physics_mut();
        let floor_body = physics.add_body_2d(BodyDesc2D::static_body(floor_entity, [0.0, 0.0]));
        let mut floor_collider = ColliderDesc2D::box_collider(floor_entity, [3.0, 0.2]);
        floor_collider.attached_body = Some(floor_body);
        physics.add_collider_2d(floor_collider);
        let ball_body = physics.add_body_2d(BodyDesc2D::dynamic(ball_entity, [0.0, 1.0]));
        let mut ball_collider = ColliderDesc2D::circle_collider(ball_entity, 0.2);
        ball_collider.attached_body = Some(ball_body);
        physics.add_collider_2d(ball_collider);
        ball_body
    };

    runtime.set_callbacks(subsystem_entry_callbacks());
    runtime.tick(0.02)?;
    let start = runtime
        .physics_mut()
        .inspect_body_2d(ball_body)
        .ok_or("missing 2D ball body")?
        .position;
    assert_eq!(
        start,
        [0.0, 1.0],
        "the 2D ball must not move before the first simulated frame"
    );

    for _ in 0..6 {
        runtime.tick(0.1)?;
    }

    let end = runtime
        .physics_mut()
        .inspect_body_2d(ball_body)
        .ok_or("missing 2D ball body")?
        .position;
    assert!(
        end[1] < start[1],
        "the 2D rigidbody must fall under gravity across the fixed steps: start {start:?} end {end:?}"
    );

    let events = subsystem_entry_collision_events()?;
    let ball = ball_entity.as_u64();
    let floor = floor_entity.as_u64();
    assert!(
        events.iter().any(|(a, b, phase)| {
            *phase == 0 && ((*a == ball && *b == floor) || (*a == floor && *b == ball))
        }),
        "the 2D contact must reach the script collision callback path: {events:?}"
    );
    assert!(
        subsystem_entry_trigger_events()?.is_empty(),
        "a 2D contact is not a trigger contact"
    );
    Ok(())
}

#[test]
fn subsystem_entry_runtime_tick_ages_and_retires_emitted_particles(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = Runtime::new()?;
    let emitter = EntityId::new(41);
    let emitter_2d = EntityId::new(42);
    {
        let system = runtime.particles_mut().register_system(emitter, 7);
        system.set_shape(ParticleShape::Point);
        system.set_emit_position([0.0, 0.0, 0.0]);
        system.set_emit_params(EmitParams {
            start_speed: 0.0,
            start_size: 0.1,
            start_lifetime: 0.05,
            start_color: [1.0, 1.0, 1.0, 1.0],
        });
    }
    {
        let system = runtime.particles_2d_mut().register_system(emitter_2d, 9);
        system.system_mut().set_shape(ParticleShape::Point);
        system.system_mut().set_emit_params(EmitParams {
            start_speed: 0.0,
            start_size: 0.1,
            start_lifetime: 0.05,
            start_color: [1.0, 1.0, 1.0, 1.0],
        });
    }
    assert!(runtime.particles_mut().emit(emitter, 3));
    runtime
        .particles_2d_mut()
        .system_mut(emitter_2d)
        .ok_or("missing 2D particle system")?
        .system_mut()
        .emit(2);
    assert_eq!(
        runtime
            .particles_mut()
            .particles(emitter)
            .map(|particles| particles.len()),
        Some(3),
        "the emitted particles must be live before the frame loop runs"
    );
    assert_eq!(
        runtime
            .particles_2d_mut()
            .system(emitter_2d)
            .map(|system| system.system().get_particles().len()),
        Some(2),
        "the emitted 2D particles must be live before the frame loop runs"
    );

    runtime.tick(0.02)?;
    assert_eq!(
        runtime
            .particles_mut()
            .particles(emitter)
            .map(|particles| particles.len()),
        Some(3),
        "the first tick only initializes the runtime"
    );

    runtime.tick(0.1)?;
    assert_eq!(
        runtime
            .particles_mut()
            .particles(emitter)
            .map(|particles| particles.len()),
        Some(0),
        "the particle must age past its lifetime and die inside the frame loop"
    );
    assert_eq!(
        runtime
            .particles_2d_mut()
            .system(emitter_2d)
            .map(|system| system.system().get_particles().len()),
        Some(0),
        "the 2D particle must age past its lifetime and die inside the frame loop"
    );
    Ok(())
}

#[test]
fn subsystem_entry_runtime_tick_advances_a_nav_agent_along_its_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = Runtime::new()?;
    let agent_entity = EntityId::new(51);
    runtime
        .navigation_mut()
        .set_navmesh(subsystem_entry_navmesh()?);
    runtime
        .navigation_mut()
        .add_agent(agent_entity, "Walker", [0.5, 0.0, 0.5])?;
    runtime
        .navigation_mut()
        .set_destination(agent_entity, [3.5, 0.0, 0.5])?;

    runtime.tick(0.02)?;
    runtime.tick(0.1)?;
    runtime.tick(0.1)?;

    let position = runtime
        .navigation_mut()
        .position(agent_entity)
        .ok_or("missing navmesh agent")?;
    assert!(
        position[0] > 0.5,
        "the nav agent must advance along its path: {position:?}"
    );
    Ok(())
}

#[test]
fn subsystem_entry_runtime_tick_with_time_scale_zero_freezes_2d_physics_and_particles_exactly_as_3d(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = Runtime::new()?;
    let ball_entity = EntityId::new(61);
    let body_entity = EntityId::new(62);
    let emitter = EntityId::new(63);
    let ball_body = {
        let physics = runtime.physics_mut();
        physics.add_body_3d(
            body_entity,
            subsystem_entry_dynamic_sphere([0.0, 2.0, 0.0], 0.25),
        )?;
        let ball_body = physics.add_body_2d(BodyDesc2D::dynamic(ball_entity, [0.0, 2.0]));
        let mut ball_collider = ColliderDesc2D::circle_collider(ball_entity, 0.25);
        ball_collider.attached_body = Some(ball_body);
        physics.add_collider_2d(ball_collider);
        ball_body
    };
    {
        let system = runtime.particles_mut().register_system(emitter, 11);
        system.set_shape(ParticleShape::Point);
        system.set_emit_params(EmitParams {
            start_speed: 0.0,
            start_size: 0.1,
            start_lifetime: 30.0,
            start_color: [1.0, 1.0, 1.0, 1.0],
        });
    }
    assert!(runtime.particles_mut().emit(emitter, 2));

    runtime.tick(0.02)?;
    runtime.tick(0.1)?;

    let running_2d = runtime
        .physics_mut()
        .inspect_body_2d(ball_body)
        .ok_or("missing 2D ball body")?
        .position;
    let running_3d = runtime
        .physics_mut()
        .world_3d()
        .position(body_entity)
        .ok_or("missing 3D body")?;
    let running_age = runtime
        .particles_mut()
        .particles(emitter)
        .and_then(|particles| particles.first().map(|particle| particle.age))
        .ok_or("missing emitted particle")?;
    assert!(running_2d[1] < 2.0, "the 2D body must fall while time runs");
    assert!(running_3d[1] < 2.0, "the 3D body must fall while time runs");
    assert!(running_age > 0.0, "the particle must age while time runs");

    runtime.set_time_scale(0.0);
    runtime.tick(0.1)?;

    let frozen_2d = runtime
        .physics_mut()
        .inspect_body_2d(ball_body)
        .ok_or("missing 2D ball body")?
        .position;
    let frozen_3d = runtime
        .physics_mut()
        .world_3d()
        .position(body_entity)
        .ok_or("missing 3D body")?;
    let frozen_age = runtime
        .particles_mut()
        .particles(emitter)
        .and_then(|particles| particles.first().map(|particle| particle.age))
        .ok_or("missing emitted particle")?;
    assert_eq!(
        frozen_2d, running_2d,
        "time scale zero must freeze the 2D physics exactly as it freezes the 3D physics"
    );
    assert_eq!(
        frozen_3d, running_3d,
        "time scale zero must freeze the 3D physics"
    );
    assert_eq!(
        frozen_age, running_age,
        "time scale zero must freeze the particle system exactly as it freezes the 3D physics"
    );
    Ok(())
}
