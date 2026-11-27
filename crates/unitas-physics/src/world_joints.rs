use std::collections::BTreeSet;

use unitas_core::EntityId;

use crate::body::{
    add, dot, inverse_rotation, multiply_rotation, normalize_rotation, rotate_vector,
    slerp_rotation, subtract, vector_length, BodyRole, IDENTITY_ROTATION,
};
use crate::joint::{
    BrokenJointEvent3D, CharacterJoint3D, ConfigurableJoint3D, ConfigurableJointMotion,
    FixedJoint3D, HingeJoint3D, Joint3D, JointDrive3D, JointId3D, SpringJoint3D,
};
use crate::query::normalize;

use super::{
    check_break_3d, clamp_character_rotation, clamp_configurable_rotation, cross3,
    decompose_twist_swing_angle, orthonormal_frame, quat_from_axis_angle, rotation_axis_and_angle,
    scale3, BodyState3D, World,
};

impl World {
    pub(super) fn solve_joints_3d(&mut self, dt: f64) {
        if dt <= 0.0 || self.joints_3d.is_empty() {
            return;
        }
        let joint_entries: Vec<(JointId3D, Joint3D)> = self
            .joints_3d
            .iter()
            .map(|(id, j)| (*id, j.clone()))
            .collect();
        let mut broken: Vec<BrokenJointEvent3D> = Vec::new();
        let mut touched_indices: BTreeSet<usize> = BTreeSet::new();

        for (joint_id, joint) in joint_entries {
            match joint {
                Joint3D::Fixed(fj) => {
                    if let Some(ev) =
                        self.solve_fixed_joint_3d(joint_id, fj, dt, &mut touched_indices)
                    {
                        broken.push(ev);
                    }
                }
                Joint3D::Hinge(hj) => {
                    if let Some(ev) =
                        self.solve_hinge_joint_3d(joint_id, hj, dt, &mut touched_indices)
                    {
                        broken.push(ev);
                    }
                }
                Joint3D::Spring(sj) => {
                    if let Some(ev) =
                        self.solve_spring_joint_3d(joint_id, sj, dt, &mut touched_indices)
                    {
                        broken.push(ev);
                    }
                }
                Joint3D::Character(cj) => {
                    if let Some(ev) =
                        self.solve_character_joint_3d(joint_id, cj, dt, &mut touched_indices)
                    {
                        broken.push(ev);
                    }
                }
                Joint3D::Configurable(cfg) => {
                    if let Some(ev) =
                        self.solve_configurable_joint_3d(joint_id, &cfg, dt, &mut touched_indices)
                    {
                        broken.push(ev);
                    }
                }
            }
        }

        for idx in touched_indices {
            if idx < self.bodies.len() {
                let body = &self.bodies[idx];
                if body.simulated && body.role == BodyRole::Dynamic {
                    let handles = body.handles;
                    self.backend.set_pose(handles, body.position, body.rotation);
                    self.backend.set_velocity(handles, body.velocity, true);
                    self.backend
                        .set_angular_velocity(handles, body.angular_velocity, true);
                }
            }
        }

        for ev in broken {
            self.joints_3d.remove(&ev.joint_id);
            self.broken_joints_3d.push(ev);
        }
    }

    fn body_state_3d(&self, entity: EntityId) -> Option<BodyState3D> {
        let idx = *self.indices.get(&entity)?;
        let b = &self.bodies[idx];
        if !b.simulated {
            return None;
        }
        Some((
            idx,
            b.position,
            b.rotation,
            b.velocity,
            b.angular_velocity,
            b.inverse_mass(),
        ))
    }

