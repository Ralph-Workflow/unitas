use std::error::Error;
use std::path::Path;

use unitas_render::cube;
use unitas_runtime::{
    Runtime, test_read_error_message, test_read_path_buffer, test_read_render_buffer,
    test_write_path_buffer, unitas_runtime_load_scene, unitas_runtime_new,
    unitas_runtime_path_buffer, unitas_runtime_render_buffer, unitas_runtime_render_camera_create,
    unitas_runtime_render_camera_screen_point_to_ray, unitas_runtime_render_camera_update,
    unitas_runtime_render_camera_world_to_screen_point, unitas_runtime_render_frame,
    unitas_runtime_render_light_create, unitas_runtime_render_light_update,
    unitas_runtime_render_material_create, unitas_runtime_render_material_set_color,
    unitas_runtime_render_mesh_create, unitas_runtime_render_mesh_destroy,
    unitas_runtime_render_renderer_create, unitas_runtime_render_renderer_set_enabled,
};

const RENDER_SCENE: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: MainCamera
--- !u!4 &1001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: -10}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!20 &1002
Camera:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  m_Enabled: 1
  m_BackGroundColor: {r: 0.1, g: 0.2, b: 0.3, a: 0}
  near clip plane: 0.3
  far clip plane: 1000
  field of view: 60
  orthographic: 0
  orthographic size: 5
--- !u!1 &2000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: KeyLight
--- !u!4 &2001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 2000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 5, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!108 &2002
Light:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 2000}
  m_Enabled: 1
  m_Type: 1
  m_Color: {r: 1, g: 1, b: 1, a: 1}
  m_Intensity: 1
--- !u!1 &3000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: Cube
--- !u!4 &3001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 3000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!33 &3002
MeshFilter:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 3000}
  m_Mesh: {fileID: 10202, guid: 0000000000000000e000000000000000, type: 0}
--- !u!23 &3003
MeshRenderer:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 3000}
  m_Enabled: 1
  m_Materials:
  - {fileID: 10303, guid: 0000000000000000f000000000000000, type: 0}
";

const BACKGROUND: [u8; 3] = [26, 51, 77];
const FRAME_SIZE: u32 = 64;
const PATH_BUFFER_SIZE: u64 = 4096;

fn new_handle() -> Result<u64, Box<dyn Error>> {
    let handle = unitas_runtime_new();
    if handle == 0 {
        return Err(String::from("runtime handle creation failed").into());
    }
    Ok(handle)
}

fn path_buffer(handle: u64) -> Result<u64, Box<dyn Error>> {
    allocate_path_buffer(handle, PATH_BUFFER_SIZE)
}

fn allocate_path_buffer(handle: u64, size: u64) -> Result<u64, Box<dyn Error>> {
    let address = unitas_runtime_path_buffer(handle, size);
    if address == 0 {
        return Err(String::from("path buffer allocation failed").into());
    }
    Ok(address)
}

fn write_render_scene(name: &str) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let dir = std::env::temp_dir().join(format!("unitas_render_{}_{}", std::process::id(), name));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("render_scene.unity");
    std::fs::write(&path, RENDER_SCENE)?;
    Ok(path)
}

fn scene_path_text(path: &Path) -> Result<String, Box<dyn Error>> {
    match path.to_str() {
        Some(text) => Ok(text.to_string()),
        None => Err(String::from("non-utf8 temp path").into()),
    }
}

fn entity_id_for(path: &str, name: &str) -> Result<u64, Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(path))?;
    let graph = match runtime.scene_graph() {
        Some(graph) => graph,
        None => return Err(String::from("no scene graph found").into()),
    };
    for id in graph.breadth_first_order() {
        if let Some(entity) = graph.entity(id)
            && entity.name == name
        {
            return Ok(id.as_u64());
        }
    }
    Err(String::from("named entity not found in scene").into())
}

fn load_render_scene(handle: u64, path: &str) -> Result<(), Box<dyn Error>> {
    let address = path_buffer(handle)?;
    let bytes = path.as_bytes();
    test_write_path_buffer(handle, bytes)?;
    let code = unitas_runtime_load_scene(handle, address, u64::try_from(bytes.len())?);
    if code != 0 {
        return Err(format!("scene load failed with code {code}").into());
    }
    Ok(())
}

