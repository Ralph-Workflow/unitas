use std::cell::RefCell;
use std::rc::Rc;

use unitas_core::EntityId;

use crate::body::{
    capsule_half_segment, capsule_local_axis, distance, dot, rotate_vector, subtract, vector_length,
    Shape,
};
use crate::world::World;

pub const LARGE_WORLD_PRECISION_THRESHOLD: f64 = 100_000.0;

const DEFAULT_FRICTION: f64 = 0.6;
const MIN_NORMAL_Y: f64 = 1e-6;
const RAYCAST_STEPS: usize = 64;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainRayHit {
    pub point: [f64; 3],
    pub normal: [f64; 3],
    pub distance: f64,
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
                "animation float-precision transform handling",
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

#[derive(Clone, Debug, PartialEq)]
pub struct TerrainLayer {
    pub name: String,
    pub tile_size: [f64; 2],
}

impl TerrainLayer {
    pub fn new(name: impl Into<String>, tile_size: [f64; 2]) -> Self {
        Self {
            name: name.into(),
            tile_size,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailRenderMode {
    GrassBillboard,
    VertexLit,
    Grass,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DetailPrototype {
    pub name: String,
    pub render_mode: DetailRenderMode,
    pub use_instancing: bool,
    pub healthy_color: [f64; 3],
    pub dry_color: [f64; 3],
}

impl DetailPrototype {
    pub fn instanced_grass(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            render_mode: DetailRenderMode::Grass,
            use_instancing: true,
            healthy_color: [0.3, 0.8, 0.2],
            dry_color: [0.7, 0.6, 0.2],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreePrototype {
    pub name: String,
    pub wind_excluded: bool,
    pub lod_count: u32,
}

impl TreePrototype {
    pub fn tree_creator(name: impl Into<String>, lod_count: u32) -> Self {
        Self {
            name: name.into(),
            wind_excluded: false,
            lod_count,
        }
    }

    pub fn speed_tree_8(name: impl Into<String>, lod_count: u32) -> Self {
        Self {
            name: name.into(),
            wind_excluded: true,
            lod_count,
        }
    }
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
pub struct GrassWindSettings {
    pub speed: f64,
    pub size: f64,
    pub bending: f64,
    pub tint: [f64; 3],
}

impl Default for GrassWindSettings {
    fn default() -> Self {
        Self {
            speed: 0.5,
            size: 0.5,
            bending: 0.5,
            tint: [0.9, 0.9, 0.9],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindZoneMode {
    Directional,
    Spherical,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindZone {
    pub mode: WindZoneMode,
    pub position: [f64; 3],
    pub direction: [f64; 3],
    pub main: f64,
    pub turbulence: f64,
    pub pulse_magnitude: f64,
    pub pulse_frequency: f64,
    pub radius: f64,
}

impl WindZone {
    pub fn directional(direction: [f64; 3], main: f64, turbulence: f64) -> Self {
        let len = vector_length(direction);
        let unit = if len > 1e-9 {
            [direction[0] / len, direction[1] / len, direction[2] / len]
        } else {
            [1.0, 0.0, 0.0]
        };
        Self {
            mode: WindZoneMode::Directional,
            position: [0.0; 3],
            direction: unit,
            main,
            turbulence,
            pulse_magnitude: 0.5,
            pulse_frequency: 1.0,
            radius: 0.0,
        }
    }

    pub fn spherical(position: [f64; 3], radius: f64, main: f64, turbulence: f64) -> Self {
        Self {
            mode: WindZoneMode::Spherical,
            position,
            direction: [1.0, 0.0, 0.0],
            main,
            turbulence,
            pulse_magnitude: 0.5,
            pulse_frequency: 1.0,
            radius,
        }
    }

    pub fn with_pulse(mut self, magnitude: f64, frequency: f64) -> Self {
        self.pulse_magnitude = magnitude;
        self.pulse_frequency = frequency;
        self
    }

    pub fn force_at(&self, point: [f64; 3], time_seconds: f64) -> [f64; 3] {
        let pulse = (time_seconds * self.pulse_frequency * std::f64::consts::TAU).sin()
            * self.pulse_magnitude;
        let strength = (self.main + pulse + self.turbulence * 0.25).max(0.0);
        match self.mode {
            WindZoneMode::Directional => [
                self.direction[0] * strength,
                self.direction[1] * strength,
                self.direction[2] * strength,
            ],
            WindZoneMode::Spherical => {
                if self.radius <= 0.0 {
                    return [0.0; 3];
                }
                let offset = subtract(point, self.position);
                let dist = vector_length(offset);
                if dist >= self.radius {
                    return [0.0; 3];
                }
                let falloff = 1.0 - dist / self.radius;
                let dir = if dist > 1e-9 {
                    [offset[0] / dist, offset[1] / dist, offset[2] / dist]
                } else {
                    self.direction
                };
                [
                    dir[0] * strength * falloff,
                    dir[1] * strength * falloff,
                    dir[2] * strength * falloff,
                ]
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeWindSample {
    pub instance_index: usize,
    pub prototype_index: usize,
    pub prototype_name: String,
    pub lod_count: u32,
    pub wind_excluded: bool,
    pub excluded_reason: Option<String>,
    pub base_position: [f64; 3],
    pub bent_position: [f64; 3],
    pub bend_offset: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct DetailWindSample {
    pub instance_index: usize,
    pub prototype_index: usize,
    pub base_position: [f64; 3],
    pub wave_position: [f64; 3],
    pub wave_offset: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct TerrainWindReport {
    pub trees: Vec<TreeWindSample>,
    pub details: Vec<DetailWindSample>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstancedTreeBatch {
    pub prototype_index: usize,
    pub prototype_name: String,
    pub lod_count: u32,
    pub billboard_active: bool,
    pub wind_excluded: bool,
    pub excluded_reason: Option<String>,
    pub world_positions: Vec<[f64; 3]>,
    pub scales: Vec<f64>,
    pub rotations_y: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstancedDetailBatch {
    pub prototype_index: usize,
    pub prototype_name: String,
    pub render_mode: DetailRenderMode,
    pub use_instancing: bool,
    pub world_positions: Vec<[f64; 3]>,
    pub scales: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TerrainInstancedDrawData {
    pub material_shader: String,
    pub draw_instanced: bool,
    pub billboard_start: f64,
    pub tree_distance: f64,
    pub detail_distance: f64,
    pub tree_batches: Vec<InstancedTreeBatch>,
    pub detail_batches: Vec<InstancedDetailBatch>,
    pub wind_exclusion_diagnostics: Vec<String>,
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
    delayed_heights: Option<Vec<f64>>,
    holes: Vec<bool>,
    layers: Vec<TerrainLayer>,
    alphamaps: Vec<Vec<f64>>,
    tree_prototypes: Vec<TreePrototype>,
    tree_instances: Vec<TreeInstance>,
    detail_prototypes: Vec<DetailPrototype>,
    detail_instances: Vec<DetailInstance>,
    grass_wind: GrassWindSettings,
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
                delayed_heights: None,
                holes: vec![false; cells],
                layers: Vec::new(),
                alphamaps: Vec::new(),
                tree_prototypes: Vec::new(),
                tree_instances: Vec::new(),
                detail_prototypes: Vec::new(),
                detail_instances: Vec::new(),
                grass_wind: GrassWindSettings::default(),
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
        inner.delayed_heights = None;
    }

    pub fn set_heights_delay_lod(&self, updates: &[([usize; 2], f64)]) {
        let mut inner = self.inner.borrow_mut();
        let mut buffer = inner
            .delayed_heights
            .take()
            .unwrap_or_else(|| inner.heights.clone());
        for (position, height) in updates {
            let (column, row) = inner.clamp_indices(position[0], position[1]);
            let index = inner.index(column, row);
            buffer[index] = *height;
        }
        inner.delayed_heights = Some(buffer);
    }

    pub fn sync_heightmap(&self) {
        let mut inner = self.inner.borrow_mut();
        if let Some(delayed) = inner.delayed_heights.take() {
            inner.heights = delayed;
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

    pub fn add_terrain_layer(&self, layer: TerrainLayer) -> usize {
        let mut inner = self.inner.borrow_mut();
        let cells = inner.heightmap_resolution * inner.heightmap_resolution;
        let is_first = inner.layers.is_empty();
        inner.layers.push(layer);
        for cell_weights in &mut inner.alphamaps {
            cell_weights.push(if is_first { 1.0 } else { 0.0 });
        }
        if inner.alphamaps.is_empty() {
            inner.alphamaps = vec![vec![1.0]; cells];
        }
        inner.layers.len() - 1
    }

    pub fn terrain_layers(&self) -> Vec<TerrainLayer> {
        self.inner.borrow().layers.clone()
    }

    pub fn set_alphamap_weights(&self, column: usize, row: usize, weights: &[f64]) {
        let mut inner = self.inner.borrow_mut();
        if inner.layers.is_empty() {
            return;
        }
        let (column, row) = inner.clamp_indices(column, row);
        let index = inner.index(column, row);
        let layer_count = inner.layers.len();
        let cells = inner.heightmap_resolution * inner.heightmap_resolution;
        if inner.alphamaps.len() != cells {
            inner.alphamaps = vec![vec![0.0; layer_count]; cells];
        }
        let mut row_weights = vec![0.0; layer_count];
        let mut sum = 0.0;
        for (i, w) in weights.iter().take(layer_count).enumerate() {
            let clamped = w.max(0.0);
            row_weights[i] = clamped;
            sum += clamped;
        }
        if sum > 1e-9 {
            for w in &mut row_weights {
                *w /= sum;
            }
        }
        inner.alphamaps[index] = row_weights;
    }

    pub fn alphamap_weights(&self, column: usize, row: usize) -> Vec<f64> {
        let inner = self.inner.borrow();
        if inner.alphamaps.is_empty() {
            return Vec::new();
        }
        let (column, row) = inner.clamp_indices(column, row);
        inner.alphamaps[inner.index(column, row)].clone()
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

    pub fn add_detail_prototype(&self, prototype: DetailPrototype) -> usize {
        let mut inner = self.inner.borrow_mut();
        inner.detail_prototypes.push(prototype);
        inner.detail_prototypes.len() - 1
    }

    pub fn detail_prototypes(&self) -> Vec<DetailPrototype> {
        self.inner.borrow().detail_prototypes.clone()
    }

    pub fn set_detail_instances(&self, instances: &[DetailInstance]) {
        self.inner.borrow_mut().detail_instances = instances.to_vec();
    }

    pub fn detail_instances(&self) -> Vec<DetailInstance> {
        self.inner.borrow().detail_instances.clone()
    }

    pub fn set_grass_wind_settings(&self, settings: GrassWindSettings) {
        self.inner.borrow_mut().grass_wind = settings;
    }

    pub fn grass_wind_settings(&self) -> GrassWindSettings {
        self.inner.borrow().grass_wind
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
    draw_instanced: bool,
    material_shader: String,
    billboard_start: f64,
    tree_distance: f64,
    detail_distance: f64,
}

impl Terrain {
    pub fn new(data: TerrainData) -> Self {
        Self {
            data,
            position: [0.0; 3],
            collider_enabled: true,
            draw_instanced: true,
            material_shader: "Universal Render Pipeline/Terrain/Lit".to_string(),
            billboard_start: 50.0,
            tree_distance: 2000.0,
            detail_distance: 80.0,
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

    pub fn with_draw_instanced(mut self, draw_instanced: bool) -> Self {
        self.draw_instanced = draw_instanced;
        self
    }

    pub fn with_material_shader(mut self, shader: impl Into<String>) -> Self {
        self.material_shader = shader.into();
        self
    }

    pub fn with_billboard_start(mut self, billboard_start: f64) -> Self {
        self.billboard_start = billboard_start;
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

    pub fn draw_instanced(&self) -> bool {
        self.draw_instanced
    }

    pub fn material_shader(&self) -> &str {
        &self.material_shader
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

    pub fn sample_height(&self, world_position: [f64; 3]) -> Option<f64> {
        self.height_at(world_position)
    }

    pub fn height_at(&self, world_position: [f64; 3]) -> Option<f64> {
        self.sample_surface(world_position)
            .map(|(surface, _)| surface[1])
    }

    pub fn normal_at(&self, world_position: [f64; 3]) -> Option<[f64; 3]> {
        self.sample_surface(world_position)
            .map(|(_, normal)| normal)
    }

    pub fn splat_weights_at(&self, world_position: [f64; 3]) -> Option<Vec<f64>> {
        let (column, row) = self.cell_index(world_position[0], world_position[2])?;
        Some(self.data.alphamap_weights(column, row))
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

    pub fn raycast(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
    ) -> Option<TerrainRayHit> {
        if !self.collider_enabled || max_distance <= 0.0 || !max_distance.is_finite() {
            return None;
        }
        let dir_len = vector_length(direction);
        if dir_len <= 1e-9 {
            return None;
        }
        let unit = [
            direction[0] / dir_len,
            direction[1] / dir_len,
            direction[2] / dir_len,
        ];
        let step = max_distance / RAYCAST_STEPS as f64;
        let mut prev_t = 0.0;
        let mut prev_above = true;
        for i in 0..=RAYCAST_STEPS {
            let t = i as f64 * step;
            let point = [
                origin[0] + unit[0] * t,
                origin[1] + unit[1] * t,
                origin[2] + unit[2] * t,
            ];
            let Some((surface, normal)) = self.sample_surface(point) else {
                prev_t = t;
                continue;
            };
            let above = point[1] >= surface[1];
            if i == 0 {
                if !above && !self.is_hole(point) {
                    return Some(TerrainRayHit {
                        point: surface,
                        normal,
                        distance: 0.0,
                    });
                }
            } else if prev_above && !above {
                let mut low = prev_t;
                let mut high = t;
                for _ in 0..16 {
                    let mid = 0.5 * (low + high);
                    let mid_pt = [
                        origin[0] + unit[0] * mid,
                        origin[1] + unit[1] * mid,
                        origin[2] + unit[2] * mid,
                    ];
                    if let Some((surf, _)) = self.sample_surface(mid_pt) {
                        if mid_pt[1] >= surf[1] {
                            low = mid;
                        } else {
                            high = mid;
                        }
                    } else {
                        low = mid;
                    }
                }
                let hit_t = 0.5 * (low + high);
                let hit_pt = [
                    origin[0] + unit[0] * hit_t,
                    origin[1] + unit[1] * hit_t,
                    origin[2] + unit[2] * hit_t,
                ];
                if !self.is_hole(hit_pt) {
                    if let Some((surf, norm)) = self.sample_surface(hit_pt) {
                        return Some(TerrainRayHit {
                            point: surf,
                            normal: norm,
                            distance: distance(origin, surf),
                        });
                    }
                }
            }
            prev_above = above;
            prev_t = t;
        }
        None
    }

    pub fn evaluate_wind(&self, wind_zone: &WindZone, time_seconds: f64) -> TerrainWindReport {
        let prototypes = self.data.tree_prototypes();
        let tree_instances = self.data.tree_instances();
        let detail_instances = self.data.detail_instances();
        let grass_wind = self.data.grass_wind_settings();

        let mut trees = Vec::with_capacity(tree_instances.len());
        for (idx, instance) in tree_instances.iter().enumerate() {
            let base =
                self.instance_world_position(instance.normalized_position, instance.height_offset);
            let proto = prototypes.get(instance.prototype);
            let (name, lod_count, wind_excluded) = match proto {
                Some(p) => (p.name.clone(), p.lod_count, p.wind_excluded),
                None => ("MissingTreePrototype".to_string(), 0, true),
            };
            let (bend_offset, excluded_reason) = if wind_excluded {
                (
                    [0.0; 3],
                    Some(format!(
                        "SpeedTree wind animation is excluded on prototype '{name}' ({lod_count} LODs imported)"
                    )),
                )
            } else {
                let force = wind_zone.force_at(base, time_seconds);
                (
                    [
                        force[0] * 0.25 * instance.scale.max(0.1),
                        force[1] * 0.05 * instance.scale.max(0.1),
                        force[2] * 0.25 * instance.scale.max(0.1),
                    ],
                    None,
                )
            };
            let bent = [
                base[0] + bend_offset[0],
                base[1] + bend_offset[1],
                base[2] + bend_offset[2],
            ];
            trees.push(TreeWindSample {
                instance_index: idx,
                prototype_index: instance.prototype,
                prototype_name: name,
                lod_count,
                wind_excluded,
                excluded_reason,
                base_position: base,
                bent_position: bent,
                bend_offset,
            });
        }

        let mut details = Vec::with_capacity(detail_instances.len());
        for (idx, instance) in detail_instances.iter().enumerate() {
            let base =
                self.instance_world_position(instance.normalized_position, instance.height_offset);
            let zone_force = wind_zone.force_at(base, time_seconds);
            let phase = (base[0] + base[2]) * grass_wind.size.max(0.1)
                + time_seconds * grass_wind.speed.max(0.1) * std::f64::consts::TAU;
            let wave = phase.sin() * grass_wind.bending.max(0.05);
            let wave_offset = [
                (zone_force[0] + wave) * 0.15 * instance.scale.max(0.1),
                0.0,
                (zone_force[2] + wave * 0.5) * 0.15 * instance.scale.max(0.1),
            ];
            let wave_position = [
                base[0] + wave_offset[0],
                base[1] + wave_offset[1],
                base[2] + wave_offset[2],
            ];
            details.push(DetailWindSample {
                instance_index: idx,
                prototype_index: instance.prototype,
                base_position: base,
                wave_position,
                wave_offset,
            });
        }

        TerrainWindReport { trees, details }
    }

    pub fn instanced_draw_data(&self) -> TerrainInstancedDrawData {
        let tree_prototypes = self.data.tree_prototypes();
        let tree_instances = self.data.tree_instances();
        let detail_prototypes = self.data.detail_prototypes();
        let detail_instances = self.data.detail_instances();

        let mut tree_batches = Vec::with_capacity(tree_prototypes.len());
        let mut wind_exclusion_diagnostics = Vec::new();
        for (proto_idx, proto) in tree_prototypes.iter().enumerate() {
            let mut world_positions = Vec::new();
            let mut scales = Vec::new();
            let mut rotations_y = Vec::new();
            for inst in tree_instances.iter().filter(|i| i.prototype == proto_idx) {
                let pos =
                    self.instance_world_position(inst.normalized_position, inst.height_offset);
                if !self.is_hole(pos) {
                    world_positions.push(pos);
                    scales.push(inst.scale);
                    rotations_y.push(inst.rotation_y);
                }
            }
            let excluded_reason = if proto.wind_excluded {
                let msg = format!(
                    "SpeedTree wind animation is excluded on prototype '{}' ({} LODs imported)",
                    proto.name, proto.lod_count
                );
                wind_exclusion_diagnostics.push(msg.clone());
                Some(msg)
            } else {
                None
            };
            tree_batches.push(InstancedTreeBatch {
                prototype_index: proto_idx,
                prototype_name: proto.name.clone(),
                lod_count: proto.lod_count,
                billboard_active: self.billboard_start < self.tree_distance,
                wind_excluded: proto.wind_excluded,
                excluded_reason,
                world_positions,
                scales,
                rotations_y,
            });
        }

        let mut detail_batches = Vec::new();
        if detail_prototypes.is_empty() && !detail_instances.is_empty() {
            let mut world_positions = Vec::new();
            let mut scales = Vec::new();
            for inst in &detail_instances {
                let pos =
                    self.instance_world_position(inst.normalized_position, inst.height_offset);
                if !self.is_hole(pos) {
                    world_positions.push(pos);
                    scales.push(inst.scale);
                }
            }
            detail_batches.push(InstancedDetailBatch {
                prototype_index: 0,
                prototype_name: "DefaultDetailGrass".to_string(),
                render_mode: DetailRenderMode::Grass,
                use_instancing: true,
                world_positions,
                scales,
            });
        } else {
            for (proto_idx, proto) in detail_prototypes.iter().enumerate() {
                let mut world_positions = Vec::new();
                let mut scales = Vec::new();
                for inst in detail_instances.iter().filter(|i| i.prototype == proto_idx) {
                    let pos =
                        self.instance_world_position(inst.normalized_position, inst.height_offset);
                    if !self.is_hole(pos) {
                        world_positions.push(pos);
                        scales.push(inst.scale);
                    }
                }
                detail_batches.push(InstancedDetailBatch {
                    prototype_index: proto_idx,
                    prototype_name: proto.name.clone(),
                    render_mode: proto.render_mode,
                    use_instancing: proto.use_instancing,
                    world_positions,
                    scales,
                });
            }
        }

        TerrainInstancedDrawData {
            material_shader: self.material_shader.clone(),
            draw_instanced: self.draw_instanced,
            billboard_start: self.billboard_start,
            tree_distance: self.tree_distance,
            detail_distance: self.detail_distance,
            tree_batches,
            detail_batches,
            wind_exclusion_diagnostics,
        }
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
