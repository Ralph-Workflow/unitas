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

type OptionalBodyState3D = (Option<usize>, [f64; 3], [f64; 4], [f64; 3], [f64; 3], f64);

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
            if let Some(ev) = self.solve_single_joint_3d(joint_id, joint, dt, &mut touched_indices)
            {
                broken.push(ev);
            }
        }

        self.sync_touched_joint_bodies_3d(touched_indices);
        for ev in broken {
            self.joints_3d.remove(&ev.joint_id);
            self.broken_joints_3d.push(ev);
        }
    }

    fn solve_single_joint_3d(
        &mut self,
        joint_id: JointId3D,
        joint: Joint3D,
        dt: f64,
        touched_indices: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        match joint {
            Joint3D::Fixed(fj) => self.solve_fixed_joint_3d(joint_id, fj, dt, touched_indices),
            Joint3D::Hinge(hj) => self.solve_hinge_joint_3d(joint_id, hj, dt, touched_indices),
            Joint3D::Spring(sj) => self.solve_spring_joint_3d(joint_id, sj, dt, touched_indices),
            Joint3D::Character(cj) => {
                self.solve_character_joint_3d(joint_id, cj, dt, touched_indices)
            }
            Joint3D::Configurable(cfg) => {
                self.solve_configurable_joint_3d(joint_id, &cfg, dt, touched_indices)
            }
        }
    }

    fn sync_touched_joint_bodies_3d(&mut self, touched_indices: BTreeSet<usize>) {
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

    fn optional_body_state_3d(&self, entity: Option<EntityId>) -> Option<OptionalBodyState3D> {
        match entity {
            Some(bid) => {
                let (ib, pb, rb, vb, ab, invb) = self.body_state_3d(bid)?;
                Some((Some(ib), pb, rb, vb, ab, invb))
            }
            None => Some((None, [0.0; 3], IDENTITY_ROTATION, [0.0; 3], [0.0; 3], 0.0)),
        }
    }

    fn solve_fixed_joint_3d(
        &mut self,
        joint_id: JointId3D,
        fj: FixedJoint3D,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        let (idx_a, pos_a, rot_a, vel_a, ang_a, inv_a) = self.body_state_3d(fj.body_a)?;
        let (opt_idx_b, pos_b, rot_b, vel_b, ang_b, inv_b) =
            self.optional_body_state_3d(fj.body_b)?;
        let inv_sum = inv_a + inv_b;
        if inv_sum <= 0.0 {
            return None;
        }
        let world_a = add(pos_a, rotate_vector(rot_a, fj.anchor_a));
        let world_b = opt_idx_b.map_or(fj.anchor_b, |_| {
            add(pos_b, rotate_vector(rot_b, fj.anchor_b))
        });
        let pos_delta = subtract(world_b, world_a);
        let reaction_force = (vector_length(pos_delta) / (inv_sum * dt * dt))
            + (vector_length(subtract(vel_b, vel_a)) / (inv_sum * dt));
        let (_, rot_err) =
            rotation_axis_and_angle(multiply_rotation(inverse_rotation(rot_a), rot_b));
        let reaction_torque = (rot_err.abs() / (inv_sum * dt * dt))
            + (vector_length(subtract(ang_b, ang_a)) / (inv_sum * dt));
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
        self.apply_fixed_joint_poses(
            &fj,
            (idx_a, opt_idx_b, inv_a, inv_b),
            (world_a, world_b, pos_delta, slerp_rotation(rot_a, rot_b, inv_a / inv_sum)),
            (vel_a, ang_a, vel_b, ang_b),
            touched,
        );
        None
    }

    fn apply_fixed_joint_poses(
        &mut self,
        fj: &FixedJoint3D,
        indices: (usize, Option<usize>, f64, f64),
        poses: ([f64; 3], [f64; 3], [f64; 3], [f64; 4]),
        velocities: ([f64; 3], [f64; 3], [f64; 3], [f64; 3]),
        touched: &mut BTreeSet<usize>,
    ) {
        let (idx_a, opt_idx_b, inv_a, inv_b) = indices;
        let (world_a, world_b, pos_delta, target_rot) = poses;
        let (vel_a, ang_a, vel_b, ang_b) = velocities;
        let inv_sum = inv_a + inv_b;
        let (w_a, w_b) = (inv_a / inv_sum, inv_b / inv_sum);
        let avg_vel = add(scale3(vel_a, w_b), scale3(vel_b, w_a));
        let avg_ang = add(scale3(ang_a, w_b), scale3(ang_b, w_a));
        if inv_a > 0.0 {
            let next_motion = if inv_b == 0.0 {
                (vel_b, ang_b)
            } else {
                (avg_vel, avg_ang)
            };
            self.apply_joint_body_pose(
                idx_a,
                (fj.anchor_a, add(world_a, scale3(pos_delta, w_a)), target_rot),
                next_motion,
                touched,
            );
        }
        if let Some(idx_b) = opt_idx_b.filter(|_| inv_b > 0.0) {
            let next_motion = if inv_a == 0.0 {
                (vel_a, ang_a)
            } else {
                (avg_vel, avg_ang)
            };
            self.apply_joint_body_pose(
                idx_b,
                (fj.anchor_b, subtract(world_b, scale3(pos_delta, w_b)), target_rot),
                next_motion,
                touched,
            );
        }
    }

    fn apply_joint_body_pose(
        &mut self,
        idx: usize,
        pose: ([f64; 3], [f64; 3], [f64; 4]),
        motion: ([f64; 3], [f64; 3]),
        touched: &mut BTreeSet<usize>,
    ) {
        let (local_anchor, target_anchor_world, target_rot) = pose;
        let (velocity, angular_velocity) = motion;
        let body = &mut self.bodies[idx];
        body.rotation = target_rot;
        let new_anchor_offset = rotate_vector(body.rotation, local_anchor);
        body.position = subtract(target_anchor_world, new_anchor_offset);
        body.velocity = velocity;
        body.angular_velocity = angular_velocity;
        body.sleeping = false;
        body.sleep_timer = 0.0;
        touched.insert(idx);
    }

    fn solve_hinge_joint_3d(
        &mut self,
        joint_id: JointId3D,
        hj: HingeJoint3D,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        let (idx_a, pos_a, rot_a, vel_a, ang_a, inv_a) = self.body_state_3d(hj.body_a)?;
        let (opt_idx_b, pos_b, rot_b, vel_b, ang_b, inv_b) =
            self.optional_body_state_3d(hj.body_b)?;
        let inv_sum = inv_a + inv_b;
        if inv_sum <= 0.0 {
            return None;
        }
        let axis_local = normalize(hj.axis).unwrap_or([1.0, 0.0, 0.0]);
        let world_a = add(pos_a, rotate_vector(rot_a, hj.anchor_a));
        let world_b = opt_idx_b.map_or(hj.anchor_b, |_| {
            add(pos_b, rotate_vector(rot_b, hj.anchor_b))
        });
        let pos_delta = subtract(world_b, world_a);
        let reaction_force = (vector_length(pos_delta) / (inv_sum * dt * dt))
            + (vector_length(subtract(vel_b, vel_a)) / (inv_sum * dt));
        let (ref_rot, mov_rot, ref_ang, mov_ang, mov_inv) = if opt_idx_b.is_some() && inv_b > 0.0 {
            (rot_a, rot_b, ang_a, ang_b, inv_b)
        } else {
            (IDENTITY_ROTATION, rot_a, [0.0; 3], ang_a, inv_a)
        };
        let axis_world = rotate_vector(ref_rot, axis_local);
        let rel_rot = multiply_rotation(inverse_rotation(ref_rot), mov_rot);
        let (twist_angle, swing_err) = decompose_twist_swing_angle(rel_rot, axis_local);
        let rel_ang_vel = dot(subtract(mov_ang, ref_ang), axis_world);
        let (active_angle, active_omega, applied_torque) = solve_hinge_axis_drive(
            &hj,
            (twist_angle, rel_ang_vel, swing_err / (inv_sum * dt * dt)),
            mov_inv,
            dt,
        );
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
        let desired_mov_rot = normalize_rotation(multiply_rotation(
            ref_rot,
            quat_from_axis_angle(axis_local, active_angle),
        ));
        let desired_mov_ang = add(ref_ang, scale3(axis_world, active_omega));
        self.apply_hinge_pose_3d(
            &hj,
            (idx_a, opt_idx_b, inv_a, inv_b),
            (world_a, world_b, pos_delta, vel_a),
            (desired_mov_rot, desired_mov_ang),
            touched,
        );
        None
    }

    fn apply_hinge_pose_3d(
        &mut self,
        hj: &HingeJoint3D,
        indices: (usize, Option<usize>, f64, f64),
        kinematics: ([f64; 3], [f64; 3], [f64; 3], [f64; 3]),
        desired: ([f64; 4], [f64; 3]),
        touched: &mut BTreeSet<usize>,
    ) {
        let (idx_a, opt_idx_b, inv_a, inv_b) = indices;
        let (world_a, world_b, pos_delta, vel_a) = kinematics;
        let (desired_mov_rot, desired_mov_ang) = desired;
        let inv_sum = inv_a + inv_b;

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
                let new_anchor_b_world = rotate_vector(desired_mov_rot, hj.anchor_b);
                let vel_b = add(
                    vel_a,
                    cross3(desired_mov_ang, scale3(new_anchor_b_world, -1.0)),
                );
                self.apply_joint_body_pose(
                    idx_b,
                    (hj.anchor_b, target_anchor_world, desired_mov_rot),
                    (vel_b, desired_mov_ang),
                    touched,
                );
                return;
            }
        }

        if inv_a > 0.0 {
            let new_anchor_a_world = rotate_vector(desired_mov_rot, hj.anchor_a);
            let vel_a_next = cross3(desired_mov_ang, scale3(new_anchor_a_world, -1.0));
            self.apply_joint_body_pose(
                idx_a,
                (hj.anchor_a, world_b, desired_mov_rot),
                (vel_a_next, desired_mov_ang),
                touched,
            );
        }
    }

    fn solve_spring_joint_3d(
        &mut self,
        joint_id: JointId3D,
        sj: SpringJoint3D,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        let (idx_a, pos_a, rot_a, vel_a, _ang_a, inv_a) = self.body_state_3d(sj.body_a)?;
        let (opt_idx_b, pos_b, rot_b, vel_b, _ang_b, inv_b) =
            self.optional_body_state_3d(sj.body_b)?;
        if inv_a + inv_b <= 0.0 {
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
        let stretch = spring_joint_stretch(&sj, dist);
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
            self.apply_spring_impulse_3d(idx_a, dir, force_mag * inv_a * dt, dt, touched);
        }
        if let Some(idx_b) = opt_idx_b {
            if inv_b > 0.0 {
                self.apply_spring_impulse_3d(idx_b, dir, -force_mag * inv_b * dt, dt, touched);
            }
        }
        None
    }

    fn apply_spring_impulse_3d(
        &mut self,
        idx: usize,
        dir: [f64; 3],
        dv: f64,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) {
        let body = &mut self.bodies[idx];
        body.velocity = add(body.velocity, scale3(dir, dv));
        body.position = add(body.position, scale3(dir, dv * dt));
        body.sleeping = false;
        body.sleep_timer = 0.0;
        touched.insert(idx);
    }

    fn solve_character_joint_3d(
        &mut self,
        joint_id: JointId3D,
        cj: CharacterJoint3D,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        let (idx_a, pos_a, rot_a, vel_a, _ang_a, inv_a) = self.body_state_3d(cj.body_a)?;
        let (opt_idx_b, pos_b, rot_b, vel_b, _ang_b, inv_b) =
            self.optional_body_state_3d(cj.body_b)?;
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
        let reaction_force = (vector_length(pos_delta) / (inv_sum * dt * dt))
            + (vector_length(subtract(vel_b, vel_a)) / (inv_sum * dt));

        let mov_is_b = opt_idx_b.is_some() && inv_b > 0.0;
        let ref_rot = if mov_is_b { rot_a } else { IDENTITY_ROTATION };
        let mov_rot = if mov_is_b { rot_b } else { rot_a };
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
        self.apply_character_joint_pose_3d(
            &cj,
            (idx_a, opt_idx_b, inv_a, inv_b),
            (world_a, world_b, pos_delta, vel_a),
            (desired_mov_rot, clamped_angle_err > 1e-6),
            touched,
        );
        None
    }

    fn apply_character_joint_pose_3d(
        &mut self,
        cj: &CharacterJoint3D,
        indices: (usize, Option<usize>, f64, f64),
        kinematics: ([f64; 3], [f64; 3], [f64; 3], [f64; 3]),
        rot_state: ([f64; 4], bool),
        touched: &mut BTreeSet<usize>,
    ) {
        let (idx_a, opt_idx_b, inv_a, inv_b) = indices;
        let (world_a, world_b, pos_delta, vel_a) = kinematics;
        let (desired_mov_rot, zero_ang_vel) = rot_state;
        let inv_sum = inv_a + inv_b;

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
                if zero_ang_vel {
                    b_b.angular_velocity = [0.0; 3];
                }
                b_b.velocity = vel_a;
                b_b.sleeping = false;
                b_b.sleep_timer = 0.0;
                touched.insert(idx_b);
                return;
            }
        }

        if inv_a > 0.0 {
            let b_a = &mut self.bodies[idx_a];
            b_a.rotation = desired_mov_rot;
            let new_anchor_a_world = rotate_vector(b_a.rotation, cj.anchor_a);
            b_a.position = subtract(world_b, new_anchor_a_world);
            if zero_ang_vel {
                b_a.angular_velocity = [0.0; 3];
            }
            b_a.velocity = [0.0; 3];
            b_a.sleeping = false;
            b_a.sleep_timer = 0.0;
            touched.insert(idx_a);
        }
    }

    fn solve_configurable_joint_3d(
        &mut self,
        joint_id: JointId3D,
        cfg: &ConfigurableJoint3D,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        let (idx_a, pos_a, rot_a, vel_a, ang_a, inv_a) = self.body_state_3d(cfg.body_a)?;
        let (opt_idx_b, pos_b, rot_b, vel_b, ang_b, inv_b) =
            self.optional_body_state_3d(cfg.body_b)?;
        if inv_a + inv_b <= 0.0 {
            return None;
        }
        let mov_is_b = opt_idx_b.is_some() && inv_b > 0.0;
        let (ref_rot, mov_rot, ref_vel, mov_vel, ref_ang, mov_ang, mov_inv) = if mov_is_b {
            (rot_a, rot_b, vel_a, vel_b, ang_a, ang_b, inv_b)
        } else {
            (IDENTITY_ROTATION, rot_a, [0.0; 3], vel_a, [0.0; 3], ang_a, inv_a)
        };
        let world_a = add(pos_a, rotate_vector(rot_a, cfg.anchor_a));
        let world_b = opt_idx_b.map_or(cfg.anchor_b, |_| {
            add(pos_b, rotate_vector(rot_b, cfg.anchor_b))
        });
        let (ref_anchor, mov_anchor) = if mov_is_b {
            (world_a, world_b)
        } else {
            (world_b, world_a)
        };
        let local_axes = orthonormal_frame(cfg.axis, cfg.secondary_axis);
        let bases = [
            rotate_vector(ref_rot, local_axes.0),
            rotate_vector(ref_rot, local_axes.1),
            rotate_vector(ref_rot, local_axes.2),
        ];
        let (new_offset_world, new_rel_vel_world, reaction_force) = solve_configurable_linear_axes(
            cfg,
            &bases,
            subtract(mov_anchor, ref_anchor),
            subtract(mov_vel, ref_vel),
            mov_inv,
            dt,
        );
        let rel_rot = multiply_rotation(inverse_rotation(ref_rot), mov_rot);
        let (clamped_rel_rot, ang_err, zero_ang) =
            clamp_configurable_joint_rot(cfg, rel_rot, local_axes);
        if let Some(ev) = check_break_3d(
            joint_id,
            cfg.body_a,
            reaction_force,
            ang_err / (mov_inv * dt * dt),
            cfg.break_force,
            cfg.break_torque,
        ) {
            return Some(ev);
        }
        let desired_pose = (
            if mov_is_b { cfg.anchor_b } else { cfg.anchor_a },
            add(ref_anchor, new_offset_world),
            normalize_rotation(multiply_rotation(ref_rot, clamped_rel_rot)),
        );
        let desired_motion = (
            add(ref_vel, new_rel_vel_world),
            if zero_ang { ref_ang } else { mov_ang },
        );
        let target_idx = if mov_is_b {
            opt_idx_b
        } else {
            (inv_a > 0.0).then_some(idx_a)
        };
        if let Some(idx) = target_idx {
            self.apply_joint_body_pose(idx, desired_pose, desired_motion, touched);
        }
        None
    }
}

