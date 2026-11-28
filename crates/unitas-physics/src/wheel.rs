use unitas_core::EntityId;

use crate::body::{add, dot, multiply_rotation, normalize_rotation, rotate_vector, subtract};
use crate::query::normalize;
use crate::world::World;

const DEFAULT_RADIUS: f64 = 0.35;
const DEFAULT_WHEEL_MASS: f64 = 30.0;
const DEFAULT_SUSPENSION_DISTANCE: f64 = 0.3;
const MINIMUM_BODY_DIMENSION: f64 = 0.5;
const BUMP_STOP_FACTOR: f64 = 10.0;
const RAD_PER_MINUTE: f64 = 60.0 / std::f64::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelError {
    InvalidWheelDesc,
    InvalidVehicle,
    InvalidDeltaTime(f64),
}

impl std::fmt::Display for WheelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidWheelDesc => {
                write!(formatter, "invalid wheel collider description")
            }
            Self::InvalidVehicle => write!(formatter, "invalid vehicle configuration"),
            Self::InvalidDeltaTime(delta_seconds) => write!(
                formatter,
                "delta seconds {delta_seconds} is not a finite positive number"
            ),
        }
    }
}

impl std::error::Error for WheelError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelFrictionCurve {
    pub extremum_slip: f64,
    pub extremum_value: f64,
    pub asymptote_slip: f64,
    pub asymptote_value: f64,
    pub stiffness: f64,
}

impl WheelFrictionCurve {
    pub fn new(
        extremum_slip: f64,
        extremum_value: f64,
        asymptote_slip: f64,
        asymptote_value: f64,
        stiffness: f64,
    ) -> Self {
        Self {
            extremum_slip,
            extremum_value,
            asymptote_slip,
            asymptote_value,
            stiffness,
        }
    }

    pub fn evaluate(&self, slip: f64) -> f64 {
        let magnitude = slip.abs();
        if !magnitude.is_finite() {
            return 0.0;
        }
        if magnitude <= self.extremum_slip {
            return self.extremum_value;
        }
        if magnitude >= self.asymptote_slip {
            return self.asymptote_value;
        }
        let span = self.asymptote_slip - self.extremum_slip;
        if span <= 0.0 {
            return self.asymptote_value;
        }
        let blend = (magnitude - self.extremum_slip) / span;
        self.extremum_value + blend * (self.asymptote_value - self.extremum_value)
    }
}

