use std::collections::{BTreeMap, BTreeSet};

use unitas_core::EntityId;

use crate::event::Phase;
use crate::matrix::{CollisionMatrix, LAYER_COUNT};
use crate::two_d::body::{
    BodyDesc2D, BodyId2D, BodyType2D, CollisionDetectionMode2D, Rigidbody2D, RigidbodySleepMode2D,
};
use crate::two_d::collider::{
    compute_contact2d, dot2, inverse_transform_point2, length2, normalize2, point_in_shape2d,
    raycast_shape2d, rotate_vec2, shape_to_local_shapes_in_frame, sub2, transform_point2,
    Collider2D, ColliderDesc2D, ColliderId2D, PhysicsMaterial2D, Shape2D,
};
use crate::two_d::composite::{
    CompositeCollider2D, CompositeGenerationType2D, TileColliderType2D, TilemapCollider2D,
};
use crate::two_d::effector::{Effector2D, PlatformEffector2D, SurfaceEffector2D};
use crate::two_d::event::{route_pair_messages_2d, ContactPoint2D, Endpoint2D, RoutedMessage2D};

const DEFAULT_GRAVITY_2D: [f32; 2] = [0.0, -9.81];
const DEFAULT_FIXED_TIMESTEP_2D: f32 = 0.02;
const DEFAULT_MAX_TIMESTEP_2D: f32 = 0.333_333_34;
const DEFAULT_SOLVER_ITERATIONS_2D: usize = 6;
const SLEEP_LINEAR_THRESHOLD: f32 = 0.01;
const SLEEP_ANGULAR_THRESHOLD: f32 = 0.01;
const SLEEP_TIME_THRESHOLD: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactFilter2D {
    pub use_triggers: bool,
    pub use_layer_mask: bool,
    pub layer_mask: u32,
    pub use_depth: bool,
    pub min_depth: f32,
    pub max_depth: f32,
    pub use_normal_angle: bool,
    pub min_normal_angle: f32,
    pub max_normal_angle: f32,
}

impl Default for ContactFilter2D {
    fn default() -> Self {
        Self {
            use_triggers: true,
            use_layer_mask: false,
            layer_mask: u32::MAX,
            use_depth: false,
            min_depth: f32::NEG_INFINITY,
            max_depth: f32::INFINITY,
            use_normal_angle: false,
            min_normal_angle: 0.0,
            max_normal_angle: 360.0,
        }
    }
}

impl ContactFilter2D {
    pub fn no_filter() -> Self {
        Self::default()
    }

    pub fn with_triggers(mut self, use_triggers: bool) -> Self {
        self.use_triggers = use_triggers;
        self
    }

    pub fn with_layer_mask(mut self, layer_mask: u32) -> Self {
        self.use_layer_mask = true;
        self.layer_mask = layer_mask;
        self
    }

    pub fn with_depth(mut self, min_depth: f32, max_depth: f32) -> Self {
        self.use_depth = true;
        self.min_depth = min_depth;
        self.max_depth = max_depth;
        self
    }

