use unitas_render::RenderError;
use unitas_render::capture::{CaptureFrame, CaptureInstance};
use unitas_render::ui_pass::{GlyphTextPainter, composite_ui_onto_capture};

use crate::presenter::{
    GpuFramePresenter, effect_model_mat4, scene_gpu_meshes, transform_to_mat4,
};
use crate::runtime::Runtime;

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
    presenter: &mut GpuFramePresenter<'_>,
    runtime: &Runtime,
    width: u32,
    height: u32,
) -> Result<CaptureFrame, CaptureError> {
    if presenter.meshes().is_empty() {
        presenter.set_meshes(scene_gpu_meshes(runtime));
    }
    let (camera, view, background) = presenter.resolved_camera_and_view(runtime, width, height);
    let light = *presenter.light();
    let instances = capture_instances(presenter, runtime);
    let mut frame = presenter.renderer().render_offscreen_with_ids(
        &instances,
        &camera,
        view,
        &light,
        background,
        (width, height),
    )?;
    let ui_items = runtime.ui_draw_items_for([width as f32, height as f32]);
    if !ui_items.is_empty() {
        if ui_items.iter().any(|item| item.text.is_some()) {
            let mut painter = GlyphTextPainter::new();
            composite_ui_onto_capture(&mut frame, &ui_items, Some(&mut painter));
        } else {
            composite_ui_onto_capture(&mut frame, &ui_items, None);
        }
    }
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
            None if entry.entity.is_some() => continue,
            None => entry.model,
        };
        let name = entry
            .entity
            .and_then(|entity| runtime.entity_name(entity))
            .unwrap_or(UNNAMED_OBJECT);
        instances.push(CaptureInstance {
            name,
            mesh: &entry.mesh,
            model,
            color: entry.color,
            lit: entry.lit,
        });
    }
    let effect_mesh = runtime.effect_mesh();
    for item in runtime.particle_draw_list() {
        let name = runtime
            .entity_name(item.entity)
            .unwrap_or(UNNAMED_OBJECT);
        instances.push(CaptureInstance {
            name,
            mesh: effect_mesh,
            model: effect_model_mat4(item.position, item.size),
            color: [item.color[0], item.color[1], item.color[2]],
            lit: true,
        });
    }
    for item in runtime.trail_draw_list() {
        let name = runtime
            .entity_name(item.entity)
            .unwrap_or(UNNAMED_OBJECT);
        instances.push(CaptureInstance {
            name,
            mesh: effect_mesh,
            model: effect_model_mat4(item.position, item.width),
            color: [item.color[0], item.color[1], item.color[2]],
            lit: true,
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
        presenter.set_meshes(vec![crate::presenter::GpuMeshBinding::for_entity(
            entity,
            cube(1.0),
            [0.2, 0.4, 0.8],
            true,
        )]);
        let frame = capture_offscreen(&mut presenter, &runtime, WIDTH, HEIGHT)?;
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
        presenter.set_meshes(vec![crate::presenter::GpuMeshBinding::standalone(
            cube(1.0),
            unitas_render::Mat4::identity(),
            [0.2, 0.4, 0.8],
            true,
        )]);
        let frame = capture_offscreen(&mut presenter, &runtime, WIDTH, HEIGHT)?;
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

    #[test]
    fn headless_entry_point_and_presenter_offscreen_produce_byte_identical_bmps()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut presenter = match presenter_or_skip() {
            Some(presenter) => presenter,
            None => return Ok(()),
        };
        let mut runtime = Runtime::new()?;
        runtime.load_scene(Path::new(SCENE))?;
        let entity = runtime
            .find_entity_by_name("ScriptedObject")
            .ok_or("scene entity ScriptedObject was not loaded")?;
        let mesh = cube(1.0);
        let positions: Vec<unitas_render::Vec3> = mesh.positions.clone();
        let normals: Vec<unitas_render::Vec3> = mesh.normals.clone();
        let indices: Vec<u32> = mesh.indices.clone();
        let mesh_handle = runtime.register_render_mesh(positions, normals, indices);
        let material_handle = runtime.register_render_material([0.2, 0.4, 0.8, 1.0]);
        let _renderer_handle =
            runtime.register_render_renderer(Some(entity), Some(mesh_handle), Some(material_handle));
        let camera_handle = runtime.register_render_camera(None);
        presenter.bind_scene(&runtime);
        let headless_bmp = runtime
            .render_frame(camera_handle, WIDTH, HEIGHT)
            .map_err(|error| error.to_string())?;
        let presenter_bmp = presenter
            .render_offscreen(&runtime, WIDTH, HEIGHT)?
            .to_bmp_bytes();
        assert_eq!(headless_bmp, presenter_bmp);
        Ok(())
    }

    #[test]
    fn headless_id_map_resolves_known_object_inside_bounds()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut presenter = match presenter_or_skip() {
            Some(presenter) => presenter,
            None => return Ok(()),
        };
        let mut runtime = Runtime::new()?;
        runtime.load_scene(Path::new(SCENE))?;
        let entity = runtime
            .find_entity_by_name("ScriptedObject")
            .ok_or("scene entity ScriptedObject was not loaded")?;
        let mesh = cube(1.0);
        let mesh_handle =
            runtime.register_render_mesh(mesh.positions.clone(), mesh.normals.clone(), mesh.indices.clone());
        let material_handle = runtime.register_render_material([0.2, 0.4, 0.8, 1.0]);
        let _renderer_handle =
            runtime.register_render_renderer(Some(entity), Some(mesh_handle), Some(material_handle));
        let camera_handle = runtime.register_render_camera(None);
        presenter.bind_scene(&runtime);
        let frame = capture_offscreen(&mut presenter, &runtime, WIDTH, HEIGHT)?;
        let id_bytes = runtime
            .render_id_map(camera_handle, WIDTH, HEIGHT)
            .map_err(|error| error.to_string())?;
        assert_eq!(id_bytes.len(), (WIDTH * HEIGHT * 4) as usize);
        let mut ids = Vec::with_capacity((WIDTH * HEIGHT) as usize);
        for chunk in id_bytes.chunks_exact(4) {
            ids.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
        assert_eq!(ids.len(), (WIDTH * HEIGHT) as usize);
        assert_eq!(ids, frame.id_map);
        let object = frame.objects.first().ok_or("expected captured object")?;
        let found = (object.bounds.y..object.bounds.y + object.bounds.height).any(|y| {
            (object.bounds.x..object.bounds.x + object.bounds.width)
                .any(|x| ids[(y * WIDTH + x) as usize] == object.id)
        });
        assert!(found);
        Ok(())
    }
}
