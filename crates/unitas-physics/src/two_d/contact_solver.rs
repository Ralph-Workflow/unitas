use std::collections::{BTreeMap, BTreeSet};

use crate::two_d::collider::{
    compute_contact2d, dot2, length2, shape_vertices_world, sub2, transform_point2, Collider2D,
    ColliderId2D, RawContact2D, Shape2D,
};
use crate::two_d::event::ContactPoint2D;

use super::{ordered_pair, PairState2D, World2D, DEFAULT_SOLVER_ITERATIONS_2D};

fn shape_local_bounds(shape: &Shape2D) -> ([f32; 2], [f32; 2]) {
    match shape {
        Shape2D::Circle { radius } => {
            let r = radius.max(0.0);
            ([-r, -r], [r, r])
        }
        Shape2D::Box { half_extents } => {
            let hx = half_extents[0].max(0.0);
            let hy = half_extents[1].max(0.0);
            ([-hx, -hy], [hx, hy])
        }
        Shape2D::Edge { start, end } => (
            [start[0].min(end[0]), start[1].min(end[1])],
            [start[0].max(end[0]), start[1].max(end[1])],
        ),
        Shape2D::Capsule { radius, height, .. } => {
            let extent = height.max(2.0 * radius).max(0.0) * 0.5 + radius.max(0.0);
            ([-extent, -extent], [extent, extent])
        }
        Shape2D::Polygon { .. } => {
            shape_vertices_world(shape, [0.0, 0.0], 0.0).map_or(([0.0, 0.0], [0.0, 0.0]), |verts| {
                let mut min = [f32::INFINITY; 2];
                let mut max = [f32::NEG_INFINITY; 2];
                for v in verts {
                    min = [min[0].min(v[0]), min[1].min(v[1])];
                    max = [max[0].max(v[0]), max[1].max(v[1])];
                }
                (min, max)
            })
        }
        Shape2D::Composite { shapes } => {
            if shapes.is_empty() {
                return ([0.0, 0.0], [0.0, 0.0]);
            }
            let mut min = [f32::INFINITY; 2];
            let mut max = [f32::NEG_INFINITY; 2];
            for sub in shapes {
                let (smin, smax) = shape_local_bounds(sub);
                min = [min[0].min(smin[0]), min[1].min(smin[1])];
                max = [max[0].max(smax[0]), max[1].max(smax[1])];
            }
            (min, max)
        }
    }
}

fn world_aabb(shape: &Shape2D, center: [f32; 2], angle: f32) -> ([f32; 2], [f32; 2]) {
    let (lmin, lmax) = shape_local_bounds(shape);
    if angle.abs() <= 1e-6 {
        return (
            [center[0] + lmin[0], center[1] + lmin[1]],
            [center[0] + lmax[0], center[1] + lmax[1]],
        );
    }
    let corners = [
        [lmin[0], lmin[1]],
        [lmax[0], lmin[1]],
        [lmax[0], lmax[1]],
        [lmin[0], lmax[1]],
    ];
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for c in corners {
        let w = transform_point2(c, center, angle);
        min_x = min_x.min(w[0]);
        min_y = min_y.min(w[1]);
        max_x = max_x.max(w[0]);
        max_y = max_y.max(w[1]);
    }
    ([min_x, min_y], [max_x, max_y])
}

type OverlappingPairData = (
    Collider2D,
    Collider2D,
    [f32; 2],
    f32,
    [f32; 2],
    f32,
    crate::two_d::event::CallbackEndpoint2D,
    crate::two_d::event::CallbackEndpoint2D,
    RawContact2D,
);

impl World2D {
    pub(super) fn solve_and_collect_contacts(
        &mut self,
        _dt: f32,
    ) -> BTreeMap<(ColliderId2D, ColliderId2D), PairState2D> {
        let candidate_pairs = self.collect_candidate_contact_pairs();
        let mut active_pairs: BTreeMap<(ColliderId2D, ColliderId2D), PairState2D> = BTreeMap::new();
        let mut still_overlapping_all: BTreeSet<(ColliderId2D, ColliderId2D)> = BTreeSet::new();

        for iter in 0..DEFAULT_SOLVER_ITERATIONS_2D {
            let mut any_impulse = false;
            for &(id_a, id_b) in &candidate_pairs {
                if self.solve_pair(
                    id_a,
                    id_b,
                    iter,
                    &mut active_pairs,
                    &mut still_overlapping_all,
                ) {
                    any_impulse = true;
                }
            }
            if iter == 0 && !any_impulse {
                break;
            }
        }

        self.pass_through_pairs
            .retain(|pair| still_overlapping_all.contains(pair));

        active_pairs
    }