    fn solve_fixed_joint_3d(
        &mut self,
        joint_id: JointId3D,
        fj: FixedJoint3D,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        let (idx_a, pos_a, rot_a, vel_a, ang_a, inv_a) = self.body_state_3d(fj.body_a)?;
        let (opt_idx_b, pos_b, rot_b, vel_b, ang_b, inv_b) = match fj.body_b {
            Some(bid_b) => {
                let (ib, pb, rb, vb, ab, invb) = self.body_state_3d(bid_b)?;
                (Some(ib), pb, rb, vb, ab, invb)
            }
            None => (None, [0.0; 3], IDENTITY_ROTATION, [0.0; 3], [0.0; 3], 0.0),
        };
        let inv_sum = inv_a + inv_b;
        if inv_sum <= 0.0 {
            return None;
        }
        let world_a = add(pos_a, rotate_vector(rot_a, fj.anchor_a));
        let world_b = match opt_idx_b {
            Some(_) => add(pos_b, rotate_vector(rot_b, fj.anchor_b)),
            None => fj.anchor_b,
        };
        let pos_delta = subtract(world_b, world_a);
        let pos_err = vector_length(pos_delta);
        let vel_diff = vector_length(subtract(vel_b, vel_a));
        let reaction_force = (pos_err / (inv_sum * dt * dt)) + (vel_diff / (inv_sum * dt));

        let rel_rot = multiply_rotation(inverse_rotation(rot_a), rot_b);
        let (rot_axis_local, rot_err) = rotation_axis_and_angle(rel_rot);
        let ang_diff = vector_length(subtract(ang_b, ang_a));
        let reaction_torque = (rot_err.abs() / (inv_sum * dt * dt)) + (ang_diff / (inv_sum * dt));

        if let Some(ev) = check_break_3d(
            joint_id,
            fj.body_a,
            reaction_force,
            reaction_torque,
            fj.break_force,
            fj.break_torque,
        ) {
            return Some(ev);
        }

        let w_a = inv_a / inv_sum;
        let w_b = inv_b / inv_sum;
        let avg_vel = scale3(add(scale3(vel_a, w_b), scale3(vel_b, w_a)), 1.0);
        let avg_ang = scale3(add(scale3(ang_a, w_b), scale3(ang_b, w_a)), 1.0);
        let target_rot = slerp_rotation(rot_a, rot_b, w_a);
        let _ = rot_axis_local;

        if inv_a > 0.0 {
            let b_a = &mut self.bodies[idx_a];
            b_a.rotation = target_rot;
            let new_anchor_offset_a = rotate_vector(b_a.rotation, fj.anchor_a);
            let target_anchor_world = add(world_a, scale3(pos_delta, w_a));
            b_a.position = subtract(target_anchor_world, new_anchor_offset_a);
            b_a.velocity = if inv_b == 0.0 { vel_b } else { avg_vel };
            b_a.angular_velocity = if inv_b == 0.0 { ang_b } else { avg_ang };
            b_a.sleeping = false;
            b_a.sleep_timer = 0.0;
            touched.insert(idx_a);
        }
        if let Some(idx_b) = opt_idx_b {
            if inv_b > 0.0 {
                let b_b = &mut self.bodies[idx_b];
                b_b.rotation = target_rot;
                let new_anchor_offset_b = rotate_vector(b_b.rotation, fj.anchor_b);
                let target_anchor_world = subtract(world_b, scale3(pos_delta, w_b));
                b_b.position = subtract(target_anchor_world, new_anchor_offset_b);
                b_b.velocity = if inv_a == 0.0 { vel_a } else { avg_vel };
                b_b.angular_velocity = if inv_a == 0.0 { ang_a } else { avg_ang };
                b_b.sleeping = false;
                b_b.sleep_timer = 0.0;
                touched.insert(idx_b);
            }
        }
        None
    }