    pub fn matches_collider(&self, collider: &Collider2D) -> bool {
        if !collider.enabled || !collider.active {
            return false;
        }
        if collider.is_trigger && !self.use_triggers {
            return false;
        }
        if self.use_layer_mask
            && (collider.layer >= 32 || (self.layer_mask & (1u32 << collider.layer)) == 0)
        {
            return false;
        }
        let has_depth_filter = self.use_depth
            || (self.min_depth.is_finite() && self.min_depth > f32::NEG_INFINITY)
            || (self.max_depth.is_finite() && self.max_depth < f32::INFINITY);
        if has_depth_filter
            && (collider.max_depth < self.min_depth || collider.min_depth > self.max_depth)
        {
            return false;
        }
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaycastHit2D {
    pub collider: ColliderId2D,
    pub body: Option<BodyId2D>,
    pub game_object: EntityId,
    pub point: [f32; 2],
    pub normal: [f32; 2],
    pub distance: f32,
    pub fraction: f32,
}

#[derive(Clone, Debug)]
struct PairState2D {
    is_trigger: bool,
    endpoint_a: Endpoint2D,
    endpoint_b: Endpoint2D,
    contacts_for_a: Vec<ContactPoint2D>,
    rel_vel_for_a: [f32; 2],
}

pub struct World2D {
    gravity: [f32; 2],
    fixed_timestep: f32,
    max_allowed_timestep: f32,
    accumulator: f32,
    step_count: u64,
    bodies: BTreeMap<BodyId2D, Rigidbody2D>,
    colliders: BTreeMap<ColliderId2D, Collider2D>,
    composites: BTreeMap<ColliderId2D, CompositeCollider2D>,
    tilemaps: BTreeMap<ColliderId2D, TilemapCollider2D>,
    effectors: BTreeMap<ColliderId2D, Vec<Effector2D>>,
    next_body_id: u64,
    next_collider_id: u64,
    collision_matrix: CollisionMatrix,
    ignored_pairs: BTreeSet<(ColliderId2D, ColliderId2D)>,
    touching_pairs: BTreeMap<(ColliderId2D, ColliderId2D), PairState2D>,
    pass_through_pairs: BTreeSet<(ColliderId2D, ColliderId2D)>,
    messages: Vec<RoutedMessage2D>,
    default_material: PhysicsMaterial2D,
    queries_start_in_colliders: bool,
    callbacks_on_disable: bool,
}

impl Default for World2D {
    fn default() -> Self {
        Self::new()
    }
}

impl World2D {
    pub fn new() -> Self {
        Self::with_gravity(DEFAULT_GRAVITY_2D)
    }

    pub fn with_gravity(gravity: [f32; 2]) -> Self {
        Self {
            gravity,
            fixed_timestep: DEFAULT_FIXED_TIMESTEP_2D,
            max_allowed_timestep: DEFAULT_MAX_TIMESTEP_2D,
            accumulator: 0.0,
            step_count: 0,
            bodies: BTreeMap::new(),
            colliders: BTreeMap::new(),
            composites: BTreeMap::new(),
            tilemaps: BTreeMap::new(),
            effectors: BTreeMap::new(),
            next_body_id: 1,
            next_collider_id: 1,
            collision_matrix: CollisionMatrix::new(),
            ignored_pairs: BTreeSet::new(),
            touching_pairs: BTreeMap::new(),
            pass_through_pairs: BTreeSet::new(),
            messages: Vec::new(),
            default_material: PhysicsMaterial2D::default(),
            queries_start_in_colliders: true,
            callbacks_on_disable: true,
        }
    }

    pub fn gravity(&self) -> [f32; 2] {
        self.gravity
    }

    pub fn set_gravity(&mut self, gravity: [f32; 2]) {
        self.gravity = gravity;
    }

    pub fn fixed_timestep(&self) -> f32 {
        self.fixed_timestep
    }

    pub fn set_fixed_timestep(&mut self, dt: f32) {
        if dt.is_finite() && dt > 0.0 {
            self.fixed_timestep = dt;
        }
    }

    pub fn default_material(&self) -> PhysicsMaterial2D {
        self.default_material
    }

    pub fn set_default_material(&mut self, material: PhysicsMaterial2D) {
        self.default_material = material;
    }

    pub fn queries_start_in_colliders(&self) -> bool {
        self.queries_start_in_colliders
    }

    pub fn set_queries_start_in_colliders(&mut self, value: bool) {
        self.queries_start_in_colliders = value;
    }

    pub fn collision_matrix(&self) -> &CollisionMatrix {
        &self.collision_matrix
    }

    pub fn set_collision_matrix(&mut self, matrix: CollisionMatrix) {
        self.collision_matrix = matrix;
    }

    pub fn ignore_layer_collision(&mut self, layer1: u8, layer2: u8, ignore: bool) {
        self.collision_matrix
            .ignore_layer_collision(layer1 as usize, layer2 as usize, ignore);
    }

    pub fn get_ignore_layer_collision(&self, layer1: u8, layer2: u8) -> bool {
        let a = layer1 as usize;
        let b = layer2 as usize;
        if a >= LAYER_COUNT || b >= LAYER_COUNT {
            return true;
        }
        !self.collision_matrix.is_enabled(a, b)
    }

    pub fn layer_mask_for_layer(&self, layer: u8) -> u32 {
        if layer >= 32 {
            return 0;
        }
        let mut mask = 0u32;
        for other in 0..32u8 {
            if !self.get_ignore_layer_collision(layer, other) {
                mask |= 1u32 << other;
            }
        }
        mask
    }

    pub fn ignore_collision(&mut self, c1: ColliderId2D, c2: ColliderId2D, ignore: bool) {
        if c1 == c2 {
            return;
        }
        let key = ordered_pair(c1, c2);
        if ignore {
            self.ignored_pairs.insert(key);
            self.touching_pairs.remove(&key);
        } else {
            self.ignored_pairs.remove(&key);
        }
    }

    pub fn get_ignore_collision(&self, c1: ColliderId2D, c2: ColliderId2D) -> bool {
        self.ignored_pairs.contains(&ordered_pair(c1, c2))
    }

    pub fn add_body(&mut self, desc: BodyDesc2D) -> BodyId2D {
        let id = BodyId2D::new(self.next_body_id);
        self.next_body_id += 1;
        let body = Rigidbody2D::from_desc(id, desc);
        self.bodies.insert(id, body);
        id
    }

    pub fn body(&self, id: BodyId2D) -> Option<&Rigidbody2D> {
        self.bodies.get(&id)
    }

    pub fn body_mut(&mut self, id: BodyId2D) -> Option<&mut Rigidbody2D> {
        self.bodies.get_mut(&id)
    }

    pub fn remove_body(&mut self, id: BodyId2D) -> bool {
        let existed = self.bodies.remove(&id).is_some();
        if existed {
            for collider in self.colliders.values_mut() {
                if collider.attached_body == Some(id) {
                    collider.attached_body = None;
                }
            }
        }
        existed
    }

    pub fn add_collider(&mut self, desc: ColliderDesc2D) -> ColliderId2D {
        let id = ColliderId2D::new(self.next_collider_id);
        self.next_collider_id += 1;
        let used_by_composite = desc.used_by_composite;
        let attached_body = desc.attached_body;
        let game_object = desc.game_object;
        let collider = Collider2D::from_desc(id, desc);
        self.colliders.insert(id, collider);
        if used_by_composite {
            self.sync_composites_for_owner(attached_body, game_object);
        }
        id
    }

    pub fn collider(&self, id: ColliderId2D) -> Option<&Collider2D> {
        self.colliders.get(&id)
    }

    pub fn collider_mut(&mut self, id: ColliderId2D) -> Option<&mut Collider2D> {
        self.colliders.get_mut(&id)
    }

    pub fn remove_collider(&mut self, id: ColliderId2D) -> bool {
        let removed = self.colliders.remove(&id);
        let existed = removed.is_some();
        if let Some(col) = removed {
            self.composites.remove(&id);
            self.tilemaps.remove(&id);
            self.effectors.remove(&id);
            self.ignored_pairs.retain(|(a, b)| *a != id && *b != id);
            self.touching_pairs.retain(|(a, b), _| *a != id && *b != id);
            self.pass_through_pairs
                .retain(|(a, b)| *a != id && *b != id);
            if col.used_by_composite {
                self.sync_composites_for_owner(col.attached_body, col.game_object);
            }
        }
        existed
    }

    pub fn attach_composite(&mut self, collider: ColliderId2D, composite: CompositeCollider2D) {
        let is_sync = composite.generation_type == CompositeGenerationType2D::Synchronous;
        self.composites.insert(collider, composite);
        if is_sync {
            self.regenerate_composite(collider);
        }
    }

    pub fn composite(&self, collider: ColliderId2D) -> Option<&CompositeCollider2D> {
        self.composites.get(&collider)
    }

    pub fn set_composite_generation_type(
        &mut self,
        collider: ColliderId2D,
        generation_type: CompositeGenerationType2D,
    ) {
        if let Some(comp) = self.composites.get_mut(&collider) {
            comp.generation_type = generation_type;
            if generation_type == CompositeGenerationType2D::Synchronous {
                self.regenerate_composite(collider);
            }
        }
    }

    pub fn regenerate_composite(&mut self, composite_collider: ColliderId2D) -> bool {
        if !self.composites.contains_key(&composite_collider) {
            return false;
        }
        let Some(comp_col) = self.colliders.get(&composite_collider).cloned() else {
            return false;
        };
        let (comp_center, comp_angle) = self.collider_world_pose(&comp_col);
        let mut merged_shapes: Vec<Shape2D> = Vec::new();

        if let Some(own_tilemap) = self.tilemaps.get(&composite_collider) {
            if own_tilemap.used_by_composite {
                merged_shapes.extend(own_tilemap.build_shapes());
            }
        }

        let all_ids: Vec<ColliderId2D> = self.colliders.keys().copied().collect();
        for other_id in all_ids {
            if other_id == composite_collider {
                continue;
            }
            let Some(other_col) = self.colliders.get(&other_id) else {
                continue;
            };
            if !other_col.enabled || !other_col.active {
                continue;
            }
            let same_owner = if comp_col.attached_body.is_some() {
                other_col.attached_body == comp_col.attached_body
            } else {
                other_col.attached_body.is_none() && other_col.game_object == comp_col.game_object
            };
            if !same_owner {
                continue;
            }
            let (other_center, other_angle) = self.collider_world_pose(other_col);
            let rel_offset = inverse_transform_point2(other_center, comp_center, comp_angle);
            let rel_angle = other_angle - comp_angle;

            if let Some(tm) = self.tilemaps.get(&other_id) {
                if tm.used_by_composite || other_col.used_by_composite {
                    for tm_shape in tm.build_shapes() {
                        merged_shapes.extend(shape_to_local_shapes_in_frame(
                            &tm_shape, rel_offset, rel_angle,
                        ));
                    }
                }
                continue;
            }

            if other_col.used_by_composite {
                merged_shapes.extend(shape_to_local_shapes_in_frame(
                    &other_col.shape,
                    rel_offset,
                    rel_angle,
                ));
            }
        }

        if let Some(col_mut) = self.colliders.get_mut(&composite_collider) {
            col_mut.shape = Shape2D::Composite {
                shapes: merged_shapes,
            };
        }
        true
    }

    pub fn attach_tilemap(&mut self, collider: ColliderId2D, tilemap: TilemapCollider2D) {
        let used_by_comp = tilemap.used_by_composite;
        if let Some(col) = self.colliders.get_mut(&collider) {
            if used_by_comp && !self.composites.contains_key(&collider) {
                col.used_by_composite = true;
            }
        }
        self.tilemaps.insert(collider, tilemap);
        self.rebuild_tilemap_and_composites(collider);
    }

    pub fn tilemap(&self, collider: ColliderId2D) -> Option<&TilemapCollider2D> {
        self.tilemaps.get(&collider)
    }

    pub fn set_tilemap_tile(
        &mut self,
        collider: ColliderId2D,
        cell: [i32; 2],
        collider_type: TileColliderType2D,
    ) -> bool {
        let Some(tilemap) = self.tilemaps.get_mut(&collider) else {
            return false;
        };
        tilemap.set_tile(cell, collider_type);
        self.rebuild_tilemap_and_composites(collider);
        true
    }

    pub fn remove_tilemap_tile(&mut self, collider: ColliderId2D, cell: [i32; 2]) -> bool {
        let Some(tilemap) = self.tilemaps.get_mut(&collider) else {
            return false;
        };
        let existed = tilemap.remove_tile(cell);
        if existed {
            self.rebuild_tilemap_and_composites(collider);
        }
        existed
    }

    fn rebuild_tilemap_and_composites(&mut self, collider: ColliderId2D) {
        let Some(tilemap) = self.tilemaps.get(&collider).cloned() else {
            return;
        };
        let is_also_composite = self.composites.contains_key(&collider);
        if !tilemap.used_by_composite && !is_also_composite {
            let shapes = tilemap.build_shapes();
            if let Some(col) = self.colliders.get_mut(&collider) {
                col.shape = Shape2D::Composite { shapes };
            }
            return;
        }
        if is_also_composite {
            let is_sync = self
                .composites
                .get(&collider)
                .is_some_and(|c| c.generation_type == CompositeGenerationType2D::Synchronous);
            if is_sync {
                self.regenerate_composite(collider);
            }
            return;
        }
        if let Some(col) = self.colliders.get(&collider) {
            self.sync_composites_for_owner(col.attached_body, col.game_object);
        }
    }

    fn sync_composites_for_owner(
        &mut self,
        attached_body: Option<BodyId2D>,
        game_object: EntityId,
    ) {
        let comp_ids: Vec<ColliderId2D> = self
            .composites
            .iter()
            .filter_map(|(id, comp)| {
                if comp.generation_type == CompositeGenerationType2D::Synchronous {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        for comp_id in comp_ids {
            let matches_owner = self.colliders.get(&comp_id).is_some_and(|c| {
                if attached_body.is_some() {
                    c.attached_body == attached_body
                } else {
                    c.attached_body.is_none() && c.game_object == game_object
                }
            });
            if matches_owner {
                self.regenerate_composite(comp_id);
            }
        }
    }

    pub fn attach_effector(&mut self, collider: ColliderId2D, effector: impl Into<Effector2D>) {
        if let Some(col) = self.colliders.get_mut(&collider) {
            col.used_by_effector = true;
        }
        self.effectors
            .entry(collider)
            .or_default()
            .push(effector.into());
    }

    pub fn clear_effectors(&mut self, collider: ColliderId2D) {
        self.effectors.remove(&collider);
    }

    pub fn effectors(&self, collider: ColliderId2D) -> &[Effector2D] {
        self.effectors
            .get(&collider)
            .map_or(&[], |list| list.as_slice())
    }

    pub fn is_touching(&self, c1: ColliderId2D, c2: ColliderId2D) -> bool {
        if c1 == c2 {
            return false;
        }
        self.touching_pairs.contains_key(&ordered_pair(c1, c2))
    }

    pub fn is_touching_layers(&self, collider: ColliderId2D, layer_mask: u32) -> bool {
        for ((a, b), state) in &self.touching_pairs {
            if *a == collider {
                let layer = state.endpoint_b.layer;
                if layer < 32 && (layer_mask & (1u32 << layer)) != 0 {
                    return true;
                }
            } else if *b == collider {
                let layer = state.endpoint_a.layer;
                if layer < 32 && (layer_mask & (1u32 << layer)) != 0 {
                    return true;
                }
            }
        }
        false
    }

    pub fn get_contacts(&self, collider: ColliderId2D) -> Vec<ContactPoint2D> {
        let mut out = Vec::new();
        for ((a, b), state) in &self.touching_pairs {
            if state.is_trigger {
                continue;
            }
            if *a == collider {
                out.extend_from_slice(&state.contacts_for_a);
            } else if *b == collider {
                for c in &state.contacts_for_a {
                    out.push(ContactPoint2D {
                        point: c.point,
                        normal: [-c.normal[0], -c.normal[1]],
                        separation: c.separation,
                        normal_impulse: c.normal_impulse,
                    });
                }
            }
        }
        out
    }

    pub fn drain_messages(&mut self) -> Vec<RoutedMessage2D> {
        std::mem::take(&mut self.messages)
    }

    pub fn advance(&mut self, frame_dt: f32) -> usize {
        if !frame_dt.is_finite() || frame_dt <= 0.0 {
            return 0;
        }
        let clamped = frame_dt.min(self.max_allowed_timestep);
        self.accumulator += clamped;
        let mut steps = 0usize;
        while self.accumulator + 1e-7 >= self.fixed_timestep {
            self.step(self.fixed_timestep);
            self.accumulator = (self.accumulator - self.fixed_timestep).max(0.0);
            steps += 1;
        }
        steps
    }

    pub fn interpolation_alpha(&self) -> f32 {
        if self.fixed_timestep > 0.0 {
            (self.accumulator / self.fixed_timestep).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub fn presented_pose(&self, id: BodyId2D) -> Option<([f32; 2], f32)> {
        self.presented_pose_at(id, self.interpolation_alpha())
    }

    pub fn presented_pose_at(&self, id: BodyId2D, alpha: f32) -> Option<([f32; 2], f32)> {
        let body = self.bodies.get(&id)?;
        Some(body.interpolated_pose(alpha, self.fixed_timestep))
    }

    pub fn step(&mut self, dt: f32) {
        if !dt.is_finite() || dt < 0.0 {
            return;
        }
        self.step_count = self.step_count.wrapping_add(1);
        self.apply_area_and_point_effectors(dt);
        self.integrate_bodies(dt);
        let current_pairs = self.solve_and_collect_contacts(dt);
        self.emit_transition_messages(current_pairs);
        self.update_sleeping(dt);
    }

    pub fn raycast(
        &self,
        origin: [f32; 2],
        direction: [f32; 2],
        max_distance: f32,
        filter: ContactFilter2D,
    ) -> Vec<RaycastHit2D> {
        if !max_distance.is_finite() || max_distance < 0.0 {
            return Vec::new();
        }
        let Some(dir) = normalize2(direction) else {
            return Vec::new();
        };
        let mut hits = Vec::new();
        for collider in self.colliders.values() {
            if !self.is_collider_simulated(collider) || !filter.matches_collider(collider) {
                continue;
            }
            let (center, angle) = self.collider_world_pose(collider);
            if let Some((distance, point, normal)) = raycast_shape2d(
                origin,
                dir,
                max_distance,
                &collider.shape,
                center,
                angle,
                self.queries_start_in_colliders,
            ) {
                let fraction = if max_distance > 0.0 {
                    distance / max_distance
                } else {
                    0.0
                };
                hits.push(RaycastHit2D {
                    collider: collider.id,
                    body: collider.attached_body,
                    game_object: collider.game_object,
                    point,
                    normal,
                    distance,
                    fraction,
                });
            }
        }
        hits.sort_by(|a, b| {
            a.distance
                .total_cmp(&b.distance)
                .then(a.collider.cmp(&b.collider))
        });
        hits
    }

    pub fn raycast_default_layer(
        &self,
        origin: [f32; 2],
        direction: [f32; 2],
        max_distance: f32,
        query_layer: u8,
    ) -> Vec<RaycastHit2D> {
        let mask = self.layer_mask_for_layer(query_layer);
        let filter = ContactFilter2D::default().with_layer_mask(mask);
        self.raycast(origin, direction, max_distance, filter)
    }

    pub fn overlap_point(&self, point: [f32; 2], filter: ContactFilter2D) -> Vec<ColliderId2D> {
        let mut out = Vec::new();
        for collider in self.colliders.values() {
            if !self.is_collider_simulated(collider) || !filter.matches_collider(collider) {
                continue;
            }
            let (center, angle) = self.collider_world_pose(collider);
            if point_in_shape2d(point, &collider.shape, center, angle) {
                out.push(collider.id);
            }
        }
        out
    }

    pub fn overlap_circle(
        &self,
        center: [f32; 2],
        radius: f32,
        filter: ContactFilter2D,
    ) -> Vec<ColliderId2D> {
        if !radius.is_finite() || radius < 0.0 {
            return Vec::new();
        }
        let probe = Shape2D::Circle { radius };
        self.overlap_shape(&probe, center, 0.0, filter)
    }

    pub fn overlap_box(
        &self,
        center: [f32; 2],
        half_extents: [f32; 2],
        angle: f32,
        filter: ContactFilter2D,
    ) -> Vec<ColliderId2D> {
        if !half_extents[0].is_finite()
            || !half_extents[1].is_finite()
            || half_extents[0] < 0.0
            || half_extents[1] < 0.0
        {
            return Vec::new();
        }
        let probe = Shape2D::Box { half_extents };
        self.overlap_shape(&probe, center, angle, filter)
    }

    fn overlap_shape(
        &self,
        probe: &Shape2D,
        probe_center: [f32; 2],
        probe_angle: f32,
        filter: ContactFilter2D,
    ) -> Vec<ColliderId2D> {
        let mut out = Vec::new();
        for collider in self.colliders.values() {
            if !self.is_collider_simulated(collider) || !filter.matches_collider(collider) {
                continue;
            }
            let (center, angle) = self.collider_world_pose(collider);
            if compute_contact2d(
                probe,
                probe_center,
                probe_angle,
                &collider.shape,
                center,
                angle,
            )
            .is_some()
            {
                out.push(collider.id);
            }
        }
        out
    }

    fn is_collider_simulated(&self, collider: &Collider2D) -> bool {
        if !collider.enabled || !collider.active {
            return false;
        }
        if collider.used_by_composite && !self.composites.contains_key(&collider.id) {
            return false;
        }
        if let Some(tm) = self.tilemaps.get(&collider.id) {
            if tm.used_by_composite && !self.composites.contains_key(&collider.id) {
                return false;
            }
        }
        if let Some(body_id) = collider.attached_body {
            if let Some(body) = self.bodies.get(&body_id) {
                if !body.simulated {
                    return false;
                }
            }
        }
        true
    }

    fn collider_world_pose(&self, collider: &Collider2D) -> ([f32; 2], f32) {
        if let Some(body_id) = collider.attached_body {
            if let Some(body) = self.bodies.get(&body_id) {
                let world_pos = transform_point2(collider.offset, body.position, body.rotation);
                return (world_pos, body.rotation + collider.rotation);
            }
        }
        (collider.offset, collider.rotation)
    }

    fn endpoint_for(&self, collider: &Collider2D) -> Endpoint2D {
        let mut body_game_object = None;
        let mut body_type = None;
        let mut use_full_kinematic_contacts = false;
        let mut active = collider.active && collider.enabled;
        let mut layer = collider.layer;
        if let Some(body_id) = collider.attached_body {
            if let Some(body) = self.bodies.get(&body_id) {
                body_game_object = Some(body.game_object);
                body_type = Some(body.body_type);
                use_full_kinematic_contacts = body.use_full_kinematic_contacts;
                if !body.simulated {
                    active = false;
                }
                if layer == 0 && body.layer != 0 {
                    layer = body.layer;
                }
            }
        }
        Endpoint2D {
            collider_id: collider.id,
            collider_game_object: collider.game_object,
            body_id: collider.attached_body,
            body_game_object,
            body_type,
            use_full_kinematic_contacts,
            is_trigger: collider.is_trigger,
            layer,
            active,
        }
    }

    fn collider_velocity(&self, collider: &Collider2D) -> [f32; 2] {
        if let Some(body_id) = collider.attached_body {
            if let Some(body) = self.bodies.get(&body_id) {
                return body.linear_velocity;
            }
        }
        [0.0, 0.0]
    }

    fn apply_area_and_point_effectors(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let collider_ids: Vec<ColliderId2D> = self.colliders.keys().copied().collect();
        for eff_col_id in &collider_ids {
            let Some(eff_col) = self.colliders.get(eff_col_id).cloned() else {
                continue;
            };
            if !eff_col.used_by_effector || !self.is_collider_simulated(&eff_col) {
                continue;
            }
            let Some(eff_list) = self.effectors.get(eff_col_id).cloned() else {
                continue;
            };
            let has_area_or_point = eff_list.iter().any(|e| {
                matches!(
                    e,
                    Effector2D::Area(_) | Effector2D::Point(_) | Effector2D::Buoyancy(_)
                )
            });
            if !has_area_or_point {
                continue;
            }
            let (eff_center, eff_angle) = self.collider_world_pose(&eff_col);
            for other_col_id in &collider_ids {
                if other_col_id == eff_col_id {
                    continue;
                }
                let Some(other_col) = self.colliders.get(other_col_id).cloned() else {
                    continue;
                };
                if !self.is_collider_simulated(&other_col) {
                    continue;
                }
                let Some(other_body_id) = other_col.attached_body else {
                    continue;
                };
                let (other_center, other_angle) = self.collider_world_pose(&other_col);
                if compute_contact2d(
                    &eff_col.shape,
                    eff_center,
                    eff_angle,
                    &other_col.shape,
                    other_center,
                    other_angle,
                )
                .is_none()
                {
                    continue;
                }
                let other_layer = self.endpoint_for(&other_col).layer;
                for effector in &eff_list {
                    match effector {
                        Effector2D::Area(area) => {
                            if !area.allows_layer(other_layer) {
                                continue;
                            }
                            let force = area.force_vector(eff_angle);
                            if let Some(body) = self.bodies.get_mut(&other_body_id) {
                                if body.is_dynamic() && body.simulated {
                                    body.add_force(force);
                                    if area.drag > 0.0 {
                                        let factor = (1.0 - area.drag * dt).max(0.0);
                                        body.linear_velocity[0] *= factor;
                                        body.linear_velocity[1] *= factor;
                                    }
                                    if area.angular_drag > 0.0 {
                                        let factor = (1.0 - area.angular_drag * dt).max(0.0);
                                        body.angular_velocity *= factor;
                                    }
                                }
                            }
                        }
                        Effector2D::Point(point_eff) => {
                            if !point_eff.allows_layer(other_layer) {
                                continue;
                            }
                            let delta = sub2(other_center, eff_center);
                            let dist = length2(delta);
                            let dir = normalize2(delta).unwrap_or([1.0, 0.0]);
                            let scaled_dist = (dist * point_eff.distance_scale).max(1.0);
                            let mag = point_eff.force_magnitude / scaled_dist;
                            let force = [dir[0] * mag, dir[1] * mag];
                            if let Some(body) = self.bodies.get_mut(&other_body_id) {
                                if body.is_dynamic() && body.simulated {
                                    body.add_force(force);
                                    if point_eff.drag > 0.0 {
                                        let factor = (1.0 - point_eff.drag * dt).max(0.0);
                                        body.linear_velocity[0] *= factor;
                                        body.linear_velocity[1] *= factor;
                                    }
                                }
                            }
                        }
                        Effector2D::Buoyancy(buoyancy) => {
                            if !buoyancy.allows_layer(other_layer) {
                                continue;
                            }
                            let surface_y = eff_center[1] + buoyancy.surface_level;
                            let (min_y, max_y, area) = shape_vertical_bounds_and_area(
                                &other_col.shape,
                                other_center,
                                other_angle,
                            );
                            if min_y >= surface_y || area <= 0.0 {
                                continue;
                            }
                            let height = (max_y - min_y).max(1e-4);
                            let submerged_fraction = ((surface_y - min_y) / height).clamp(0.0, 1.0);
                            let displaced_volume = area * submerged_fraction;
                            let grav_mag = length2(self.gravity);
                            let effective_g = if grav_mag > 1e-5 { grav_mag } else { 9.81 };
                            let upward_buoyancy =
                                buoyancy.density.max(0.0) * displaced_volume * effective_g;
                            let step_phase = (self.step_count as f32) * 0.7
                                + (other_body_id.as_u64() as f32) * 1.3;
                            let flow = buoyancy.flow_force_vector(step_phase);
                            let total_force = [
                                flow[0] * submerged_fraction,
                                upward_buoyancy + flow[1] * submerged_fraction,
                            ];
                            if let Some(body) = self.bodies.get_mut(&other_body_id) {
                                if body.is_dynamic() && body.simulated {
                                    body.add_force(total_force);
                                    if buoyancy.linear_drag > 0.0 {
                                        let factor = (1.0
                                            - buoyancy.linear_drag * submerged_fraction * dt)
                                            .max(0.0);
                                        body.linear_velocity[0] *= factor;
                                        body.linear_velocity[1] *= factor;
                                    }
                                    if buoyancy.angular_drag > 0.0 {
                                        let factor = (1.0
                                            - buoyancy.angular_drag * submerged_fraction * dt)
                                            .max(0.0);
                                        body.angular_velocity *= factor;
                                    }
                                }
                            }
                        }
                        Effector2D::Platform(_) | Effector2D::Surface(_) => {}
                    }
                }
            }
        }
    }

    fn integrate_bodies(&mut self, dt: f32) {
        let mut continuous_ids = Vec::new();
        for body in self.bodies.values_mut() {
            if !body.simulated {
                continue;
            }
            body.previous_position = body.position;
            body.previous_rotation = body.rotation;
            match body.body_type {
                BodyType2D::Dynamic => {
                    if body.sleeping {
                        body.accumulated_force = [0.0, 0.0];
                        body.accumulated_torque = 0.0;
                        continue;
                    }
                    let inv_mass = body.inverse_mass();
                    let ax =
                        self.gravity[0] * body.gravity_scale + body.accumulated_force[0] * inv_mass;
                    let ay =
                        self.gravity[1] * body.gravity_scale + body.accumulated_force[1] * inv_mass;
                    body.linear_velocity[0] += ax * dt;
                    body.linear_velocity[1] += ay * dt;
                    if body.linear_drag > 0.0 {
                        let damp = (1.0 - body.linear_drag * dt).max(0.0);
                        body.linear_velocity[0] *= damp;
                        body.linear_velocity[1] *= damp;
                    }
                    body.angular_velocity += body.accumulated_torque * inv_mass * dt;
                    if body.angular_drag > 0.0 {
                        let damp = (1.0 - body.angular_drag * dt).max(0.0);
                        body.angular_velocity *= damp;
                    }
                    if let Some(target_pos) = body.move_target_position.take() {
                        body.position = target_pos;
                    } else {
                        body.position[0] += body.linear_velocity[0] * dt;
                        body.position[1] += body.linear_velocity[1] * dt;
                    }
                    if let Some(target_rot) = body.move_target_rotation.take() {
                        body.rotation = target_rot;
                    } else {
                        body.rotation += body.angular_velocity * dt;
                    }
                    body.accumulated_force = [0.0, 0.0];
                    body.accumulated_torque = 0.0;
                    if body.collision_detection_mode == CollisionDetectionMode2D::Continuous {
                        continuous_ids.push(body.id);
                    }
                }
                BodyType2D::Kinematic => {
                    if let Some(target_pos) = body.move_target_position.take() {
                        body.position = target_pos;
                    } else {
                        body.position[0] += body.linear_velocity[0] * dt;
                        body.position[1] += body.linear_velocity[1] * dt;
                    }
                    if let Some(target_rot) = body.move_target_rotation.take() {
                        body.rotation = target_rot;
                    } else {
                        body.rotation += body.angular_velocity * dt;
                    }
                    if body.collision_detection_mode == CollisionDetectionMode2D::Continuous {
                        continuous_ids.push(body.id);
                    }
                }
                BodyType2D::Static => {}
            }
        }
        for body_id in continuous_ids {
            self.clamp_continuous_body_motion(body_id);
        }
    }

    fn clamp_continuous_body_motion(&mut self, body_id: BodyId2D) {
        let Some((prev_pos, curr_pos, rot)) = self
            .bodies
            .get(&body_id)
            .map(|b| (b.previous_position, b.position, b.rotation))
        else {
            return;
        };
        let motion = sub2(curr_pos, prev_pos);
        let travel = length2(motion);
        if travel <= 1e-5 {
            return;
        }
        let Some(dir) = normalize2(motion) else {
            return;
        };

        let own_colliders: Vec<Collider2D> = self
            .colliders
            .values()
            .filter(|c| {
                c.attached_body == Some(body_id) && !c.is_trigger && self.is_collider_simulated(c)
            })
            .cloned()
            .collect();
        if own_colliders.is_empty() {
            return;
        }

        let other_colliders: Vec<Collider2D> = self
            .colliders
            .values()
            .filter(|c| {
                c.attached_body != Some(body_id) && !c.is_trigger && self.is_collider_simulated(c)
            })
            .cloned()
            .collect();

        let mut earliest_advance: Option<f32> = None;

        for own in &own_colliders {
            let ep_own = self.endpoint_for(own);
            let own_start = transform_point2(own.offset, prev_pos, rot);
            let own_radius = shape_sweep_radius(&own.shape);

            for other in &other_colliders {
                let pair_key = ordered_pair(own.id, other.id);
                if self.ignored_pairs.contains(&pair_key)
                    || self.pass_through_pairs.contains(&pair_key)
                {
                    continue;
                }
                let ep_other = self.endpoint_for(other);
                if !self
                    .collision_matrix
                    .is_enabled(ep_own.layer as usize, ep_other.layer as usize)
                {
                    continue;
                }
                let (other_center, other_angle) = self.collider_world_pose(other);
                if compute_contact2d(
                    &own.shape,
                    own_start,
                    rot + own.rotation,
                    &other.shape,
                    other_center,
                    other_angle,
                )
                .is_some()
                {
                    continue;
                }
                if let Some((hit_dist, _, hit_normal)) = raycast_shape2d(
                    own_start,
                    dir,
                    travel + own_radius,
                    &other.shape,
                    other_center,
                    other_angle,
                    false,
                ) {
                    if let Some(plat) = self.platform_effector_for(other) {
                        if !plat.allows_layer(ep_own.layer)
                            || plat.should_pass_through(
                                other_angle,
                                hit_normal,
                                [dir[0] * travel, dir[1] * travel],
                            )
                        {
                            continue;
                        }
                    }
                    let advance = (hit_dist - own_radius * 0.9).clamp(0.0, travel);
                    if earliest_advance.is_none_or(|best| advance < best) {
                        earliest_advance = Some(advance);
                    }
                }
            }
        }

        if let Some(advance) = earliest_advance {
            if let Some(body) = self.bodies.get_mut(&body_id) {
                body.position = [
                    prev_pos[0] + dir[0] * advance,
                    prev_pos[1] + dir[1] * advance,
                ];
            }
        }
    }

    fn platform_effector_for(&self, collider: &Collider2D) -> Option<PlatformEffector2D> {
        if !collider.used_by_effector {
            return None;
        }
        let list = self.effectors.get(&collider.id)?;
        for eff in list {
            if let Effector2D::Platform(p) = eff {
                return Some(*p);
            }
        }
        None
    }

    fn surface_effector_for(&self, collider: &Collider2D) -> Option<SurfaceEffector2D> {
        if !collider.used_by_effector {
            return None;
        }
        let list = self.effectors.get(&collider.id)?;
        for eff in list {
            if let Effector2D::Surface(s) = eff {
                return Some(*s);
            }
        }
        None
    }

    fn solve_and_collect_contacts(
        &mut self,
        _dt: f32,
    ) -> BTreeMap<(ColliderId2D, ColliderId2D), PairState2D> {
        let ids: Vec<ColliderId2D> = self.colliders.keys().copied().collect();
        let mut active_pairs: BTreeMap<(ColliderId2D, ColliderId2D), PairState2D> = BTreeMap::new();
        let mut still_overlapping_all: BTreeSet<(ColliderId2D, ColliderId2D)> = BTreeSet::new();

        for iter in 0..DEFAULT_SOLVER_ITERATIONS_2D {
            for i in 0..ids.len() {
                for j in (i + 1)..ids.len() {
                    let id_a = ids[i];
                    let id_b = ids[j];
                    let pair_key = ordered_pair(id_a, id_b);
                    if self.ignored_pairs.contains(&pair_key) {
                        continue;
                    }
                    let (Some(col_a), Some(col_b)) = (
                        self.colliders.get(&id_a).cloned(),
                        self.colliders.get(&id_b).cloned(),
                    ) else {
                        continue;
                    };
                    if !self.is_collider_simulated(&col_a) || !self.is_collider_simulated(&col_b) {
                        continue;
                    }
                    if col_a.attached_body.is_some() && col_a.attached_body == col_b.attached_body {
                        continue;
                    }
                    let ep_a = self.endpoint_for(&col_a);
                    let ep_b = self.endpoint_for(&col_b);
                    if !self
                        .collision_matrix
                        .is_enabled(ep_a.layer as usize, ep_b.layer as usize)
                    {
                        continue;
                    }

                    let (center_a, angle_a) = self.collider_world_pose(&col_a);
                    let (center_b, angle_b) = self.collider_world_pose(&col_b);
                    let Some(raw_contact) = compute_contact2d(
                        &col_a.shape,
                        center_a,
                        angle_a,
                        &col_b.shape,
                        center_b,
                        angle_b,
                    ) else {
                        continue;
                    };

                    still_overlapping_all.insert(pair_key);

                    if self.pass_through_pairs.contains(&pair_key) {
                        continue;
                    }

                    let vel_a = self.collider_velocity(&col_a);
                    let vel_b = self.collider_velocity(&col_b);
                    let rel_vel_b_minus_a = sub2(vel_b, vel_a);

                    if let Some(plat_a) = self.platform_effector_for(&col_a) {
                        if !plat_a.allows_layer(ep_b.layer) {
                            self.pass_through_pairs.insert(pair_key);
                            continue;
                        }
                        if !self.touching_pairs.contains_key(&pair_key) {
                            let prev_pos_b = col_b
                                .attached_body
                                .and_then(|bid| self.bodies.get(&bid))
                                .map_or(center_b, |b| {
                                    transform_point2(col_b.offset, b.previous_position, b.rotation)
                                });
                            let up_a = plat_a.surface_up(angle_a);
                            let rel_prev = sub2(prev_pos_b, center_a);
                            let from_below = dot2(rel_prev, up_a) < 0.0;
                            if from_below
                                || plat_a.should_pass_through(
                                    angle_a,
                                    raw_contact.normal,
                                    rel_vel_b_minus_a,
                                )
                            {
                                self.pass_through_pairs.insert(pair_key);
                                continue;
                            }
                        }
                    }

                    if let Some(plat_b) = self.platform_effector_for(&col_b) {
                        if !plat_b.allows_layer(ep_a.layer) {
                            self.pass_through_pairs.insert(pair_key);
                            continue;
                        }
                        if !self.touching_pairs.contains_key(&pair_key) {
                            let prev_pos_a = col_a
                                .attached_body
                                .and_then(|bid| self.bodies.get(&bid))
                                .map_or(center_a, |b| {
                                    transform_point2(col_a.offset, b.previous_position, b.rotation)
                                });
                            let up_b = plat_b.surface_up(angle_b);
                            let rel_prev = sub2(prev_pos_a, center_b);
                            let from_below = dot2(rel_prev, up_b) < 0.0;
                            let normal_from_b_to_a =
                                [-raw_contact.normal[0], -raw_contact.normal[1]];
                            let rel_vel_a_minus_b = [-rel_vel_b_minus_a[0], -rel_vel_b_minus_a[1]];
                            if from_below
                                || plat_b.should_pass_through(
                                    angle_b,
                                    normal_from_b_to_a,
                                    rel_vel_a_minus_b,
                                )
                            {
                                self.pass_through_pairs.insert(pair_key);
                                continue;
                            }
                        }
                    }

                    let is_trigger = col_a.is_trigger || col_b.is_trigger;
                    if is_trigger {
                        if iter == 0 {
                            active_pairs.insert(
                                pair_key,
                                PairState2D {
                                    is_trigger: true,
                                    endpoint_a: ep_a,
                                    endpoint_b: ep_b,
                                    contacts_for_a: Vec::new(),
                                    rel_vel_for_a: rel_vel_b_minus_a,
                                },
                            );
                        }
                        continue;
                    }

                    let inv_a = col_a
                        .attached_body
                        .and_then(|bid| self.bodies.get(&bid))
                        .map_or(0.0, |b| b.inverse_mass());
                    let inv_b = col_b
                        .attached_body
                        .and_then(|bid| self.bodies.get(&bid))
                        .map_or(0.0, |b| b.inverse_mass());
                    let inv_sum = inv_a + inv_b;

                    let mut normal_impulse = 0.0f32;
                    if inv_sum > 0.0 {
                        let move_a = raw_contact.depth * (inv_a / inv_sum);
                        let move_b = raw_contact.depth * (inv_b / inv_sum);
                        if let Some(bid_a) = col_a.attached_body {
                            if let Some(body_a) = self.bodies.get_mut(&bid_a) {
                                if body_a.is_dynamic() {
                                    body_a.position[0] -= raw_contact.normal[0] * move_a;
                                    body_a.position[1] -= raw_contact.normal[1] * move_a;
                                }
                            }
                        }
                        if let Some(bid_b) = col_b.attached_body {
                            if let Some(body_b) = self.bodies.get_mut(&bid_b) {
                                if body_b.is_dynamic() {
                                    body_b.position[0] += raw_contact.normal[0] * move_b;
                                    body_b.position[1] += raw_contact.normal[1] * move_b;
                                }
                            }
                        }

                        let normal_speed = dot2(rel_vel_b_minus_a, raw_contact.normal);
                        if normal_speed < 0.0 {
                            let restitution =
                                0.5 * (col_a.material.bounciness + col_b.material.bounciness);
                            let friction =
                                (col_a.material.friction * col_b.material.friction).sqrt();
                            normal_impulse = -(1.0 + restitution) * normal_speed / inv_sum;

                            if let Some(bid_a) = col_a.attached_body {
                                if let Some(body_a) = self.bodies.get_mut(&bid_a) {
                                    if body_a.is_dynamic() {
                                        if body_a.is_sleeping() && normal_impulse > 1e-4 {
                                            body_a.wake_up();
                                        }
                                        body_a.linear_velocity[0] -=
                                            raw_contact.normal[0] * normal_impulse * inv_a;
                                        body_a.linear_velocity[1] -=
                                            raw_contact.normal[1] * normal_impulse * inv_a;
                                    }
                                }
                            }
                            if let Some(bid_b) = col_b.attached_body {
                                if let Some(body_b) = self.bodies.get_mut(&bid_b) {
                                    if body_b.is_dynamic() {
                                        if body_b.is_sleeping() && normal_impulse > 1e-4 {
                                            body_b.wake_up();
                                        }
                                        body_b.linear_velocity[0] +=
                                            raw_contact.normal[0] * normal_impulse * inv_b;
                                        body_b.linear_velocity[1] +=
                                            raw_contact.normal[1] * normal_impulse * inv_b;
                                    }
                                }
                            }

                            let tangent_vel = [
                                rel_vel_b_minus_a[0] - raw_contact.normal[0] * normal_speed,
                                rel_vel_b_minus_a[1] - raw_contact.normal[1] * normal_speed,
                            ];
                            let tangent_speed = length2(tangent_vel);
                            if tangent_speed > 1e-6 {
                                let friction_impulse =
                                    (tangent_speed / inv_sum).min(friction * normal_impulse);
                                let t_dir = [
                                    tangent_vel[0] / tangent_speed,
                                    tangent_vel[1] / tangent_speed,
                                ];
                                if let Some(bid_a) = col_a.attached_body {
                                    if let Some(body_a) = self.bodies.get_mut(&bid_a) {
                                        if body_a.is_dynamic() {
                                            body_a.linear_velocity[0] +=
                                                t_dir[0] * friction_impulse * inv_a;
                                            body_a.linear_velocity[1] +=
                                                t_dir[1] * friction_impulse * inv_a;
                                        }
                                    }
                                }
                                if let Some(bid_b) = col_b.attached_body {
                                    if let Some(body_b) = self.bodies.get_mut(&bid_b) {
                                        if body_b.is_dynamic() {
                                            body_b.linear_velocity[0] -=
                                                t_dir[0] * friction_impulse * inv_b;
                                            body_b.linear_velocity[1] -=
                                                t_dir[1] * friction_impulse * inv_b;
                                        }
                                    }
                                }
                            }
                        }

                        self.apply_surface_effector_on_pair(
                            &col_a,
                            angle_a,
                            &col_b,
                            ep_b.layer,
                            raw_contact.normal,
                        );
                        self.apply_surface_effector_on_pair(
                            &col_b,
                            angle_b,
                            &col_a,
                            ep_a.layer,
                            [-raw_contact.normal[0], -raw_contact.normal[1]],
                        );
                    }

                    let contact_pt = ContactPoint2D {
                        point: raw_contact.point,
                        normal: [-raw_contact.normal[0], -raw_contact.normal[1]],
                        separation: -raw_contact.depth,
                        normal_impulse,
                    };

                    active_pairs
                        .entry(pair_key)
                        .and_modify(|state| {
                            if normal_impulse > 0.0 && !state.contacts_for_a.is_empty() {
                                state.contacts_for_a[0].normal_impulse += normal_impulse;
                            }
                        })
                        .or_insert_with(|| PairState2D {
                            is_trigger: false,
                            endpoint_a: ep_a,
                            endpoint_b: ep_b,
                            contacts_for_a: vec![contact_pt],
                            rel_vel_for_a: rel_vel_b_minus_a,
                        });
                }
            }
        }

        self.pass_through_pairs
            .retain(|pair| still_overlapping_all.contains(pair));

        active_pairs
    }

    fn apply_surface_effector_on_pair(
        &mut self,
        surface_col: &Collider2D,
        surface_angle: f32,
        other_col: &Collider2D,
        other_layer: u8,
        outward_normal_from_surface: [f32; 2],
    ) {
        let Some(surf) = self.surface_effector_for(surface_col) else {
            return;
        };
        if !surf.allows_layer(other_layer) {
            return;
        }
        let Some(other_body_id) = other_col.attached_body else {
            return;
        };
        let Some(body) = self.bodies.get_mut(&other_body_id) else {
            return;
        };
        if !body.is_dynamic() || !body.simulated {
            return;
        }
        let mut tangent = [
            outward_normal_from_surface[1],
            -outward_normal_from_surface[0],
        ];
        let local_right = rotate_vec2([1.0, 0.0], surface_angle);
        if dot2(tangent, local_right) < 0.0 {
            tangent = [-tangent[0], -tangent[1]];
        }
        let current_tangent_speed = dot2(body.linear_velocity, tangent);
        let delta_speed = (surf.speed - current_tangent_speed) * surf.force_scale.clamp(0.0, 1.0);
        body.linear_velocity[0] += tangent[0] * delta_speed;
        body.linear_velocity[1] += tangent[1] * delta_speed;
        body.wake_up();
    }

    fn emit_transition_messages(
        &mut self,
        current_pairs: BTreeMap<(ColliderId2D, ColliderId2D), PairState2D>,
    ) {
        for (key, state) in &current_pairs {
            let phase = if self.touching_pairs.contains_key(key) {
                Phase::Stay
            } else {
                Phase::Enter
            };
            let routed = route_pair_messages_2d(
                phase,
                state.is_trigger,
                state.endpoint_a,
                state.endpoint_b,
                &state.contacts_for_a,
                state.rel_vel_for_a,
                &self.collision_matrix,
            );
            self.messages.extend(routed);
        }

        for (key, prev_state) in &self.touching_pairs {
            if current_pairs.contains_key(key) {
                continue;
            }
            if self.ignored_pairs.contains(key) {
                continue;
            }
            if !self.callbacks_on_disable {
                let a_active = self
                    .colliders
                    .get(&key.0)
                    .is_some_and(|c| self.is_collider_simulated(c));
                let b_active = self
                    .colliders
                    .get(&key.1)
                    .is_some_and(|c| self.is_collider_simulated(c));
                if !a_active || !b_active {
                    continue;
                }
            }
            let routed = route_pair_messages_2d(
                Phase::Exit,
                prev_state.is_trigger,
                prev_state.endpoint_a,
                prev_state.endpoint_b,
                &prev_state.contacts_for_a,
                prev_state.rel_vel_for_a,
                &self.collision_matrix,
            );
            self.messages.extend(routed);
        }

        self.touching_pairs = current_pairs;
    }

    fn update_sleeping(&mut self, dt: f32) {
        for body in self.bodies.values_mut() {
            if !body.simulated || !body.is_dynamic() || body.sleeping {
                continue;
            }
            if body.sleep_mode == RigidbodySleepMode2D::NeverSleep {
                body.sleep_timer = 0.0;
                continue;
            }
            let speed_sq = dot2(body.linear_velocity, body.linear_velocity);
            let ang_speed = body.angular_velocity.abs();
            if speed_sq <= SLEEP_LINEAR_THRESHOLD * SLEEP_LINEAR_THRESHOLD
                && ang_speed <= SLEEP_ANGULAR_THRESHOLD
            {
                body.sleep_timer += dt;
                if body.sleep_timer >= SLEEP_TIME_THRESHOLD {
                    body.sleep();
                }
            } else {
                body.sleep_timer = 0.0;
            }
        }
    }
}

fn shape_sweep_radius(shape: &Shape2D) -> f32 {
    match shape {
        Shape2D::Circle { radius } => radius.max(0.05),
        Shape2D::Box { half_extents } => half_extents[0].min(half_extents[1]).max(0.05),
        Shape2D::Edge { .. } => 0.05,
        Shape2D::Polygon { vertices } => {
            if vertices.is_empty() {
                0.05
            } else {
                let mut min_dist = f32::INFINITY;
                for v in vertices {
                    let d = length2(*v);
                    if d < min_dist {
                        min_dist = d;
                    }
                }
                min_dist.max(0.05)
            }
        }
        Shape2D::Composite { shapes } => shapes
            .iter()
            .map(shape_sweep_radius)
            .fold(0.05f32, f32::max),
    }
}

fn shape_vertical_bounds_and_area(
    shape: &Shape2D,
    center: [f32; 2],
    angle: f32,
) -> (f32, f32, f32) {
    match shape {
        Shape2D::Circle { radius } => {
            let r = radius.max(0.0);
            (center[1] - r, center[1] + r, std::f32::consts::PI * r * r)
        }
        Shape2D::Box { half_extents } => {
            let hx = half_extents[0].max(0.0);
            let hy = half_extents[1].max(0.0);
            let locals = [[-hx, -hy], [hx, -hy], [hx, hy], [-hx, hy]];
            let mut min_y = f32::INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            for p in &locals {
                let wp = transform_point2(*p, center, angle);
                min_y = min_y.min(wp[1]);
                max_y = max_y.max(wp[1]);
            }
            (min_y, max_y, 4.0 * hx * hy)
        }
        Shape2D::Edge { start, end } => {
            let a = transform_point2(*start, center, angle);
            let b = transform_point2(*end, center, angle);
            (a[1].min(b[1]), a[1].max(b[1]), 0.0)
        }
        Shape2D::Polygon { vertices } => {
            if vertices.len() < 3 {
                return (center[1], center[1], 0.0);
            }
            let mut min_y = f32::INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            let mut signed_twice_area = 0.0f32;
            for i in 0..vertices.len() {
                let a = transform_point2(vertices[i], center, angle);
                let b = transform_point2(vertices[(i + 1) % vertices.len()], center, angle);
                min_y = min_y.min(a[1]);
                max_y = max_y.max(a[1]);
                signed_twice_area += a[0] * b[1] - b[0] * a[1];
            }
            (min_y, max_y, 0.5 * signed_twice_area.abs())
        }
        Shape2D::Composite { shapes } => {
            if shapes.is_empty() {
                return (center[1], center[1], 0.0);
            }
            let mut min_y = f32::INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            let mut total_area = 0.0f32;
            for sub in shapes {
                let (s_min, s_max, s_area) = shape_vertical_bounds_and_area(sub, center, angle);
                min_y = min_y.min(s_min);
                max_y = max_y.max(s_max);
                total_area += s_area;
            }
            (min_y, max_y, total_area)
        }
    }
}

fn ordered_pair(a: ColliderId2D, b: ColliderId2D) -> (ColliderId2D, ColliderId2D) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}