fn cube_mesh_bytes() -> (Vec<u8>, u32, u32, u32) {
    let mesh = cube(1.0);
    let mut bytes = Vec::new();
    for vertex in &mesh.positions {
        for value in [vertex.x, vertex.y, vertex.z] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    let position_bytes = bytes.len() as u32;
    for normal in &mesh.normals {
        for value in [normal.x, normal.y, normal.z] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    let normal_bytes = bytes.len() as u32 - position_bytes;
    for index in &mesh.indices {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    let index_bytes = bytes.len() as u32 - position_bytes - normal_bytes;
    (bytes, position_bytes / 4, normal_bytes / 4, index_bytes / 4)
}

fn create_cube_mesh(handle: u64, base: u64) -> Result<u64, Box<dyn Error>> {
    let (bytes, position_count, normal_count, index_count) = cube_mesh_bytes();
    test_write_path_buffer(handle, &bytes)?;
    let mesh = unitas_runtime_render_mesh_create(
        handle,
        base as *const f32,
        position_count,
        (base + u64::from(position_count * 4)) as *const f32,
        normal_count,
        (base + u64::from((position_count + normal_count) * 4)) as *const u32,
        index_count,
    );
    Ok(mesh)
}

fn camera_blob_bytes() -> Vec<u8> {
    let values = [0.0f32, 60.0, 0.3, 1000.0, 0.1, 0.2, 0.3, 0.0];
    let mut bytes = Vec::new();
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn write_camera_blob(handle: u64, offset: u64) -> Result<*const f32, Box<dyn Error>> {
    let base = path_buffer(handle)?;
    let bytes = camera_blob_bytes();
    let mut padded = vec![0u8; usize::try_from(offset)?];
    padded.extend_from_slice(&bytes);
    test_write_path_buffer(handle, &padded)?;
    Ok((base + offset) as *const f32)
}

fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[offset..offset + 4]);
    f32::from_le_bytes(raw)
}

fn bmp_pixel(bytes: &[u8], x: u32, y_top_down: u32) -> [u8; 3] {
    let row = FRAME_SIZE - 1 - y_top_down;
    let offset = 54 + (row as usize) * (FRAME_SIZE as usize * 3) + (x as usize) * 3;
    [bytes[offset + 2], bytes[offset + 1], bytes[offset]]
}

fn lit_pixel_bounds(bytes: &[u8]) -> Option<(u32, u32, u32, u32)> {
    let mut min_x = FRAME_SIZE;
    let mut max_x = 0u32;
    let mut min_y = FRAME_SIZE;
    let mut max_y = 0u32;
    let mut found = false;
    for y in 0..FRAME_SIZE {
        for x in 0..FRAME_SIZE {
            if bmp_pixel(bytes, x, y) != BACKGROUND {
                found = true;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    if found {
        Some((min_x, max_x, min_y, max_y))
    } else {
        None
    }
}

#[test]
fn mesh_create_rejects_mismatched_counts() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    let base = path_buffer(handle)?;
    let (bytes, position_count, normal_count, index_count) = cube_mesh_bytes();
    test_write_path_buffer(handle, &bytes)?;
    let mesh = unitas_runtime_render_mesh_create(
        handle,
        base as *const f32,
        position_count - 1,
        (base + u64::from(position_count * 4)) as *const f32,
        normal_count,
        (base + u64::from((position_count + normal_count) * 4)) as *const u32,
        index_count,
    );
    assert_eq!(mesh, 0);
    let message = test_read_error_message(handle)?;
    assert!(
        message.contains("-2"),
        "error message must carry the -2 class: {message}"
    );
    Ok(())
}

#[test]
fn mesh_create_accepts_matching_counts() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    let base = path_buffer(handle)?;
    let mesh = create_cube_mesh(handle, base)?;
    assert_ne!(mesh, 0);
    assert_eq!(unitas_runtime_render_mesh_destroy(handle, mesh), 0);
    assert_eq!(unitas_runtime_render_mesh_destroy(handle, mesh), -1);
    Ok(())
}

#[test]
fn material_set_color_and_camera_update_accept_valid_arguments() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    let material = unitas_runtime_render_material_create(handle);
    assert_ne!(material, 0);
    assert_eq!(
        unitas_runtime_render_material_set_color(handle, material, 0.2, 0.4, 0.6, 1.0),
        0
    );
    let camera = unitas_runtime_render_camera_create(handle, 1);
    assert_ne!(camera, 0);
    let blob = write_camera_blob(handle, 32)?;
    assert_eq!(unitas_runtime_render_camera_update(handle, camera, blob), 0);
    Ok(())
}

#[test]
fn camera_update_rejects_short_blob() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    let camera = unitas_runtime_render_camera_create(handle, 1);
    assert_ne!(camera, 0);
    let base = allocate_path_buffer(handle, 16)?;
    let short = vec![0u8; 16];
    test_write_path_buffer(handle, &short)?;
    assert_eq!(
        unitas_runtime_render_camera_update(handle, camera, base as *const f32),
        -2
    );
    let message = test_read_error_message(handle)?;
    assert!(
        message.contains("-2"),
        "error message must carry the -2 class: {message}"
    );
    Ok(())
}

#[test]
fn render_frame_produces_bmp_for_scene_with_camera_and_renderer() -> Result<(), Box<dyn Error>> {
    let scene = write_render_scene("frame")?;
    let scene_text = scene_path_text(&scene)?;
    let handle = new_handle()?;
    load_render_scene(handle, &scene_text)?;
    let base = path_buffer(handle)?;
    let mesh = create_cube_mesh(handle, base)?;
    assert_ne!(mesh, 0);
    let material = unitas_runtime_render_material_create(handle);
    assert_ne!(material, 0);
    let camera_entity = entity_id_for(&scene_text, "MainCamera")?;
    let cube_entity = entity_id_for(&scene_text, "Cube")?;
    let light_entity = entity_id_for(&scene_text, "KeyLight")?;
    let camera = unitas_runtime_render_camera_create(handle, camera_entity);
    assert_ne!(camera, 0);
    let blob = write_camera_blob(handle, 1024)?;
    assert_eq!(unitas_runtime_render_camera_update(handle, camera, blob), 0);
    let renderer = unitas_runtime_render_renderer_create(handle, cube_entity, mesh, material);
    assert_ne!(renderer, 0);
    assert_eq!(
        unitas_runtime_render_renderer_set_enabled(handle, renderer, 1),
        0
    );
    let light = unitas_runtime_render_light_create(handle, light_entity);
    assert_ne!(light, 0);
    assert_eq!(
        unitas_runtime_render_light_update(handle, light, 0.0, 0.3, -1.0, 1.0, 1.0, 1.0, 1.0),
        0
    );
    let count = unitas_runtime_render_frame(handle, camera, FRAME_SIZE, FRAME_SIZE);
    assert!(count >= 0, "render frame must succeed: {count}");
    let buffer_address = unitas_runtime_render_buffer(handle, u64::try_from(count)?);
    assert_ne!(buffer_address, 0);
    let mut bytes = vec![0u8; usize::try_from(count)?];
    test_read_render_buffer(handle, &mut bytes)?;
    assert_eq!(&bytes[0..2], b"BM");
    let width = u32::from_le_bytes(bytes[18..22].try_into()?);
    let height = u32::from_le_bytes(bytes[22..26].try_into()?);
    assert_eq!(width, FRAME_SIZE);
    assert_eq!(height, FRAME_SIZE);
    assert!(
        lit_pixel_bounds(&bytes).is_some(),
        "the rendered frame must contain geometry"
    );
    let _ = std::fs::remove_dir_all(scene.parent().unwrap_or(&scene));
    Ok(())
}

#[test]
fn render_frame_rejects_unknown_camera() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    let count = unitas_runtime_render_frame(handle, 987654, FRAME_SIZE, FRAME_SIZE);
    assert!(count < 0, "unknown camera must fail: {count}");
    let message = test_read_error_message(handle)?;
    assert!(
        message.contains("-1"),
        "error message must carry the -1 class: {message}"
    );
    Ok(())
}

