use std::path::Path;

use unitas_asset::database::tilemap::{
    TileColliderType, TilemapArtifact, TilemapKind, import_tilemap, import_tilemap_file,
};
use unitas_core::EntityId;
use unitas_physics::two_d::{
    BodyDesc2D, BodyId2D, ColliderDesc2D, ColliderId2D, CompositeCollider2D,
    CompositeGenerationType2D, CompositeGeometryType2D, PhysicsMaterial2D, Shape2D,
    TileColliderType2D, TilemapCollider2D,
};

use crate::abi::RuntimeState;

fn buffer_offset(base: usize, length: usize, address: usize, size: usize) -> Option<usize> {
    let end = base.checked_add(length)?;
    let target_end = address.checked_add(size)?;
    if address < base || target_end > end {
        return None;
    }
    Some(address - base)
}

fn path_buffer_offset(state: &RuntimeState, address: usize, size: usize) -> Option<usize> {
    buffer_offset(
        state.path_buffer.as_ptr() as usize,
        state.path_buffer.len(),
        address,
        size,
    )
}

fn read_f32_le(bytes: &[u8], offset: usize) -> f32 {
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[offset..offset + 4]);
    f32::from_le_bytes(raw)
}

pub(crate) fn read_points_2d(
    state: &RuntimeState,
    points: *const f32,
    point_count: u32,
) -> Option<Vec<[f32; 2]>> {
    if points.is_null() {
        return None;
    }
    let count = usize::try_from(point_count).ok()?;
    let float_count = count.checked_mul(2)?;
    let byte_len = float_count.checked_mul(4)?;
    let offset = path_buffer_offset(state, points as usize, byte_len)?;
    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        let start = offset + index * 8;
        out.push([
            read_f32_le(&state.path_buffer, start),
            read_f32_le(&state.path_buffer, start + 4),
        ]);
    }
    Some(out)
}

pub(crate) fn read_utf8_path(
    state: &RuntimeState,
    ptr: *const u8,
    path_len: u32,
) -> Option<String> {
    let len = usize::try_from(path_len).ok()?;
    if len == 0 {
        return Some(String::new());
    }
    if ptr.is_null() {
        return None;
    }
    let offset = path_buffer_offset(state, ptr as usize, len)?;
    let slice = &state.path_buffer[offset..offset + len];
    let text = std::str::from_utf8(slice).ok()?;
    Some(text.trim_end_matches('\0').to_string())
}

fn resolve_attached_body(
    state: &mut RuntimeState,
    entity: EntityId,
    attached_body: u64,
) -> Option<BodyId2D> {
    if attached_body != 0 {
        state
            .runtime
            .physics_mut()
            .body_id_2d(EntityId::new(attached_body))
    } else {
        state.runtime.physics_mut().body_id_2d(entity)
    }
}

pub(crate) fn create_polygon_collider(
    state: &mut RuntimeState,
    entity: u64,
    attached_body: u64,
    vertices: Vec<[f32; 2]>,
    offset: [f32; 2],
    is_trigger: bool,
    layer: u8,
) -> i32 {
    if entity == 0 || vertices.len() < 3 {
        state.set_error("polygon collider requires non-zero entity and at least 3 vertices");
        return -2;
    }
    let entity_id = EntityId::new(entity);
    let body_id = resolve_attached_body(state, entity_id, attached_body);
    let mut desc = ColliderDesc2D::polygon_collider(entity_id, vertices)
        .with_offset(offset)
        .with_trigger(is_trigger)
        .with_layer(layer);
    if let Some(body) = body_id {
        desc = desc.with_attached_body(body);
    }
    state
        .runtime
        .physics_mut()
        .set_polygon_collider_2d(entity_id, desc);
    state.clear_error();
    0
}

