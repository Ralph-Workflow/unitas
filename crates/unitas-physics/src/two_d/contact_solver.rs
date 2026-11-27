use std::collections::{BTreeMap, BTreeSet};

use crate::two_d::collider::{
    compute_contact2d, dot2, length2, sub2, transform_point2, Collider2D, ColliderId2D,
    RawContact2D,
};
use crate::two_d::event::ContactPoint2D;

use super::{ordered_pair, PairState2D, World2D, DEFAULT_SOLVER_ITERATIONS_2D};

impl World2D {
    pub(super) fn solve_and_collect_contacts(
        &mut self,
        _dt: f32,
    ) -> BTreeMap<(ColliderId2D, ColliderId2D), PairState2D> {
        let ids: Vec<ColliderId2D> = self.colliders.keys().copied().collect();
        let mut active_pairs: BTreeMap<(ColliderId2D, ColliderId2D), PairState2D> = BTreeMap::new();
        let mut still_overlapping_all: BTreeSet<(ColliderId2D, ColliderId2D)> = BTreeSet::new();

        for iter in 0..DEFAULT_SOLVER_ITERATIONS_2D {
            for i in 0..ids.len() {
                for j in (i + 1)..ids.len() {
                    self.solve_pair(
                        ids[i],
                        ids[j],
                        iter,
                        &mut active_pairs,
                        &mut still_overlapping_all,
                    );
                }
            }
        }

        self.pass_through_pairs
            .retain(|pair| still_overlapping_all.contains(pair));

        active_pairs
    }

    fn solve_pair(
        &mut self,
        id_a: ColliderId2D,
        id_b: ColliderId2D,
        iter: usize,
        active_pairs: &mut BTreeMap<(ColliderId2D, ColliderId2D), PairState2D>,
        still_overlapping_all: &mut BTreeSet<(ColliderId2D, ColliderId2D)>,
    ) {
        let pair_key = ordered_pair(id_a, id_b);
        if self.ignored_pairs.contains(&pair_key) {
            return;
        }
        let (Some(col_a), Some(col_b)) = (
            self.colliders.get(&id_a).cloned(),
            self.colliders.get(&id_b).cloned(),
        ) else {
            return;
        };
        if !self.is_collider_simulated(&col_a) || !self.is_collider_simulated(&col_b) {
            return;
        }
        if col_a.attached_body.is_some() && col_a.attached_body == col_b.attached_body {
            return;
        }
        let ep_a = self.endpoint_for(&col_a);
        let ep_b = self.endpoint_for(&col_b);
        if !self
            .collision_matrix
            .is_enabled(ep_a.layer as usize, ep_b.layer as usize)
        {
            return;
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
            return;
        };

        still_overlapping_all.insert(pair_key);

        if self.pass_through_pairs.contains(&pair_key) {
            return;
        }

        let vel_a = self.collider_velocity(&col_a);
        let vel_b = self.collider_velocity(&col_b);
        let rel_vel_b_minus_a = sub2(vel_b, vel_a);

        if let Some(plat_a) = self.platform_effector_for(&col_a) {
            if !plat_a.allows_layer(ep_b.layer) {
                self.pass_through_pairs.insert(pair_key);
                return;
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
                    || plat_a.should_pass_through(angle_a, raw_contact.normal, rel_vel_b_minus_a)
                {
                    self.pass_through_pairs.insert(pair_key);
                    return;
                }
            }
        }

        if let Some(plat_b) = self.platform_effector_for(&col_b) {
            if !plat_b.allows_layer(ep_a.layer) {
                self.pass_through_pairs.insert(pair_key);
                return;
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
                let normal_from_b_to_a = [-raw_contact.normal[0], -raw_contact.normal[1]];
                let rel_vel_a_minus_b = [-rel_vel_b_minus_a[0], -rel_vel_b_minus_a[1]];
                if from_below
                    || plat_b.should_pass_through(angle_b, normal_from_b_to_a, rel_vel_a_minus_b)
                {
                    self.pass_through_pairs.insert(pair_key);
                    return;
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
            return;
        }

        let normal_impulse = match self.resolve_pair_impulse(
            &col_a,
            &col_b,
            &raw_contact,
            rel_vel_b_minus_a,
            angle_a,
            angle_b,
        ) {
            Some(impulse) => {
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

        let mut normal_impulse = 0.0f32;
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
            let mut restitution = 0.5 * (col_a.material.bounciness + col_b.material.bounciness);
            let mut friction = (col_a.material.friction * col_b.material.friction).sqrt();
            if let Some((use_side_friction, use_side_bounce)) =
                self.platform_side_rules(col_a, angle_a, col_b, angle_b, raw_contact.normal)
            {
                if !use_side_friction {
                    friction = 0.0;
                }
                if !use_side_bounce {
                    restitution = 0.0;
                }
            }
            normal_impulse = -(1.0 + restitution) * normal_speed / inv_sum;

            if let Some(bid_a) = col_a.attached_body {
                if let Some(body_a) = self.bodies.get_mut(&bid_a) {
                    if body_a.is_dynamic() {
                        if body_a.is_sleeping() && normal_impulse > 1e-4 {
                            body_a.wake_up();
                        }
                        body_a.linear_velocity[0] -= raw_contact.normal[0] * normal_impulse * inv_a;
                        body_a.linear_velocity[1] -= raw_contact.normal[1] * normal_impulse * inv_a;
                    }
                }
            }
            if let Some(bid_b) = col_b.attached_body {
                if let Some(body_b) = self.bodies.get_mut(&bid_b) {
                    if body_b.is_dynamic() {
                        if body_b.is_sleeping() && normal_impulse > 1e-4 {
                            body_b.wake_up();
                        }
                        body_b.linear_velocity[0] += raw_contact.normal[0] * normal_impulse * inv_b;
                        body_b.linear_velocity[1] += raw_contact.normal[1] * normal_impulse * inv_b;
                    }
                }
            }

            let tangent_vel = [
                rel_vel_b_minus_a[0] - raw_contact.normal[0] * normal_speed,
                rel_vel_b_minus_a[1] - raw_contact.normal[1] * normal_speed,
            ];
            let tangent_speed = length2(tangent_vel);
            if tangent_speed > 1e-6 {
                let friction_impulse = (tangent_speed / inv_sum).min(friction * normal_impulse);
                let t_dir = [
                    tangent_vel[0] / tangent_speed,
                    tangent_vel[1] / tangent_speed,
                ];
                if let Some(bid_a) = col_a.attached_body {
                    if let Some(body_a) = self.bodies.get_mut(&bid_a) {
                        if body_a.is_dynamic() {
                            body_a.linear_velocity[0] += t_dir[0] * friction_impulse * inv_a;
                            body_a.linear_velocity[1] += t_dir[1] * friction_impulse * inv_a;
                        }
                    }
                }
                if let Some(bid_b) = col_b.attached_body {
                    if let Some(body_b) = self.bodies.get_mut(&bid_b) {
                        if body_b.is_dynamic() {
                            body_b.linear_velocity[0] -= t_dir[0] * friction_impulse * inv_b;
                            body_b.linear_velocity[1] -= t_dir[1] * friction_impulse * inv_b;
                        }
                    }
                }
            }
        }

        Some(normal_impulse)
    }
}
