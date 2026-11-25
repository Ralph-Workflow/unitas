#![cfg(feature = "device")]

use unitas_render::RenderError;
use unitas_render::capture::{CaptureFrame, CaptureInstance};

use crate::runtime::Runtime;
use crate::window::{GpuFramePresenter, transform_to_mat4};

const UNNAMED_OBJECT: &str = "object";

#[derive(Debug)]
pub enum CaptureError {
    Render(RenderError),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureError::Render(error) => write!(formatter, "capture render failed: {error}"),
        }
    }
}

impl std::error::Error for CaptureError {}

impl From<RenderError> for CaptureError {
    fn from(error: RenderError) -> Self {
        CaptureError::Render(error)
    }
}

pub fn capture_offscreen(
    presenter: &GpuFramePresenter<'_>,
    runtime: &Runtime,
    width: u32,
    height: u32,
) -> Result<CaptureFrame, CaptureError> {
    let (camera, view, background) = presenter.resolved_camera_and_view(runtime, width, height);
    let light = *presenter.light();
    let instances = capture_instances(presenter, runtime);
    let frame = presenter.renderer().render_offscreen_with_ids(
        &instances,
        &camera,
        view,
        &light,
        background,
        (width, height),
    )?;
    Ok(frame)
}

fn capture_instances<'a>(
    presenter: &'a GpuFramePresenter<'_>,
    runtime: &'a Runtime,
) -> Vec<CaptureInstance<'a>> {
    let mut instances = Vec::with_capacity(presenter.meshes().len());
    for entry in presenter.meshes() {
        if !entry.enabled {
            continue;
        }
        let model = match entry
            .entity
            .and_then(|entity| runtime.world_transform_of(entity))
        {
            Some(transform) => transform_to_mat4(&transform),
            None => entry.model,
        };
        let name = entry
            .entity
            .and_then(|entity| runtime.scene_graph().and_then(|graph| graph.entity(entity)))
            .map(|entity| entity.name())
            .unwrap_or(UNNAMED_OBJECT);
        instances.push(CaptureInstance {
            name,
            mesh: &entry.mesh,
            model,
            color: entry.color,
            lit: entry.lit,
        });
    }
    instances
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use unitas_core::EntityId;
    use unitas_render::mesh::cube;

    const SCENE: &str = "tests/fixtures/scene_with_script.unity";
    const WIDTH: u32 = 64;
    const HEIGHT: u32 = 64;

    fn presenter_or_skip() -> Option<GpuFramePresenter<'static>> {
        match GpuFramePresenter::from_new_renderer() {
            Ok(presenter) => Some(presenter),
            Err(error) => {
                eprintln!("skipping runtime capture test, no GPU device available: {error}");
                None
            }
        }
    }

    #[test]
    fn capture_names_objects_after_their_scene_entities() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut presenter = match presenter_or_skip() {
            Some(presenter) => presenter,
            None => return Ok(()),
        };
        let mut runtime = Runtime::new()?;
        runtime.load_scene(Path::new(SCENE))?;
        let entity = runtime
            .find_entity_by_name("ScriptedObject")
            .ok_or("scene entity ScriptedObject was not loaded")?;
        presenter.set_meshes(vec![crate::window::GpuMeshBinding::for_entity(
            entity,
            cube(1.0),
            [0.2, 0.4, 0.8],
            true,
        )]);
        let frame = capture_offscreen(&presenter, &runtime, WIDTH, HEIGHT)?;
        assert_eq!(frame.id_map.len(), (WIDTH * HEIGHT) as usize);
        assert_eq!(frame.objects.len(), 1);
        assert_eq!(frame.objects[0].id, 1);
        assert_eq!(frame.objects[0].name, "ScriptedObject");
        assert!(frame.objects[0].bounds.width > 0);
        assert!(frame.objects[0].bounds.height > 0);
        Ok(())
    }

    #[test]
    fn capture_of_a_standalone_mesh_uses_the_unnamed_object_name()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut presenter = match presenter_or_skip() {
            Some(presenter) => presenter,
            None => return Ok(()),
        };
        let runtime = Runtime::new()?;
        presenter.set_meshes(vec![crate::window::GpuMeshBinding::standalone(
            cube(1.0),
            unitas_render::Mat4::identity(),
            [0.2, 0.4, 0.8],
            true,
        )]);
        let frame = capture_offscreen(&presenter, &runtime, WIDTH, HEIGHT)?;
        assert_eq!(frame.objects.len(), 1);
        assert_eq!(frame.objects[0].name, UNNAMED_OBJECT);
        Ok(())
    }

    #[test]
    fn capture_error_reports_the_render_failure() {
        let error = CaptureError::from(RenderError::InvalidSize(0, 0));
        assert_eq!(
            error.to_string(),
            "capture render failed: invalid render target size 0x0"
        );
        let _ = EntityId::new(0);
    }
}
