use std::fmt;

use crate::unity_yaml::{Document, Node};

pub const PARTICLE_SYSTEM_CLASS: u32 = 198;
pub const DEFAULT_BURST_INTERVAL: f32 = 0.1;
pub const DEFAULT_MAX_PARTICLES: usize = 1000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleBurstBinding {
    pub time: f32,
    pub count: u32,
    pub cycles: u32,
    pub interval: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParticleSystemBinding {
    pub duration: f32,
    pub looping: bool,
    pub max_particles: usize,
    pub start_lifetime: f32,
    pub start_speed: f32,
    pub start_size: f32,
    pub start_color: [f32; 4],
    pub emission_rate: f32,
    pub bursts: Vec<ParticleBurstBinding>,
}

impl Default for ParticleSystemBinding {
    fn default() -> Self {
        Self {
            duration: 5.0,
            looping: false,
            max_particles: DEFAULT_MAX_PARTICLES,
            start_lifetime: 1.0,
            start_speed: 0.0,
            start_size: 1.0,
            start_color: [1.0; 4],
            emission_rate: 0.0,
            bursts: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParticleError {
    NotParticleSystem { class_id: u32 },
    MissingParticleSystemRoot,
}

impl fmt::Display for ParticleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParticleError::NotParticleSystem { class_id } => {
                write!(
                    formatter,
                    "document class {class_id} is not a particle system"
                )
            }
            ParticleError::MissingParticleSystemRoot => {
                write!(
                    formatter,
                    "particle system document has no ParticleSystem root"
                )
            }
        }
    }
}

impl std::error::Error for ParticleError {}

pub fn parse_particle_system(document: &Document) -> Result<ParticleSystemBinding, ParticleError> {
    if document.class_id != PARTICLE_SYSTEM_CLASS {
        return Err(ParticleError::NotParticleSystem {
            class_id: document.class_id,
        });
    }
    let body = document
        .root
        .get("ParticleSystem")
        .ok_or(ParticleError::MissingParticleSystemRoot)?;
    Ok(ParticleSystemBinding {
        duration: scalar(body.get("m_Duration"), 5.0),
        looping: flag(body.get("m_Loop"), false),
        max_particles: body
            .get("m_MaxParticles")
            .and_then(integer)
            .map_or(DEFAULT_MAX_PARTICLES, |value| value as usize),
        start_lifetime: scalar(body.get("m_StartLifetime"), 1.0),
        start_speed: scalar(body.get("m_StartSpeed"), 0.0),
        start_size: scalar(body.get("m_StartSize"), 1.0),
        start_color: color(body.get("m_StartColor"), [1.0; 4]),
        emission_rate: scalar(body.get("m_EmissionRate"), 0.0),
        bursts: bursts(body.get("m_Bursts")),
    })
}

fn scalar(node: Option<&Node>, default: f32) -> f32 {
    match node.and_then(Node::as_f64) {
        Some(value) if value.is_finite() => value as f32,
        _ => constant(node, default),
    }
}

fn constant(node: Option<&Node>, default: f32) -> f32 {
    match node
        .and_then(|node| node.get("m_Constant"))
        .and_then(Node::as_f64)
    {
        Some(value) if value.is_finite() => value as f32,
        _ => default,
    }
}

fn integer(node: &Node) -> Option<u32> {
    let value = match node.as_i64() {
        Some(value) => value,
        None => node.as_f64()?.trunc() as i64,
    };
    u32::try_from(value).ok()
}

fn flag(node: Option<&Node>, default: bool) -> bool {
    match node.and_then(Node::as_f64) {
        Some(value) => value != 0.0,
        None => match node.and_then(Node::as_str) {
            Some("true") => true,
            Some("false") => false,
            _ => default,
        },
    }
}

fn color(node: Option<&Node>, default: [f32; 4]) -> [f32; 4] {
    let Some(node) = node else {
        return default;
    };
    if let Some(items) = node.as_sequence() {
        if items.len() == 4 {
            let mut color = default;
            for (slot, item) in color.iter_mut().zip(items.iter()) {
                if let Some(value) = item.as_f64() {
                    if value.is_finite() {
                        *slot = value as f32;
                    }
                }
            }
            return color;
        }
    }
    let mut color = default;
    for (index, key) in ["r", "g", "b", "a"].iter().enumerate() {
        if let Some(value) = node.get(key).and_then(Node::as_f64) {
            if value.is_finite() {
                color[index] = value as f32;
            }
        }
    }
    color
}

fn bursts(node: Option<&Node>) -> Vec<ParticleBurstBinding> {
    let Some(items) = node.and_then(Node::as_sequence) else {
        return Vec::new();
    };
    let mut bursts = Vec::new();
    for item in items {
        let count = item.get("m_Count").and_then(integer).unwrap_or(0);
        let cycles = item.get("m_Cycles").and_then(integer).unwrap_or(1).max(1);
        let interval = scalar(item.get("m_RepeatInterval"), DEFAULT_BURST_INTERVAL);
        bursts.push(ParticleBurstBinding {
            time: scalar(item.get("m_Time"), 0.0).max(0.0),
            count,
            cycles,
            interval: if interval.is_finite() && interval > 0.0 {
                interval
            } else {
                DEFAULT_BURST_INTERVAL
            },
        });
    }
    bursts
}
