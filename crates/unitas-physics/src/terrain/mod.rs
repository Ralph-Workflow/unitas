use std::cell::RefCell;
use std::rc::Rc;

use unitas_core::EntityId;

use crate::body::{capsule_half_segment, capsule_local_axis, dot, rotate_vector, subtract, Shape};
use crate::world::World;

pub const LARGE_WORLD_PRECISION_THRESHOLD: f64 = 100_000.0;

const DEFAULT_FRICTION: f64 = 0.6;
const MIN_NORMAL_Y: f64 = 1e-6;

#[derive(Clone, Debug, PartialEq)]
pub enum TerrainError {
    InvalidHeightmapResolution(usize),
    InvalidSize([f64; 3]),
}

impl std::fmt::Display for TerrainError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHeightmapResolution(resolution) => {
                write!(
                    formatter,
                    "heightmap resolution {resolution} must be at least one cell per axis"
                )
            }
            Self::InvalidSize(size) => {
                write!(
                    formatter,
                    "terrain size {size:?} needs positive finite components"
                )
            }
        }
    }
}

impl std::error::Error for TerrainError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainBounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainContact {
    pub point: [f64; 3],
    pub normal: [f64; 3],
    pub depth: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LargeWorldImpact {
    pub position: [f64; 3],
    pub threshold: f64,
    pub exceeds_threshold: bool,
    pub affected_systems: Vec<&'static str>,
}

impl LargeWorldImpact {
    pub fn for_position(position: [f64; 3]) -> Self {
        let mut magnitude = 0.0_f64;
        for component in position {
            magnitude = magnitude.max(component.abs());
        }
        let exceeds_threshold = magnitude >= LARGE_WORLD_PRECISION_THRESHOLD;
        let affected_systems = if exceeds_threshold {
            vec![
                "physics float-precision position handling",
                "rendering float-precision position handling",
                "navigation float-precision path handling",
            ]
        } else {
            Vec::new()
        };
        Self {
            position,
            threshold: LARGE_WORLD_PRECISION_THRESHOLD,
            exceeds_threshold,
            affected_systems,
        }
    }