fn clamp_configurable_joint_rot(
    cfg: &ConfigurableJoint3D,
    rel_rot: [f64; 4],
    local_axes: ([f64; 3], [f64; 3], [f64; 3]),
) -> ([f64; 4], f64, bool) {
    clamp_configurable_rotation(
        rel_rot,
        local_axes,
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
    )
}

fn solve_hinge_axis_drive(
    hj: &HingeJoint3D,
    initial: (f64, f64, f64),
    mov_inv: f64,
    dt: f64,
) -> (f64, f64, f64) {
    let (mut active_angle, mut active_omega, mut applied_torque) = initial;

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

    (active_angle, active_omega, applied_torque)
}

fn spring_joint_stretch(sj: &SpringJoint3D, dist: f64) -> f64 {
    let min_d = sj.min_distance.max(0.0);
    let max_d = sj.max_distance.max(min_d);
    let tol = sj.tolerance.max(0.0);
    if dist > max_d + tol {
        dist - max_d
    } else if dist < (min_d - tol).max(0.0) {
        dist - min_d
    } else {
        0.0
    }
}

fn solve_configurable_linear_axes(
    cfg: &ConfigurableJoint3D,
    bases: &[[f64; 3]; 3],
    rel_offset_world: [f64; 3],
    rel_vel_world: [f64; 3],
    mov_inv: f64,
    dt: f64,
) -> ([f64; 3], [f64; 3], f64) {
    let mut coords = [
        dot(rel_offset_world, bases[0]),
        dot(rel_offset_world, bases[1]),
        dot(rel_offset_world, bases[2]),
    ];
    let mut v_coords = [
        dot(rel_vel_world, bases[0]),
        dot(rel_vel_world, bases[1]),
        dot(rel_vel_world, bases[2]),
    ];

    let motions = [cfg.x_motion, cfg.y_motion, cfg.z_motion];
    let drives = [cfg.x_drive, cfg.y_drive, cfg.z_drive];
    let lin_lim = cfg.linear_limit.max(0.0);
    let mut total_force_sq = 0.0f64;

    for axis_i in 0..3 {
        total_force_sq += solve_configurable_single_linear_axis(
            motions[axis_i],
            drives[axis_i],
            (cfg.target_position[axis_i], cfg.target_velocity[axis_i]),
            lin_lim,
            (mov_inv, dt),
            &mut coords[axis_i],
            &mut v_coords[axis_i],
        );
    }

    let new_offset_world = add(
        add(scale3(bases[0], coords[0]), scale3(bases[1], coords[1])),
        scale3(bases[2], coords[2]),
    );
    let new_rel_vel_world = add(
        add(scale3(bases[0], v_coords[0]), scale3(bases[1], v_coords[1])),
        scale3(bases[2], v_coords[2]),
    );
    (new_offset_world, new_rel_vel_world, total_force_sq.sqrt())
}