pub(crate) fn create_edge_collider(
    state: &mut RuntimeState,
    entity: u64,
    attached_body: u64,
    points: Vec<[f32; 2]>,
    offset: [f32; 2],
    is_trigger: bool,
    layer: u8,
) -> i32 {
    if entity == 0 || points.len() < 2 {
        state.set_error("edge collider requires non-zero entity and at least 2 vertices");
        return -2;
    }
    let entity_id = EntityId::new(entity);
    let body_id = resolve_attached_body(state, entity_id, attached_body);
    let shape = if points.len() == 2 {
        Shape2D::Edge {
            start: points[0],
            end: points[1],
        }
    } else {
        let segments = points
            .windows(2)
            .map(|pair| Shape2D::Edge {
                start: pair[0],
                end: pair[1],
            })
            .collect();
        Shape2D::Composite { shapes: segments }
    };
    let mut desc = ColliderDesc2D {
        shape,
        game_object: entity_id,
        ..ColliderDesc2D::default()
    }
    .with_offset(offset)
    .with_trigger(is_trigger)
    .with_layer(layer);
    if let Some(body) = body_id {
        desc = desc.with_attached_body(body);
    }
    state
        .runtime
        .physics_mut()
        .set_edge_collider_2d(entity_id, desc);
    state.clear_error();
    0
}

fn assign_body_if_unattached(
    state: &mut RuntimeState,
    collider_id: ColliderId2D,
    fallback_body: Option<BodyId2D>,
) {
    let Some(body) = fallback_body else {
        return;
    };
    let Some(col) = state
        .runtime
        .physics_mut()
        .world_2d_mut()
        .collider_mut(collider_id)
    else {
        return;
    };
    if col.attached_body.is_none() {
        col.attached_body = Some(body);
    }
}

fn ensure_composite_collider_on_entity(
    state: &mut RuntimeState,
    composite_entity: EntityId,
    fallback_body: Option<BodyId2D>,
    geometry_type: CompositeGeometryType2D,
) -> ColliderId2D {
    let existing_ids = state
        .runtime
        .physics_mut()
        .collider_ids_2d(composite_entity)
        .to_vec();
    for id in &existing_ids {
        if state
            .runtime
            .physics_mut()
            .world_2d()
            .composite(*id)
            .is_some()
        {
            assign_body_if_unattached(state, *id, fallback_body);
            let composite =
                CompositeCollider2D::new(geometry_type, CompositeGenerationType2D::Synchronous);
            state
                .runtime
                .physics_mut()
                .world_2d_mut()
                .attach_composite(*id, composite);
            return *id;
        }
    }
    let body_id = state
        .runtime
        .physics_mut()
        .body_id_2d(composite_entity)
        .or(fallback_body)
        .or_else(|| {
            Some(
                state
                    .runtime
                    .physics_mut()
                    .add_body_2d(BodyDesc2D::static_body(composite_entity, [0.0, 0.0])),
            )
        });
    let mut desc = ColliderDesc2D::box_collider(composite_entity, [0.0, 0.0]);
    if let Some(body) = body_id {
        desc = desc.with_attached_body(body);
    }
    let comp_col_id = state.runtime.physics_mut().add_collider_2d(desc);
    let composite = CompositeCollider2D::new(geometry_type, CompositeGenerationType2D::Synchronous);
    state
        .runtime
        .physics_mut()
        .world_2d_mut()
        .attach_composite(comp_col_id, composite);
    comp_col_id
}

