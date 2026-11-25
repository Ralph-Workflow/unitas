use unitas_core::EntityId;

use crate::body::{add, capsule_world_segment, closest_point_on_segment, subtract, vector_length};
use crate::world::World;

const CONSTRAINT_ITERATIONS: usize = 8;
const DEFAULT_MASS: f64 = 1.0;
const DEFAULT_DAMPING: f64 = 0.98;
const DEFAULT_PARTICLE_RADIUS: f64 = 0.05;
const LENGTH_EPSILON: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClothError {
    InvalidDescription,
    InvalidDeltaTime(f64),
    MissingBody(EntityId),
}

impl std::fmt::Display for ClothError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDescription => {
                write!(formatter, "the cloth description is invalid")
            }
            Self::InvalidDeltaTime(delta_seconds) => write!(
                formatter,
                "delta seconds {delta_seconds} is not a finite positive number"
            ),
            Self::MissingBody(entity) => {
                write!(formatter, "entity {entity} has no body in the world")
            }
        }
    }
}

impl std::error::Error for ClothError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClothCollider {
    pub entity: EntityId,
    pub radius: f64,
    pub height: f64,
    pub direction: u8,
}

impl ClothCollider {
    pub fn capsule(entity: EntityId, radius: f64, height: f64) -> Self {
        Self {
            entity,
            radius,
            height,
            direction: 1,
        }
    }

