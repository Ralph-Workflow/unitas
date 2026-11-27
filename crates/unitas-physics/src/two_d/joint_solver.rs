use crate::two_d::body::{
    DistanceJoint2D, FixedJoint2D, FrictionJoint2D, HingeJoint2D, Joint2D, JointId2D,
    RelativeJoint2D, SliderJoint2D, SpringJoint2D, TargetJoint2D, WheelJoint2D,
};
use crate::two_d::collider::{dot2, length2, normalize2, rotate_vec2, sub2, transform_point2};

use super::World2D;

impl World2D {
    pub(super) fn solve_joints(&mut self, dt: f32) {
        if dt <= 0.0 || self.joints.is_empty() {
            return;
        }
        let joint_entries: Vec<(JointId2D, Joint2D)> =
            self.joints.iter().map(|(id, j)| (*id, *j)).collect();
        let mut broken = Vec::new();

        for (joint_id, joint) in joint_entries {
            let broke = match joint {
                Joint2D::Distance(dj) => solve_distance_joint(self, &dj, dt),
                Joint2D::Fixed(fj) => solve_fixed_joint(self, &fj, dt),
                Joint2D::Friction(frj) => solve_friction_joint(self, &frj, dt),
                Joint2D::Hinge(hj) => solve_hinge_joint(self, &hj, dt),
                Joint2D::Relative(rj) => solve_relative_joint(self, &rj, dt),
                Joint2D::Slider(slj) => solve_slider_joint(self, &slj, dt),
                Joint2D::Spring(sj) => solve_spring_joint(self, &sj, dt),
                Joint2D::Target(tj) => solve_target_joint(self, &tj, dt),
                Joint2D::Wheel(wj) => solve_wheel_joint(self, &wj, dt),
            };
            if broke {
                broken.push(joint_id);
            }
        }

        for id in broken {
            self.joints.remove(&id);
            self.broken_joints.push(id);
        }
    }
}

fn solve_distance_joint(world: &mut World2D, dj: &DistanceJoint2D, dt: f32) -> bool {
    let (pos_a, rot_a, _) = world.body_pose_and_inv(dj.body_a);
    let world_a = transform_point2(dj.anchor_a, pos_a, rot_a);
    let Some(bid_b) = dj.body_b else {
        return false;
    };
    let (pos_b, rot_b, inv_b) = world.body_pose_and_inv(bid_b);
    if inv_b <= 0.0 {
        return false;
    }
    let world_b = transform_point2(dj.anchor_b, pos_b, rot_b);
    let delta = sub2(world_b, world_a);
    let dist = length2(delta);
    let err = dist - dj.distance.max(0.0);
    if dj.max_distance_only && err <= 0.0 {
        return false;
    }
    let reaction_force = (err.abs() / (inv_b * dt * dt)).min(1e9);
    if dj.break_force.is_finite() && reaction_force > dj.break_force {
        return true;
    }
    let dir = normalize2(delta).unwrap_or([1.0, 0.0]);
    if let Some(b) = world.bodies.get_mut(&bid_b) {
        b.position[0] -= dir[0] * err;
        b.position[1] -= dir[1] * err;
        let radial_v = dot2(b.linear_velocity, dir);
        b.linear_velocity[0] -= dir[0] * radial_v;
        b.linear_velocity[1] -= dir[1] * radial_v;
    }
    false
}

fn solve_spring_joint(world: &mut World2D, sj: &SpringJoint2D, dt: f32) -> bool {
    let (pos_a, rot_a, _) = world.body_pose_and_inv(sj.body_a);
    let world_a = transform_point2(sj.anchor_a, pos_a, rot_a);
    let Some(bid_b) = sj.body_b else {
        return false;
    };
    let (pos_b, rot_b, inv_b) = world.body_pose_and_inv(bid_b);
    if inv_b <= 0.0 {
        return false;
    }
    let world_b = transform_point2(sj.anchor_b, pos_b, rot_b);
    let delta = sub2(world_b, world_a);
    let dist = length2(delta);
    let stretch = dist - sj.distance.max(0.0);
    let dir = normalize2(delta).unwrap_or([1.0, 0.0]);
    let k = (sj.frequency.max(0.5) * std::f32::consts::TAU).powi(2) / inv_b;
    let force_mag = -k * stretch;
    if sj.break_force.is_finite() && force_mag.abs() > sj.break_force {
        return true;
    }
    if let Some(b) = world.bodies.get_mut(&bid_b) {
        let dv = force_mag * inv_b * dt;
        b.linear_velocity[0] += dir[0] * dv;
        b.linear_velocity[1] += dir[1] * dv;
        let damp = (1.0 - sj.damping_ratio.clamp(0.0, 1.0) * dt).max(0.0);
        b.linear_velocity[0] *= damp;
        b.linear_velocity[1] *= damp;
        b.position[0] += b.linear_velocity[0] * dt;
        b.position[1] += b.linear_velocity[1] * dt;
    }
    false
}