pub(crate) fn set_collider_composite(
    state: &mut RuntimeState,
    entity: u64,
    composite_entity: u64,
    geometry_type_code: u32,
) -> i32 {
    if entity == 0 || composite_entity == 0 {
        state.set_error("composite collider requires non-zero entity and composite_entity");
        return -2;
    }
    let geometry_type = match geometry_type_code {
        0 => CompositeGeometryType2D::Polygons,
        1 => CompositeGeometryType2D::Outlines,
        _ => {
            state.set_error("unknown composite geometry type");
            return -2;
        }
    };
    let entity_id = EntityId::new(entity);
    let comp_entity_id = EntityId::new(composite_entity);
    let part_colliders = state
        .runtime
        .physics_mut()
        .collider_ids_2d(entity_id)
        .to_vec();
    let mut fallback_body = state
        .runtime
        .physics_mut()
        .body_id_2d(comp_entity_id)
        .or_else(|| state.runtime.physics_mut().body_id_2d(entity_id));
    if fallback_body.is_none() {
        for &cid in &part_colliders {
            if let Some(body) = state
                .runtime
                .physics_mut()
                .world_2d()
                .collider(cid)
                .and_then(|c| c.attached_body)
            {
                fallback_body = Some(body);
                break;
            }
        }
    }
    let comp_col_id =
        ensure_composite_collider_on_entity(state, comp_entity_id, fallback_body, geometry_type);
    let comp_body = state
        .runtime
        .physics_mut()
        .world_2d()
        .collider(comp_col_id)
        .and_then(|c| c.attached_body);
    for cid in part_colliders {
        if cid == comp_col_id {
            continue;
        }
        if let Some(col) = state.runtime.physics_mut().world_2d_mut().collider_mut(cid) {
            col.used_by_composite = true;
            if let Some(body) = comp_body {
                col.attached_body = Some(body);
            }
        }
    }
    state
        .runtime
        .physics_mut()
        .world_2d_mut()
        .regenerate_composite(comp_col_id);
    state.clear_error();
    0
}

pub(crate) fn set_collider_material(
    state: &mut RuntimeState,
    entity: u64,
    friction: f32,
    bounciness: f32,
) -> i32 {
    if entity == 0 || !friction.is_finite() || !bounciness.is_finite() {
        state.set_error("invalid collider material arguments");
        return -2;
    }
    let entity_id = EntityId::new(entity);
    let colliders = state
        .runtime
        .physics_mut()
        .collider_ids_2d(entity_id)
        .to_vec();
    if colliders.is_empty() {
        state.set_error("collider not found");
        return -4;
    }
    let material = PhysicsMaterial2D::new(friction.max(0.0), bounciness.max(0.0));
    for cid in colliders {
        if let Some(col) = state.runtime.physics_mut().world_2d_mut().collider_mut(cid) {
            col.material = material;
            col.uses_default_material = false;
        }
    }
    state.clear_error();
    0
}

fn load_tilemap_artifact(path_text: &str) -> Result<TilemapArtifact, String> {
    let trimmed = path_text.trim();
    if trimmed.is_empty() {
        return Err("empty tilemap path".to_string());
    }
    let path = Path::new(trimmed);
    if path.exists() {
        return import_tilemap_file(path).map_err(|err| err.to_string());
    }
    if trimmed.contains("%YAML") || trimmed.contains("Tilemap:") || trimmed.contains("m_Tiles:") {
        return import_tilemap(trimmed.as_bytes(), "inline_tilemap.asset")
            .map_err(|err| err.to_string());
    }
    Err(format!("tilemap asset not found: {trimmed}"))
}

fn build_tilemap_collider_from_artifact(
    artifact: &TilemapArtifact,
    used_by_composite: bool,
) -> TilemapCollider2D {
    let cell_size = artifact.grid.map_or([1.0, 1.0], |g| g.cell_size);
    let mut tilemap = TilemapCollider2D::new(cell_size).with_used_by_composite(used_by_composite);
    match (
        &artifact.tilemap,
        artifact.identity.kind,
        artifact.tiles.first(),
    ) {
        (Some(settings), _, _) => {
            for cell in &settings.cells {
                let tile_type = usize::try_from(cell.tile_index)
                    .ok()
                    .and_then(|idx| artifact.tiles.get(idx))
                    .map_or(TileColliderType::Grid, |tile| tile.collider_type);
                let kind = match tile_type {
                    TileColliderType::None => TileColliderType2D::None,
                    TileColliderType::Grid => TileColliderType2D::Grid,
                    TileColliderType::Sprite => TileColliderType2D::Sprite {
                        vertices: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                    },
                };
                tilemap.set_tile([cell.position[0], cell.position[1]], kind);
            }
        }
        (None, TilemapKind::Tile | TilemapKind::RuleTile, Some(tile)) => {
            let kind = match tile.collider_type {
                TileColliderType::None => TileColliderType2D::None,
                TileColliderType::Grid => TileColliderType2D::Grid,
                TileColliderType::Sprite => TileColliderType2D::Sprite {
                    vertices: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                },
            };
            tilemap.set_tile([0, 0], kind);
        }
        _ => {}
    }
    tilemap
}