    fn collect_candidate_contact_pairs(&self) -> Vec<(ColliderId2D, ColliderId2D)> {
        let mut active_colliders: Vec<(ColliderId2D, [f32; 2], [f32; 2])> = self
            .colliders
            .iter()
            .filter(|(_, col)| self.is_collider_simulated(col))
            .map(|(id, col)| {
                let (center, angle) = self.collider_world_pose(col);
                let (bmin, bmax) = world_aabb(&col.shape, center, angle);
                (
                    *id,
                    [bmin[0] - 0.05, bmin[1] - 0.05],
                    [bmax[0] + 0.05, bmax[1] + 0.05],
                )
            })
            .collect();
        active_colliders.sort_by(|a, b| a.1[0].total_cmp(&b.1[0]).then(a.0.cmp(&b.0)));

        let mut candidate_pairs: Vec<(ColliderId2D, ColliderId2D)> = Vec::new();
        for i in 0..active_colliders.len() {
            let (id_a, min_a, max_a) = active_colliders[i];
            for &(id_b, min_b, max_b) in &active_colliders[(i + 1)..] {
                if min_b[0] > max_a[0] {
                    break;
                }
                if max_b[1] >= min_a[1] && min_b[1] <= max_a[1] {
                    let pair = ordered_pair(id_a, id_b);
                    if !self.ignored_pairs.contains(&pair) {
                        candidate_pairs.push(pair);
                    }
                }
            }
        }
        candidate_pairs
    }

    fn overlapping_pair_data(
        &self,
        id_a: ColliderId2D,
        id_b: ColliderId2D,
    ) -> Option<OverlappingPairData> {
        let ca = self.colliders.get(&id_a)?;
        let cb = self.colliders.get(&id_b)?;
        if !self.is_collider_simulated(ca) || !self.is_collider_simulated(cb) {
            return None;
        }
        if ca.attached_body.is_some() && ca.attached_body == cb.attached_body {
            return None;
        }
        let epa = self.endpoint_for(ca);
        let epb = self.endpoint_for(cb);
        if !self
            .collision_matrix
            .is_enabled(epa.layer as usize, epb.layer as usize)
        {
            return None;
        }
        let (ctra, anga) = self.collider_world_pose(ca);
        let (ctrb, angb) = self.collider_world_pose(cb);
        let rc = compute_contact2d(&ca.shape, ctra, anga, &cb.shape, ctrb, angb)?;
        Some((ca.clone(), cb.clone(), ctra, anga, ctrb, angb, epa, epb, rc))
    }

    fn check_platform_pass_through(
        &mut self,
        pair_key: (ColliderId2D, ColliderId2D),
        platform_col: &Collider2D,
        platform_pose: ([f32; 2], f32),
        other_col: &Collider2D,
        other_center_and_layer: ([f32; 2], u8),
        normal_and_rel_vel: ([f32; 2], [f32; 2]),
    ) -> bool {
        let Some(plat) = self.platform_effector_for(platform_col) else {
            return false;
        };
        let (plat_center, plat_angle) = platform_pose;
        let (other_center, other_layer) = other_center_and_layer;
        let (normal, rel_vel) = normal_and_rel_vel;
        if !plat.allows_layer(other_layer) {
            self.pass_through_pairs.insert(pair_key);
            return true;
        }
        if !self.touching_pairs.contains_key(&pair_key) {
            let prev_pos = other_col
                .attached_body
                .and_then(|bid| self.bodies.get(&bid))
                .map_or(other_center, |b| {
                    transform_point2(other_col.offset, b.previous_position, b.rotation)
                });
            let up = plat.surface_up(plat_angle);
            let rel_prev = sub2(prev_pos, plat_center);
            let from_below = dot2(rel_prev, up) < 0.0;
            if from_below || plat.should_pass_through(plat_angle, normal, rel_vel) {
                self.pass_through_pairs.insert(pair_key);
                return true;
            }
        }
        false
    }