fn solve_target_joint(world: &mut World2D, tj: &TargetJoint2D, dt: f32) -> bool {
    let (pos, rot, inv_m) = world.body_pose_and_inv(tj.body);
    if inv_m <= 0.0 {
        return false;
    }
    let world_anchor = transform_point2(tj.anchor, pos, rot);
    let to_target = sub2(tj.target, world_anchor);
    let dist = length2(to_target);
    if dist <= 1e-6 {
        return false;
    }
    let dir = [to_target[0] / dist, to_target[1] / dist];
    let k = (tj.frequency.max(0.5) * std::f32::consts::TAU).powi(2) / inv_m;
    let force_mag = (k * dist).min(tj.max_force.max(0.0));
    if tj.break_force.is_finite() && force_mag > tj.break_force {
        return true;
    }
    if let Some(b) = world.bodies.get_mut(&tj.body) {
        let dv = force_mag * inv_m * dt;
        b.linear_velocity[0] += dir[0] * dv;
        b.linear_velocity[1] += dir[1] * dv;
        b.position[0] += b.linear_velocity[0] * dt;
        b.position[1] += b.linear_velocity[1] * dt;
    }
    false
}

fn solve_fixed_joint(world: &mut World2D, fj: &FixedJoint2D, dt: f32) -> bool {
    let (pos_a, rot_a, _) = world.body_pose_and_inv(fj.body_a);
    let Some(bid_b) = fj.body_b else {
        return false;
    };
    let (pos_b, rot_b, inv_b) = world.body_pose_and_inv(bid_b);
    if inv_b <= 0.0 {
        return false;
    }
    let target_pos = transform_point2(fj.local_offset, pos_a, rot_a);
    let target_rot = rot_a + fj.reference_angle;
    let pos_err = length2(sub2(target_pos, pos_b));
    let rot_err = (target_rot - rot_b).abs();
    if (fj.break_force.is_finite() && pos_err / (inv_b * dt * dt) > fj.break_force)
        || (fj.break_torque.is_finite() && rot_err / (inv_b * dt * dt) > fj.break_torque)
    {
        return true;
    }
    if let Some(b) = world.bodies.get_mut(&bid_b) {
        b.position = target_pos;
        b.rotation = target_rot;
        b.linear_velocity = [0.0, 0.0];
        b.angular_velocity = 0.0;
    }
    false
}

fn solve_relative_joint(world: &mut World2D, rj: &RelativeJoint2D, dt: f32) -> bool {
    let (pos_a, rot_a, _) = world.body_pose_and_inv(rj.body_a);
    let Some(bid_b) = rj.body_b else {
        return false;
    };
    let (pos_b, rot_b, inv_b) = world.body_pose_and_inv(bid_b);
    if inv_b <= 0.0 {
        return false;
    }
    let desired_pos = transform_point2(rj.linear_offset, pos_a, rot_a);
    let desired_rot = rot_a + rj.angular_offset;
    let scale = rj.correction_scale.clamp(0.0, 1.0);
    let delta_p = sub2(desired_pos, pos_b);
    let max_disp = rj.max_force.max(0.0) * inv_b * dt * dt;
    let disp_len = length2(delta_p);
    let step_p = if disp_len > max_disp && disp_len > 1e-6 {
        [
            delta_p[0] / disp_len * max_disp * scale,
            delta_p[1] / disp_len * max_disp * scale,
        ]
    } else {
        [delta_p[0] * scale, delta_p[1] * scale]
    };
    let delta_r = (desired_rot - rot_b) * scale;
    if let Some(b) = world.bodies.get_mut(&bid_b) {
        b.position[0] += step_p[0];
        b.position[1] += step_p[1];
        b.rotation += delta_r;
    }
    false
}

fn solve_friction_joint(world: &mut World2D, frj: &FrictionJoint2D, dt: f32) -> bool {
    let Some(bid_b) = frj.body_b else {
        return false;
    };
    let (_, _, inv_b) = world.body_pose_and_inv(bid_b);
    if inv_b <= 0.0 {
        return false;
    }
    if (frj.break_force.is_finite() && frj.max_force > frj.break_force)
        || (frj.break_torque.is_finite() && frj.max_torque > frj.break_torque)
    {
        return true;
    }
    if let Some(b) = world.bodies.get_mut(&bid_b) {
        let speed = length2(b.linear_velocity);
        let max_dv = frj.max_force.max(0.0) * inv_b * dt;
        if speed <= max_dv {
            b.linear_velocity = [0.0, 0.0];
        } else if speed > 1e-6 {
            let factor = (speed - max_dv) / speed;
            b.linear_velocity[0] *= factor;
            b.linear_velocity[1] *= factor;
        }
        let ang_speed = b.angular_velocity.abs();
        let max_dw = frj.max_torque.max(0.0) * inv_b * dt;
        if ang_speed <= max_dw {
            b.angular_velocity = 0.0;
        } else {
            b.angular_velocity -= b.angular_velocity.signum() * max_dw;
        }
    }
    false
}

