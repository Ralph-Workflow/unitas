use std::collections::{HashMap, HashSet};
use std::path::Path;

use unitas_core::{rotate_vector, EntityId, SceneGraph, Transform};

use crate::collision::{cook_builtin_collision_mesh, cook_collision_mesh, CollisionMesh};
use crate::meta::GuidIndex;
use crate::primitive::{primitive_mesh, BuiltinPrimitive};
use crate::unity_yaml::{Node, Stream};

use super::{
    component_entity, reference_id, resolve_mesh_reference_for, vector3_or, yaml_bool, yaml_f64,
    yaml_i32, yaml_u32, BoxColliderComponent, ColliderComponent, GameObjectRecords,
    MeshColliderComponent, MeshRendererComponent, PhysicsBinding, RenderBinding, SceneError,
    MESH_FILTER_CLASS, MONO_BEHAVIOUR_CLASS,
};

pub const NAV_MESH_AGENT_CLASS: u32 = 195;
pub const NAV_MESH_OBSTACLE_CLASS: u32 = 208;
pub const OFF_MESH_LINK_CLASS: u32 = 191;
pub const NAV_MESH_SURFACE_SCRIPT_GUID: &str = "7a5ac11cc976e418e8d13136b07e1f52";

const NAV_MESH_SURFACE_FIELD_KEYS: [&str; 4] = [
    "m_CollectObjects",
    "m_AgentTypeID",
    "m_BuildHeightMesh",
    "m_DefaultArea",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CollectObjects {
    #[default]
    All = 0,
    Volume = 1,
    Children = 2,
}

impl CollectObjects {
    pub const fn from_i32(value: i32) -> Self {
        match value {
            1 => Self::Volume,
            2 => Self::Children,
            _ => Self::All,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavMeshObstacleShape {
    Capsule = 0,
    #[default]
    Box = 1,
}

impl NavMeshObstacleShape {
    pub const fn from_i32(value: i32) -> Self {
        match value {
            0 => Self::Capsule,
            _ => Self::Box,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneNavMeshTriangle {
    pub vertices: [[f64; 3]; 3],
    pub area: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NavMeshAgentComponent {
    pub entity: EntityId,
    pub enabled: bool,
    pub agent_type_id: i32,
    pub radius: f64,
    pub speed: f64,
    pub acceleration: f64,
    pub avoidance_priority: u32,
    pub angular_speed: f64,
    pub stopping_distance: f64,
    pub auto_traverse_off_mesh_link: bool,
    pub auto_braking: bool,
    pub auto_repath: bool,
    pub height: f64,
    pub base_offset: f64,
    pub walkable_mask: u32,
    pub obstacle_avoidance_type: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NavMeshObstacleComponent {
    pub entity: EntityId,
    pub enabled: bool,
    pub shape: NavMeshObstacleShape,
    pub center: [f64; 3],
    pub size: [f64; 3],
    pub radius: f64,
    pub height: f64,
    pub carving: bool,
    pub carve_only_stationary: bool,
    pub move_threshold: f64,
    pub time_to_stationary: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OffMeshLinkComponent {
    pub entity: EntityId,
    pub enabled: bool,
    pub start: Option<EntityId>,
    pub end: Option<EntityId>,
    pub cost_override: f64,
    pub bi_directional: bool,
    pub activated: bool,
    pub auto_update_positions: bool,
    pub area_index: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NavMeshSurfaceComponent {
    pub entity: EntityId,
    pub enabled: bool,
    pub agent_type_id: i32,
    pub collect_objects: CollectObjects,
    pub size: [f64; 3],
    pub center: [f64; 3],
    pub layer_mask: u32,
    pub use_geometry: i32,
    pub default_area: u32,
    pub triangles: Vec<SceneNavMeshTriangle>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NavigationBinding {
    pub agents: Vec<NavMeshAgentComponent>,
    pub obstacles: Vec<NavMeshObstacleComponent>,
    pub links: Vec<OffMeshLinkComponent>,
    pub surfaces: Vec<NavMeshSurfaceComponent>,
}

impl NavigationBinding {
    pub fn extend(&mut self, other: Self) {
        self.agents.extend(other.agents);
        self.obstacles.extend(other.obstacles);
        self.links.extend(other.links);
        self.surfaces.extend(other.surfaces);
    }
}

pub(super) fn is_nav_mesh_surface(body: &Node) -> bool {
    let guid_matches = body
        .get("m_Script")
        .and_then(|node| node.get("guid"))
        .and_then(Node::as_str)
        .map(str::trim)
        .is_some_and(|guid| guid.eq_ignore_ascii_case(NAV_MESH_SURFACE_SCRIPT_GUID));
    if guid_matches {
        return true;
    }
    let name_matches = body
        .get("m_Name")
        .and_then(Node::as_str)
        .is_some_and(|name| name == "NavMeshSurface");
    let field_count = NAV_MESH_SURFACE_FIELD_KEYS
        .iter()
        .filter(|key| body.get(key).is_some())
        .count();
    name_matches || field_count >= 2
}

pub(super) fn collect_navigation(
    stream: &Stream,
    graph: &SceneGraph,
    game_objects: &GameObjectRecords,
    transforms: &HashMap<i64, EntityId>,
    instance_roots: &HashSet<i64>,
    physics: &[PhysicsBinding],
    render: &RenderBinding,
    index: Option<&GuidIndex>,
) -> Result<NavigationBinding, SceneError> {
    let mut binding = NavigationBinding::default();
    let mesh_filters = collect_standalone_mesh_filters(stream, game_objects, instance_roots, index);
    for doc in &stream.documents {
        if doc.stripped {
            continue;
        }
        match doc.class_id {
            NAV_MESH_AGENT_CLASS => {
                if let Some(body) = doc.root.get("NavMeshAgent") {
                    if let Some(entity) = component_entity(body, game_objects, instance_roots) {
                        binding.agents.push(parse_nav_mesh_agent(entity, body));
                    }
                }
            }
            NAV_MESH_OBSTACLE_CLASS => {
                if let Some(body) = doc.root.get("NavMeshObstacle") {
                    if let Some(entity) = component_entity(body, game_objects, instance_roots) {
                        binding
                            .obstacles
                            .push(parse_nav_mesh_obstacle(entity, body));
                    }
                }
            }
            OFF_MESH_LINK_CLASS => {
                if let Some(body) = doc.root.get("OffMeshLink") {
                    if let Some(entity) = component_entity(body, game_objects, instance_roots) {
                        binding.links.push(parse_off_mesh_link(
                            entity,
                            body,
                            game_objects,
                            transforms,
                        )?);
                    }
                }
            }
            MONO_BEHAVIOUR_CLASS => {
                if let Some(body) = doc.root.get("MonoBehaviour") {
                    if is_nav_mesh_surface(body) {
                        if let Some(entity) = component_entity(body, game_objects, instance_roots) {
                            binding.surfaces.push(parse_nav_mesh_surface(
                                entity,
                                body,
                                graph,
                                game_objects,
                                physics,
                                render,
                                &mesh_filters,
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(binding)
}

fn parse_nav_mesh_agent(entity: EntityId, body: &Node) -> NavMeshAgentComponent {
    NavMeshAgentComponent {
        entity,
        enabled: yaml_bool(body.get("m_Enabled"), true),
        agent_type_id: yaml_i32(body.get("m_AgentTypeID"), 0),
        radius: yaml_f64(body.get("m_Radius"), 0.5, |v| v > 0.0),
        speed: yaml_f64(body.get("m_Speed"), 3.5, |v| v >= 0.0),
        acceleration: yaml_f64(body.get("m_Acceleration"), 8.0, |v| v >= 0.0),
        avoidance_priority: yaml_u32(body.get("avoidancePriority"), 50),
        angular_speed: yaml_f64(body.get("m_AngularSpeed"), 120.0, |v| v >= 0.0),
        stopping_distance: yaml_f64(body.get("m_StoppingDistance"), 0.0, |v| v >= 0.0),
        auto_traverse_off_mesh_link: yaml_bool(body.get("m_AutoTraverseOffMeshLink"), true),
        auto_braking: yaml_bool(body.get("m_AutoBraking"), true),
        auto_repath: yaml_bool(body.get("m_AutoRepath"), true),
        height: yaml_f64(body.get("m_Height"), 2.0, |v| v > 0.0),
        base_offset: yaml_f64(body.get("m_BaseOffset"), 0.0, |v| v.is_finite()),
        walkable_mask: yaml_u32(body.get("m_WalkableMask"), u32::MAX),
        obstacle_avoidance_type: yaml_i32(body.get("m_ObstacleAvoidanceType"), 4),
    }
}

fn parse_nav_mesh_obstacle(entity: EntityId, body: &Node) -> NavMeshObstacleComponent {
    let shape = NavMeshObstacleShape::from_i32(yaml_i32(body.get("m_Shape"), 1));
    let size = vector3_or(
        body.get("m_Size").or_else(|| body.get("m_Extents")),
        [1.0, 1.0, 1.0],
    );
    let radius = yaml_f64(body.get("m_Radius"), size[0] * 0.5, |v| v > 0.0);
    let height = yaml_f64(body.get("m_Height"), size[1], |v| v > 0.0);
    NavMeshObstacleComponent {
        entity,
        enabled: yaml_bool(body.get("m_Enabled"), true),
        shape,
        center: vector3_or(body.get("m_Center"), [0.0, 0.0, 0.0]),
        size,
        radius,
        height,
        carving: yaml_bool(body.get("m_Carve"), false),
        carve_only_stationary: yaml_bool(body.get("m_CarveOnlyStationary"), true),
        move_threshold: yaml_f64(body.get("m_MoveThreshold"), 0.1, |v| v >= 0.0),
        time_to_stationary: yaml_f64(body.get("m_TimeToStationary"), 0.5, |v| v >= 0.0),
    }
}

fn parse_off_mesh_link(
    entity: EntityId,
    body: &Node,
    game_objects: &GameObjectRecords,
    transforms: &HashMap<i64, EntityId>,
) -> Result<OffMeshLinkComponent, SceneError> {
    let start = resolve_link_endpoint(body.get("m_Start"), game_objects, transforms)?;
    let end = resolve_link_endpoint(body.get("m_End"), game_objects, transforms)?;
    Ok(OffMeshLinkComponent {
        entity,
        enabled: yaml_bool(body.get("m_Enabled"), true),
        start,
        end,
        cost_override: yaml_f64(body.get("m_CostOverride"), -1.0, |v| v.is_finite()),
        bi_directional: yaml_bool(body.get("m_BiDirectional"), true),
        activated: yaml_bool(body.get("m_Activated"), true),
        auto_update_positions: yaml_bool(body.get("m_AutoUpdatePositions"), false),
        area_index: yaml_u32(
            body.get("m_AreaIndex")
                .or_else(|| body.get("m_NavMeshLayer")),
            0,
        ),
    })
}

fn resolve_link_endpoint(
    node: Option<&Node>,
    game_objects: &GameObjectRecords,
    transforms: &HashMap<i64, EntityId>,
) -> Result<Option<EntityId>, SceneError> {
    let Some(file_id) = reference_id(node)? else {
        return Ok(None);
    };
    if file_id == 0 {
        return Ok(None);
    }
    if let Some(entity) = transforms.get(&file_id) {
        return Ok(Some(*entity));
    }
    Ok(game_objects.get(&file_id).map(|record| record.entity))
}

struct StandaloneMeshFilter {
    entity: EntityId,
    mesh_guid: Option<String>,
    mesh_file_id: Option<i64>,
    mesh_path: Option<std::path::PathBuf>,
}

fn collect_standalone_mesh_filters(
    stream: &Stream,
    game_objects: &GameObjectRecords,
    instance_roots: &HashSet<i64>,
    index: Option<&GuidIndex>,
) -> Vec<StandaloneMeshFilter> {
    let mut filters = Vec::new();
    for doc in &stream.documents {
        if doc.stripped || doc.class_id != MESH_FILTER_CLASS {
            continue;
        }
        let Some(body) = doc.root.get("MeshFilter") else {
            continue;
        };
        let Some(entity) = component_entity(body, game_objects, instance_roots) else {
            continue;
        };
        let (mesh_guid, mesh_file_id, mesh_path, _) =
            resolve_mesh_reference_for(body, index, "MeshFilter");
        filters.push(StandaloneMeshFilter {
            entity,
            mesh_guid,
            mesh_file_id,
            mesh_path,
        });
    }
    filters
}

fn parse_nav_mesh_surface(
    entity: EntityId,
    body: &Node,
    graph: &SceneGraph,
    game_objects: &GameObjectRecords,
    physics: &[PhysicsBinding],
    render: &RenderBinding,
    mesh_filters: &[StandaloneMeshFilter],
) -> NavMeshSurfaceComponent {
    let collect_objects = CollectObjects::from_i32(yaml_i32(body.get("m_CollectObjects"), 0));
    let size = vector3_or(body.get("m_Size"), [10.0, 10.0, 10.0]);
    let center = vector3_or(body.get("m_Center"), [0.0, 0.0, 0.0]);
    let layer_mask = body
        .get("m_LayerMask")
        .and_then(|n| n.get("m_Bits").or(Some(n)))
        .and_then(Node::as_i64)
        .map_or(u32::MAX, |bits| bits as u32);
    let default_area = yaml_u32(body.get("m_DefaultArea"), 0);
    let surface_ctx = SurfaceGatherContext {
        surface_entity: entity,
        collect_objects,
        size,
        center,
        layer_mask,
        default_area,
    };
    let triangles = gather_surface_triangles(
        &surface_ctx,
        graph,
        game_objects,
        physics,
        render,
        mesh_filters,
    );
    NavMeshSurfaceComponent {
        entity,
        enabled: yaml_bool(body.get("m_Enabled"), true),
        agent_type_id: yaml_i32(body.get("m_AgentTypeID"), 0),
        collect_objects,
        size,
        center,
        layer_mask,
        use_geometry: yaml_i32(body.get("m_UseGeometry"), 0),
        default_area,
        triangles,
    }
}

struct SurfaceGatherContext {
    surface_entity: EntityId,
    collect_objects: CollectObjects,
    size: [f64; 3],
    center: [f64; 3],
    layer_mask: u32,
    default_area: u32,
}

fn gather_surface_triangles(
    ctx: &SurfaceGatherContext,
    graph: &SceneGraph,
    game_objects: &GameObjectRecords,
    physics: &[PhysicsBinding],
    render: &RenderBinding,
    mesh_filters: &[StandaloneMeshFilter],
) -> Vec<SceneNavMeshTriangle> {
    let mut triangles = Vec::new();
    let mut covered_entities = HashSet::new();
    let entity_layers: HashMap<EntityId, u32> = game_objects
        .values()
        .map(|record| (record.entity, record.layer))
        .collect();
    for filter in mesh_filters {
        if !entity_matches_surface(ctx, filter.entity, graph, &entity_layers) {
            continue;
        }
        if let Some(mesh) = resolve_collision_mesh(
            filter.mesh_path.as_deref(),
            filter.mesh_file_id,
            filter.mesh_guid.as_deref(),
        ) {
            append_world_triangles(
                &mut triangles,
                graph,
                filter.entity,
                &mesh,
                ctx.default_area,
            );
            covered_entities.insert(filter.entity);
        }
    }
    for renderer in &render.renderers {
        if covered_entities.contains(&renderer.entity)
            || !entity_matches_surface(ctx, renderer.entity, graph, &entity_layers)
        {
            continue;
        }
        if let Some(mesh) = resolve_renderer_mesh(renderer) {
            append_world_triangles(
                &mut triangles,
                graph,
                renderer.entity,
                &mesh,
                ctx.default_area,
            );
            covered_entities.insert(renderer.entity);
        }
    }
    gather_collider_triangles(
        ctx,
        graph,
        physics,
        &entity_layers,
        &covered_entities,
        &mut triangles,
    );
    triangles
}

fn gather_collider_triangles(
    ctx: &SurfaceGatherContext,
    graph: &SceneGraph,
    physics: &[PhysicsBinding],
    entity_layers: &HashMap<EntityId, u32>,
    covered_entities: &HashSet<EntityId>,
    triangles: &mut Vec<SceneNavMeshTriangle>,
) {
    for binding in physics {
        if covered_entities.contains(&binding.entity)
            || !entity_matches_surface(ctx, binding.entity, graph, entity_layers)
        {
            continue;
        }
        for collider in &binding.colliders {
            match collider {
                ColliderComponent::Mesh(mesh_col) => {
                    if let Some(mesh) = resolve_mesh_collider_geometry(mesh_col) {
                        append_world_triangles(
                            triangles,
                            graph,
                            binding.entity,
                            &mesh,
                            ctx.default_area,
                        );
                    }
                }
                ColliderComponent::Box(box_col) if !box_col.is_trigger => {
                    append_box_collider_top_triangles(
                        triangles,
                        graph,
                        binding.entity,
                        box_col,
                        ctx.default_area,
                    );
                }
                _ => {}
            }
        }
    }
}

fn entity_matches_surface(
    ctx: &SurfaceGatherContext,
    candidate: EntityId,
    graph: &SceneGraph,
    entity_layers: &HashMap<EntityId, u32>,
) -> bool {
    let layer = entity_layers.get(&candidate).copied().unwrap_or(0);
    if layer < 32 && (ctx.layer_mask & (1u32 << layer)) == 0 {
        return false;
    }
    match ctx.collect_objects {
        CollectObjects::All => true,
        CollectObjects::Children => is_self_or_descendant(graph, candidate, ctx.surface_entity),
        CollectObjects::Volume => is_within_surface_volume(ctx, graph, candidate),
    }
}

fn is_self_or_descendant(graph: &SceneGraph, candidate: EntityId, ancestor: EntityId) -> bool {
    let mut current = Some(candidate);
    while let Some(entity) = current {
        if entity == ancestor {
            return true;
        }
        current = graph.parent(entity);
    }
    false
}

fn is_within_surface_volume(
    ctx: &SurfaceGatherContext,
    graph: &SceneGraph,
    candidate: EntityId,
) -> bool {
    let surface_world = graph
        .world_transform(ctx.surface_entity)
        .unwrap_or_default();
    let candidate_world = graph.world_transform(candidate).unwrap_or_default();
    let center = [
        f64::from(surface_world.translation[0]) + ctx.center[0],
        f64::from(surface_world.translation[1]) + ctx.center[1],
        f64::from(surface_world.translation[2]) + ctx.center[2],
    ];
    let pos = [
        f64::from(candidate_world.translation[0]),
        f64::from(candidate_world.translation[1]),
        f64::from(candidate_world.translation[2]),
    ];
    (pos[0] - center[0]).abs() <= ctx.size[0] * 0.5
        && (pos[1] - center[1]).abs() <= ctx.size[1] * 0.5
        && (pos[2] - center[2]).abs() <= ctx.size[2] * 0.5
}

fn resolve_renderer_mesh(renderer: &MeshRendererComponent) -> Option<CollisionMesh> {
    resolve_collision_mesh(
        renderer.mesh_path.as_deref(),
        renderer.mesh_file_id,
        renderer.mesh_guid.as_deref(),
    )
}

fn resolve_mesh_collider_geometry(collider: &MeshColliderComponent) -> Option<CollisionMesh> {
    resolve_collision_mesh(
        collider.mesh_path.as_deref(),
        collider.mesh_file_id,
        collider.mesh_guid.as_deref(),
    )
}

fn resolve_collision_mesh(
    mesh_path: Option<&Path>,
    mesh_file_id: Option<i64>,
    mesh_guid: Option<&str>,
) -> Option<CollisionMesh> {
    if let Some(path) = mesh_path {
        if let Ok(bytes) = std::fs::read(path) {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if let Ok(mesh) = cook_collision_mesh(&bytes, file_name, mesh_file_id) {
                return Some(mesh);
            }
        }
    }
    if let (Some(file_id), Some(guid)) = (mesh_file_id, mesh_guid) {
        if let Some(mesh) = cook_builtin_collision_mesh(file_id, guid) {
            return Some(mesh);
        }
    }
    if let Some(file_id) = mesh_file_id {
        let primitive = match file_id {
            10202 => Some(BuiltinPrimitive::Cube),
            10206 => Some(BuiltinPrimitive::Cylinder),
            10207 => Some(BuiltinPrimitive::Sphere),
            10208 => Some(BuiltinPrimitive::Capsule),
            10209 => Some(BuiltinPrimitive::Plane),
            10210 => Some(BuiltinPrimitive::Quad),
            _ => None,
        };
        if let Some(prim) = primitive {
            let data = primitive_mesh(prim);
            return Some(CollisionMesh {
                vertices: data.positions,
                indices: data.indices,
            });
        }
    }
    None
}

fn append_world_triangles(
    triangles: &mut Vec<SceneNavMeshTriangle>,
    graph: &SceneGraph,
    entity: EntityId,
    mesh: &CollisionMesh,
    area: u32,
) {
    let world = graph.world_transform(entity).unwrap_or_default();
    for chunk in mesh.indices.chunks_exact(3) {
        let (Some(&v0), Some(&v1), Some(&v2)) = (
            mesh.vertices.get(chunk[0] as usize),
            mesh.vertices.get(chunk[1] as usize),
            mesh.vertices.get(chunk[2] as usize),
        ) else {
            continue;
        };
        let p0 = transform_point(&world, v0);
        let p1 = transform_point(&world, v1);
        let p2 = transform_point(&world, v2);
        if is_walkable_upward_triangle(p0, p1, p2) {
            triangles.push(SceneNavMeshTriangle {
                vertices: [p0, p1, p2],
                area,
            });
        }
    }
}

fn append_box_collider_top_triangles(
    triangles: &mut Vec<SceneNavMeshTriangle>,
    graph: &SceneGraph,
    entity: EntityId,
    box_col: &BoxColliderComponent,
    area: u32,
) {
    let world = graph.world_transform(entity).unwrap_or_default();
    let hx = (box_col.size[0] * 0.5) as f32;
    let hy = (box_col.size[1] * 0.5) as f32;
    let hz = (box_col.size[2] * 0.5) as f32;
    let cx = box_col.center[0] as f32;
    let cy = box_col.center[1] as f32 + hy;
    let cz = box_col.center[2] as f32;
    let p0 = transform_point(&world, [cx - hx, cy, cz - hz]);
    let p1 = transform_point(&world, [cx - hx, cy, cz + hz]);
    let p2 = transform_point(&world, [cx + hx, cy, cz + hz]);
    let p3 = transform_point(&world, [cx + hx, cy, cz - hz]);
    triangles.push(SceneNavMeshTriangle {
        vertices: [p0, p1, p2],
        area,
    });
    triangles.push(SceneNavMeshTriangle {
        vertices: [p0, p2, p3],
        area,
    });
}

fn transform_point(world: &Transform, local: [f32; 3]) -> [f64; 3] {
    let scaled = [
        local[0] * world.scale[0],
        local[1] * world.scale[1],
        local[2] * world.scale[2],
    ];
    let rotated = rotate_vector(world.rotation, scaled);
    [
        f64::from(world.translation[0] + rotated[0]),
        f64::from(world.translation[1] + rotated[1]),
        f64::from(world.translation[2] + rotated[2]),
    ]
}

fn is_walkable_upward_triangle(p0: [f64; 3], p1: [f64; 3], p2: [f64; 3]) -> bool {
    let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
    let nx = e1[1] * e2[2] - e1[2] * e2[1];
    let ny = e1[2] * e2[0] - e1[0] * e2[2];
    let nz = e1[0] * e2[1] - e1[1] * e2[0];
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    len > 1e-6 && (ny.abs() / len) >= 0.5
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use unitas_core::EntityId;

    use super::{CollectObjects, NavMeshObstacleShape, NAV_MESH_SURFACE_SCRIPT_GUID};
    use crate::scene::assemble_scene_bundle;
    use crate::unity_yaml;

    #[test]
    fn extracts_navmesh_agent_obstacle_link_and_surface() -> Result<(), Box<dyn Error>> {
        let yaml = format!(
            "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Name: Floor
--- !u!4 &101
Transform:
  m_GameObject: {{fileID: 100}}
  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}
  m_LocalPosition: {{x: 0, y: 0, z: 0}}
  m_LocalScale: {{x: 1, y: 1, z: 1}}
  m_Children: []
  m_Father: {{fileID: 0}}
--- !u!33 &102
MeshFilter:
  m_GameObject: {{fileID: 100}}
  m_Mesh: {{fileID: 10209, guid: 0000000000000000e000000000000000, type: 0}}
--- !u!114 &103
MonoBehaviour:
  m_GameObject: {{fileID: 100}}
  m_Enabled: 1
  m_Script: {{fileID: 11500000, guid: {NAV_MESH_SURFACE_SCRIPT_GUID}, type: 3}}
  m_AgentTypeID: 0
  m_CollectObjects: 0
  m_DefaultArea: 0
--- !u!1 &200
GameObject:
  m_Name: Runner
--- !u!4 &201
Transform:
  m_GameObject: {{fileID: 200}}
  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}
  m_LocalPosition: {{x: -2, y: 0, z: 0}}
  m_LocalScale: {{x: 1, y: 1, z: 1}}
  m_Children: []
  m_Father: {{fileID: 0}}
--- !u!195 &202
NavMeshAgent:
  m_GameObject: {{fileID: 200}}
  m_Enabled: 1
  m_Radius: 0.4
  m_Speed: 4.25
  m_StoppingDistance: 0.2
  m_Height: 1.8
  avoidancePriority: 35
--- !u!1 &300
GameObject:
  m_Name: Blocker
--- !u!4 &301
Transform:
  m_GameObject: {{fileID: 300}}
  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}
  m_LocalPosition: {{x: 0, y: 0, z: 0}}
  m_LocalScale: {{x: 1, y: 1, z: 1}}
  m_Children: []
  m_Father: {{fileID: 0}}
--- !u!208 &302
NavMeshObstacle:
  m_GameObject: {{fileID: 300}}
  m_Enabled: 1
  m_Shape: 1
  m_Center: {{x: 0, y: 0.5, z: 0}}
  m_Size: {{x: 2, y: 1, z: 2}}
  m_Carve: 1
  m_CarveOnlyStationary: 0
--- !u!191 &303
OffMeshLink:
  m_GameObject: {{fileID: 300}}
  m_Enabled: 1
  m_Start: {{fileID: 101}}
  m_End: {{fileID: 201}}
  m_BiDirectional: 1
  m_Activated: 1
  m_AreaIndex: 2
"
        );
        let assembled = assemble_scene_bundle(&unity_yaml::parse(&yaml)?)?;
        assert_eq!(assembled.navigation.agents.len(), 1);
        let agent = &assembled.navigation.agents[0];
        assert_eq!(agent.entity, EntityId::new(1));
        assert_eq!(agent.speed, 4.25);
        assert_eq!(agent.radius, 0.4);
        assert_eq!(agent.avoidance_priority, 35);

        assert_eq!(assembled.navigation.obstacles.len(), 1);
        let obstacle = &assembled.navigation.obstacles[0];
        assert_eq!(obstacle.entity, EntityId::new(2));
        assert_eq!(obstacle.shape, NavMeshObstacleShape::Box);
        assert!(obstacle.carving);

        assert_eq!(assembled.navigation.links.len(), 1);
        let link = &assembled.navigation.links[0];
        assert_eq!(link.start, Some(EntityId::new(0)));
        assert_eq!(link.end, Some(EntityId::new(1)));
        assert_eq!(link.area_index, 2);

        assert_eq!(assembled.navigation.surfaces.len(), 1);
        let surface = &assembled.navigation.surfaces[0];
        assert_eq!(surface.collect_objects, CollectObjects::All);
        assert_eq!(surface.triangles.len(), 2);
        assert!(assembled.diagnostics.is_empty());
        Ok(())
    }
}