pub(crate) fn build_tilemap_collider(
    state: &mut RuntimeState,
    entity: u64,
    composite_entity: u64,
    path_text: &str,
) -> i32 {
    if entity == 0 {
        state.set_error("tilemap collider requires non-zero entity");
        return -2;
    }
    let artifact = match load_tilemap_artifact(path_text) {
        Ok(artifact) => artifact,
        Err(msg) => {
            state.set_error(&msg);
            return -4;
        }
    };
    let used_by_composite = composite_entity != 0;
    let tilemap = build_tilemap_collider_from_artifact(&artifact, used_by_composite);
    let entity_id = EntityId::new(entity);

    let mut target_body = if composite_entity != 0 {
        state
            .runtime
            .physics_mut()
            .body_id_2d(EntityId::new(composite_entity))
            .or_else(|| state.runtime.physics_mut().body_id_2d(entity_id))
    } else {
        state.runtime.physics_mut().body_id_2d(entity_id)
    };
    if target_body.is_none() {
        let owner = if composite_entity != 0 {
            EntityId::new(composite_entity)
        } else {
            entity_id
        };
        target_body = Some(
            state
                .runtime
                .physics_mut()
                .add_body_2d(BodyDesc2D::static_body(owner, [0.0, 0.0])),
        );
    }

    let comp_col_id = if composite_entity != 0 {
        let comp_entity_id = EntityId::new(composite_entity);
        let comp_ids = state
            .runtime
            .physics_mut()
            .collider_ids_2d(comp_entity_id)
            .to_vec();
        let existing_geom = comp_ids
            .iter()
            .find_map(|id| {
                state
                    .runtime
                    .physics_mut()
                    .world_2d()
                    .composite(*id)
                    .map(|c| c.geometry_type)
            })
            .unwrap_or(CompositeGeometryType2D::Outlines);
        Some(ensure_composite_collider_on_entity(
            state,
            comp_entity_id,
            target_body,
            existing_geom,
        ))
    } else {
        None
    };

    let tm_col_id = match (composite_entity == entity, comp_col_id) {
        (true, Some(comp_id)) => comp_id,
        _ => {
            let existing = state
                .runtime
                .physics_mut()
                .collider_ids_2d(entity_id)
                .iter()
                .copied()
                .find(|id| Some(*id) != comp_col_id);
            if let Some(id) = existing {
                if let Some(col) = state.runtime.physics_mut().world_2d_mut().collider_mut(id) {
                    col.attached_body = target_body;
                    col.used_by_composite = used_by_composite;
                }
                id
            } else {
                let mut desc = ColliderDesc2D::box_collider(entity_id, [0.0, 0.0])
                    .with_used_by_composite(used_by_composite);
                if let Some(body) = target_body {
                    desc = desc.with_attached_body(body);
                }
                state.runtime.physics_mut().add_collider_2d(desc)
            }
        }
    };

    state
        .runtime
        .physics_mut()
        .world_2d_mut()
        .attach_tilemap(tm_col_id, tilemap);
    if let Some(comp_id) = comp_col_id {
        state
            .runtime
            .physics_mut()
            .world_2d_mut()
            .regenerate_composite(comp_id);
    }
    state.clear_error();
    0
}