fn solve_hinge_joint(world: &mut World2D, hj: &HingeJoint2D, dt: f32) -> bool {
    let (pos_a, rot_a, _) = world.body_pose_and_inv(hj.body_a);
    let world_a = transform_point2(hj.anchor_a, pos_a, rot_a);
    let Some(bid_b) = hj.body_b else {
        return false;
    };
    let (_, _, inv_b) = world.body_pose_and_inv(bid_b);
    if inv_b <= 0.0 {
        return false;
    }
    if let Some(b) = world.bodies.get_mut(&bid_b) {
        if hj.use_motor {
            let max_dw = hj.motor.max_motor_torque.max(0.0) * inv_b * dt;
            let diff = (hj.motor.motor_speed - b.angular_velocity).clamp(-max_dw, max_dw);
            b.angular_velocity += diff;
            b.rotation += b.angular_velocity * dt;
        }
        if hj.use_limits {
            let rel_angle = b.rotation - rot_a;
            let clamped = rel_angle.clamp(hj.limits.min, hj.limits.max);
            b.rotation = rot_a + clamped;
        }
        let rotated_anchor_b = rotate_vec2(hj.anchor_b, b.rotation);
        b.position = sub2(world_a, rotated_anchor_b);
    }
    false
}

fn solve_slider_joint(world: &mut World2D, slj: &SliderJoint2D, dt: f32) -> bool {
    let (pos_a, rot_a, _) = world.body_pose_and_inv(slj.body_a);
    let world_a = transform_point2(slj.anchor_a, pos_a, rot_a);
    let axis_angle = rot_a + slj.angle_degrees.to_radians();
    let axis = rotate_vec2([1.0, 0.0], axis_angle);
    let Some(bid_b) = slj.body_b else {
        return false;
    };
    let (_, _, inv_b) = world.body_pose_and_inv(bid_b);
    if inv_b <= 0.0 {
        return false;
    }
    if let Some(b) = world.bodies.get_mut(&bid_b) {
        let rel = sub2(b.position, world_a);
        let mut coord = dot2(rel, axis);
        if slj.use_motor {
            let max_dv = slj.motor.max_motor_torque.max(0.0) * inv_b * dt;
            let v_along = dot2(b.linear_velocity, axis);
            let dv = (slj.motor.motor_speed - v_along).clamp(-max_dv, max_dv);
            let new_v = v_along + dv;
            b.linear_velocity = [axis[0] * new_v, axis[1] * new_v];
            coord += new_v * dt;
        }
        if slj.use_limits {
            coord = coord.clamp(slj.limits.min, slj.limits.max);
        }
        b.position = [world_a[0] + axis[0] * coord, world_a[1] + axis[1] * coord];
    }
    false
}

fn solve_wheel_joint(world: &mut World2D, wj: &WheelJoint2D, dt: f32) -> bool {
    let (pos_a, rot_a, _) = world.body_pose_and_inv(wj.body_a);
    let world_a = transform_point2(wj.anchor_a, pos_a, rot_a);
    let susp_angle = rot_a + wj.suspension_angle_degrees.to_radians();
    let susp_axis = rotate_vec2([1.0, 0.0], susp_angle);
    let Some(bid_b) = wj.body_b else {
        return false;
    };
    let (_, _, inv_b) = world.body_pose_and_inv(bid_b);
    if inv_b <= 0.0 {
        return false;
    }
    if let Some(b) = world.bodies.get_mut(&bid_b) {
        let rel = sub2(b.position, world_a);
        let offset_along = dot2(rel, susp_axis);
        let k = (wj.suspension_frequency.max(0.5) * std::f32::consts::TAU).powi(2) / inv_b;
        let spring_acc = -k * offset_along * inv_b;
        let v_along = dot2(b.linear_velocity, susp_axis) + spring_acc * dt;
        let damp = (1.0 - wj.suspension_damping_ratio.clamp(0.0, 1.0) * dt).max(0.0);
        let new_v_along = v_along * damp;
        let new_coord = offset_along + new_v_along * dt;
        b.linear_velocity = [susp_axis[0] * new_v_along, susp_axis[1] * new_v_along];
        b.position = [
            world_a[0] + susp_axis[0] * new_coord,
            world_a[1] + susp_axis[1] * new_coord,
        ];
        if wj.use_motor {
            let max_dw = wj.motor.max_motor_torque.max(0.0) * inv_b * dt;
            let dw = (wj.motor.motor_speed - b.angular_velocity).clamp(-max_dw, max_dw);
            b.angular_velocity += dw;
            b.rotation += b.angular_velocity * dt;
        }
    }
    false
}
