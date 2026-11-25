use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

use unitas_core::EntityId;

use crate::body::{add, dot, subtract, vector_length};
use crate::error::PhysicsError;
use crate::query::{normalize, QueryHit};
use crate::world::World;

const MAX_SLIDE_ITERATIONS: usize = 4;
const MAX_DEPENETRATION_ITERATIONS: usize = 8;
const DEPENETRATION_DIRECTIONS: [[f64; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
];
const MOTION_EPSILON: f64 = 1e-6;
const CEILING_NORMAL_THRESHOLD: f64 = -0.5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CollisionFlags(u32);

impl CollisionFlags {
    pub const NONE: Self = Self(0);
    pub const SIDES: Self = Self(1);
    pub const ABOVE: Self = Self(2);
    pub const BELOW: Self = Self(4);

    pub const fn empty() -> Self {
        Self::NONE
    }

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits & 0x7)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl BitOr for CollisionFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for CollisionFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for CollisionFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for CollisionFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControllerColliderHit {
    pub collider: EntityId,
    pub point: [f64; 3],
    pub normal: [f64; 3],
    pub move_direction: [f64; 3],
    pub move_length: f64,
}

impl Default for ControllerColliderHit {
    fn default() -> Self {
        Self {
            collider: EntityId::new(0),
            point: [0.0; 3],
            normal: [0.0; 3],
            move_direction: [0.0; 3],
            move_length: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveResult {
    pub position: [f64; 3],
    pub is_grounded: bool,
    pub collision_flags: CollisionFlags,
    pub ground_entity: Option<EntityId>,
}

const MAX_RECORDED_HITS: usize = 8;

#[derive(Default)]
struct MoveStepOutcome {
    flags: CollisionFlags,
    grounded: bool,
    ground_entity: Option<EntityId>,
    hits: [ControllerColliderHit; MAX_RECORDED_HITS],
    hit_count: usize,
}

impl MoveStepOutcome {
    fn push_hit(&mut self, hit: ControllerColliderHit) {
        if self.hit_count < MAX_RECORDED_HITS {
            self.hits[self.hit_count] = hit;
            self.hit_count += 1;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterController {
    pub radius: f64,
    pub height: f64,
    pub center: [f64; 3],
    pub slope_limit_deg: f64,
    pub step_offset: f64,
    pub skin_width: f64,
    pub min_move_distance: f64,
    pub layer_mask: u32,
    pub enabled: bool,
    pub excluded_entity: Option<EntityId>,
    is_grounded: bool,
    collision_flags: CollisionFlags,
    ground_entity: Option<EntityId>,
    velocity: [f64; 3],
    collider_hits: [ControllerColliderHit; MAX_RECORDED_HITS],
    collider_hit_count: usize,
}

impl Default for CharacterController {
    fn default() -> Self {
        Self {
            radius: 0.5,
            height: 2.0,
            center: [0.0, 0.0, 0.0],
            slope_limit_deg: 45.0,
            step_offset: 0.3,
            skin_width: 0.08,
            min_move_distance: 0.001,
            layer_mask: u32::MAX,
            enabled: true,
            excluded_entity: None,
            is_grounded: false,
            collision_flags: CollisionFlags::NONE,
            ground_entity: None,
            velocity: [0.0; 3],
            collider_hits: [ControllerColliderHit::default(); MAX_RECORDED_HITS],
            collider_hit_count: 0,
        }
    }
}

impl CharacterController {
    pub fn new(radius: f64, height: f64) -> Result<Self, PhysicsError> {
        let controller = Self {
            radius,
            height,
            ..Self::default()
        };
        controller.validate()?;
        Ok(controller)
    }

    pub fn validate(&self) -> Result<(), PhysicsError> {
        let valid = self.radius.is_finite()
            && self.radius > 0.0
            && self.height.is_finite()
            && self.height > 0.0
            && self.center.iter().all(|component| component.is_finite())
            && self.slope_limit_deg.is_finite()
            && (0.0..=90.0).contains(&self.slope_limit_deg)
            && self.step_offset.is_finite()
            && self.step_offset >= 0.0
            && self.step_offset <= self.height
            && self.skin_width.is_finite()
            && self.skin_width > 0.0
            && self.min_move_distance.is_finite()
            && self.min_move_distance >= 0.0;
        if !valid {
            return Err(PhysicsError::InvalidCharacterController);
        }
        Ok(())
    }

    pub fn is_grounded(&self) -> bool {
        self.is_grounded
    }

    pub fn collision_flags(&self) -> CollisionFlags {
        self.collision_flags
    }

    pub fn ground_entity(&self) -> Option<EntityId> {
        self.ground_entity
    }

    pub fn velocity(&self) -> [f64; 3] {
        self.velocity
    }

    pub fn collider_hits(&self) -> &[ControllerColliderHit] {
        &self.collider_hits[..self.collider_hit_count]
    }

    pub fn drain_collider_hits(&mut self) -> Vec<ControllerColliderHit> {
        let out = self.collider_hits[..self.collider_hit_count].to_vec();
        self.collider_hit_count = 0;
        out
    }

    pub fn teleport(&mut self, position: [f64; 3]) -> Result<[f64; 3], PhysicsError> {
        self.validate()?;
        if !position.iter().all(|component| component.is_finite()) {
            return Err(PhysicsError::InvalidCharacterController);
        }
        self.is_grounded = false;
        self.collision_flags = CollisionFlags::NONE;
        self.ground_entity = None;
        self.velocity = [0.0; 3];
        self.collider_hit_count = 0;
        Ok(position)
    }

    pub fn simple_move(
        &mut self,
        world: &World,
        current_position: [f64; 3],
        speed: [f64; 3],
        delta_seconds: f64,
    ) -> Result<MoveResult, PhysicsError> {
        if !delta_seconds.is_finite() || delta_seconds < 0.0 {
            return Err(PhysicsError::InvalidDeltaTime(delta_seconds));
        }
        let g = world.gravity();
        let motion = [
            speed[0] * delta_seconds,
            g[1] * delta_seconds,
            speed[2] * delta_seconds,
        ];
        self.move_character_with_dt(world, current_position, motion, delta_seconds)
    }

    pub fn move_character(
        &mut self,
        world: &World,
        current_position: [f64; 3],
        motion: [f64; 3],
    ) -> Result<MoveResult, PhysicsError> {
        self.move_character_with_dt(world, current_position, motion, 1.0 / 50.0)
    }

    pub fn move_character_with_dt(
        &mut self,
        world: &World,
        current_position: [f64; 3],
        motion: [f64; 3],
        delta_seconds: f64,
    ) -> Result<MoveResult, PhysicsError> {
        self.validate()?;
        if !current_position
            .iter()
            .all(|component| component.is_finite())
            || !motion.iter().all(|component| component.is_finite())
        {
            return Err(PhysicsError::InvalidCharacterController);
        }
        if !delta_seconds.is_finite() || delta_seconds < 0.0 {
            return Err(PhysicsError::InvalidDeltaTime(delta_seconds));
        }
        if !self.enabled {
            self.is_grounded = false;
            self.collision_flags = CollisionFlags::NONE;
            self.ground_entity = None;
            self.velocity = [0.0; 3];
            self.collider_hit_count = 0;
            return Ok(MoveResult {
                position: current_position,
                is_grounded: false,
                collision_flags: CollisionFlags::NONE,
                ground_entity: None,
            });
        }

        self.collider_hit_count = 0;
        let platform_delta = self.platform_displacement(world, delta_seconds);
        let total_motion = add(motion, platform_delta);
        let mut position = current_position;
        let mut outcome = MoveStepOutcome::default();

        let depenetrated = self.depenetrate(world, position, &mut outcome.flags);
        let did_depenetrate = vector_length(subtract(depenetrated, position)) > MOTION_EPSILON;
        position = depenetrated;

        if vector_length(total_motion) < self.min_move_distance && !did_depenetrate {
            self.collision_flags = CollisionFlags::NONE;
            self.velocity = [0.0; 3];
            return Ok(MoveResult {
                position,
                is_grounded: self.is_grounded,
                collision_flags: CollisionFlags::NONE,
                ground_entity: self.ground_entity,
            });
        }

        let lateral_motion = [total_motion[0], 0.0, total_motion[2]];
        if vector_length(lateral_motion) > MOTION_EPSILON {
            position = self.resolve_lateral(world, position, lateral_motion, &mut outcome);
        }

        let vertical_motion = [0.0, total_motion[1], 0.0];
        if vertical_motion[1].abs() > MOTION_EPSILON {
            position = self.resolve_vertical(world, position, vertical_motion[1], &mut outcome);
        }

        if total_motion[1] <= MOTION_EPSILON {
            if let Some(hit) = self.probe_ground(world, position) {
                if self.is_walkable_normal(hit.normal) {
                    outcome.grounded = true;
                    outcome.flags.insert(CollisionFlags::BELOW);
                    outcome.ground_entity = Some(hit.entity);
                    if hit.distance > self.skin_width && total_motion[1] <= 0.0 {
                        let snap = (hit.distance - self.skin_width).min(self.skin_width);
                        position[1] -= snap;
                    }
                }
            }
        }

        self.is_grounded = outcome.grounded;
        self.collision_flags = outcome.flags;
        self.ground_entity = outcome.ground_entity;
        self.collider_hits = outcome.hits;
        self.collider_hit_count = outcome.hit_count;
        if delta_seconds > MOTION_EPSILON {
            let disp = subtract(position, current_position);
            self.velocity = [
                disp[0] / delta_seconds,
                disp[1] / delta_seconds,
                disp[2] / delta_seconds,
            ];
        } else {
            self.velocity = [0.0; 3];
        }

        Ok(MoveResult {
            position,
            is_grounded: outcome.grounded,
            collision_flags: outcome.flags,
            ground_entity: outcome.ground_entity,
        })
    }

    fn platform_displacement(&self, world: &World, delta_seconds: f64) -> [f64; 3] {
        if !self.is_grounded || delta_seconds <= 0.0 {
            return [0.0; 3];
        }
        let Some(entity) = self.ground_entity else {
            return [0.0; 3];
        };
        let Some(velocity) = world.effective_velocity(entity) else {
            return [0.0; 3];
        };
        [
            velocity[0] * delta_seconds,
            velocity[1] * delta_seconds,
            velocity[2] * delta_seconds,
        ]
    }

    fn is_walkable_normal(&self, normal: [f64; 3]) -> bool {
        let min_y = self.slope_limit_deg.to_radians().cos();
        normal[1] >= min_y - 1e-6 && normal[1] > 0.0
    }

    fn classify_normal(&self, normal: [f64; 3]) -> CollisionFlags {
        if self.is_walkable_normal(normal) {
            CollisionFlags::BELOW
        } else if normal[1] <= CEILING_NORMAL_THRESHOLD {
            CollisionFlags::ABOVE
        } else {
            CollisionFlags::SIDES
        }
    }

    fn sample_centers(&self, position: [f64; 3]) -> Vec<[f64; 3]> {
        let capsule_center = add(position, self.center);
        let half_segment = (0.5 * self.height - self.radius).max(0.0);
        if half_segment <= 1e-6 {
            return vec![capsule_center];
        }
        let segments = ((2.0 * half_segment) / self.radius).ceil() as usize;
        let count = segments.max(2);
        let mut points = Vec::with_capacity(count + 1);
        for index in 0..=count {
            let fraction = index as f64 / count as f64;
            let offset_y = -half_segment + 2.0 * half_segment * fraction;
            points.push([
                capsule_center[0],
                capsule_center[1] + offset_y,
                capsule_center[2],
            ]);
        }
        points
    }

    fn bottom_sphere_center(&self, position: [f64; 3]) -> [f64; 3] {
        let capsule_center = add(position, self.center);
        let half_segment = (0.5 * self.height - self.radius).max(0.0);
        [
            capsule_center[0],
            capsule_center[1] - half_segment,
            capsule_center[2],
        ]
    }

    fn cast_capsule(
        &self,
        world: &World,
        position: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
    ) -> Option<QueryHit> {
        if max_distance <= 0.0 {
            return None;
        }
        let unit_dir = normalize(direction)?;
        let mut best: Option<QueryHit> = None;
        for sample in self.sample_centers(position) {
            let hits = world.sphere_cast(
                sample,
                self.radius,
                unit_dir,
                max_distance,
                self.layer_mask,
                false,
            );
            for hit in hits {
                if self.excluded_entity == Some(hit.entity) {
                    continue;
                }
                if dot(hit.normal, unit_dir) >= -1e-9 {
                    continue;
                }
                let replace = match best {
                    None => true,
                    Some(current) => {
                        hit.distance < current.distance - 1e-9
                            || ((hit.distance - current.distance).abs() <= 1e-9
                                && hit.entity < current.entity)
                    }
                };
                if replace {
                    best = Some(hit);
                }
            }
        }
        best
    }

    fn overlaps_any(&self, world: &World, position: [f64; 3]) -> bool {
        let probe_radius = (self.radius - 1e-5).max(1e-5);
        for sample in self.sample_centers(position) {
            let hits = world.overlap_sphere(sample, probe_radius, self.layer_mask);
            if hits
                .into_iter()
                .any(|entity| Some(entity) != self.excluded_entity)
            {
                return true;
            }
        }
        false
    }

    fn depenetrate(
        &self,
        world: &World,
        start_position: [f64; 3],
        flags: &mut CollisionFlags,
    ) -> [f64; 3] {
        let mut position = start_position;
        for _ in 0..MAX_DEPENETRATION_ITERATIONS {
            if !self.overlaps_any(world, position) {
                break;
            }
            let mut best_push: Option<([f64; 3], f64, [f64; 3])> = None;
            let search_dist = (self.radius * 2.0 + self.height).max(1.0);
            for dir in DEPENETRATION_DIRECTIONS {
                let probe_origin = [
                    position[0] + dir[0] * search_dist,
                    position[1] + dir[1] * search_dist,
                    position[2] + dir[2] * search_dist,
                ];
                let inward = [-dir[0], -dir[1], -dir[2]];
                if let Some(hit) = self.cast_capsule(world, probe_origin, inward, search_dist) {
                    let push_dist = (search_dist - hit.distance) + self.skin_width;
                    if push_dist > 0.0
                        && best_push.is_none_or(|(_, best_dist, _)| push_dist < best_dist)
                    {
                        let candidate = [
                            position[0] + dir[0] * push_dist,
                            position[1] + dir[1] * push_dist,
                            position[2] + dir[2] * push_dist,
                        ];
                        if !self.overlaps_any(world, candidate) {
                            best_push = Some((candidate, push_dist, hit.normal));
                        }
                    }
                }
            }
            let Some((next_pos, _, normal)) = best_push else {
                break;
            };
            position = next_pos;
            flags.insert(self.classify_normal(normal));
        }
        position
    }

    fn resolve_lateral(
        &self,
        world: &World,
        start_position: [f64; 3],
        lateral_motion: [f64; 3],
        outcome: &mut MoveStepOutcome,
    ) -> [f64; 3] {
        let mut position = start_position;
        let mut remaining = lateral_motion;

        for _ in 0..MAX_SLIDE_ITERATIONS {
            let distance = vector_length(remaining);
            if distance <= MOTION_EPSILON {
                break;
            }
            let Some(direction) = normalize(remaining) else {
                break;
            };
            let cast_distance = distance + self.skin_width;
            let Some(hit) = self.cast_capsule(world, position, direction, cast_distance) else {
                position = add(position, remaining);
                break;
            };

            outcome.push_hit(ControllerColliderHit {
                collider: hit.entity,
                point: hit.point,
                normal: hit.normal,
                move_direction: direction,
                move_length: distance,
            });

            let safe_travel = (hit.distance - self.skin_width).max(0.0);
            position = [
                position[0] + direction[0] * safe_travel,
                position[1] + direction[1] * safe_travel,
                position[2] + direction[2] * safe_travel,
            ];
            let leftover_dist = (distance - safe_travel).max(0.0);
            let leftover = [
                direction[0] * leftover_dist,
                direction[1] * leftover_dist,
                direction[2] * leftover_dist,
            ];

            if self.is_walkable_normal(hit.normal) {
                outcome.flags.insert(CollisionFlags::BELOW);
                outcome.grounded = true;
                outcome.ground_entity = Some(hit.entity);
                remaining = project_onto_plane(leftover, hit.normal);
                continue;
            }

            if let Some((stepped_pos, step_ground)) = self.try_step_up(world, position, leftover) {
                position = stepped_pos;
                outcome.flags.insert(CollisionFlags::BELOW);
                outcome.grounded = true;
                outcome.ground_entity = Some(step_ground);
                break;
            }

            outcome.flags.insert(self.classify_normal(hit.normal));
            let mut wall_normal = [hit.normal[0], 0.0, hit.normal[2]];
            if let Some(norm_xz) = normalize(wall_normal) {
                wall_normal = norm_xz;
            } else {
                wall_normal = hit.normal;
            }
            let into = dot(leftover, wall_normal);
            if into < 0.0 {
                remaining = [
                    leftover[0] - wall_normal[0] * into,
                    0.0,
                    leftover[2] - wall_normal[2] * into,
                ];
            } else {
                break;
            }
        }

        position
    }

    fn try_step_up(
        &self,
        world: &World,
        position: [f64; 3],
        horizontal_delta: [f64; 3],
    ) -> Option<([f64; 3], EntityId)> {
        if self.step_offset <= 0.0 {
            return None;
        }
        let horizontal = [horizontal_delta[0], 0.0, horizontal_delta[2]];
        let forward_dist = vector_length(horizontal);
        if forward_dist <= MOTION_EPSILON {
            return None;
        }
        let forward_dir = normalize(horizontal)?;

        let up_cast_dist = self.step_offset + self.skin_width;
        let up_clear = match self.cast_capsule(world, position, [0.0, 1.0, 0.0], up_cast_dist) {
            Some(hit) => (hit.distance - self.skin_width).max(0.0),
            None => self.step_offset,
        };
        if up_clear <= MOTION_EPSILON {
            return None;
        }
        let raised = [position[0], position[1] + up_clear, position[2]];

        let forward_cast_dist = forward_dist + self.skin_width;
        let forward_travel = match self.cast_capsule(world, raised, forward_dir, forward_cast_dist)
        {
            Some(hit) => (hit.distance - self.skin_width).max(0.0),
            None => forward_dist,
        };
        let min_advance = forward_dist.min(self.skin_width.max(1e-3));
        if forward_travel < min_advance {
            return None;
        }
        let advanced = [
            raised[0] + forward_dir[0] * forward_travel,
            raised[1],
            raised[2] + forward_dir[2] * forward_travel,
        ];

        let down_cast_dist = up_clear + self.skin_width * 2.0;
        let down_hit = self.cast_capsule(world, advanced, [0.0, -1.0, 0.0], down_cast_dist)?;
        if !self.is_walkable_normal(down_hit.normal) {
            return None;
        }
        let drop = (down_hit.distance - self.skin_width).max(0.0);
        let landed = [advanced[0], advanced[1] - drop, advanced[2]];
        let climb_height = landed[1] - position[1];
        if climb_height <= 1e-4 || climb_height > self.step_offset + 1e-4 {
            return None;
        }
        let forward_ledge_depth = (down_hit.point[0] - position[0]) * forward_dir[0]
            + (down_hit.point[2] - position[2]) * forward_dir[2];
        let min_step_depth = (self.radius * 0.25).max(self.skin_width);
        let tread_probe_origin = [
            down_hit.point[0] + forward_dir[0] * min_step_depth,
            down_hit.point[1] + self.skin_width + 1e-2,
            down_hit.point[2] + forward_dir[2] * min_step_depth,
        ];
        let tread_probe_dist = self.skin_width * 2.0 + 2e-2;
        let tread_supported = world
            .raycast(
                tread_probe_origin,
                [0.0, -1.0, 0.0],
                tread_probe_dist,
                self.layer_mask,
            )
            .into_iter()
            .find(|hit| Some(hit.entity) != self.excluded_entity && hit.normal[1] > 0.0)
            .is_some_and(|hit| {
                self.is_walkable_normal(hit.normal)
                    && (hit.point[1] - down_hit.point[1]).abs() <= 1e-2
            });
        if !tread_supported || forward_ledge_depth <= 0.0 {
            return None;
        }
        if self.overlaps_any(world, landed) {
            return None;
        }
        Some((landed, down_hit.entity))
    }

    fn resolve_vertical(
        &self,
        world: &World,
        start_position: [f64; 3],
        vertical_delta: f64,
        outcome: &mut MoveStepOutcome,
    ) -> [f64; 3] {
        let direction = if vertical_delta > 0.0 {
            [0.0, 1.0, 0.0]
        } else {
            [0.0, -1.0, 0.0]
        };
        let distance = vertical_delta.abs();
        let cast_distance = distance + self.skin_width;
        let Some(hit) = self.cast_capsule(world, start_position, direction, cast_distance) else {
            return [
                start_position[0],
                start_position[1] + vertical_delta,
                start_position[2],
            ];
        };

        outcome.push_hit(ControllerColliderHit {
            collider: hit.entity,
            point: hit.point,
            normal: hit.normal,
            move_direction: direction,
            move_length: distance,
        });

        let safe_travel = (hit.distance - self.skin_width).max(0.0);
        let mut position = [
            start_position[0],
            start_position[1] + direction[1] * safe_travel,
            start_position[2],
        ];

        if vertical_delta > 0.0 {
            outcome.flags.insert(CollisionFlags::ABOVE);
        } else if self.is_walkable_normal(hit.normal) {
            outcome.flags.insert(CollisionFlags::BELOW);
            outcome.grounded = true;
            outcome.ground_entity = Some(hit.entity);
        } else {
            outcome.flags.insert(CollisionFlags::SIDES);
            let leftover_dist = (distance - safe_travel).max(0.0);
            if leftover_dist > MOTION_EPSILON {
                let slide = project_onto_plane([0.0, -leftover_dist, 0.0], hit.normal);
                let slide_len = vector_length(slide);
                if slide_len > MOTION_EPSILON {
                    if let Some(slide_dir) = normalize(slide) {
                        let slide_travel = match self.cast_capsule(
                            world,
                            position,
                            slide_dir,
                            slide_len + self.skin_width,
                        ) {
                            Some(second_hit) => {
                                if self.is_walkable_normal(second_hit.normal) {
                                    outcome.flags.insert(CollisionFlags::BELOW);
                                    outcome.grounded = true;
                                    outcome.ground_entity = Some(second_hit.entity);
                                }
                                (second_hit.distance - self.skin_width).max(0.0)
                            }
                            None => slide_len,
                        };
                        position = [
                            position[0] + slide_dir[0] * slide_travel,
                            position[1] + slide_dir[1] * slide_travel,
                            position[2] + slide_dir[2] * slide_travel,
                        ];
                    }
                }
            }
        }

        position
    }

    fn probe_ground(&self, world: &World, position: [f64; 3]) -> Option<QueryHit> {
        let bottom = self.bottom_sphere_center(position);
        let probe_dist = self.skin_width * 2.0 + 1e-3;
        let hits = world.sphere_cast(
            bottom,
            self.radius,
            [0.0, -1.0, 0.0],
            probe_dist,
            self.layer_mask,
            false,
        );
        hits.into_iter()
            .find(|hit| Some(hit.entity) != self.excluded_entity && hit.normal[1] > 0.0)
    }
}

fn project_onto_plane(vector: [f64; 3], normal: [f64; 3]) -> [f64; 3] {
    let into = dot(vector, normal);
    if into >= 0.0 {
        return vector;
    }
    [
        vector[0] - normal[0] * into,
        vector[1] - normal[1] * into,
        vector[2] - normal[2] * into,
    ]
}
