use std::path::PathBuf;

use unitas_asset::scene::SpriteDrawMode;

use super::{
    CAMERA_BLOB_FLOATS, CAMERA_BLOB_SIZE, EntityId, RenderCameraParams, Vec3, path_buffer_offset,
    read_f32_le, read_u32_le, with_runtime_state,
};

/// The floats a sprite state travels in: colour, sorting layer and order, the
/// flips, the enabled flag, the texture flag and the texture size in pixels.
const SPRITE_STATE_FLOATS: usize = 12;
const SPRITE_STATE_BYTES: usize = SPRITE_STATE_FLOATS * 4;

ffi_export! {
pub extern "C" fn unitas_runtime_render_buffer(handle: u64, capacity: u64) -> u64 {
    if capacity == 0 {
        return 0;
    }
    let Ok(size) = usize::try_from(capacity) else {
        return 0;
    };
    with_runtime_state(handle, |state| {
        if state.render_buffer.len() < size {
            state.render_buffer.resize(size, 0);
        }
        state.render_buffer.as_ptr() as usize as u64
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_buffer_word(handle: u64, offset: u64) -> u64 {
    let Ok(start) = usize::try_from(offset) else {
        return 0;
    };
    with_runtime_state(handle, |state| {
        if start >= state.render_buffer.len() {
            return 0;
        }
        let end = (start + 8).min(state.render_buffer.len());
        let mut chunk = [0u8; 8];
        chunk[..end - start].copy_from_slice(&state.render_buffer[start..end]);
        u64::from_le_bytes(chunk)
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_capture_scene(
    handle: u64,
    width: u32,
    height: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        match state.runtime.capture_scene_frame(width, height) {
            Ok(frame) => {
                let bytes = encode_capture_frame(&frame);
                let count = bytes.len();
                state.render_buffer = bytes;
                state.clear_error();
                match i32::try_from(count) {
                    Ok(value) => value,
                    Err(_) => {
                        state.set_error(
                            "-2 invalid argument: capture size exceeds the supported range",
                        );
                        -2
                    }
                }
            }
            Err(error) => {
                state.set_error(&error.to_string());
                error.class()
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

fn encode_capture_frame(frame: &unitas_render::CaptureFrame) -> Vec<u8> {
    let image_bytes = frame.image.as_bytes();
    let id_count = frame.id_map.len();
    let object_count = frame.objects.len();
    let mut bytes =
        Vec::with_capacity(16 + image_bytes.len() + (id_count * 4) + (object_count * 32));
    bytes.extend_from_slice(&frame.image.width.to_le_bytes());
    bytes.extend_from_slice(&frame.image.height.to_le_bytes());
    bytes.extend_from_slice(&(id_count as u32).to_le_bytes());
    bytes.extend_from_slice(&(object_count as u32).to_le_bytes());
    bytes.extend_from_slice(image_bytes);
    for id in &frame.id_map {
        bytes.extend_from_slice(&id.to_le_bytes());
    }
    for object in &frame.objects {
        bytes.extend_from_slice(&object.id.to_le_bytes());
        bytes.extend_from_slice(&object.bounds.x.to_le_bytes());
        bytes.extend_from_slice(&object.bounds.y.to_le_bytes());
        bytes.extend_from_slice(&object.bounds.width.to_le_bytes());
        bytes.extend_from_slice(&object.bounds.height.to_le_bytes());
        let name_bytes = object.name.as_bytes();
        bytes.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
        bytes.extend_from_slice(name_bytes);
    }
    bytes
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_mesh_create(
    handle: u64,
    positions: *const f32,
    position_count: u32,
    normals: *const f32,
    normal_count: u32,
    indices: *const u32,
    index_count: u32,
) -> u64 {
    if !position_count.is_multiple_of(3)
        || !normal_count.is_multiple_of(3)
        || !index_count.is_multiple_of(3)
        || position_count != normal_count
    {
        return with_runtime_state(handle, |state| {
            state.set_error(
                "-2 invalid argument: mesh vertex, normal and index counts do not match",
            );
            0u64
        })
        .unwrap_or_default();
    }
    with_runtime_state(handle, |state| {
        let position_bytes = position_count as usize * 4;
        let normal_bytes = normal_count as usize * 4;
        let index_bytes = index_count as usize * 4;
        let Some(position_offset) = path_buffer_offset(state, positions as usize, position_bytes)
        else {
            state.set_error("-2 invalid argument: mesh position buffer is out of range");
            return 0u64;
        };
        let Some(normal_offset) = path_buffer_offset(state, normals as usize, normal_bytes) else {
            state.set_error("-2 invalid argument: mesh normal buffer is out of range");
            return 0u64;
        };
        let Some(index_offset) = path_buffer_offset(state, indices as usize, index_bytes) else {
            state.set_error("-2 invalid argument: mesh index buffer is out of range");
            return 0u64;
        };
        let mut position_values = Vec::with_capacity(position_count as usize / 3);
        for index in 0..position_count as usize / 3 {
            let start = position_offset + index * 12;
            position_values.push(Vec3::new(
                read_f32_le(&state.path_buffer, start),
                read_f32_le(&state.path_buffer, start + 4),
                read_f32_le(&state.path_buffer, start + 8),
            ));
        }
        let mut normal_values = Vec::with_capacity(normal_count as usize / 3);
        for index in 0..normal_count as usize / 3 {
            let start = normal_offset + index * 12;
            normal_values.push(Vec3::new(
                read_f32_le(&state.path_buffer, start),
                read_f32_le(&state.path_buffer, start + 4),
                read_f32_le(&state.path_buffer, start + 8),
            ));
        }
        let mut index_values = Vec::with_capacity(index_count as usize);
        for index in 0..index_count as usize {
            index_values.push(read_u32_le(&state.path_buffer, index_offset + index * 4));
        }
        state
            .runtime
            .create_render_mesh(position_values, normal_values, index_values)
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_mesh_destroy(handle: u64, mesh: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        if state.runtime.destroy_render_mesh(mesh) {
            state.clear_error();
            0
        } else {
            state.set_error("-1 unknown handle: mesh handle is not valid");
            -1
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_material_create(handle: u64) -> u64 {
    with_runtime_state(handle, |state| state.runtime.create_render_material()).unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_material_set_color(
    handle: u64,
    material: u64,
    red: f32,
    green: f32,
    blue: f32,
    alpha: f32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if state
            .runtime
            .set_render_material_color(material, [red, green, blue, alpha])
        {
            state.clear_error();
            0
        } else {
            state.set_error("-1 unknown handle: material handle is not valid");
            -1
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_camera_create(handle: u64, entity: u64) -> u64 {
    with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("-2 invalid argument: camera entity id is zero");
            return 0u64;
        }
        state
            .runtime
            .create_render_camera(Some(EntityId::new(entity)))
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_camera_update(
    handle: u64,
    camera: u64,
    blob: *const f32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if blob.is_null() {
            state.set_error("-2 invalid argument: camera blob pointer is null");
            return -2;
        }
        let base = state.path_buffer.as_ptr() as usize;
        let address = blob as usize;
        if address < base {
            state.set_error("-2 invalid argument: camera blob buffer address mismatch");
            return -2;
        }
        let offset = address - base;
        if offset > state.path_buffer.len() {
            state.set_error("-2 invalid argument: camera blob buffer address mismatch");
            return -2;
        }
        if state.path_buffer.len() - offset < CAMERA_BLOB_SIZE {
            state.set_error("-2 invalid argument: camera blob is shorter than 32 bytes");
            return -2;
        }
        let mut values = [0.0f32; CAMERA_BLOB_FLOATS];
        for (index, value) in values.iter_mut().enumerate() {
            *value = read_f32_le(&state.path_buffer, offset + index * 4);
        }
        let params = RenderCameraParams {
            orthographic: values[0] > 0.0,
            orthographic_size: values[0],
            field_of_view: values[1],
            near_clip_plane: values[2],
            far_clip_plane: values[3],
            background_color: [values[4], values[5], values[6]],
        };
        if state.runtime.update_render_camera(camera, params) {
            state.clear_error();
            0
        } else {
            state.set_error("-1 unknown handle: camera handle is not valid");
            -1
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_renderer_create(
    handle: u64,
    entity: u64,
    mesh: u64,
    material: u64,
) -> u64 {
    with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("-2 invalid argument: renderer entity id is zero");
            return 0u64;
        }
        if mesh == 0 {
            state.set_error("-2 invalid argument: renderer mesh handle is zero");
            return 0u64;
        }
        if material == 0 {
            state.set_error("-2 invalid argument: renderer material handle is zero");
            return 0u64;
        }
        state.runtime.create_render_renderer(
            Some(EntityId::new(entity)),
            Some(mesh),
            Some(material),
        )
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_renderer_set_enabled(
    handle: u64,
    renderer: u64,
    enabled: u8,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if state
            .runtime
            .set_render_renderer_enabled(renderer, enabled != 0)
        {
            state.clear_error();
            0
        } else {
            state.set_error("-1 unknown handle: renderer handle is not valid");
            -1
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_light_create(handle: u64, entity: u64) -> u64 {
    with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("-2 invalid argument: light entity id is zero");
            return 0u64;
        }
        state
            .runtime
            .create_render_light(Some(EntityId::new(entity)))
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_light_update(
    handle: u64,
    light: u64,
    direction_x: f32,
    direction_y: f32,
    direction_z: f32,
    color_red: f32,
    color_green: f32,
    color_blue: f32,
    intensity: f32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if state.runtime.update_render_light(
            light,
            [direction_x, direction_y, direction_z],
            [color_red, color_green, color_blue],
            intensity,
        ) {
            state.clear_error();
            0
        } else {
            state.set_error("-1 unknown handle: light handle is not valid");
            -1
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_frame(
    handle: u64,
    camera: u64,
    width: u32,
    height: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        match state.runtime.render_frame(camera, width, height) {
            Ok(bytes) => {
                let count = bytes.len();
                state.render_buffer = bytes;
                state.clear_error();
                match i32::try_from(count) {
                    Ok(value) => value,
                    Err(_) => {
                        state.set_error(
                            "-2 invalid argument: frame size exceeds the supported range",
                        );
                        -2
                    }
                }
            }
            Err(error) => {
                state.set_error(&error.to_string());
                error.class()
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_id_map(
    handle: u64,
    camera: u64,
    width: u32,
    height: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        match state.runtime.render_id_map(camera, width, height) {
            Ok(bytes) => {
                let count = bytes.len();
                state.render_buffer = bytes;
                state.clear_error();
                match i32::try_from(count) {
                    Ok(value) => value,
                    Err(_) => {
                        state.set_error(
                            "-2 invalid argument: id map size exceeds the supported range",
                        );
                        -2
                    }
                }
            }
            Err(error) => {
                state.set_error(&error.to_string());
                error.class()
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_camera_world_to_screen_point(
    handle: u64,
    camera: u64,
    x: f32,
    y: f32,
    z: f32,
    out: *mut f32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if out.is_null() {
            state.set_error("-2 invalid argument: output pointer is null");
            return -2;
        }
        let Some(offset) = path_buffer_offset(state, out as usize, 12) else {
            state.set_error("-2 invalid argument: output buffer is out of range");
            return -2;
        };
        match state
            .runtime
            .camera_world_to_screen_point(camera, [x, y, z])
        {
            Ok(values) => {
                for (index, value) in values.iter().enumerate() {
                    let start = offset + index * 4;
                    state.path_buffer[start..start + 4].copy_from_slice(&value.to_le_bytes());
                }
                state.clear_error();
                0
            }
            Err(error) => {
                state.set_error(&error.to_string());
                error.class()
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_camera_screen_point_to_ray(
    handle: u64,
    camera: u64,
    x: f32,
    y: f32,
    _z: f32,
    out: *mut f32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if out.is_null() {
            state.set_error("-2 invalid argument: output pointer is null");
            return -2;
        }
        let Some(offset) = path_buffer_offset(state, out as usize, 24) else {
            state.set_error("-2 invalid argument: output buffer is out of range");
            return -2;
        };
        match state
            .runtime
            .camera_screen_point_to_ray(camera, [x, y, 0.0])
        {
            Ok(values) => {
                for (index, value) in values.iter().enumerate() {
                    let start = offset + index * 4;
                    state.path_buffer[start..start + 4].copy_from_slice(&value.to_le_bytes());
                }
                state.clear_error();
                0
            }
            Err(error) => {
                state.set_error(&error.to_string());
                error.class()
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
ffi_export! {
pub extern "C" fn unitas_runtime_sprite_create(
    handle: u64,
    entity: u64,
    address: u64,
    length: u64,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    let Ok(len) = usize::try_from(length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        let path = if len == 0 {
            None
        } else {
            match super::read_path_buffer(state, address, len) {
                Ok(text) => Some(PathBuf::from(text)),
                Err(code) => return code,
            }
        };
        if state
            .runtime
            .create_runtime_sprite(EntityId::new(entity), path.as_deref())
        {
            state.clear_error();
            0
        } else {
            state.set_error("invalid argument: sprite entity id is zero");
            -2
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_sprite_set_color(
    handle: u64,
    entity: u64,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        if state
            .runtime
            .set_runtime_sprite_color(EntityId::new(entity), [r, g, b, a])
        {
            state.clear_error();
            0
        } else {
            state.set_error("not found: the entity draws no sprite");
            -3
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_sprite_set_sorting(
    handle: u64,
    entity: u64,
    sorting_layer: i32,
    sorting_order: i32,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        if state.runtime.set_runtime_sprite_sorting(
            EntityId::new(entity),
            sorting_layer,
            sorting_order,
        ) {
            state.clear_error();
            0
        } else {
            state.set_error("not found: the entity draws no sprite");
            -3
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_sprite_set_flip(
    handle: u64,
    entity: u64,
    flip_x: u8,
    flip_y: u8,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        if state.runtime.set_runtime_sprite_flip(
            EntityId::new(entity),
            flip_x != 0,
            flip_y != 0,
        ) {
            state.clear_error();
            0
        } else {
            state.set_error("not found: the entity draws no sprite");
            -3
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_sprite_set_enabled(handle: u64, entity: u64, enabled: u8) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        if state
            .runtime
            .set_runtime_sprite_enabled(EntityId::new(entity), enabled != 0)
        {
            state.clear_error();
            0
        } else {
            state.set_error("not found: the entity draws no sprite");
            -3
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_sprite_set_draw_mode(
    handle: u64,
    entity: u64,
    mode: u32,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    let mode = match mode {
        0 => SpriteDrawMode::Simple,
        1 => SpriteDrawMode::Sliced,
        2 => SpriteDrawMode::Tiled,
        _ => return -2,
    };
    match with_runtime_state(handle, |state| {
        if state
            .runtime
            .set_runtime_sprite_draw_mode(EntityId::new(entity), mode)
        {
            state.clear_error();
            0
        } else {
            state.set_error("invalid argument: sprite entity id is zero");
            -2
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_sprite_get_state(
    handle: u64,
    entity: u64,
    blob: *mut f32,
) -> i32 {
    if entity == 0 || blob.is_null() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, blob as usize, SPRITE_STATE_BYTES) else {
            state.set_error("invalid argument: output buffer is out of range");
            return -2;
        };
        let Some(values) = state.runtime.runtime_sprite_state(EntityId::new(entity)) else {
            state.set_error("not found: the entity draws no sprite");
            return -3;
        };
        for (index, value) in values.iter().enumerate() {
            let start = offset + index * 4;
            state.path_buffer[start..start + 4].copy_from_slice(&value.to_le_bytes());
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