    pub fn summary(&self) -> String {
        if !self.exceeds_threshold {
            return format!(
                "world position {:?} stays within the {} unit float-precision threshold",
                self.position, self.threshold
            );
        }
        format!(
            "world position {:?} reaches the {} unit float-precision threshold; affected systems: {}",
            self.position,
            self.threshold,
            self.affected_systems.join(", ")
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreePrototype {
    pub name: String,
    pub wind_excluded: bool,
    pub lod_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreeInstance {
    pub prototype: usize,
    pub normalized_position: [f64; 2],
    pub height_offset: f64,
    pub rotation_y: f64,
    pub scale: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DetailInstance {
    pub prototype: usize,
    pub normalized_position: [f64; 2],
    pub height_offset: f64,
    pub scale: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainBody {
    pub entity: EntityId,
    pub shape: Shape,
}

impl TerrainBody {
    pub fn new(entity: EntityId, shape: Shape) -> Self {
        Self { entity, shape }
    }
}

#[derive(Clone, Debug)]
struct TerrainHeightData {
    heightmap_resolution: usize,
    size: [f64; 3],
    heights: Vec<f64>,
    holes: Vec<bool>,
    tree_prototypes: Vec<TreePrototype>,
    tree_instances: Vec<TreeInstance>,
    detail_instances: Vec<DetailInstance>,
}

impl TerrainHeightData {
    fn clamp_indices(&self, column: usize, row: usize) -> (usize, usize) {
        let last = self.heightmap_resolution - 1;
        (column.min(last), row.min(last))
    }

    fn index(&self, column: usize, row: usize) -> usize {
        row * self.heightmap_resolution + column
    }
}

#[derive(Clone, Debug)]
pub struct TerrainData {
    inner: Rc<RefCell<TerrainHeightData>>,
}

impl TerrainData {
    pub fn new(heightmap_resolution: usize, size: [f64; 3]) -> Result<Self, TerrainError> {
        if heightmap_resolution == 0 {
            return Err(TerrainError::InvalidHeightmapResolution(
                heightmap_resolution,
            ));
        }
        if !size
            .iter()
            .all(|component| component.is_finite() && *component > 0.0)
        {
            return Err(TerrainError::InvalidSize(size));
        }
        let cells = heightmap_resolution * heightmap_resolution;
        Ok(Self {
            inner: Rc::new(RefCell::new(TerrainHeightData {
                heightmap_resolution,
                size,
                heights: vec![0.0; cells],
                holes: vec![false; cells],
                tree_prototypes: Vec::new(),
                tree_instances: Vec::new(),
                detail_instances: Vec::new(),
            })),
        })
    }

    pub fn heightmap_resolution(&self) -> usize {
        self.inner.borrow().heightmap_resolution
    }

    pub fn size(&self) -> [f64; 3] {
        self.inner.borrow().size
    }

    pub fn height_at(&self, column: usize, row: usize) -> f64 {
        let inner = self.inner.borrow();
        let (column, row) = inner.clamp_indices(column, row);
        inner.heights[inner.index(column, row)]
    }

    pub fn set_height(&self, column: usize, row: usize, height: f64) {
        let mut inner = self.inner.borrow_mut();
        let (column, row) = inner.clamp_indices(column, row);
        let index = inner.index(column, row);
        inner.heights[index] = height;
    }

    pub fn set_heights(&self, updates: &[([usize; 2], f64)]) {
        let mut inner = self.inner.borrow_mut();
        for (position, height) in updates {
            let (column, row) = inner.clamp_indices(position[0], position[1]);
            let index = inner.index(column, row);
            inner.heights[index] = *height;
        }
    }

    pub fn heights(&self) -> Vec<f64> {
        self.inner.borrow().heights.clone()
    }

    pub fn hole_at(&self, column: usize, row: usize) -> bool {
        let inner = self.inner.borrow();
        let (column, row) = inner.clamp_indices(column, row);
        inner.holes[inner.index(column, row)]
    }

    pub fn set_hole(&self, column: usize, row: usize, is_hole: bool) {
        let mut inner = self.inner.borrow_mut();
        let (column, row) = inner.clamp_indices(column, row);
        let index = inner.index(column, row);
        inner.holes[index] = is_hole;
    }

    pub fn add_tree_prototype(&self, prototype: TreePrototype) -> usize {
        let mut inner = self.inner.borrow_mut();
        inner.tree_prototypes.push(prototype);
        inner.tree_prototypes.len() - 1
    }

    pub fn tree_prototypes(&self) -> Vec<TreePrototype> {
        self.inner.borrow().tree_prototypes.clone()
    }

    pub fn set_tree_instances(&self, instances: &[TreeInstance]) {
        self.inner.borrow_mut().tree_instances = instances.to_vec();
    }

    pub fn tree_instances(&self) -> Vec<TreeInstance> {
        self.inner.borrow().tree_instances.clone()
    }

    pub fn set_detail_instances(&self, instances: &[DetailInstance]) {
        self.inner.borrow_mut().detail_instances = instances.to_vec();
    }

    pub fn detail_instances(&self) -> Vec<DetailInstance> {
        self.inner.borrow().detail_instances.clone()
    }

    pub fn wind_excluded_prototypes(&self) -> Vec<usize> {
        self.inner
            .borrow()
            .tree_prototypes
            .iter()
            .enumerate()
            .filter(|(_, prototype)| prototype.wind_excluded)
            .map(|(index, _)| index)
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct Terrain {
    data: TerrainData,
    position: [f64; 3],
    collider_enabled: bool,
}

impl Terrain {
    pub fn new(data: TerrainData) -> Self {
        Self {
            data,
            position: [0.0; 3],
            collider_enabled: true,
        }
    }

    pub fn with_position(mut self, position: [f64; 3]) -> Self {
        self.position = position;
        self
    }

    pub fn with_collider_enabled(mut self, enabled: bool) -> Self {
        self.collider_enabled = enabled;
        self
    }

    pub fn position(&self) -> [f64; 3] {
        self.position
    }

    pub fn data(&self) -> &TerrainData {
        &self.data
    }

    pub fn size(&self) -> [f64; 3] {
        self.data.size()
    }

    pub fn collider_enabled(&self) -> bool {
        self.collider_enabled
    }

    pub fn bounds(&self) -> TerrainBounds {
        let size = self.data.size();
        let inner = self.data.inner.borrow();
        let mut lowest = f64::INFINITY;
        let mut highest = f64::NEG_INFINITY;
        for height in &inner.heights {
            lowest = lowest.min(*height);
            highest = highest.max(*height);
        }
        TerrainBounds {
            min: [
                self.position[0] - size[0] * 0.5,
                self.position[1] + lowest,
                self.position[2] - size[2] * 0.5,
            ],
            max: [
                self.position[0] + size[0] * 0.5,
                self.position[1] + highest,
                self.position[2] + size[2] * 0.5,
            ],
        }
    }

    pub fn height_at(&self, world_position: [f64; 3]) -> Option<f64> {
        self.sample_surface(world_position)
            .map(|(surface, _)| surface[1])
    }

    pub fn normal_at(&self, world_position: [f64; 3]) -> Option<[f64; 3]> {
        self.sample_surface(world_position)
            .map(|(_, normal)| normal)
    }

    pub fn is_hole(&self, world_position: [f64; 3]) -> bool {
        match self.cell_index(world_position[0], world_position[2]) {
            Some((column, row)) => self.data.hole_at(column, row),
            None => false,
        }
    }

    pub fn is_walkable(&self, world_position: [f64; 3]) -> bool {
        match self.cell_index(world_position[0], world_position[2]) {
            Some((column, row)) => !self.data.hole_at(column, row),
            None => false,
        }
    }

    pub fn is_collidable(&self, world_position: [f64; 3]) -> bool {
        self.collider_enabled && self.is_walkable(world_position)
    }

    pub fn contact(
        &self,
        shape: &Shape,
        position: [f64; 3],
        rotation: [f64; 4],
    ) -> Option<TerrainContact> {
        if !self.collider_enabled {
            return None;
        }
        match shape {
            Shape::Sphere { radius } => self.sphere_contact(*radius, position),
            Shape::Box { half_extents } => self.box_contact(*half_extents, position, rotation),
            Shape::Capsule {
                radius,
                height,
                direction,
            } => self.capsule_contact(*radius, *height, *direction, position, rotation),
        }
    }

    pub fn support_body(&self, world: &mut World, body: &TerrainBody) -> Option<TerrainContact> {
        let position = world.position(body.entity)?;
        let rotation = world.rotation(body.entity)?;
        let velocity = world.velocity(body.entity)?;
        let contact = self.contact(&body.shape, position, rotation)?;
        let resolved_position = self.resolved_position(position, &contact);
        let normal = contact.normal;
        let normal_velocity = dot(velocity, normal);
        if normal_velocity < 0.0 {
            let without_normal = subtract(
                velocity,
                [
                    normal[0] * normal_velocity,
                    normal[1] * normal_velocity,
                    normal[2] * normal_velocity,
                ],
            );
            let friction = world
                .body_material(body.entity)
                .map_or(DEFAULT_FRICTION, |material| material.friction);
            let along_normal = dot(without_normal, normal);
            let tangential = subtract(
                without_normal,
                [
                    normal[0] * along_normal,
                    normal[1] * along_normal,
                    normal[2] * along_normal,
                ],
            );
            let tangential_speed = dot(tangential, tangential).sqrt();
            let limit = friction * -normal_velocity;
            let friction_velocity = if tangential_speed > limit {
                let remaining = 1.0 - limit / tangential_speed;
                [
                    tangential[0] * remaining,
                    tangential[1] * remaining,
                    tangential[2] * remaining,
                ]
            } else {
                [0.0; 3]
            };
            let resolved_velocity = [
                normal[0] * along_normal + friction_velocity[0],
                normal[1] * along_normal + friction_velocity[1],
                normal[2] * along_normal + friction_velocity[2],
            ];
            world.set_velocity(body.entity, resolved_velocity).ok()?;
        }
        world
            .teleport_position(body.entity, resolved_position)
            .ok()?;
        Some(contact)
    }

    pub fn support_bodies(&self, world: &mut World, bodies: &[TerrainBody]) -> Vec<TerrainContact> {
        let mut contacts = Vec::new();
        for body in bodies {
            if let Some(contact) = self.support_body(world, body) {
                contacts.push(contact);
            }
        }
        contacts
    }

    pub fn large_world_impact(&self) -> LargeWorldImpact {
        LargeWorldImpact::for_position(self.position)
    }

    pub fn tree_world_positions(&self) -> Vec<[f64; 3]> {
        let mut positions = Vec::new();
        for instance in self.data.tree_instances() {
            positions.push(
                self.instance_world_position(instance.normalized_position, instance.height_offset),
            );
        }
        positions
    }

    pub fn detail_world_positions(&self) -> Vec<[f64; 3]> {
        let mut positions = Vec::new();
        for instance in self.data.detail_instances() {
            positions.push(
                self.instance_world_position(instance.normalized_position, instance.height_offset),
            );
        }
        positions
    }

    fn resolved_position(&self, position: [f64; 3], contact: &TerrainContact) -> [f64; 3] {
        let normal = contact.normal;
        if normal[1] > MIN_NORMAL_Y {
            let lift = contact.depth / normal[1];
            [position[0], position[1] + lift, position[2]]
        } else {
            [
                position[0] + normal[0] * contact.depth,
                position[1] + normal[1] * contact.depth,
                position[2] + normal[2] * contact.depth,
            ]
        }
    }

    fn sphere_contact(&self, radius: f64, position: [f64; 3]) -> Option<TerrainContact> {
        let (surface, normal) = self.sample_surface(position)?;
        if self.is_hole(position) {
            return None;
        }
        let offset = subtract(position, surface);
        let distance = dot(offset, normal);
        if distance >= radius {
            return None;
        }
        Some(TerrainContact {
            point: surface,
            normal,
            depth: radius - distance,
        })
    }

    fn box_contact(
        &self,
        half_extents: [f64; 3],
        position: [f64; 3],
        rotation: [f64; 4],
    ) -> Option<TerrainContact> {
        let offsets = [
            [-half_extents[0], -half_extents[1], -half_extents[2]],
            [half_extents[0], -half_extents[1], -half_extents[2]],
            [-half_extents[0], -half_extents[1], half_extents[2]],
            [half_extents[0], -half_extents[1], half_extents[2]],
            [0.0, -half_extents[1], 0.0],
        ];
        let mut deepest: Option<TerrainContact> = None;
        for offset in offsets {
            let rotated = rotate_vector(rotation, offset);
            let sample = [
                position[0] + rotated[0],
                position[1] + rotated[1],
                position[2] + rotated[2],
            ];
            if self.is_hole(sample) {
                continue;
            }
            let Some((surface, normal)) = self.sample_surface(sample) else {
                continue;
            };
            let penetration = surface[1] - sample[1];
            if penetration <= 0.0 {
                continue;
            }
            let candidate = TerrainContact {
                point: surface,
                normal,
                depth: penetration * normal[1],
            };
            deepest = match deepest {
                Some(current) if current.depth >= candidate.depth => Some(current),
                _ => Some(candidate),
            };
        }
        deepest
    }

    fn capsule_contact(
        &self,
        radius: f64,
        height: f64,
        direction: u8,
        position: [f64; 3],
        rotation: [f64; 4],
    ) -> Option<TerrainContact> {
        let half = capsule_half_segment(radius, height);
        let axis = rotate_vector(rotation, capsule_local_axis(direction));
        let bottom = [
            position[0] - axis[0] * half,
            position[1] - axis[1] * half,
            position[2] - axis[2] * half,
        ];
        self.sphere_contact(radius, bottom)
    }

    fn cell_index(&self, world_x: f64, world_z: f64) -> Option<(usize, usize)> {
        let size = self.data.size();
        let resolution = self.data.heightmap_resolution();
        let local_x = world_x - self.position[0];
        let local_z = world_z - self.position[2];
        let half_x = size[0] * 0.5;
        let half_z = size[2] * 0.5;
        if local_x < -half_x || local_x > half_x || local_z < -half_z || local_z > half_z {
            return None;
        }
        let cell_width = size[0] / resolution as f64;
        let cell_depth = size[2] / resolution as f64;
        let last = resolution - 1;
        let column = (((local_x + half_x) / cell_width).floor() as usize).min(last);
        let row = (((local_z + half_z) / cell_depth).floor() as usize).min(last);
        Some((column, row))
    }

    fn sample_surface(&self, world_position: [f64; 3]) -> Option<([f64; 3], [f64; 3])> {
        let size = self.data.size();
        let resolution = self.data.heightmap_resolution();
        let local_x = world_position[0] - self.position[0];
        let local_z = world_position[2] - self.position[2];
        let half_x = size[0] * 0.5;
        let half_z = size[2] * 0.5;
        if local_x < -half_x || local_x > half_x || local_z < -half_z || local_z > half_z {
            return None;
        }
        let cell_width = size[0] / resolution as f64;
        let cell_depth = size[2] / resolution as f64;
        let grid_x = ((local_x + half_x) / cell_width - 0.5).clamp(0.0, (resolution - 1) as f64);
        let grid_z = ((local_z + half_z) / cell_depth - 0.5).clamp(0.0, (resolution - 1) as f64);
        let height = self.interpolated_height(grid_x, grid_z);
        let normal = self.surface_normal(grid_x, grid_z, cell_width, cell_depth);
        Some((
            [
                self.position[0] + local_x,
                self.position[1] + height,
                self.position[2] + local_z,
            ],
            normal,
        ))
    }

    fn interpolated_height(&self, grid_x: f64, grid_z: f64) -> f64 {
        let inner = self.data.inner.borrow();
        let resolution = inner.heightmap_resolution;
        let last = (resolution - 1) as f64;
        let grid_x = grid_x.clamp(0.0, last);
        let grid_z = grid_z.clamp(0.0, last);
        let column0 = grid_x.floor() as usize;
        let row0 = grid_z.floor() as usize;
        let column1 = (column0 + 1).min(resolution - 1);
        let row1 = (row0 + 1).min(resolution - 1);
        let blend_x = grid_x - column0 as f64;
        let blend_z = grid_z - row0 as f64;
        let lower_left = inner.heights[inner.index(column0, row0)];
        let lower_right = inner.heights[inner.index(column1, row0)];
        let upper_left = inner.heights[inner.index(column0, row1)];
        let upper_right = inner.heights[inner.index(column1, row1)];
        let lower = lower_left + (lower_right - lower_left) * blend_x;
        let upper = upper_left + (upper_right - upper_left) * blend_x;
        lower + (upper - lower) * blend_z
    }

    fn surface_normal(
        &self,
        grid_x: f64,
        grid_z: f64,
        cell_width: f64,
        cell_depth: f64,
    ) -> [f64; 3] {
        let ahead_x = self.interpolated_height(grid_x + 0.5, grid_z);
        let behind_x = self.interpolated_height(grid_x - 0.5, grid_z);
        let ahead_z = self.interpolated_height(grid_x, grid_z + 0.5);
        let behind_z = self.interpolated_height(grid_x, grid_z - 0.5);
        let slope_x = (ahead_x - behind_x) / cell_width;
        let slope_z = (ahead_z - behind_z) / cell_depth;
        let length = (slope_x * slope_x + 1.0 + slope_z * slope_z).sqrt();
        [-slope_x / length, 1.0 / length, -slope_z / length]
    }

    fn instance_world_position(
        &self,
        normalized_position: [f64; 2],
        height_offset: f64,
    ) -> [f64; 3] {
        let size = self.data.size();
        let local_x = -size[0] * 0.5 + normalized_position[0] * size[0];
        let local_z = -size[2] * 0.5 + normalized_position[1] * size[2];
        let world_x = self.position[0] + local_x;
        let world_z = self.position[2] + local_z;
        let height = match self.sample_surface([world_x, self.position[1], world_z]) {
            Some((surface, _)) => surface[1],
            None => self.position[1],
        };
        [world_x, height + height_offset, world_z]
    }
}
