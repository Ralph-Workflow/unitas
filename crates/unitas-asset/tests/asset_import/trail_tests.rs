use std::error::Error;

use unitas_asset::trail::{trail_renderer_component, TrailRendererBinding, TRAIL_RENDERER_CLASS};
use unitas_asset::unity_yaml;
use unitas_core::EntityId;

type TestResult = Result<(), Box<dyn Error>>;

const TRAIL_ENTITY: EntityId = EntityId::new(7);

fn binding_from_yaml(text: &str) -> Result<TrailRendererBinding, Box<dyn Error>> {
    let stream = unity_yaml::parse(text)?;
    let document = stream
        .documents
        .first()
        .ok_or("the stream holds no document")?;
    let body = document
        .root
        .get("TrailRenderer")
        .ok_or("the document holds no TrailRenderer body")?;
    Ok(trail_renderer_component(TRAIL_ENTITY, body))
}

#[test]
fn a_trail_renderer_document_uses_the_trail_renderer_class() -> TestResult {
    let stream = unity_yaml::parse("%YAML 1.1\n--- !u!96 &12\nTrailRenderer:\n  m_Time: 1.0\n")?;
    let document = stream
        .documents
        .first()
        .ok_or("the stream holds no document")?;
    assert_eq!(document.class_id, TRAIL_RENDERER_CLASS);
    Ok(())
}

#[test]
fn a_trail_renderer_document_binds_its_unity_fields() -> TestResult {
    let binding = binding_from_yaml(
        "%YAML 1.1\n--- !u!96 &12\nTrailRenderer:\n  m_GameObject: {fileID: 10}\n  m_Time: 1.5\n  m_Width: 0.4\n  m_MinVertexDistance: 0.25\n  m_StartColor: {r: 1, g: 0.5, b: 0, a: 1}\n  m_EndColor: {r: 0, g: 0, b: 1, a: 0}\n",
    )?;
    assert_eq!(binding.entity, TRAIL_ENTITY);
    assert!((binding.time - 1.5).abs() < 1e-6);
    assert!((binding.width_start - 0.4).abs() < 1e-6);
    assert!((binding.width_end - 0.4).abs() < 1e-6);
    assert!((binding.min_vertex_distance - 0.25).abs() < 1e-6);
    assert!((binding.start_color[0] - 1.0).abs() < 1e-6);
    assert!((binding.start_color[1] - 0.5).abs() < 1e-6);
    assert!((binding.start_color[3] - 1.0).abs() < 1e-6);
    assert!((binding.end_color[2] - 1.0).abs() < 1e-6);
    assert!(binding.end_color[3].abs() < 1e-6);
    Ok(())
}

#[test]
fn a_width_curve_replaces_the_flat_width() -> TestResult {
    let binding = binding_from_yaml(
        "%YAML 1.1\n--- !u!96 &12\nTrailRenderer:\n  m_Width: 0.4\n  m_WidthCurve:\n    start: 1.0\n    end: 0.2\n",
    )?;
    assert!((binding.width_start - 1.0).abs() < 1e-6);
    assert!((binding.width_end - 0.2).abs() < 1e-6);
    Ok(())
}

#[test]
fn a_width_curve_with_keyframes_binds_its_endpoints() -> TestResult {
    let binding = binding_from_yaml(
        "%YAML 1.1\n--- !u!96 &12\nTrailRenderer:\n  m_WidthCurve:\n    m_Curve:\n    - time: 0\n      value: 0.9\n    - time: 1\n      value: 0.1\n",
    )?;
    assert!((binding.width_start - 0.9).abs() < 1e-6);
    assert!((binding.width_end - 0.1).abs() < 1e-6);
    Ok(())
}

#[test]
fn a_trail_renderer_without_fields_falls_back_to_defaults() -> TestResult {
    let binding = binding_from_yaml(
        "%YAML 1.1\n--- !u!96 &12\nTrailRenderer:\n  m_GameObject: {fileID: 10}\n",
    )?;
    assert!((binding.time - 1.0).abs() < 1e-6);
    assert!((binding.width_start - 1.0).abs() < 1e-6);
    assert!((binding.width_end - 1.0).abs() < 1e-6);
    assert!(binding.min_vertex_distance.abs() < 1e-6);
    assert!((binding.start_color[3] - 1.0).abs() < 1e-6);
    assert!((binding.end_color[3] - 1.0).abs() < 1e-6);
    Ok(())
}

#[test]
fn a_negative_trail_time_falls_back_to_the_default() -> TestResult {
    let binding = binding_from_yaml(
        "%YAML 1.1\n--- !u!96 &12\nTrailRenderer:\n  m_Time: -2.0\n  m_MinVertexDistance: -1.0\n",
    )?;
    assert!((binding.time - 1.0).abs() < 1e-6);
    assert!(binding.min_vertex_distance.abs() < 1e-6);
    Ok(())
}
