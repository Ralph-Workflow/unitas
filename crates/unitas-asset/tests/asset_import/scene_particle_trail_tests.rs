use std::error::Error;
use std::path::Path;

use unitas_asset::scene::{
    assemble_scene_file_bundle, AssembledScene, PARTICLE_SYSTEM_CLASS, TRAIL_RENDERER_CLASS,
};
use unitas_core::{EntityId, SceneGraph};

type TestResult = Result<(), Box<dyn Error>>;

const PARTICLE_TRAIL_FIXTURE: &str = "tests/fixtures/particles_trails.unity";
const SCENE_FIXTURE: &str = "tests/fixtures/scene.unity";
const UNSUPPORTED_CLASS: u32 = 222;

fn entity_named(graph: &SceneGraph, name: &str) -> Result<EntityId, Box<dyn Error>> {
    for id in graph.breadth_first_order() {
        if let Some(entity) = graph.entity(id) {
            if entity.name == name {
                return Ok(id);
            }
        }
    }
    Err(format!("no entity named {name}").into())
}

fn assembled() -> Result<AssembledScene, Box<dyn Error>> {
    Ok(assemble_scene_file_bundle(Path::new(
        PARTICLE_TRAIL_FIXTURE,
    ))?)
}

#[test]
fn a_scene_particle_system_binds_its_unity_fields_to_its_entity() -> TestResult {
    let assembled = assembled()?;
    assert_eq!(PARTICLE_SYSTEM_CLASS, 198);
    assert_eq!(assembled.particles.len(), 1);
    let emitter = entity_named(&assembled.graph, "Emitter")?;
    let particle = &assembled.particles[0];
    assert_eq!(particle.entity, emitter);
    assert!((particle.binding.duration - 2.5).abs() < 1e-6);
    assert!(particle.binding.looping);
    assert_eq!(particle.binding.max_particles, 64);
    assert!((particle.binding.start_lifetime - 1.5).abs() < 1e-6);
    assert!((particle.binding.start_speed - 3.0).abs() < 1e-6);
    assert!((particle.binding.start_size - 0.25).abs() < 1e-6);
    assert!((particle.binding.start_color[1] - 0.5).abs() < 1e-6);
    assert!((particle.binding.start_color[3] - 0.75).abs() < 1e-6);
    assert!((particle.binding.emission_rate - 12.0).abs() < 1e-6);
    assert_eq!(particle.binding.bursts.len(), 1);
    assert!((particle.binding.bursts[0].time - 0.5).abs() < 1e-6);
    assert_eq!(particle.binding.bursts[0].count, 7);
    assert_eq!(particle.binding.bursts[0].cycles, 3);
    assert!((particle.binding.bursts[0].interval - 0.25).abs() < 1e-6);
    Ok(())
}

#[test]
fn a_scene_trail_renderer_binds_its_unity_fields_to_its_entity() -> TestResult {
    let assembled = assembled()?;
    assert_eq!(TRAIL_RENDERER_CLASS, 96);
    assert_eq!(assembled.trails.len(), 1);
    let trail_owner = entity_named(&assembled.graph, "Trail")?;
    let trail = &assembled.trails[0];
    assert_eq!(trail.entity, trail_owner);
    assert_eq!(trail.binding.entity, trail_owner);
    assert!((trail.binding.time - 2.0).abs() < 1e-6);
    assert!((trail.binding.width_start - 0.75).abs() < 1e-6);
    assert!((trail.binding.width_end - 0.75).abs() < 1e-6);
    assert!((trail.binding.min_vertex_distance - 0.25).abs() < 1e-6);
    assert!((trail.binding.start_color[0] - 1.0).abs() < 1e-6);
    assert!((trail.binding.end_color[2] - 1.0).abs() < 1e-6);
    assert!((trail.binding.end_color[3] - 0.5).abs() < 1e-6);
    Ok(())
}

#[test]
fn an_unsupported_component_class_reports_one_diagnostic_per_class_id() -> TestResult {
    let assembled = assembled()?;
    let unsupported: Vec<&(EntityId, String)> = assembled
        .diagnostics
        .iter()
        .filter(|(_, message)| message.contains(&format!("class {UNSUPPORTED_CLASS}")))
        .collect();
    assert_eq!(unsupported.len(), 1);
    assert_eq!(
        unsupported[0].1,
        format!("unsupported component class {UNSUPPORTED_CLASS}")
    );
    assert_eq!(assembled.diagnostics.len(), 1);
    Ok(())
}

#[test]
fn a_scene_without_particles_or_trails_yields_empty_bindings() -> TestResult {
    let assembled = assemble_scene_file_bundle(Path::new(SCENE_FIXTURE))?;
    assert!(assembled.particles.is_empty());
    assert!(assembled.trails.is_empty());
    assert!(assembled.diagnostics.is_empty());
    Ok(())
}