fn solve_configurable_single_linear_axis(
    motion: ConfigurableJointMotion,
    drive: JointDrive3D,
    targets: (f64, f64),
    lin_lim: f64,
    step: (f64, f64),
    coord: &mut f64,
    v_coord: &mut f64,
) -> f64 {
    let (target_pos, target_vel) = targets;
    let (mov_inv, dt) = step;
    let mut force_sq = 0.0;
    if motion != ConfigurableJointMotion::Locked
        && (drive.position_spring > 0.0 || drive.position_damper > 0.0)
    {
        let pos_err = target_pos - *coord;
        let vel_err = target_vel - *v_coord;
        let mut f = drive.position_spring * pos_err + drive.position_damper * vel_err;
        if drive.maximum_force.is_finite() {
            let max_f = drive.maximum_force.max(0.0);
            f = f.clamp(-max_f, max_f);
        }
        force_sq += f * f;
        *v_coord += f * mov_inv * dt;
        *coord += *v_coord * dt;
    }

    match motion {
        ConfigurableJointMotion::Locked => {
            let err = coord.abs();
            let f = (err / (mov_inv * dt * dt)) + (v_coord.abs() / (mov_inv * dt));
            force_sq += f * f;
            *coord = 0.0;
            *v_coord = 0.0;
        }
        ConfigurableJointMotion::Limited => {
            if *coord < -lin_lim {
                let err = -lin_lim - *coord;
                let f = err / (mov_inv * dt * dt);
                force_sq += f * f;
                *coord = -lin_lim;
                if *v_coord < 0.0 {
                    *v_coord = 0.0;
                }
            } else if *coord > lin_lim {
                let err = *coord - lin_lim;
                let f = err / (mov_inv * dt * dt);
                force_sq += f * f;
                *coord = lin_lim;
                if *v_coord > 0.0 {
                    *v_coord = 0.0;
                }
            }
        }
        ConfigurableJointMotion::Free => {}
    }
    force_sq
}
