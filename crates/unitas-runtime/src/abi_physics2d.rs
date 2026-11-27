use crate::physics_settings::{
    apply_physics2d_runtime_settings, get_physics2d_layer_collision, set_physics2d_layer_collision,
};
use crate::physics2d_colliders::{
    build_tilemap_collider, create_edge_collider, create_polygon_collider, read_points_2d,
    read_utf8_path, set_collider_composite, set_collider_material,
};

use super::{
    BodyDesc2D, BodyType2D, ColliderDesc2D, ColliderId2D, CollisionContactPoint2D,
    CollisionDetectionMode2D, CollisionPayload2D, CollisionSummary2D, ContactFilter2D, EntityId,
    PlatformEffector2D, QUERY_RECORD_2D_SIZE, QueryRecord2D, RigidbodyInterpolation2D,
    RuntimeState, SurfaceEffector2D, with_active_runtime, with_registered_runtime,
    with_runtime_state, write_output,
};

ffi_export! {
pub extern "C" fn unitas_runtime_body2d_create(
    handle: u64,
    entity: u64,
    position_x: f32,
    position_y: f32,
    rotation: f32,
    mass: f32,
    gravity_scale: f32,
    body_type: u32,
    layer: u32,
    simulated: u32,
    interpolation: u32,
    collision_detection_mode: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let entity = EntityId::new(entity);
        let position = [position_x, position_y];
        let mut body = match body_type {
            1 => BodyDesc2D::kinematic(entity, position),
            2 => BodyDesc2D::static_body(entity, position),
            _ => BodyDesc2D::dynamic(entity, position),
        };
        body = body
            .with_rotation(rotation)
            .with_mass(mass)
            .with_gravity_scale(gravity_scale)
            .with_layer(layer as u8)
            .with_simulated(simulated != 0);
        body = match interpolation {
            1 => body.with_interpolation(RigidbodyInterpolation2D::Interpolate),
            2 => body.with_interpolation(RigidbodyInterpolation2D::Extrapolate),
            _ => body,
        };
        body = match collision_detection_mode {
            1 => body.with_collision_detection_mode(CollisionDetectionMode2D::Continuous),
            _ => body,
        };
        state.runtime.physics_mut().add_body_2d(body);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_collider2d_create(
    handle: u64,
    entity: u64,
    attached_body: u64,
    shape: u32,
    offset_x: f32,
    offset_y: f32,
    size_x: f32,
    size_y: f32,
    radius: f32,
    is_trigger: u32,
    layer: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let entity = EntityId::new(entity);
        let mut collider = match shape {
            1 => ColliderDesc2D::circle_collider(entity, radius),
            _ => ColliderDesc2D::box_collider(entity, [size_x / 2.0, size_y / 2.0]),
        };
        collider = collider
            .with_offset([offset_x, offset_y])
            .with_trigger(is_trigger != 0)
            .with_layer(layer as u8);
        if attached_body != 0 {
            let attached = state
                .runtime
                .physics_mut()
                .body_id_2d(EntityId::new(attached_body));
            if let Some(attached) = attached {
                collider = collider.with_attached_body(attached);
            }
        }
        state.runtime.physics_mut().add_collider_2d(collider);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_collider2d_create_capsule(
    handle: u64,
    entity: u64,
    radius: f32,
    height: f32,
    direction: u32,
    offset_x: f32,
    offset_y: f32,
    is_trigger: u32,
    layer: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let entity = EntityId::new(entity);
        let mut collider =
            ColliderDesc2D::capsule_collider(entity, radius, height, u8::from(direction != 0));
        collider = collider
            .with_offset([offset_x, offset_y])
            .with_trigger(is_trigger != 0)
            .with_layer(layer as u8);
        if let Some(attached) = state.runtime.physics_mut().body_id_2d(entity) {
            collider = collider.with_attached_body(attached);
        }
        state.runtime.physics_mut().set_capsule_collider_2d(entity, collider);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_collider2d_create_polygon(
    handle: u64,
    entity: u64,
    attached_body: u64,
    points: *const f32,
    point_count: u32,
    offset_x: f32,
    offset_y: f32,
    is_trigger: u32,
    layer: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some(vertices) = read_points_2d(state, points, point_count) else {
            state.set_error("invalid polygon collider points buffer");
            return -2;
        };
        create_polygon_collider(
            state,
            entity,
            attached_body,
            vertices,
            [offset_x, offset_y],
            is_trigger != 0,
            layer as u8,
        )
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_collider2d_create_edge(
    handle: u64,
    entity: u64,
    attached_body: u64,
    points: *const f32,
    point_count: u32,
    offset_x: f32,
    offset_y: f32,
    is_trigger: u32,
    layer: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some(vertices) = read_points_2d(state, points, point_count) else {
            state.set_error("invalid edge collider points buffer");
            return -2;
        };
        create_edge_collider(
            state,
            entity,
            attached_body,
            vertices,
            [offset_x, offset_y],
            is_trigger != 0,
            layer as u8,
        )
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_collider2d_set_composite(
    handle: u64,
    entity: u64,
    composite_entity: u64,
    geometry_type: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        set_collider_composite(state, entity, composite_entity, geometry_type)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_tilemap_collider2d_build(
    handle: u64,
    entity: u64,
    composite_entity: u64,
    tilemap_path: *const u8,
    path_len: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some(path_text) = read_utf8_path(state, tilemap_path, path_len) else {
            state.set_error("invalid tilemap path buffer");
            return -2;
        };
        build_tilemap_collider(state, entity, composite_entity, &path_text)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_collider2d_set_material(
    handle: u64,
    entity: u64,
    friction: f32,
    bounciness: f32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        set_collider_material(state, entity, friction, bounciness)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body2d_velocity(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if out_buffer == 0
            || out_buffer != state.path_buffer.as_ptr() as usize as u64
            || state.path_buffer.len() < 8
        {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let Some(velocity) = state
            .runtime
            .physics_mut()
            .body_velocity_2d(EntityId::new(entity))
        else {
            state.set_error("body not found");
            return -4;
        };
        state.path_buffer[0..4].copy_from_slice(&velocity[0].to_le_bytes());
        state.path_buffer[4..8].copy_from_slice(&velocity[1].to_le_bytes());
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body2d_set_velocity(
    handle: u64,
    entity: u64,
    x: f32,
    y: f32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if !state
            .runtime
            .physics_mut()
            .set_body_velocity_2d(EntityId::new(entity), [x, y])
        {
            state.set_error("body not found");
            return -4;
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body2d_move_position(
    handle: u64,
    entity: u64,
    x: f32,
    y: f32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if !state
            .runtime
            .physics_mut()
            .move_body_position_2d(EntityId::new(entity), [x, y])
        {
            state.set_error("body not found");
            return -4;
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body2d_set_simulated(
    handle: u64,
    entity: u64,
    simulated: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if !state
            .runtime
            .physics_mut()
            .set_body_simulated_2d(EntityId::new(entity), simulated != 0)
        {
            state.set_error("body not found");
            return -4;
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body2d_set_body_type(
    handle: u64,
    entity: u64,
    body_type: u32,
) -> i32 {
    let body_type = match body_type {
        0 => BodyType2D::Dynamic,
        1 => BodyType2D::Kinematic,
        2 => BodyType2D::Static,
        _ => {
            return match with_runtime_state(handle, |state| {
                state.set_error("unknown 2D body type");
                -2
            }) {
                Ok(code) => code,
                Err(code) => code,
            };
        }
    };
    match with_runtime_state(handle, |state| {
        if !state
            .runtime
            .physics_mut()
            .set_body_type_2d(EntityId::new(entity), body_type)
        {
            state.set_error("body not found");
            return -4;
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body2d_set_interpolation(
    handle: u64,
    entity: u64,
    interpolation: u32,
) -> i32 {
    let interpolation = match interpolation {
        0 => RigidbodyInterpolation2D::None,
        1 => RigidbodyInterpolation2D::Interpolate,
        2 => RigidbodyInterpolation2D::Extrapolate,
        _ => {
            return match with_runtime_state(handle, |state| {
                state.set_error("unknown 2D interpolation mode");
                -2
            }) {
                Ok(code) => code,
                Err(code) => code,
            };
        }
    };
    match with_runtime_state(handle, |state| {
        if !state
            .runtime
            .physics_mut()
            .set_body_interpolation_2d(EntityId::new(entity), interpolation)
        {
            state.set_error("body not found");
            return -4;
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body2d_set_collision_detection_mode(
    handle: u64,
    entity: u64,
    mode: u32,
) -> i32 {
    let mode = match mode {
        0 => CollisionDetectionMode2D::Discrete,
        1 => CollisionDetectionMode2D::Continuous,
        _ => {
            return match with_runtime_state(handle, |state| {
                state.set_error("unknown 2D collision detection mode");
                -2
            }) {
                Ok(code) => code,
                Err(code) => code,
            };
        }
    };
    match with_runtime_state(handle, |state| {
        if !state
            .runtime
            .physics_mut()
            .set_body_collision_detection_mode_2d(EntityId::new(entity), mode)
        {
            state.set_error("body not found");
            return -4;
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body2d_set_constraints(
    handle: u64,
    entity: u64,
    constraints: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if !state
            .runtime
            .physics_mut()
            .set_body_constraints_2d(EntityId::new(entity), constraints)
        {
            state.set_error("body not found");
            return -4;
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_raycast2d(
    handle: u64,
    origin_x: f32,
    origin_y: f32,
    direction_x: f32,
    direction_y: f32,
    max_distance: f32,
    layer_mask: u32,
    include_triggers: i32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let filter = ContactFilter2D::default()
            .with_triggers(include_triggers != 0)
            .with_layer_mask(layer_mask);
        let hits = state.runtime.physics_mut().raycast_2d(
            [origin_x, origin_y],
            [direction_x, direction_y],
            max_distance,
            filter,
        );
        let records = hits
            .iter()
            .map(|hit| QueryRecord2D {
                entity: hit.game_object.as_u64(),
                distance: hit.distance,
                point: hit.point,
                normal: hit.normal,
            })
            .collect();
        store_query_records_2d(state, records)
    }) {
        Ok(count) => count,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_circle_cast2d(
    handle: u64,
    origin_x: f32,
    origin_y: f32,
    radius: f32,
    direction_x: f32,
    direction_y: f32,
    distance: f32,
    use_triggers: i32,
    layer_mask: u32,
    use_depth: i32,
    min_depth: f32,
    max_depth: f32,
    use_normal_angle: i32,
    min_normal_angle: f32,
    max_normal_angle: f32,
) -> i32 {
    let filter = build_contact_filter_2d(
        use_triggers,
        layer_mask,
        (use_depth, min_depth, max_depth),
        (use_normal_angle, min_normal_angle, max_normal_angle),
    );
    match with_runtime_state(handle, |state| {
        let hits = state.runtime.physics_mut().world_2d().circle_cast(
            [origin_x, origin_y],
            radius,
            [direction_x, direction_y],
            distance,
            filter,
        );
        let records = hits
            .iter()
            .map(|hit| QueryRecord2D {
                entity: hit.game_object.as_u64(),
                distance: hit.distance,
                point: hit.point,
                normal: hit.normal,
            })
            .collect();
        store_query_records_2d(state, records)
    }) {
        Ok(count) => count,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_capsule_cast2d(
    handle: u64,
    origin_x: f32,
    origin_y: f32,
    size_x: f32,
    size_y: f32,
    capsule_direction: u32,
    angle_deg: f32,
    direction_x: f32,
    direction_y: f32,
    distance: f32,
    use_triggers: i32,
    layer_mask: u32,
    use_depth: i32,
    min_depth: f32,
    max_depth: f32,
    use_normal_angle: i32,
    min_normal_angle: f32,
    max_normal_angle: f32,
) -> i32 {
    let filter = build_contact_filter_2d(
        use_triggers,
        layer_mask,
        (use_depth, min_depth, max_depth),
        (use_normal_angle, min_normal_angle, max_normal_angle),
    );
    match with_runtime_state(handle, |state| {
        let hits = state.runtime.physics_mut().world_2d().capsule_cast(
            [origin_x, origin_y],
            [size_x, size_y],
            u8::from(capsule_direction != 0),
            angle_deg.to_radians(),
            [direction_x, direction_y],
            distance,
            filter,
        );
        let records = hits
            .iter()
            .map(|hit| QueryRecord2D {
                entity: hit.game_object.as_u64(),
                distance: hit.distance,
                point: hit.point,
                normal: hit.normal,
            })
            .collect();
        store_query_records_2d(state, records)
    }) {
        Ok(count) => count,
        Err(code) => code,
    }
}
}

fn build_contact_filter_2d(
    use_triggers: i32,
    layer_mask: u32,
    depth: (i32, f32, f32),
    normal_angle: (i32, f32, f32),
) -> ContactFilter2D {
    let mut filter = ContactFilter2D::default()
        .with_triggers(use_triggers != 0)
        .with_layer_mask(layer_mask);
    if depth.0 != 0 && depth.1.is_finite() && depth.2.is_finite() {
        filter.use_depth = true;
        filter.min_depth = depth.1;
        filter.max_depth = depth.2;
    }
    if normal_angle.0 != 0 {
        filter.use_normal_angle = true;
        filter.min_normal_angle = normal_angle.1;
        filter.max_normal_angle = normal_angle.2;
    }
    filter
}

ffi_export! {
pub extern "C" fn unitas_runtime_overlap_point2d(
    handle: u64,
    x: f32,
    y: f32,
    layer_mask: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let filter = ContactFilter2D::default().with_layer_mask(layer_mask);
        let colliders = state
            .runtime
            .physics_mut()
            .overlap_point_2d([x, y], filter);
        let records = collider_records_2d(state, &colliders, [x, y]);
        store_query_records_2d(state, records)
    }) {
        Ok(count) => count,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_overlap_circle2d(
    handle: u64,
    x: f32,
    y: f32,
    radius: f32,
    layer_mask: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let filter = ContactFilter2D::default().with_layer_mask(layer_mask);
        let colliders = state
            .runtime
            .physics_mut()
            .overlap_circle_2d([x, y], radius, filter);
        let records = collider_records_2d(state, &colliders, [x, y]);
        store_query_records_2d(state, records)
    }) {
        Ok(count) => count,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_overlap_box2d(
    handle: u64,
    x: f32,
    y: f32,
    half_x: f32,
    half_y: f32,
    angle: f32,
    layer_mask: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let filter = ContactFilter2D::default().with_layer_mask(layer_mask);
        let colliders = state
            .runtime
            .physics_mut()
            .overlap_box_2d([x, y], [half_x, half_y], angle, filter);
        let records = collider_records_2d(state, &colliders, [x, y]);
        store_query_records_2d(state, records)
    }) {
        Ok(count) => count,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_box_cast2d(
    handle: u64,
    origin_x: f32,
    origin_y: f32,
    half_x: f32,
    half_y: f32,
    angle: f32,
    direction_x: f32,
    direction_y: f32,
    distance: f32,
    use_triggers: i32,
    layer_mask: u32,
    use_depth: i32,
    min_depth: f32,
    max_depth: f32,
    use_normal_angle: i32,
    min_normal_angle: f32,
    max_normal_angle: f32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let mut filter = ContactFilter2D::default()
            .with_triggers(use_triggers != 0)
            .with_layer_mask(layer_mask);
        if use_depth != 0 {
            filter.use_depth = true;
            filter.min_depth = min_depth;
            filter.max_depth = max_depth;
        }
        if use_normal_angle != 0 {
            filter.use_normal_angle = true;
            filter.min_normal_angle = min_normal_angle;
            filter.max_normal_angle = max_normal_angle;
        }
        let hits = state.runtime.physics_mut().box_cast_2d(
            [origin_x, origin_y],
            [half_x, half_y],
            angle,
            [direction_x, direction_y],
            distance,
            filter,
        );
        let records = hits
            .iter()
            .map(|hit| QueryRecord2D {
                entity: hit.game_object.as_u64(),
                distance: hit.distance,
                point: hit.point,
                normal: hit.normal,
            })
            .collect();
        store_query_records_2d(state, records)
    }) {
        Ok(count) => count,
        Err(code) => code,
    }
}
}


ffi_export! {
pub extern "C" fn unitas_runtime_query_buffer_2d(handle: u64, capacity: u32) -> u64 {
    if capacity == 0 {
        return 0;
    }
    let size = match usize::try_from(capacity) {
        Ok(value) => value,
        Err(_) => return 0,
    };
    with_runtime_state(handle, |state| {
        state.query_buffer_2d = vec![0u8; size];
        write_query_records_2d(state);
        state.query_buffer_2d.as_ptr() as usize as u64
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_collision_summary_2d(
    a: u64,
    b: u64,
    out: *mut CollisionSummary2D,
) -> i32 {
    let Some(summary) = with_collision_payload_2d(a, b, &|payload| CollisionSummary2D {
        impulse: payload.impulse,
        relative_velocity: payload.relative_velocity,
        contact_count: payload.contacts.len() as u32,
        point: payload
            .contacts
            .first()
            .map_or([0.0; 2], |contact| contact.point),
        normal: payload
            .contacts
            .first()
            .map_or([0.0; 2], |contact| contact.normal),
    }) else {
        return -1;
    };
    if !write_output(out, summary) {
        return -2;
    }
    0
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_collision_contact_2d(
    a: u64,
    b: u64,
    index: u32,
    out: *mut CollisionContactPoint2D,
) -> i32 {
    let Some(contact) = with_collision_payload_2d(a, b, &|payload| {
        payload
            .contacts
            .get(index as usize)
            .map(|contact| CollisionContactPoint2D {
                point: contact.point,
                normal: contact.normal,
                impulse: contact.normal_impulse,
            })
    }) else {
        return -1;
    };
    let Some(contact) = contact else {
        return -1;
    };
    if !write_output(out, contact) {
        return -2;
    }
    0
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_physics2d_clear(handle: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        state.runtime.physics_mut().clear_2d();
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_physics2d_apply_settings(
    handle: u64,
    gravity_x: f32,
    gravity_y: f32,
    velocity_iterations: u32,
    position_iterations: u32,
    queries_hit_triggers: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        apply_physics2d_runtime_settings(
            state,
            gravity_x,
            gravity_y,
            velocity_iterations,
            position_iterations,
            queries_hit_triggers,
        )
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_physics2d_set_layer_collision(
    handle: u64,
    layer_a: u32,
    layer_b: u32,
    enabled: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        set_physics2d_layer_collision(state, layer_a, layer_b, enabled)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_physics2d_get_layer_collision(
    handle: u64,
    layer_a: u32,
    layer_b: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        get_physics2d_layer_collision(state, layer_a, layer_b)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_collider2d_set_platform_effector(
    handle: u64,
    entity: u64,
    use_one_way: u32,
    surface_arc: f32,
    rotational_offset: f32,
    side_arc: f32,
    use_side_friction: u32,
    use_side_bounce: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let entity = EntityId::new(entity);
        let colliders = state.runtime.physics_mut().collider_ids_2d(entity).to_vec();
        let mut applied = false;
        for collider in colliders {
            let effector = PlatformEffector2D {
                use_one_way: use_one_way != 0,
                surface_arc,
                rotational_offset,
                side_arc,
                use_side_friction: use_side_friction != 0,
                use_side_bounce: use_side_bounce != 0,
                collider_mask: u32::MAX,
            };
            applied |= state
                .runtime
                .physics_mut()
                .set_platform_effector_2d(collider, effector);
        }
        if !applied {
            state.set_error("collider not found");
            return -4;
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_collider2d_set_surface_effector(
    handle: u64,
    entity: u64,
    speed: f32,
    force_scale: f32,
    use_friction: u32,
    use_bounce: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let entity = EntityId::new(entity);
        let colliders = state.runtime.physics_mut().collider_ids_2d(entity).to_vec();
        let mut applied = false;
        for collider in colliders {
            let effector = SurfaceEffector2D {
                speed,
                speed_variation: 0.0,
                force_scale,
                use_contact_force: false,
                use_friction: use_friction != 0,
                use_bounce: use_bounce != 0,
                collider_mask: u32::MAX,
            };
            applied |= state
                .runtime
                .physics_mut()
                .set_surface_effector_2d(collider, effector);
        }
        if !applied {
            state.set_error("collider not found");
            return -4;
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

fn collider_records_2d(
    state: &mut RuntimeState,
    colliders: &[ColliderId2D],
    point: [f32; 2],
) -> Vec<QueryRecord2D> {
    colliders
        .iter()
        .filter_map(|collider| {
            state
                .runtime
                .physics_mut()
                .collider_game_object_2d(*collider)
                .map(|entity| QueryRecord2D {
                    entity: entity.as_u64(),
                    distance: 0.0,
                    point,
                    normal: [0.0; 2],
                })
        })
        .collect()
}

fn store_query_records_2d(state: &mut RuntimeState, records: Vec<QueryRecord2D>) -> i32 {
    let count = records.len();
    state.query_records_2d = records;
    write_query_records_2d(state);
    let Ok(count_value) = i32::try_from(count) else {
        return -1;
    };
    count_value
}

fn write_query_records_2d(state: &mut RuntimeState) {
    state.query_buffer_2d.fill(0);
    let capacity = state.query_buffer_2d.len() / QUERY_RECORD_2D_SIZE;
    let count = state.query_records_2d.len().min(capacity);
    for (index, record) in state.query_records_2d.iter().take(count).enumerate() {
        let offset = index * QUERY_RECORD_2D_SIZE;
        state.query_buffer_2d[offset..offset + 8].copy_from_slice(&record.entity.to_le_bytes());
        state.query_buffer_2d[offset + 8..offset + 12]
            .copy_from_slice(&record.distance.to_le_bytes());
        state.query_buffer_2d[offset + 12..offset + 16]
            .copy_from_slice(&record.point[0].to_le_bytes());
        state.query_buffer_2d[offset + 16..offset + 20]
            .copy_from_slice(&record.point[1].to_le_bytes());
        state.query_buffer_2d[offset + 20..offset + 24]
            .copy_from_slice(&record.normal[0].to_le_bytes());
        state.query_buffer_2d[offset + 24..offset + 28]
            .copy_from_slice(&record.normal[1].to_le_bytes());
    }
}

fn with_collision_payload_2d<T>(
    first: u64,
    second: u64,
    action: &impl Fn(&CollisionPayload2D) -> T,
) -> Option<T> {
    if let Some(value) =
        with_active_runtime(|runtime| runtime.collision_payload_2d(first, second).map(action))
    {
        return Some(value);
    }
    with_registered_runtime(|runtime| runtime.collision_payload_2d(first, second).map(action))
}