    fn solve_hinge_joint_3d(
        &mut self,
        joint_id: JointId3D,
        hj: HingeJoint3D,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        let (idx_a, pos_a, rot_a, vel_a, ang_a, inv_a) = self.body_state_3d(hj.body_a)?;
        let (opt_idx_b, pos_b, rot_b, vel_b, ang_b, inv_b) = match hj.body_b {
            Some(bid_b) => {
                let (ib, pb, rb, vb, ab, invb) = self.body_state_3d(bid_b)?;
                (Some(ib), pb, rb, vb, ab, invb)
            }
            None => (None, [0.0; 3], IDENTITY_ROTATION, [0.0; 3], [0.0; 3], 0.0),
        };
        let inv_sum = inv_a + inv_b;
        if inv_sum <= 0.0 {
            return None;
        }
        let axis_local = normalize(hj.axis).unwrap_or([1.0, 0.0, 0.0]);
        let world_a = add(pos_a, rotate_vector(rot_a, hj.anchor_a));
        let world_b = match opt_idx_b {
            Some(_) => add(pos_b, rotate_vector(rot_b, hj.anchor_b)),
            None => hj.anchor_b,
        };
        let pos_delta = subtract(world_b, world_a);
        let pos_err = vector_length(pos_delta);
        let vel_diff = vector_length(subtract(vel_b, vel_a));
        let reaction_force = (pos_err / (inv_sum * dt * dt)) + (vel_diff / (inv_sum * dt));

        let ref_rot = if opt_idx_b.is_some() && inv_b > 0.0 {
            rot_a
        } else {
            IDENTITY_ROTATION
        };
        let mov_rot = if opt_idx_b.is_some() && inv_b > 0.0 {
            rot_b
        } else {
            rot_a
        };
        let axis_world = rotate_vector(ref_rot, axis_local);
        let rel_rot = multiply_rotation(inverse_rotation(ref_rot), mov_rot);
        let (twist_angle, swing_err) = decompose_twist_swing_angle(rel_rot, axis_local);

        let mov_ang = if opt_idx_b.is_some() && inv_b > 0.0 {
            ang_b
        } else {
            ang_a
        };
        let ref_ang = if opt_idx_b.is_some() && inv_b > 0.0 {
            ang_a
        } else {
            [0.0; 3]
        };
        let rel_ang_vel = dot(subtract(mov_ang, ref_ang), axis_world);
        let mut active_angle = twist_angle;
        let mut active_omega = rel_ang_vel;
        let mut applied_torque = swing_err / (inv_sum * dt * dt);

        let mov_inv = if opt_idx_b.is_some() && inv_b > 0.0 {
            inv_b
        } else {
            inv_a
        };

        if hj.use_spring {
            let spring_torque = hj.spring.spring * (hj.spring.target_position - active_angle)
                - hj.spring.damper * active_omega;
            applied_torque = applied_torque.max(spring_torque.abs());
            active_omega += spring_torque * mov_inv * dt;
            active_angle += spring_torque * mov_inv * dt * dt;
        }

        if hj.use_motor {
            let diff = hj.motor.target_velocity - active_omega;
            let should_drive = if hj.motor.free_spin {
                if hj.motor.target_velocity >= 0.0 {
                    diff > 0.0
                } else {
                    diff < 0.0
                }
            } else {
                true
            };
            if should_drive {
                let max_dw = hj.motor.force.max(0.0) * mov_inv * dt;
                let dw = diff.clamp(-max_dw, max_dw);
                let motor_torque = (dw / (mov_inv * dt)).abs();
                applied_torque = applied_torque.max(motor_torque);
                active_omega += dw;
                active_angle += active_omega * dt;
            }
        }

        if hj.use_limits {
            let low = hj.limits.min.min(hj.limits.max);
            let high = hj.limits.min.max(hj.limits.max);
            if active_angle < low {
                let limit_err = low - active_angle;
                applied_torque = applied_torque.max(limit_err / (mov_inv * dt * dt));
                active_angle = low;
                if active_omega < 0.0 {
                    active_omega = 0.0;
                }
            } else if active_angle > high {
                let limit_err = active_angle - high;
                applied_torque = applied_torque.max(limit_err / (mov_inv * dt * dt));
                active_angle = high;
                if active_omega > 0.0 {
                    active_omega = 0.0;
                }
            }
        }

        if let Some(ev) = check_break_3d(
            joint_id,
            hj.body_a,
            reaction_force,
            applied_torque,
            hj.break_force,
            hj.break_torque,
        ) {
            return Some(ev);
        }

        let desired_rel_rot = quat_from_axis_angle(axis_local, active_angle);
        let desired_mov_rot = normalize_rotation(multiply_rotation(ref_rot, desired_rel_rot));
        let desired_mov_ang = add(ref_ang, scale3(axis_world, active_omega));

        if let Some(idx_b) = opt_idx_b {
            if inv_b > 0.0 {
                let w_a = inv_a / inv_sum;
                let w_b = inv_b / inv_sum;
                let target_anchor_world = subtract(world_b, scale3(pos_delta, w_b));
                if inv_a > 0.0 {
                    let b_a = &mut self.bodies[idx_a];
                    let anchor_a_world = rotate_vector(b_a.rotation, hj.anchor_a);
                    b_a.position = subtract(add(world_a, scale3(pos_delta, w_a)), anchor_a_world);
                    b_a.sleeping = false;
                    b_a.sleep_timer = 0.0;
                    touched.insert(idx_a);
                }
                let b_b = &mut self.bodies[idx_b];
                b_b.rotation = desired_mov_rot;
                let new_anchor_b_world = rotate_vector(b_b.rotation, hj.anchor_b);
                b_b.position = subtract(target_anchor_world, new_anchor_b_world);
                b_b.angular_velocity = desired_mov_ang;
                let arm_b = new_anchor_b_world;
                b_b.velocity = add(vel_a, cross3(b_b.angular_velocity, scale3(arm_b, -1.0)));
                b_b.sleeping = false;
                b_b.sleep_timer = 0.0;
                touched.insert(idx_b);
                return None;
            }
        }

        if inv_a > 0.0 {
            let b_a = &mut self.bodies[idx_a];
            b_a.rotation = desired_mov_rot;
            let new_anchor_a_world = rotate_vector(b_a.rotation, hj.anchor_a);
            b_a.position = subtract(world_b, new_anchor_a_world);
            b_a.angular_velocity = desired_mov_ang;
            b_a.velocity = cross3(b_a.angular_velocity, scale3(new_anchor_a_world, -1.0));
            b_a.sleeping = false;
            b_a.sleep_timer = 0.0;
            touched.insert(idx_a);
        }
        None
    }

