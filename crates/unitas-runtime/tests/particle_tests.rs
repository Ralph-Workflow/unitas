use unitas_asset::particle::{ParticleSystemBinding, parse_particle_system};
use unitas_asset::unity_yaml;
use unitas_core::EntityId;
use unitas_physics::particles::SubEmitterTrigger;
use unitas_runtime::Runtime;
use unitas_runtime::particles::{parse_particle_system_modules, serialize_particle_system_modules};

const PARTICLE_SCENE: &str = "--- !u!198 &1000\nParticleSystem:\n  m_Duration: 5\n  m_Loop: 0\n  m_MaxParticles: 32\n  m_StartLifetime: 2\n  m_StartSpeed: 0\n  m_StartSize: 0.25\n  m_StartColor:\n    r: 1\n    g: 0.5\n    b: 0\n    a: 1\n  m_EmissionRate: 0\n  m_Bursts:\n  - m_Time: 0\n    m_Count: 5\n    m_Cycles: 1\n";

fn scene_binding() -> Result<ParticleSystemBinding, Box<dyn std::error::Error>> {
    let stream = unity_yaml::parse(PARTICLE_SCENE)?;
    let document = stream
        .documents
        .first()
        .ok_or("the scene holds one particle system document")?;
    Ok(parse_particle_system(document)?)
}

#[test]
fn a_registered_particle_binding_bursts_into_the_runtime() -> Result<(), Box<dyn std::error::Error>>
{
    let mut runtime = Runtime::new()?;
    let entity = EntityId::new(77);
    let binding = scene_binding()?;
    runtime.register_particle_system(entity, 4242, &binding);

    runtime.tick(0.02)?;
    runtime.tick(0.1)?;

    let snapshot = runtime
        .particles_mut()
        .snapshot(entity)
        .ok_or("the registered particle system is missing")?;
    assert_eq!(
        snapshot.live_particle_count, 5,
        "the burst of the registered binding must be live in the runtime"
    );
    assert_eq!(snapshot.duration, 5.0);
    assert!(!snapshot.looping);
    assert_eq!(snapshot.emission_rate, 0.0);
    Ok(())
}

