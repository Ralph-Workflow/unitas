use unitas_asset::particle::{ParticleSystemBinding, parse_particle_system};
use unitas_asset::unity_yaml;
use unitas_core::EntityId;
use unitas_runtime::Runtime;

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
