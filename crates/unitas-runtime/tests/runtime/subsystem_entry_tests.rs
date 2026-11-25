use unitas_core::EntityId;
use unitas_physics::matrix::{CollisionMessage, PhysicsObject};
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