#[test]
fn world_to_screen_point_agrees_with_rendered_geometry() -> Result<(), Box<dyn Error>> {
    let scene = write_render_scene("world_to_screen")?;
    let scene_text = scene_path_text(&scene)?;
    let handle = new_handle()?;
    load_render_scene(handle, &scene_text)?;
    let base = path_buffer(handle)?;
    let mesh = create_cube_mesh(handle, base)?;
    let material = unitas_runtime_render_material_create(handle);
    let camera_entity = entity_id_for(&scene_text, "MainCamera")?;
    let cube_entity = entity_id_for(&scene_text, "Cube")?;
    let camera = unitas_runtime_render_camera_create(handle, camera_entity);
    let blob = write_camera_blob(handle, 1024)?;
    assert_eq!(unitas_runtime_render_camera_update(handle, camera, blob), 0);
    let renderer = unitas_runtime_render_renderer_create(handle, cube_entity, mesh, material);
    assert_ne!(renderer, 0);
    let count = unitas_runtime_render_frame(handle, camera, FRAME_SIZE, FRAME_SIZE);
    assert!(count >= 0, "render frame must succeed: {count}");
    let mut bytes = vec![0u8; usize::try_from(count)?];
    test_read_render_buffer(handle, &mut bytes)?;
    let (min_x, max_x, min_y, max_y) = match lit_pixel_bounds(&bytes) {
        Some(bounds) => bounds,
        None => return Err(String::from("the rendered frame has no geometry").into()),
    };
    let image_center_x = (min_x + max_x) as f32 * 0.5;
    let image_center_y = (min_y + max_y) as f32 * 0.5;
    assert!(
        (image_center_x - FRAME_SIZE as f32 * 0.5).abs() <= 1.5,
        "the cube must project to the frame centre: {image_center_x}"
    );
    assert!(
        (image_center_y - FRAME_SIZE as f32 * 0.5).abs() <= 1.5,
        "the cube must project to the frame centre: {image_center_y}"
    );
    let out = path_buffer(handle)? as *mut f32;
    assert_eq!(
        unitas_runtime_render_camera_world_to_screen_point(handle, camera, 0.0, 0.0, 0.0, out),
        0
    );
    let mut result = [0u8; 12];
    test_read_path_buffer(handle, &mut result)?;
    let screen_x = read_f32(&result, 0);
    let screen_y = read_f32(&result, 4);
    let distance = read_f32(&result, 8);
    assert!(
        (screen_x - FRAME_SIZE as f32 * 0.5).abs() <= 0.5,
        "world to screen x must be the frame centre: {screen_x}"
    );
    assert!(
        (screen_y - FRAME_SIZE as f32 * 0.5).abs() <= 0.5,
        "world to screen y must be the frame centre: {screen_y}"
    );
    assert!(
        (screen_x - image_center_x).abs() <= 1.5,
        "world to screen x must agree with the rendered image: {screen_x}"
    );
    assert!(
        (screen_y - image_center_y).abs() <= 1.5,
        "world to screen y must agree with the rendered image: {screen_y}"
    );
    assert!(
        (distance - 10.0).abs() <= 0.01,
        "the cube centre is ten units in front of the camera: {distance}"
    );
    let _ = std::fs::remove_dir_all(scene.parent().unwrap_or(&scene));
    Ok(())
}