    fn solve_spring_joint_3d(
        &mut self,
        joint_id: JointId3D,
        sj: SpringJoint3D,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        let (idx_a, pos_a, rot_a, vel_a, _ang_a, inv_a) = self.body_state_3d(sj.body_a)?;
        let (opt_idx_b, pos_b, rot_b, vel_b, _ang_b, inv_b) = match sj.body_b {
            Some(bid_b) => {
                let (ib, pb, rb, vb, ab, invb) = self.body_state_3d(bid_b)?;
                (Some(ib), pb, rb, vb, ab, invb)
            }
            None => (None, [0.0; 3], IDENTITY_ROTATION, [0.0; 3], [0.0; 3], 0.0),
        };
        let inv_sum = inv_a + inv_b;
        if inv_sum <= 0.0 {
            return None;
        }
        let world_a = add(pos_a, rotate_vector(rot_a, sj.anchor_a));
        let world_b = match opt_idx_b {
            Some(_) => add(pos_b, rotate_vector(rot_b, sj.anchor_b)),
            None => sj.anchor_b,
        };
        let delta = subtract(world_b, world_a);
        let dist = vector_length(delta);
        let dir = normalize(delta).unwrap_or([1.0, 0.0, 0.0]);
        let min_d = sj.min_distance.max(0.0);
        let max_d = sj.max_distance.max(min_d);
        let tol = sj.tolerance.max(0.0);

        let stretch = if dist > max_d + tol {
            dist - max_d
        } else if dist < (min_d - tol).max(0.0) {
            dist - min_d
        } else {
            0.0
        };

        let rel_vel_along = dot(subtract(vel_b, vel_a), dir);
        let force_mag = sj.spring.max(0.0) * stretch + sj.damper.max(0.0) * rel_vel_along;

        if let Some(ev) = check_break_3d(
            joint_id,
            sj.body_a,
            force_mag.abs(),
            0.0,
            sj.break_force,
            sj.break_torque,
        ) {
            return Some(ev);
        }

        if force_mag.abs() <= 1e-12 {
            return None;
        }

        if inv_a > 0.0 {
            let b_a = &mut self.bodies[idx_a];
            let dv_a = force_mag * inv_a * dt;
            b_a.velocity = add(b_a.velocity, scale3(dir, dv_a));
            b_a.position = add(b_a.position, scale3(dir, dv_a * dt));
            b_a.sleeping = false;
            b_a.sleep_timer = 0.0;
            touched.insert(idx_a);
        }
        if let Some(idx_b) = opt_idx_b {
            if inv_b > 0.0 {
                let b_b = &mut self.bodies[idx_b];
                let dv_b = -force_mag * inv_b * dt;
                b_b.velocity = add(b_b.velocity, scale3(dir, dv_b));
                b_b.position = add(b_b.position, scale3(dir, dv_b * dt));
                b_b.sleeping = false;
                b_b.sleep_timer = 0.0;
                touched.insert(idx_b);
            }
        }
        None
    }

