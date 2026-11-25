use std::collections::{BTreeMap, BTreeSet};

use unitas_core::EntityId;

use crate::event::Phase;
use crate::matrix::{CollisionMatrix, LAYER_COUNT};
use crate::two_d::body::{BodyDesc2D, BodyId2D, BodyType2D, Rigidbody2D};
use crate::two_d::collider::{
    compute_contact2d, dot2, length2, normalize2, point_in_shape2d, raycast_shape2d, rotate_vec2,
    sub2, transform_point2, Collider2D, ColliderDesc2D, ColliderId2D, PhysicsMaterial2D, Shape2D,
};
use crate::two_d::effector::{Effector2D, PlatformEffector2D, SurfaceEffector2D};
use crate::two_d::event::{
    route_pair_messages_2d, ContactPoint2D, Endpoint2D, RoutedMessage2D,
};

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
    bodies: BTreeMap<BodyId2D, Rigidbody2D>,
    colliders: BTreeMap<ColliderId2D, Collider2D>,
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
            bodies: BTreeMap::new(),
            colliders: BTreeMap::new(),
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
        let collider = Collider2D::from_desc(id, desc);
        self.colliders.insert(id, collider);
        id
    }

    pub fn collider(&self, id: ColliderId2D) -> Option<&Collider2D> {
        self.colliders.get(&id)
    }

    pub fn collider_mut(&mut self, id: ColliderId2D) -> Option<&mut Collider2D> {
        self.colliders.get_mut(&id)
    }

    pub fn remove_collider(&mut self, id: ColliderId2D) -> bool {
        let existed = self.colliders.remove(&id).is_some();
        if existed {
            self.effectors.remove(&id);
            self.ignored_pairs
                .retain(|(a, b)| *a != id && *b != id);
            self.touching_pairs
                .retain(|(a, b), _| *a != id && *b != id);
            self.pass_through_pairs
                .retain(|(a, b)| *a != id && *b != id);
        }
        existed
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

    pub fn step(&mut self, dt: f32) {
        if !dt.is_finite() || dt < 0.0 {
            return;
        }
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
            if compute_contact2d(probe, probe_center, probe_angle, &collider.shape, center, angle)
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
            let has_area_or_point = eff_list
                .iter()
                .any(|e| matches!(e, Effector2D::Area(_) | Effector2D::Point(_)));
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
                        Effector2D::Platform(_) | Effector2D::Surface(_) => {}
                    }
                }
            }
        }
    }

    fn integrate_bodies(&mut self, dt: f32) {
        for body in self.bodies.values_mut() {
            if !body.simulated {
                continue;
            }
            body.previous_position = body.position;
            match body.body_type {
                BodyType2D::Dynamic => {
                    if body.sleeping {
                        body.accumulated_force = [0.0, 0.0];
                        body.accumulated_torque = 0.0;
                        continue;
                    }
                    let inv_mass = body.inverse_mass();
                    let ax = self.gravity[0] * body.gravity_scale
                        + body.accumulated_force[0] * inv_mass;
                    let ay = self.gravity[1] * body.gravity_scale
                        + body.accumulated_force[1] * inv_mass;
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
                }
                BodyType2D::Static => {}
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
                            let rel_vel_a_minus_b =
                                [-rel_vel_b_minus_a[0], -rel_vel_b_minus_a[1]];
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
        let mut tangent = [outward_normal_from_surface[1], -outward_normal_from_surface[0]];
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

fn ordered_pair(a: ColliderId2D, b: ColliderId2D) -> (ColliderId2D, ColliderId2D) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}