    fn solve_pair(
        &mut self,
        id_a: ColliderId2D,
        id_b: ColliderId2D,
        iter: usize,
        active_pairs: &mut BTreeMap<(ColliderId2D, ColliderId2D), PairState2D>,
        still_overlapping_all: &mut BTreeSet<(ColliderId2D, ColliderId2D)>,
    ) -> bool {
        let pair_key = ordered_pair(id_a, id_b);
        if self.ignored_pairs.contains(&pair_key) {
            return false;
        }
        let Some((col_a, col_b, center_a, angle_a, center_b, angle_b, ep_a, ep_b, raw_contact)) =
            self.overlapping_pair_data(id_a, id_b)
        else {
            return false;
        };
        still_overlapping_all.insert(pair_key);
        if self.pass_through_pairs.contains(&pair_key) {
            return false;
        }

        let rel_vel_b_minus_a = sub2(
            self.collider_velocity(&col_b),
            self.collider_velocity(&col_a),
        );
        if self.check_platform_pass_through(
            pair_key,
            &col_a,
            (center_a, angle_a),
            &col_b,
            (center_b, ep_b.layer),
            (raw_contact.normal, rel_vel_b_minus_a),
        ) || self.check_platform_pass_through(
            pair_key,
            &col_b,
            (center_b, angle_b),
            &col_a,
            (center_a, ep_a.layer),
            (
                [-raw_contact.normal[0], -raw_contact.normal[1]],
                [-rel_vel_b_minus_a[0], -rel_vel_b_minus_a[1]],
            ),
        ) {
            return false;
        }

        if col_a.is_trigger || col_b.is_trigger {
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
            return false;
        }

        self.record_resolved_pair(
            pair_key,
            (&col_a, angle_a, ep_a),
            (&col_b, angle_b, ep_b),
            &raw_contact,
            rel_vel_b_minus_a,
            active_pairs,
        )
    }

    fn record_resolved_pair(
        &mut self,
        pair_key: (ColliderId2D, ColliderId2D),
        side_a: (&Collider2D, f32, crate::two_d::event::CallbackEndpoint2D),
        side_b: (&Collider2D, f32, crate::two_d::event::CallbackEndpoint2D),
        raw_contact: &RawContact2D,
        rel_vel_b_minus_a: [f32; 2],
        active_pairs: &mut BTreeMap<(ColliderId2D, ColliderId2D), PairState2D>,
    ) -> bool {
        let (col_a, angle_a, ep_a) = side_a;
        let (col_b, angle_b, ep_b) = side_b;
        let normal_impulse = match self.resolve_pair_impulse(
            col_a,
            col_b,
            raw_contact,
            rel_vel_b_minus_a,
            angle_a,
            angle_b,
        ) {
            Some(impulse) => {
                self.apply_surface_effector_on_pair(
                    col_a,
                    angle_a,
                    col_b,
                    ep_b.layer,
                    raw_contact.normal,
                );
                self.apply_surface_effector_on_pair(
                    col_b,
                    angle_b,
                    col_a,
                    ep_a.layer,
                    [-raw_contact.normal[0], -raw_contact.normal[1]],
                );
                impulse
            }
            None => 0.0,
        };

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
        normal_impulse > 0.0 || raw_contact.depth > 1e-6
    }

    fn resolve_pair_impulse(
        &mut self,
        col_a: &Collider2D,
        col_b: &Collider2D,
        raw_contact: &RawContact2D,
        rel_vel_b_minus_a: [f32; 2],
        angle_a: f32,
        angle_b: f32,
    ) -> Option<f32> {
        let inv_a = col_a
            .attached_body
            .and_then(|bid| self.bodies.get(&bid))
            .map_or(0.0, |b| b.inverse_mass());
        let inv_b = col_b
            .attached_body
            .and_then(|bid| self.bodies.get(&bid))
            .map_or(0.0, |b| b.inverse_mass());
        let inv_sum = inv_a + inv_b;
        if inv_sum <= 0.0 {
            return None;
        }

        self.depentrate_pair(col_a, col_b, raw_contact, (inv_a, inv_b, inv_sum));

        let normal_speed = dot2(rel_vel_b_minus_a, raw_contact.normal);
        let tangent_vel = [
            rel_vel_b_minus_a[0] - raw_contact.normal[0] * normal_speed,
            rel_vel_b_minus_a[1] - raw_contact.normal[1] * normal_speed,
        ];
        let tangent_speed = length2(tangent_vel);

        let (restitution, friction) =
            self.contact_material_params(col_a, angle_a, col_b, angle_b, raw_contact.normal);

        let mut normal_impulse = 0.0f32;
        if normal_speed < 0.0 {
            normal_impulse = -(1.0 + restitution) * normal_speed / inv_sum;
            self.apply_normal_impulse_2d(
                col_a,
                col_b,
                raw_contact.normal,
                normal_impulse,
                (inv_a, inv_b),
                length2(rel_vel_b_minus_a) > 1e-4,
            );
        }

        let effective_normal_impulse = if normal_impulse > 0.0 {
            normal_impulse
        } else if raw_contact.depth > 1e-6 && normal_speed <= 1e-4 {
            let grav_mag = length2(self.gravity).max(9.81);
            grav_mag * self.fixed_timestep / inv_sum
        } else {
            0.0
        };

        if tangent_speed > 1e-6 && effective_normal_impulse > 0.0 && friction > 0.0 {
            let friction_impulse =
                (tangent_speed / inv_sum).min(friction * effective_normal_impulse);
            let t_dir = [
                tangent_vel[0] / tangent_speed,
                tangent_vel[1] / tangent_speed,
            ];
            self.apply_friction_impulse_2d(col_a, col_b, t_dir, friction_impulse, (inv_a, inv_b));
        }

        Some(normal_impulse)
    }