    fn solve_character_joint_3d(
        &mut self,
        joint_id: JointId3D,
        cj: CharacterJoint3D,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        let (idx_a, pos_a, rot_a, vel_a, _ang_a, inv_a) = self.body_state_3d(cj.body_a)?;
        let (opt_idx_b, pos_b, rot_b, vel_b, _ang_b, inv_b) = match cj.body_b {
            Some(bid_b) => {
                let (ib, pb, rb, vb, ab, invb) = self.body_state_3d(bid_b)?;
                (Some(ib), pb, rb, vb, ab, invb)
            }
            None => (None, [0.0; 3], IDENTITY_ROTATION, [0.0; 3], [0.0; 3], 0.0),
        };
        let inv_sum = inv_a + inv_b;
        if inv_sum <= 0.0 {
            return None;
        }
        let world_a = add(pos_a, rotate_vector(rot_a, cj.anchor_a));
        let world_b = match opt_idx_b {
            Some(_) => add(pos_b, rotate_vector(rot_b, cj.anchor_b)),
            None => cj.anchor_b,
        };
        let pos_delta = subtract(world_b, world_a);
        let pos_err = vector_length(pos_delta);
        let vel_diff = vector_length(subtract(vel_b, vel_a));
        let reaction_force = (pos_err / (inv_sum * dt * dt)) + (vel_diff / (inv_sum * dt));

        let ref_rot = if opt_idx_b.is_some() && inv_b > 0.0 {
            rot_a
        } else {
            IDENTITY_ROTATION
        };
        let mov_rot = if opt_idx_b.is_some() && inv_b > 0.0 {
            rot_b
        } else {
            rot_a
        };
        let rel_rot = multiply_rotation(inverse_rotation(ref_rot), mov_rot);
        let (clamped_rel, clamped_angle_err) = clamp_character_rotation(
            rel_rot,
            cj.axis,
            cj.swing_axis,
            (cj.twist_limit_low, cj.twist_limit_high),
            (cj.swing1_limit, cj.swing2_limit),
        );
        let reaction_torque = clamped_angle_err / (inv_sum * dt * dt);

        if let Some(ev) = check_break_3d(
            joint_id,
            cj.body_a,
            reaction_force,
            reaction_torque,
            cj.break_force,
            cj.break_torque,
        ) {
            return Some(ev);
        }

        let desired_mov_rot = normalize_rotation(multiply_rotation(ref_rot, clamped_rel));

        if let Some(idx_b) = opt_idx_b {
            if inv_b > 0.0 {
                let w_a = inv_a / inv_sum;
                let w_b = inv_b / inv_sum;
                let target_anchor_world = subtract(world_b, scale3(pos_delta, w_b));
                if inv_a > 0.0 {
                    let b_a = &mut self.bodies[idx_a];
                    let anchor_a_world = rotate_vector(b_a.rotation, cj.anchor_a);
                    b_a.position = subtract(add(world_a, scale3(pos_delta, w_a)), anchor_a_world);
                    b_a.sleeping = false;
                    b_a.sleep_timer = 0.0;
                    touched.insert(idx_a);
                }
                let b_b = &mut self.bodies[idx_b];
                b_b.rotation = desired_mov_rot;
                let new_anchor_b_world = rotate_vector(b_b.rotation, cj.anchor_b);
                b_b.position = subtract(target_anchor_world, new_anchor_b_world);
                if clamped_angle_err > 1e-6 {
                    b_b.angular_velocity = [0.0; 3];
                }
                b_b.velocity = vel_a;
                b_b.sleeping = false;
                b_b.sleep_timer = 0.0;
                touched.insert(idx_b);
                return None;
            }
        }

        if inv_a > 0.0 {
            let b_a = &mut self.bodies[idx_a];
            b_a.rotation = desired_mov_rot;
            let new_anchor_a_world = rotate_vector(b_a.rotation, cj.anchor_a);
            b_a.position = subtract(world_b, new_anchor_a_world);
            if clamped_angle_err > 1e-6 {
                b_a.angular_velocity = [0.0; 3];
            }
            b_a.velocity = [0.0; 3];
            b_a.sleeping = false;
            b_a.sleep_timer = 0.0;
            touched.insert(idx_a);
        }
        None
    }