#[test]
fn the_draw_list_exposes_the_live_particles_after_a_tick() -> Result<(), Box<dyn std::error::Error>>
{
    let mut runtime = Runtime::new()?;
    let entity = EntityId::new(78);
    let binding = scene_binding()?;
    runtime.register_particle_system(entity, 99, &binding);

    runtime.tick(0.02)?;
    runtime.tick(0.1)?;

    let draw_list = runtime.particle_draw_list();
    assert_eq!(
        draw_list.len(),
        5,
        "the draw list must hold every live particle of the tick"
    );
    let item = draw_list.first().ok_or("the draw list is empty")?;
    assert_eq!(item.entity, entity);
    assert_eq!(item.size, 0.25);
    assert_eq!(item.color, [1.0, 0.5, 0.0, 1.0]);
    assert_eq!(item.position, [0.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn the_draw_list_is_emptied_when_the_particles_expire() -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = Runtime::new()?;
    let entity = EntityId::new(79);
    let binding = scene_binding()?;
    runtime.register_particle_system(entity, 7, &binding);

    runtime.tick(0.02)?;
    runtime.tick(0.1)?;
    assert_eq!(runtime.particle_draw_list().len(), 5);

    runtime.tick(3.0)?;
    assert!(
        runtime.particle_draw_list().is_empty(),
        "the draw list must drop the particles that expired"
    );
    Ok(())
}

const MODULE_SCENE: &str = "--- !u!198 &1000\nParticleSystem:\n  m_Duration: 5\n  m_Loop: 0\n  m_MaxParticles: 32\n  m_StartLifetime: 1\n  m_StartSpeed: 0\n  m_StartSize: 0.5\n  m_StartColor:\n    r: 1\n    g: 0.5\n    b: 0\n    a: 1\n  m_EmissionRate: 0\n  m_Bursts:\n  - m_Time: 0\n    m_Count: 1\n    m_Cycles: 1\n  SubModule:\n    enabled: 1\n    subEmitters:\n    - subEmitter: {fileID: 2000}\n      type: 0\n      emitProbability: 1\n      emitCount: 2\n    - subEmitter: {fileID: 2001}\n      type: 1\n      emitProbability: 1\n      emitCount: 3\n    - subEmitter: {fileID: 2002}\n      type: 2\n      emitProbability: 1\n      emitCount: 4\n    - subEmitter: {fileID: 2003}\n      type: 3\n      emitProbability: 1\n      emitCount: 5\n  UVModule:\n    enabled: 1\n    tilesX: 4\n    tilesY: 2\n    animationType: 0\n    rowIndex: 0\n    cycles: 1\n    startFrame: 0\n    frameOverTime:\n      start: 0\n      end: 1\n";

#[test]
fn particle_sub_emitters_and_texture_sheet_round_trip_through_scene_yaml_and_step_in_runtime()
-> Result<(), Box<dyn std::error::Error>> {
    let stream = unity_yaml::parse(MODULE_SCENE)?;
    let doc = stream
        .documents
        .first()
        .ok_or("missing particle document")?;
    let binding = parse_particle_system(doc)?;
    let modules = parse_particle_system_modules(doc)?;
    assert_eq!(modules.sub_emitters.len(), 4);
    assert_eq!(modules.sub_emitters[0].trigger, SubEmitterTrigger::Birth);
    assert_eq!(
        modules.sub_emitters[1].trigger,
        SubEmitterTrigger::Collision
    );
    assert_eq!(modules.sub_emitters[2].trigger, SubEmitterTrigger::Death);
    assert_eq!(modules.sub_emitters[3].trigger, SubEmitterTrigger::Trigger);
    assert!(modules.texture_sheet_animation.is_some());

    let serialized = serialize_particle_system_modules(&binding, &modules);
    let reparsed_stream = unity_yaml::parse(&serialized)?;
    let reparsed_doc = reparsed_stream
        .documents
        .first()
        .ok_or("missing reparsed document")?;
    let reparsed_binding = parse_particle_system(reparsed_doc)?;
    let reparsed_modules = parse_particle_system_modules(reparsed_doc)?;
    assert_eq!(reparsed_binding, binding);
    assert_eq!(reparsed_modules, modules);

    let mut runtime = Runtime::new()?;
    let parent = EntityId::new(1000);
    let birth_child = EntityId::new(2000);
    let collision_child = EntityId::new(2001);
    let death_child = EntityId::new(2002);
    let trigger_child = EntityId::new(2003);

    let child_binding = ParticleSystemBinding {
        duration: 5.0,
        looping: false,
        max_particles: 32,
        start_lifetime: 5.0,
        start_speed: 0.0,
        start_size: 0.25,
        start_color: [0.0, 1.0, 0.0, 1.0],
        emission_rate: 0.0,
        bursts: Vec::new(),
    };
    runtime.register_particle_system(birth_child, 10, &child_binding);
    runtime.register_particle_system(collision_child, 11, &child_binding);
    runtime.register_particle_system(death_child, 12, &child_binding);
    runtime.register_particle_system(trigger_child, 13, &child_binding);
    runtime
        .particles_mut()
        .register_binding_with_modules(parent, 99, &binding, &modules);

    runtime.physics_mut().add_body_3d(
        EntityId::new(50),
        unitas_physics::BodyDesc {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0; 3],
            use_gravity: false,
            is_trigger: false,
            collider_shape: unitas_physics::ColliderShape::Primitive(
                unitas_physics::Shape::Sphere { radius: 1.0 },
            ),
            layer: 1,
            role: unitas_physics::BodyRole::Static,
            friction: 0.5,
            restitution: 0.0,
            collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
        },
    )?;

    runtime.tick(0.02)?;
    runtime.tick(0.5)?;

    let birth_count = runtime
        .particles_mut()
        .snapshot(birth_child)
        .ok_or("missing birth child")?
        .live_particle_count;
    let collision_count = runtime
        .particles_mut()
        .snapshot(collision_child)
        .ok_or("missing collision child")?
        .live_particle_count;
    assert_eq!(birth_count, 2);
    assert_eq!(collision_count, 3);

    let parent_draw = runtime
        .particle_draw_list()
        .iter()
        .find(|item| item.entity == parent)
        .ok_or("missing parent draw item")?;
    assert_eq!(parent_draw.uv_rect, [0.0, 0.5, 0.25, 0.5]);

    runtime.tick(0.6)?;
    let death_count = runtime
        .particles_mut()
        .snapshot(death_child)
        .ok_or("missing death child")?
        .live_particle_count;
    assert_eq!(death_count, 4);
    Ok(())
}