#[test]
fn screen_point_to_ray_points_from_the_camera() -> Result<(), Box<dyn Error>> {
    let scene = write_render_scene("screen_to_ray")?;
    let scene_text = scene_path_text(&scene)?;
    let handle = new_handle()?;
    load_render_scene(handle, &scene_text)?;
    let camera_entity = entity_id_for(&scene_text, "MainCamera")?;
    let camera = unitas_runtime_render_camera_create(handle, camera_entity);
    let blob = write_camera_blob(handle, 1024)?;
    assert_eq!(unitas_runtime_render_camera_update(handle, camera, blob), 0);
    assert!(unitas_runtime_render_frame(handle, camera, FRAME_SIZE, FRAME_SIZE) >= 0);
    let out = path_buffer(handle)? as *mut f32;
    assert_eq!(
        unitas_runtime_render_camera_screen_point_to_ray(
            handle,
            camera,
            FRAME_SIZE as f32 * 0.5,
            FRAME_SIZE as f32 * 0.5,
            0.0,
            out
        ),
        0
    );
    let mut result = [0u8; 24];
    test_read_path_buffer(handle, &mut result)?;
    let origin = [
        read_f32(&result, 0),
        read_f32(&result, 4),
        read_f32(&result, 8),
    ];
    let direction = [
        read_f32(&result, 12),
        read_f32(&result, 16),
        read_f32(&result, 20),
    ];
    assert_eq!(origin, [0.0, 0.0, -10.0]);
    let length =
        (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2])
            .sqrt();
    assert!(
        (length - 1.0).abs() <= 1e-4,
        "ray direction must be normalised: {length}"
    );
    let _ = std::fs::remove_dir_all(scene.parent().unwrap_or(&scene));
    Ok(())
}

#[test]
fn renderer_without_mesh_reports_diagnostic() -> Result<(), Box<dyn Error>> {
    let scene = write_render_scene("diagnostic")?;
    let scene_text = scene_path_text(&scene)?;
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(&scene_text))?;
    let cube_entity = entity_id_for(&scene_text, "Cube")?;
    let diagnostics = runtime.diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|(entity, message)| entity.as_u64() == cube_entity
                && message.to_lowercase().contains("mesh")),
        "the unimportable mesh reference must produce a diagnostic: {diagnostics:?}"
    );
    let _ = std::fs::remove_dir_all(scene.parent().unwrap_or(&scene));
    Ok(())
}