    fn solve_configurable_joint_3d(
        &mut self,
        joint_id: JointId3D,
        cfg: &ConfigurableJoint3D,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        let (idx_a, pos_a, rot_a, vel_a, ang_a, inv_a) = self.body_state_3d(cfg.body_a)?;
        let (opt_idx_b, pos_b, rot_b, vel_b, ang_b, inv_b) = match cfg.body_b {
            Some(bid_b) => {
                let (ib, pb, rb, vb, ab, invb) = self.body_state_3d(bid_b)?;
                (Some(ib), pb, rb, vb, ab, invb)
            }
            None => (None, [0.0; 3], IDENTITY_ROTATION, [0.0; 3], [0.0; 3], 0.0),
        };
        let inv_sum = inv_a + inv_b;
        if inv_sum <= 0.0 {
            return None;
        }
        let mov_is_b = opt_idx_b.is_some() && inv_b > 0.0;
        let ref_rot = if mov_is_b { rot_a } else { IDENTITY_ROTATION };
        let mov_rot = if mov_is_b { rot_b } else { rot_a };
        let ref_vel = if mov_is_b { vel_a } else { [0.0; 3] };
        let mov_vel = if mov_is_b { vel_b } else { vel_a };
        let ref_ang = if mov_is_b { ang_a } else { [0.0; 3] };
        let mov_ang = if mov_is_b { ang_b } else { ang_a };
        let mov_inv = if mov_is_b { inv_b } else { inv_a };

        let world_a = add(pos_a, rotate_vector(rot_a, cfg.anchor_a));
        let world_b = match opt_idx_b {
            Some(_) => add(pos_b, rotate_vector(rot_b, cfg.anchor_b)),
            None => cfg.anchor_b,
        };
        let ref_anchor = if mov_is_b { world_a } else { world_b };
        let mov_anchor = if mov_is_b { world_b } else { world_a };

        let (basis_x_local, basis_y_local, basis_z_local) =
            orthonormal_frame(cfg.axis, cfg.secondary_axis);
        let basis_x = rotate_vector(ref_rot, basis_x_local);
        let basis_y = rotate_vector(ref_rot, basis_y_local);
        let basis_z = rotate_vector(ref_rot, basis_z_local);
        let bases = [basis_x, basis_y, basis_z];

        let rel_offset_world = subtract(mov_anchor, ref_anchor);
        let rel_vel_world = subtract(mov_vel, ref_vel);

        let mut coords = [
            dot(rel_offset_world, basis_x),
            dot(rel_offset_world, basis_y),
            dot(rel_offset_world, basis_z),
        ];
        let mut v_coords = [
            dot(rel_vel_world, basis_x),
            dot(rel_vel_world, basis_y),
            dot(rel_vel_world, basis_z),
        ];

        let motions = [cfg.x_motion, cfg.y_motion, cfg.z_motion];
        let drives = [cfg.x_drive, cfg.y_drive, cfg.z_drive];
        let lin_lim = cfg.linear_limit.max(0.0);
        let mut total_force_sq = 0.0f64;

        for axis_i in 0..3 {
            let drive = drives[axis_i];
            if motions[axis_i] != ConfigurableJointMotion::Locked
                && (drive.position_spring > 0.0 || drive.position_damper > 0.0)
            {
                let pos_err = cfg.target_position[axis_i] - coords[axis_i];
                let vel_err = cfg.target_velocity[axis_i] - v_coords[axis_i];
                let mut f = drive.position_spring * pos_err + drive.position_damper * vel_err;
                if drive.maximum_force.is_finite() {
                    let max_f = drive.maximum_force.max(0.0);
                    f = f.clamp(-max_f, max_f);
                }
                total_force_sq += f * f;
                v_coords[axis_i] += f * mov_inv * dt;
                coords[axis_i] += v_coords[axis_i] * dt;
            }

            match motions[axis_i] {
                ConfigurableJointMotion::Locked => {
                    let err = coords[axis_i].abs();
                    let f = (err / (mov_inv * dt * dt)) + (v_coords[axis_i].abs() / (mov_inv * dt));
                    total_force_sq += f * f;
                    coords[axis_i] = 0.0;
                    v_coords[axis_i] = 0.0;
                }
                ConfigurableJointMotion::Limited => {
                    if coords[axis_i] < -lin_lim {
                        let err = -lin_lim - coords[axis_i];
                        let f = err / (mov_inv * dt * dt);
                        total_force_sq += f * f;
                        coords[axis_i] = -lin_lim;
                        if v_coords[axis_i] < 0.0 {
                            v_coords[axis_i] = 0.0;
                        }
                    } else if coords[axis_i] > lin_lim {
                        let err = coords[axis_i] - lin_lim;
                        let f = err / (mov_inv * dt * dt);
                        total_force_sq += f * f;
                        coords[axis_i] = lin_lim;
                        if v_coords[axis_i] > 0.0 {
                            v_coords[axis_i] = 0.0;
                        }
                    }
                }
                ConfigurableJointMotion::Free => {}
            }
        }

        let rel_rot = multiply_rotation(inverse_rotation(ref_rot), mov_rot);
        let (clamped_rel_rot, ang_err, zero_ang) = clamp_configurable_rotation(
            rel_rot,
            (basis_x_local, basis_y_local, basis_z_local),
            (
                cfg.angular_x_motion,
                cfg.angular_y_motion,
                cfg.angular_z_motion,
            ),
            (
                cfg.angular_x_limit_low,
                cfg.angular_x_limit_high,
                cfg.angular_y_limit,
                cfg.angular_z_limit,
            ),
        );
        let reaction_torque = ang_err / (mov_inv * dt * dt);
        let reaction_force = total_force_sq.sqrt();

        if let Some(ev) = check_break_3d(
            joint_id,
            cfg.body_a,
            reaction_force,
            reaction_torque,
            cfg.break_force,
            cfg.break_torque,
        ) {
            return Some(ev);
        }

        let new_offset_world = add(
            add(scale3(bases[0], coords[0]), scale3(bases[1], coords[1])),
            scale3(bases[2], coords[2]),
        );
        let new_rel_vel_world = add(
            add(scale3(bases[0], v_coords[0]), scale3(bases[1], v_coords[1])),
            scale3(bases[2], v_coords[2]),
        );
        let desired_mov_anchor = add(ref_anchor, new_offset_world);
        let desired_mov_vel = add(ref_vel, new_rel_vel_world);
        let desired_mov_rot = normalize_rotation(multiply_rotation(ref_rot, clamped_rel_rot));
        let desired_mov_ang = if zero_ang { ref_ang } else { mov_ang };
        let _ = JointDrive3D::default();

        if mov_is_b {
            if let Some(idx_b) = opt_idx_b {
                let b_b = &mut self.bodies[idx_b];
                b_b.rotation = desired_mov_rot;
                let anchor_b_world = rotate_vector(b_b.rotation, cfg.anchor_b);
                b_b.position = subtract(desired_mov_anchor, anchor_b_world);
                b_b.velocity = desired_mov_vel;
                b_b.angular_velocity = desired_mov_ang;
                b_b.sleeping = false;
                b_b.sleep_timer = 0.0;
                touched.insert(idx_b);
            }
        } else if inv_a > 0.0 {
            let b_a = &mut self.bodies[idx_a];
            b_a.rotation = desired_mov_rot;
            let anchor_a_world = rotate_vector(b_a.rotation, cfg.anchor_a);
            b_a.position = subtract(desired_mov_anchor, anchor_a_world);
            b_a.velocity = desired_mov_vel;
            b_a.angular_velocity = desired_mov_ang;
            b_a.sleeping = false;
            b_a.sleep_timer = 0.0;
            touched.insert(idx_a);
        }
        None
    }
}