    pub fn with_direction(mut self, direction: u8) -> Self {
        self.direction = direction;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClothDesc {
    pub position: [f64; 3],
    pub width: f64,
    pub height: f64,
    pub resolution_x: usize,
    pub resolution_y: usize,
    pub mass: f64,
    pub damping: f64,
    pub particle_radius: f64,
}

impl ClothDesc {
    pub fn new(
        position: [f64; 3],
        width: f64,
        height: f64,
        resolution_x: usize,
        resolution_y: usize,
    ) -> Self {
        Self {
            position,
            width,
            height,
            resolution_x,
            resolution_y,
            mass: DEFAULT_MASS,
            damping: DEFAULT_DAMPING,
            particle_radius: DEFAULT_PARTICLE_RADIUS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClothParticle {
    pub position: [f64; 3],
    pub previous_position: [f64; 3],
    pub inverse_mass: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClothConstraint {
    pub first: usize,
    pub second: usize,
    pub rest_length: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClothContact {
    pub entity: EntityId,
    pub particle: usize,
    pub normal: [f64; 3],
}

pub struct Cloth {
    desc: ClothDesc,
    particles: Vec<ClothParticle>,
    constraints: Vec<ClothConstraint>,
    colliders: Vec<ClothCollider>,
    contacts: Vec<ClothContact>,
}

impl Cloth {
    pub fn new(desc: ClothDesc) -> Result<Self, ClothError> {
        if !desc.position.iter().all(|value| value.is_finite())
            || !desc.width.is_finite()
            || desc.width <= 0.0
            || !desc.height.is_finite()
            || desc.height <= 0.0
            || desc.resolution_x < 2
            || desc.resolution_y < 2
            || !desc.mass.is_finite()
            || desc.mass <= 0.0
            || !desc.damping.is_finite()
            || desc.damping < 0.0
            || desc.damping > 1.0
            || !desc.particle_radius.is_finite()
            || desc.particle_radius < 0.0
        {
            return Err(ClothError::InvalidDescription);
        }
        let spacing_x = desc.width / (desc.resolution_x - 1) as f64;
        let spacing_y = desc.height / (desc.resolution_y - 1) as f64;
        let count = desc.resolution_x * desc.resolution_y;
        let inverse_mass = count as f64 / desc.mass;
        let mut particles = Vec::with_capacity(count);
        for row in 0..desc.resolution_y {
            for column in 0..desc.resolution_x {
                let position = [
                    desc.position[0] + column as f64 * spacing_x,
                    desc.position[1] + row as f64 * spacing_y,
                    desc.position[2],
                ];
                particles.push(ClothParticle {
                    position,
                    previous_position: position,
                    inverse_mass,
                });
            }
        }
        let mut constraints = Vec::new();
        for row in 0..desc.resolution_y {
            for column in 0..desc.resolution_x {
                let index = row * desc.resolution_x + column;
                if column + 1 < desc.resolution_x {
                    constraints.push(ClothConstraint {
                        first: index,
                        second: index + 1,
                        rest_length: spacing_x,
                    });
                }
                if row + 1 < desc.resolution_y {
                    constraints.push(ClothConstraint {
                        first: index,
                        second: index + desc.resolution_x,
                        rest_length: spacing_y,
                    });
                }
            }
        }
        Ok(Self {
            desc,
            particles,
            constraints,
            colliders: Vec::new(),
            contacts: Vec::new(),
        })
    }

    pub fn desc(&self) -> &ClothDesc {
        &self.desc
    }

    pub fn particles(&self) -> &[ClothParticle] {
        &self.particles
    }

    pub fn particle(&self, index: usize) -> Option<&ClothParticle> {
        self.particles.get(index)
    }

    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    pub fn constraints(&self) -> &[ClothConstraint] {
        &self.constraints
    }

    pub fn colliders(&self) -> &[ClothCollider] {
        &self.colliders
    }

    pub fn assign_collider(&mut self, collider: ClothCollider) -> Result<(), ClothError> {
        if !collider.radius.is_finite()
            || collider.radius <= 0.0
            || !collider.height.is_finite()
            || collider.height < 0.0
            || collider.direction > 2
        {
            return Err(ClothError::InvalidDescription);
        }
        self.colliders.push(collider);
        Ok(())
    }

    pub fn contacts(&self) -> &[ClothContact] {
        &self.contacts
    }

    pub fn simulate(&mut self, world: &World, delta_seconds: f64) -> Result<(), ClothError> {
        if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return Err(ClothError::InvalidDeltaTime(delta_seconds));
        }
        self.integrate(world.gravity(), delta_seconds);
        self.solve_constraints();
        self.resolve_collisions(world)
    }

    fn integrate(&mut self, gravity: [f64; 3], delta_seconds: f64) {
        let damping = self.desc.damping;
        let squared = delta_seconds * delta_seconds;
        for particle in &mut self.particles {
            let velocity = [
                (particle.position[0] - particle.previous_position[0]) * damping,
                (particle.position[1] - particle.previous_position[1]) * damping,
                (particle.position[2] - particle.previous_position[2]) * damping,
            ];
            let previous = particle.position;
            particle.position = [
                particle.position[0] + velocity[0] + gravity[0] * squared,
                particle.position[1] + velocity[1] + gravity[1] * squared,
                particle.position[2] + velocity[2] + gravity[2] * squared,
            ];
            particle.previous_position = previous;
        }
    }

    fn solve_constraints(&mut self) {
        for _ in 0..CONSTRAINT_ITERATIONS {
            for index in 0..self.constraints.len() {
                let constraint = self.constraints[index];
                let delta = subtract(
                    self.particles[constraint.second].position,
                    self.particles[constraint.first].position,
                );
                let distance = vector_length(delta);
                if distance <= LENGTH_EPSILON {
                    continue;
                }
                let first_weight = self.particles[constraint.first].inverse_mass;
                let second_weight = self.particles[constraint.second].inverse_mass;
                let total = first_weight + second_weight;
                if total <= 0.0 {
                    continue;
                }
                let correction = scale(delta, (distance - constraint.rest_length) / distance);
                self.particles[constraint.first].position = add(
                    self.particles[constraint.first].position,
                    scale(correction, first_weight / total),
                );
                self.particles[constraint.second].position = subtract(
                    self.particles[constraint.second].position,
                    scale(correction, second_weight / total),
                );
            }
        }
    }

    fn resolve_collisions(&mut self, world: &World) -> Result<(), ClothError> {
        self.contacts.clear();
        for particle_index in 0..self.particles.len() {
            for collider_index in 0..self.colliders.len() {
                let collider = self.colliders[collider_index];
                let position = world
                    .position(collider.entity)
                    .ok_or(ClothError::MissingBody(collider.entity))?;
                let rotation = world
                    .rotation(collider.entity)
                    .ok_or(ClothError::MissingBody(collider.entity))?;
                let (start, end) = capsule_world_segment(
                    position,
                    rotation,
                    collider.radius,
                    collider.height,
                    collider.direction,
                );
                let closest =
                    closest_point_on_segment(start, end, self.particles[particle_index].position);
                let offset = subtract(self.particles[particle_index].position, closest);
                let distance = vector_length(offset);
                let contact_distance = collider.radius + self.desc.particle_radius;
                if distance >= contact_distance {
                    continue;
                }
                let normal = if distance > LENGTH_EPSILON {
                    scale(offset, 1.0 / distance)
                } else {
                    [0.0, 1.0, 0.0]
                };
                self.particles[particle_index].position =
                    add(closest, scale(normal, contact_distance));
                self.contacts.push(ClothContact {
                    entity: collider.entity,
                    particle: particle_index,
                    normal,
                });
            }
        }
        Ok(())
    }
}

fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}