impl Default for WheelFrictionCurve {
    fn default() -> Self {
        Self::new(0.01, 1.0, 0.5, 0.75, 1.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SuspensionSpring {
    pub spring: f64,
    pub damper: f64,
    pub target_position: f64,
}

impl SuspensionSpring {
    pub fn new(spring: f64, damper: f64, target_position: f64) -> Self {
        Self {
            spring,
            damper,
            target_position,
        }
    }
}

impl Default for SuspensionSpring {
    fn default() -> Self {
        Self::new(40_000.0, 4_000.0, 0.5)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelDesc {
    pub radius: f64,
    pub mass: f64,
    pub suspension_distance: f64,
    pub suspension_spring: SuspensionSpring,
    pub motor_torque: f64,
    pub brake_torque: f64,
    pub steer_angle_deg: f64,
    pub has_motor: bool,
    pub has_steer: bool,
    pub forward_friction: WheelFrictionCurve,
    pub sideways_friction: WheelFrictionCurve,
}

impl WheelDesc {
    pub fn new(radius: f64, suspension_distance: f64) -> Result<Self, WheelError> {
        let desc = Self {
            radius,
            mass: DEFAULT_WHEEL_MASS,
            suspension_distance,
            suspension_spring: SuspensionSpring::default(),
            motor_torque: 0.0,
            brake_torque: 0.0,
            steer_angle_deg: 0.0,
            has_motor: false,
            has_steer: false,
            forward_friction: WheelFrictionCurve::default(),
            sideways_friction: WheelFrictionCurve::default(),
        };
        desc.validate()?;
        Ok(desc)
    }

    pub fn validate(&self) -> Result<(), WheelError> {
        let valid = self.radius.is_finite()
            && self.radius > 0.0
            && self.mass.is_finite()
            && self.mass > 0.0
            && self.suspension_distance.is_finite()
            && self.suspension_distance > 0.0
            && self.suspension_spring.spring.is_finite()
            && self.suspension_spring.spring >= 0.0
            && self.suspension_spring.damper.is_finite()
            && self.suspension_spring.damper >= 0.0
            && self.suspension_spring.target_position.is_finite()
            && (0.0..=1.0).contains(&self.suspension_spring.target_position)
            && self.motor_torque.is_finite()
            && self.brake_torque.is_finite()
            && self.brake_torque >= 0.0
            && self.steer_angle_deg.is_finite();
        if !valid {
            return Err(WheelError::InvalidWheelDesc);
        }
        Ok(())
    }

    pub fn with_mass(mut self, mass: f64) -> Self {
        self.mass = mass;
        self
    }

    pub fn with_suspension_spring(mut self, spring: SuspensionSpring) -> Self {
        self.suspension_spring = spring;
        self
    }

    pub fn with_motor_torque(mut self, motor_torque: f64) -> Self {
        self.motor_torque = motor_torque.max(0.0);
        self.has_motor = true;
        self
    }

    pub fn with_brake_torque(mut self, brake_torque: f64) -> Self {
        self.brake_torque = brake_torque.max(0.0);
        self
    }

    pub fn with_steer_angle(mut self, steer_angle_deg: f64) -> Self {
        self.steer_angle_deg = steer_angle_deg;
        self.has_steer = true;
        self
    }

    pub fn with_forward_friction(mut self, curve: WheelFrictionCurve) -> Self {
        self.forward_friction = curve;
        self
    }

    pub fn with_sideways_friction(mut self, curve: WheelFrictionCurve) -> Self {
        self.sideways_friction = curve;
        self
    }
}

impl Default for WheelDesc {
    fn default() -> Self {
        match Self::new(DEFAULT_RADIUS, DEFAULT_SUSPENSION_DISTANCE) {
            Ok(desc) => desc,
            Err(_) => Self {
                radius: DEFAULT_RADIUS,
                mass: DEFAULT_WHEEL_MASS,
                suspension_distance: DEFAULT_SUSPENSION_DISTANCE,
                suspension_spring: SuspensionSpring::default(),
                motor_torque: 0.0,
                brake_torque: 0.0,
                steer_angle_deg: 0.0,
                has_motor: false,
                has_steer: false,
                forward_friction: WheelFrictionCurve::default(),
                sideways_friction: WheelFrictionCurve::default(),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundHit {
    pub entity: EntityId,
    pub point: [f64; 3],
    pub normal: [f64; 3],
    pub forward_dir: [f64; 3],
    pub sideways_dir: [f64; 3],
    pub forward_slip: f64,
    pub sideways_slip: f64,
    pub compression: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelCollider {
    desc: WheelDesc,
    local_position: [f64; 3],
    is_grounded: bool,
    compression: f64,
    angular_velocity: f64,
    rotation_angle: f64,
    sprung_mass: f64,
    ground_hit: Option<GroundHit>,
}

impl WheelCollider {
    pub fn new(desc: WheelDesc, local_position: [f64; 3]) -> Result<Self, WheelError> {
        desc.validate()?;
        if !local_position.iter().all(|component| component.is_finite()) {
            return Err(WheelError::InvalidWheelDesc);
        }
        Ok(Self {
            desc,
            local_position,
            is_grounded: false,
            compression: 0.0,
            angular_velocity: 0.0,
            rotation_angle: 0.0,
            sprung_mass: 0.0,
            ground_hit: None,
        })
    }

    pub fn desc(&self) -> &WheelDesc {
        &self.desc
    }

    pub fn desc_mut(&mut self) -> &mut WheelDesc {
        &mut self.desc
    }

    pub fn local_position(&self) -> [f64; 3] {
        self.local_position
    }

    pub fn is_grounded(&self) -> bool {
        self.is_grounded
    }

    pub fn compression(&self) -> f64 {
        self.compression
    }

    pub fn angular_velocity(&self) -> f64 {
        self.angular_velocity
    }

    pub fn rpm(&self) -> f64 {
        self.angular_velocity * RAD_PER_MINUTE
    }

    pub fn sprung_mass(&self) -> f64 {
        self.sprung_mass
    }

    pub fn ground_hit(&self) -> Option<&GroundHit> {
        self.ground_hit.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Vehicle {
    mass: f64,
    inertia: [f64; 3],
    position: [f64; 3],
    velocity: [f64; 3],
    rotation: [f64; 4],
    angular_velocity: [f64; 3],
    wheels: Vec<WheelCollider>,
    excluded_entity: Option<EntityId>,
    layer_mask: u32,
    substep_speed_threshold: f64,
    steps_below_threshold: usize,
    steps_above_threshold: usize,
}

impl Vehicle {
    pub fn new(mass: f64, mut wheels: Vec<WheelCollider>) -> Result<Self, WheelError> {
        if !mass.is_finite() || mass <= 0.0 || wheels.is_empty() {
            return Err(WheelError::InvalidVehicle);
        }
        let per_wheel_sprung = mass / wheels.len() as f64;
        for wheel in &mut wheels {
            wheel.sprung_mass = per_wheel_sprung;
        }
        let inertia = body_inertia(mass, &wheels);
        Ok(Self {
            mass,
            inertia,
            position: [0.0; 3],
            velocity: [0.0; 3],
            rotation: crate::body::IDENTITY_ROTATION,
            angular_velocity: [0.0; 3],
            wheels,
            excluded_entity: None,
            layer_mask: u32::MAX,
            substep_speed_threshold: 5.0,
            steps_below_threshold: 1,
            steps_above_threshold: 1,
        })
    }

    pub fn configure_vehicle_substeps(
        &mut self,
        speed_threshold: f64,
        steps_below_threshold: usize,
        steps_above_threshold: usize,
    ) -> Result<(), WheelError> {
        if !speed_threshold.is_finite()
            || speed_threshold < 0.0
            || steps_below_threshold == 0
            || steps_above_threshold == 0
        {
            return Err(WheelError::InvalidVehicle);
        }
        self.substep_speed_threshold = speed_threshold;
        self.steps_below_threshold = steps_below_threshold;
        self.steps_above_threshold = steps_above_threshold;
        Ok(())
    }

    pub fn vehicle_substeps(&self) -> (f64, usize, usize) {
        (
            self.substep_speed_threshold,
            self.steps_below_threshold,
            self.steps_above_threshold,
        )
    }

    pub fn wheel_world_pose(&self, index: usize) -> Option<([f64; 3], [f64; 4])> {
        let wheel = self.wheels.get(index)?;
        let attachment = add(
            self.position,
            rotate_vector(self.rotation, wheel.local_position),
        );
        let down = rotate_vector(self.rotation, [0.0, -1.0, 0.0]);
        let extension = (wheel.desc.suspension_distance - wheel.compression).max(0.0);
        let world_pos = add(attachment, scale(down, extension));
        let steer_rad = if wheel.desc.has_steer {
            wheel.desc.steer_angle_deg.to_radians()
        } else {
            0.0
        };
        let steer_quat = [0.0, (steer_rad * 0.5).sin(), 0.0, (steer_rad * 0.5).cos()];
        let spin_quat = [
            (wheel.rotation_angle * 0.5).sin(),
            0.0,
            0.0,
            (wheel.rotation_angle * 0.5).cos(),
        ];
        let world_rot = normalize_rotation(multiply_rotation(
            self.rotation,
            multiply_rotation(steer_quat, spin_quat),
        ));
        Some((world_pos, world_rot))
    }

    pub fn with_position(mut self, position: [f64; 3]) -> Self {
        self.position = position;
        self
    }

    pub fn with_velocity(mut self, velocity: [f64; 3]) -> Self {
        self.velocity = velocity;
        self
    }

    pub fn with_rotation(mut self, rotation: [f64; 4]) -> Self {
        self.rotation = normalize_rotation(rotation);
        self
    }

    pub fn with_excluded_entity(mut self, entity: EntityId) -> Self {
        self.excluded_entity = Some(entity);
        self
    }

    pub fn with_ground_layer_mask(mut self, layer_mask: u32) -> Self {
        self.layer_mask = layer_mask;
        self
    }

    pub fn wheels(&self) -> &[WheelCollider] {
        &self.wheels
    }

    pub fn wheels_mut(&mut self) -> &mut [WheelCollider] {
        &mut self.wheels
    }

    pub fn wheel(&self, index: usize) -> Option<&WheelCollider> {
        self.wheels.get(index)
    }

    pub fn wheel_mut(&mut self, index: usize) -> Option<&mut WheelCollider> {
        self.wheels.get_mut(index)
    }

    pub fn position(&self) -> [f64; 3] {
        self.position
    }

    pub fn velocity(&self) -> [f64; 3] {
        self.velocity
    }

    pub fn rotation(&self) -> [f64; 4] {
        self.rotation
    }

    pub fn angular_velocity(&self) -> [f64; 3] {
        self.angular_velocity
    }

    pub fn mass(&self) -> f64 {
        self.mass
    }

    pub fn forward_direction(&self) -> [f64; 3] {
        rotate_vector(self.rotation, [0.0, 0.0, 1.0])
    }

    pub fn grounded_wheel_count(&self) -> usize {
        self.wheels.iter().filter(|wheel| wheel.is_grounded).count()
    }

    pub fn step(&mut self, world: &World, delta_seconds: f64) -> Result<(), WheelError> {
        if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return Err(WheelError::InvalidDeltaTime(delta_seconds));
        }
        let frame = VehicleFrame {
            mass: self.mass,
            position: self.position,
            velocity: self.velocity,
            rotation: self.rotation,
            angular_velocity: self.angular_velocity,
            excluded_entity: self.excluded_entity,
            layer_mask: self.layer_mask,
        };
        let gravity = world.gravity();
        let mut force = [
            frame.mass * gravity[0],
            frame.mass * gravity[1],
            frame.mass * gravity[2],
        ];
        let mut torque = [0.0; 3];
        for wheel in &mut self.wheels {
            let (wheel_force, wheel_torque) = step_wheel(wheel, &frame, world, delta_seconds);
            force = add(force, wheel_force);
            torque = add(torque, wheel_torque);
        }
        self.integrate(force, torque, delta_seconds);
        Ok(())
    }

    fn integrate(&mut self, force: [f64; 3], torque: [f64; 3], delta_seconds: f64) {
        let acceleration = [
            force[0] / self.mass,
            force[1] / self.mass,
            force[2] / self.mass,
        ];
        self.velocity = add(self.velocity, scale(acceleration, delta_seconds));
        self.position = add(self.position, scale(self.velocity, delta_seconds));

        let momentum = [
            self.inertia[0] * self.angular_velocity[0],
            self.inertia[1] * self.angular_velocity[1],
            self.inertia[2] * self.angular_velocity[2],
        ];
        let gyroscopic = cross(self.angular_velocity, momentum);
        let angular_acceleration = [
            (torque[0] - gyroscopic[0]) / self.inertia[0],
            (torque[1] - gyroscopic[1]) / self.inertia[1],
            (torque[2] - gyroscopic[2]) / self.inertia[2],
        ];
        self.angular_velocity = add(
            self.angular_velocity,
            scale(angular_acceleration, delta_seconds),
        );

        let derivative = rotation_derivative(self.rotation, self.angular_velocity);
        let integrated = [
            self.rotation[0] + 0.5 * delta_seconds * derivative[0],
            self.rotation[1] + 0.5 * delta_seconds * derivative[1],
            self.rotation[2] + 0.5 * delta_seconds * derivative[2],
            self.rotation[3] + 0.5 * delta_seconds * derivative[3],
        ];
        self.rotation = normalize_rotation(integrated);
    }
}

struct VehicleFrame {
    mass: f64,
    position: [f64; 3],
    velocity: [f64; 3],
    rotation: [f64; 4],
    angular_velocity: [f64; 3],
    excluded_entity: Option<EntityId>,
    layer_mask: u32,
}

fn body_inertia(mass: f64, wheels: &[WheelCollider]) -> [f64; 3] {
    let mut extent_x = 0.0_f64;
    let mut extent_y = 0.0_f64;
    let mut extent_z = 0.0_f64;
    for wheel in wheels {
        let position = wheel.local_position();
        extent_x = extent_x.max(position[0].abs());
        extent_y = extent_y.max(position[1].abs());
        extent_z = extent_z.max(position[2].abs());
    }
    let width = (2.0 * extent_x).max(MINIMUM_BODY_DIMENSION);
    let height = (2.0 * extent_y).max(MINIMUM_BODY_DIMENSION);
    let depth = (2.0 * extent_z).max(MINIMUM_BODY_DIMENSION);
    [
        mass * (height * height + depth * depth) / 12.0,
        mass * (width * width + depth * depth) / 12.0,
        mass * (width * width + height * height) / 12.0,
    ]
}

fn step_wheel(
    wheel: &mut WheelCollider,
    frame: &VehicleFrame,
    world: &World,
    delta_seconds: f64,
) -> ([f64; 3], [f64; 3]) {
    let desc = wheel.desc;
    let inertia = desc.mass * desc.radius * desc.radius;
    let attachment = add(
        frame.position,
        rotate_vector(frame.rotation, wheel.local_position),
    );
    let down = rotate_vector(frame.rotation, [0.0, -1.0, 0.0]);
    let hit = world
        .sphere_cast(
            attachment,
            desc.radius,
            down,
            desc.suspension_distance,
            frame.layer_mask,
            false,
        )
        .into_iter()
        .find(|candidate| Some(candidate.entity) != frame.excluded_entity);
    let motor_torque = if desc.has_motor {
        desc.motor_torque
    } else {
        0.0
    };
    let Some(hit) = hit else {
        return step_airborne_wheel(wheel, motor_torque, inertia, delta_seconds);
    };
    step_grounded_wheel(
        wheel,
        frame,
        world,
        delta_seconds,
        (motor_torque, inertia),
        (attachment, down),
        &hit,
    )
}

fn step_grounded_wheel(
    wheel: &mut WheelCollider,
    frame: &VehicleFrame,
    world: &World,
    delta_seconds: f64,
    drive: (f64, f64),
    pose: ([f64; 3], [f64; 3]),
    hit: &crate::query::QueryHit,
) -> ([f64; 3], [f64; 3]) {
    let (motor_torque, inertia) = drive;
    let (attachment, down) = pose;
    let desc = wheel.desc;
    let (normal, forward_dir, sideways_dir) = wheel_contact_axes(&desc, frame, down, hit.normal);
    let (compression, normal_force) =
        compute_wheel_suspension(&desc, frame, world, hit, attachment, down);
    let contact_lever = subtract(hit.point, frame.position);
    let contact_velocity = add(frame.velocity, cross(frame.angular_velocity, contact_lever));
    let (forward_speed, sideways_speed) = (
        dot(contact_velocity, forward_dir),
        dot(contact_velocity, sideways_dir),
    );
    let (forward_slip, sideways_slip) = (
        wheel.angular_velocity * desc.radius - forward_speed,
        -sideways_speed,
    );
    let (longitudinal, lateral) = compute_wheel_tire_forces(
        wheel,
        motor_torque,
        inertia,
        delta_seconds,
        normal_force.max(0.0),
        (forward_speed, sideways_speed),
        (forward_slip, sideways_slip),
    );
    wheel.is_grounded = true;
    wheel.compression = compression;
    wheel.ground_hit = Some(GroundHit {
        entity: hit.entity,
        point: hit.point,
        normal,
        forward_dir,
        sideways_dir,
        forward_slip,
        sideways_slip,
        compression,
    });
    let force = add(
        add(
            scale(forward_dir, longitudinal),
            scale(sideways_dir, lateral),
        ),
        scale(normal, normal_force),
    );
    (force, cross(contact_lever, force))
}

fn step_airborne_wheel(
    wheel: &mut WheelCollider,
    motor_torque: f64,
    inertia: f64,
    delta_seconds: f64,
) -> ([f64; 3], [f64; 3]) {
    wheel.is_grounded = false;
    wheel.compression = 0.0;
    wheel.ground_hit = None;
    let brake_torque = wheel.desc.brake_torque * sign_of(wheel.angular_velocity);
    wheel.angular_velocity += (motor_torque - brake_torque) / inertia * delta_seconds;
    wheel.rotation_angle += wheel.angular_velocity * delta_seconds;
    ([0.0; 3], [0.0; 3])
}

fn wheel_contact_axes(
    desc: &WheelDesc,
    frame: &VehicleFrame,
    down: [f64; 3],
    hit_normal: [f64; 3],
) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let up = [-down[0], -down[1], -down[2]];
    let contact_normal = normalize(hit_normal).unwrap_or(up);
    let normal = if dot(contact_normal, up) < 0.0 {
        [-contact_normal[0], -contact_normal[1], -contact_normal[2]]
    } else {
        contact_normal
    };
    let steer_radians = if desc.has_steer {
        desc.steer_angle_deg.to_radians()
    } else {
        0.0
    };
    let steered = rotate_vector(
        frame.rotation,
        [steer_radians.sin(), 0.0, steer_radians.cos()],
    );
    let forward_dir =
        normalize(subtract(steered, scale(normal, dot(steered, normal)))).unwrap_or(steered);
    let sideways_dir = normalize(cross(normal, forward_dir)).unwrap_or([1.0, 0.0, 0.0]);
    (normal, forward_dir, sideways_dir)
}

fn compute_wheel_suspension(
    desc: &WheelDesc,
    frame: &VehicleFrame,
    world: &World,
    hit: &crate::query::QueryHit,
    attachment: [f64; 3],
    down: [f64; 3],
) -> (f64, f64) {
    let surface_height = dot(subtract(hit.point, attachment), down);
    let suspension_length = hit.distance.clamp(0.0, desc.suspension_distance);
    let compression =
        (desc.suspension_distance - suspension_length).clamp(0.0, desc.suspension_distance);
    let attachment_lever = subtract(attachment, frame.position);
    let attachment_velocity = add(
        frame.velocity,
        cross(frame.angular_velocity, attachment_lever),
    );
    let ground_velocity = world.velocity(hit.entity).unwrap_or([0.0; 3]);
    let compression_rate = dot(subtract(attachment_velocity, ground_velocity), down);

    let rest_compression = desc.suspension_spring.target_position * desc.suspension_distance;
    let penetration = (desc.radius - surface_height).max(0.0);
    let bump_stop = desc.suspension_spring.spring * BUMP_STOP_FACTOR * penetration;
    let normal_force = desc.suspension_spring.spring * (compression - rest_compression)
        + desc.suspension_spring.damper * compression_rate
        + bump_stop;
    (compression, normal_force)
}

fn compute_wheel_tire_forces(
    wheel: &mut WheelCollider,
    motor_torque: f64,
    inertia: f64,
    delta_seconds: f64,
    load: f64,
    speeds: (f64, f64),
    slips: (f64, f64),
) -> (f64, f64) {
    let (forward_speed, sideways_speed) = speeds;
    let (forward_slip, sideways_slip) = slips;
    let desc = wheel.desc;
    let motion_sign = if forward_speed > 0.0 {
        1.0
    } else if forward_speed < 0.0 {
        -1.0
    } else {
        sign_of(wheel.angular_velocity)
    };
    let brake_force = -motion_sign * desc.brake_torque / desc.radius;
    let requested = motor_torque / desc.radius + brake_force;
    let grip =
        desc.forward_friction.evaluate(forward_slip.abs()) * load * desc.forward_friction.stiffness;
    let sliding = requested.abs() > grip;
    let longitudinal = if sliding {
        let slip_sign = sign_of(forward_slip);
        if slip_sign == 0.0 {
            sign_of(requested) * grip
        } else {
            slip_sign * grip
        }
    } else {
        requested
    };

    let lateral_grip = desc.sideways_friction.evaluate(sideways_slip.abs())
        * load
        * desc.sideways_friction.stiffness;
    let lateral = -sign_of(sideways_speed) * lateral_grip;

    if sliding {
        let brake_torque = desc.brake_torque * sign_of(wheel.angular_velocity);
        let wheel_torque = motor_torque - brake_torque - longitudinal * desc.radius;
        wheel.angular_velocity += wheel_torque / inertia * delta_seconds;
    } else {
        wheel.angular_velocity = forward_speed / desc.radius;
    }
    wheel.rotation_angle += wheel.angular_velocity * delta_seconds;
    (longitudinal, lateral)
}

fn rotation_derivative(rotation: [f64; 4], angular_velocity: [f64; 3]) -> [f64; 4] {
    if angular_velocity == [0.0; 3] {
        return [0.0; 4];
    }
    multiply_rotation(
        [
            angular_velocity[0],
            angular_velocity[1],
            angular_velocity[2],
            0.0,
        ],
        rotation,
    )
}

fn cross(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ]
}

fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

fn sign_of(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}