    fn depentrate_pair(
        &mut self,
        col_a: &Collider2D,
        col_b: &Collider2D,
        raw_contact: &RawContact2D,
        masses: (f32, f32, f32),
    ) {
        let (inv_a, inv_b, inv_sum) = masses;
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
    }

    fn contact_material_params(
        &self,
        col_a: &Collider2D,
        angle_a: f32,
        col_b: &Collider2D,
        angle_b: f32,
        normal: [f32; 2],
    ) -> (f32, f32) {
        let mut restitution = 0.5 * (col_a.material.bounciness + col_b.material.bounciness);
        let mut friction = (col_a.material.friction * col_b.material.friction).sqrt();
        if let Some((use_side_friction, use_side_bounce)) =
            self.platform_side_rules(col_a, angle_a, col_b, angle_b, normal)
        {
            if !use_side_friction {
                friction = 0.0;
            }
            if !use_side_bounce {
                restitution = 0.0;
            }
        }
        (restitution, friction)
    }

    fn apply_normal_impulse_2d(
        &mut self,
        col_a: &Collider2D,
        col_b: &Collider2D,
        normal: [f32; 2],
        normal_impulse: f32,
        inv_masses: (f32, f32),
        rel_vel_wake: bool,
    ) {
        let (inv_a, inv_b) = inv_masses;
        let wake = normal_impulse > 1e-4 || rel_vel_wake;
        if let Some(bid_a) = col_a.attached_body {
            if let Some(body_a) = self.bodies.get_mut(&bid_a) {
                if body_a.is_dynamic() {
                    if body_a.is_sleeping() && wake {
                        body_a.wake_up();
                    }
                    body_a.linear_velocity[0] -= normal[0] * normal_impulse * inv_a;
                    body_a.linear_velocity[1] -= normal[1] * normal_impulse * inv_a;
                }
            }
        }
        if let Some(bid_b) = col_b.attached_body {
            if let Some(body_b) = self.bodies.get_mut(&bid_b) {
                if body_b.is_dynamic() {
                    if body_b.is_sleeping() && wake {
                        body_b.wake_up();
                    }
                    body_b.linear_velocity[0] += normal[0] * normal_impulse * inv_b;
                    body_b.linear_velocity[1] += normal[1] * normal_impulse * inv_b;
                }
            }
        }
    }

    fn apply_friction_impulse_2d(
        &mut self,
        col_a: &Collider2D,
        col_b: &Collider2D,
        t_dir: [f32; 2],
        friction_impulse: f32,
        inv_masses: (f32, f32),
    ) {
        let (inv_a, inv_b) = inv_masses;
        if let Some(bid_a) = col_a.attached_body {
            if let Some(body_a) = self.bodies.get_mut(&bid_a) {
                if body_a.is_dynamic() {
                    if body_a.is_sleeping() && friction_impulse > 1e-5 {
                        body_a.wake_up();
                    }
                    body_a.linear_velocity[0] += t_dir[0] * friction_impulse * inv_a;
                    body_a.linear_velocity[1] += t_dir[1] * friction_impulse * inv_a;
                }
            }
        }
        if let Some(bid_b) = col_b.attached_body {
            if let Some(body_b) = self.bodies.get_mut(&bid_b) {
                if body_b.is_dynamic() {
                    if body_b.is_sleeping() && friction_impulse > 1e-5 {
                        body_b.wake_up();
                    }
                    body_b.linear_velocity[0] -= t_dir[0] * friction_impulse * inv_b;
                    body_b.linear_velocity[1] -= t_dir[1] * friction_impulse * inv_b;
                }
            }
        }
    }
}
