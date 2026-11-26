use std::error::Error;
use std::path::Path;

use unitas_runtime::{
    Runtime, test_read_render_buffer, test_write_path_buffer, unitas_runtime_load_scene,
    unitas_runtime_new, unitas_runtime_path_buffer, unitas_runtime_render_buffer,
    unitas_runtime_render_camera_create, unitas_runtime_render_camera_update,
    unitas_runtime_render_frame, unitas_runtime_render_light_create,
    unitas_runtime_render_light_update, unitas_runtime_tick,
};

const EFFECTS_RENDER_FIXTURE: &str = "tests/fixtures/scene_effects_render.unity";
const FRAME_SIZE: u32 = 128;
const BACKGROUND: [u8; 3] = [26, 51, 77];
const PATH_BUFFER_SIZE: u64 = 4096;

fn new_handle() -> Result<u64, Box<dyn Error>> {
    let handle = unitas_runtime_new();
    if handle == 0 {
        return Err(String::from("runtime handle creation failed").into());
    }
    Ok(handle)
}

fn path_buffer(handle: u64) -> Result<u64, Box<dyn Error>> {
    let address = unitas_runtime_path_buffer(handle, PATH_BUFFER_SIZE);
    if address == 0 {
        return Err(String::from("path buffer allocation failed").into());
    }
    Ok(address)
}

fn load_fixture(handle: u64) -> Result<(), Box<dyn Error>> {
    let address = path_buffer(handle)?;
    let bytes = EFFECTS_RENDER_FIXTURE.as_bytes();
    test_write_path_buffer(handle, bytes)?;
    let code = unitas_runtime_load_scene(handle, address, u64::try_from(bytes.len())?);
    if code != 0 {
        return Err(format!("scene load failed with code {code}").into());
    }
    Ok(())
}

fn entity_id_for(name: &str) -> Result<u64, Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(EFFECTS_RENDER_FIXTURE))?;
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

fn setup_camera(handle: u64) -> Result<u64, Box<dyn Error>> {
    let camera_entity = entity_id_for("MainCamera")?;
    let camera = unitas_runtime_render_camera_create(handle, camera_entity);
    if camera == 0 {
        return Err(String::from("camera creation failed").into());
    }
    let base = path_buffer(handle)?;
    let values = [0.0f32, 60.0, 0.3, 1000.0, 0.1, 0.2, 0.3, 0.0];
    let mut bytes = Vec::new();
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    test_write_path_buffer(handle, &bytes)?;
    let code = unitas_runtime_render_camera_update(handle, camera, base as *const f32);
    if code != 0 {
        return Err(format!("camera update failed with code {code}").into());
    }
    Ok(camera)
}

fn setup_light(handle: u64) -> Result<(), Box<dyn Error>> {
    let light_entity = entity_id_for("KeyLight")?;
    let light = unitas_runtime_render_light_create(handle, light_entity);
    if light == 0 {
        return Err(String::from("light creation failed").into());
    }
    let code = unitas_runtime_render_light_update(handle, light, 0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0);
    if code != 0 {
        return Err(format!("light update failed with code {code}").into());
    }
    Ok(())
}

fn bmp_pixel(bytes: &[u8], x: u32, y_top_down: u32) -> [u8; 3] {
    let row = FRAME_SIZE - 1 - y_top_down;
    let stride = (FRAME_SIZE as usize * 3 + 3) & !3;
    let offset = 54 + (row as usize) * stride + (x as usize) * 3;
    [bytes[offset + 2], bytes[offset + 1], bytes[offset]]
}

fn non_background_pixels(bytes: &[u8]) -> Vec<[u8; 3]> {
    let mut pixels = Vec::new();
    for y in 0..FRAME_SIZE {
        for x in 0..FRAME_SIZE {
            let pixel = bmp_pixel(bytes, x, y);
            if pixel != BACKGROUND {
                pixels.push(pixel);
            }
        }
    }
    pixels
}

fn render_frame(handle: u64, camera: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    let count = unitas_runtime_render_frame(handle, camera, FRAME_SIZE, FRAME_SIZE);
    if count < 0 {
        return Err(format!("render frame failed with code {count}").into());
    }
    let address = unitas_runtime_render_buffer(handle, u64::try_from(count)?);
    if address == 0 {
        return Err(String::from("render buffer allocation failed").into());
    }
    let mut bytes = vec![0u8; usize::try_from(count)?];
    test_read_render_buffer(handle, &mut bytes)?;
    Ok(bytes)
}

#[test]
fn the_render_frame_of_an_unticked_effect_scene_holds_no_geometry() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    load_fixture(handle)?;
    let camera = setup_camera(handle)?;
    setup_light(handle)?;

    let bytes = render_frame(handle, camera)?;

    assert!(
        non_background_pixels(&bytes).is_empty(),
        "the scene holds no renderer, so the frame must stay background"
    );
    Ok(())
}

#[test]
fn the_render_frame_draws_the_particle_and_trail_effects() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    load_fixture(handle)?;
    let camera = setup_camera(handle)?;
    setup_light(handle)?;

    for _ in 0..4 {
        let code = unitas_runtime_tick(handle, 0.05);
        if code != 0 {
            return Err(format!("tick failed with code {code}").into());
        }
    }

    let bytes = render_frame(handle, camera)?;
    let pixels = non_background_pixels(&bytes);

    assert!(
        !pixels.is_empty(),
        "the ticked particle and trail effects must draw pixels over the background"
    );
    assert!(
        pixels
            .iter()
            .any(|pixel| pixel[0] > 100 && pixel[2] < 100),
        "an effect pixel must carry the particle or trail colour: {pixels:?}"
    );
    Ok(())
}
