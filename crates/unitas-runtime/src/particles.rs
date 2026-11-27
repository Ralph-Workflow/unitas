use std::collections::BTreeMap;

use unitas_asset::particle::{
    PARTICLE_SYSTEM_CLASS, ParticleError, ParticleSystemBinding,
};
use unitas_asset::unity_yaml::{Document, Node};
use unitas_core::EntityId;
use unitas_physics::World;
use unitas_physics::particles::{
    Curve, CustomParticleDataStream, EmitParams, Particle, ParticleBurst, ParticleCollisionEvent,
    ParticleCollisionEvent2D, ParticlePropertyCurves, ParticleShape, ParticleSystem,
    ParticleSystem2D, ParticleSystemForceField, ParticleSystemState, ParticleTriggerEvent,
    ParticleTriggerEvent2D, ParticleUv, StopAction, SubEmitterConfig, SubEmitterTrigger,
    TextureSheetAnimation, TextureSheetAnimationType,
};
use unitas_physics::two_d::World2D;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParticleSystemModulesBinding {
    pub sub_emitters: Vec<SubEmitterConfig>,
    pub texture_sheet_animation: Option<TextureSheetAnimation>,
}

pub fn parse_particle_system_modules(
    document: &Document,
) -> Result<ParticleSystemModulesBinding, ParticleError> {
    if document.class_id != PARTICLE_SYSTEM_CLASS {
        return Err(ParticleError::NotParticleSystem {
            class_id: document.class_id,
        });
    }
    let body = document
        .root
        .get("ParticleSystem")
        .ok_or(ParticleError::MissingParticleSystemRoot)?;
    Ok(ParticleSystemModulesBinding {
        sub_emitters: parse_sub_module(body.get("SubModule")),
        texture_sheet_animation: parse_uv_module(body.get("UVModule")),
    })
}

fn parse_sub_module(node: Option<&Node>) -> Vec<SubEmitterConfig> {
    let Some(node) = node else {
        return Vec::new();
    };
    if !yaml_flag(node.get("enabled"), false) {
        return Vec::new();
    }
    let Some(items) = node.get("subEmitters").and_then(Node::as_sequence) else {
        return Vec::new();
    };
    let mut configs = Vec::new();
    for item in items {
        let Some(file_id) = item
            .get("subEmitter")
            .and_then(|sub| sub.get("fileID"))
            .and_then(yaml_u64)
        else {
            continue;
        };
        if file_id == 0 {
            continue;
        }
        let trigger = match item.get("type").and_then(yaml_u32).unwrap_or(0) {
            1 => SubEmitterTrigger::Collision,
            2 => SubEmitterTrigger::Death,
            3 => SubEmitterTrigger::Trigger,
            _ => SubEmitterTrigger::Birth,
        };
        let count = item.get("emitCount").and_then(yaml_u32).unwrap_or(1).max(1);
        let probability = yaml_scalar(item.get("emitProbability"), 1.0);
        configs.push(
            SubEmitterConfig::new(trigger, EntityId::new(file_id), count)
                .with_probability(probability),
        );
    }
    configs
}

fn parse_uv_module(node: Option<&Node>) -> Option<TextureSheetAnimation> {
    let node = node?;
    if !yaml_flag(node.get("enabled"), false) {
        return None;
    }
    let tiles_x = node.get("tilesX").and_then(yaml_u32).unwrap_or(1).max(1);
    let tiles_y = node.get("tilesY").and_then(yaml_u32).unwrap_or(1).max(1);
    let animation_type = match node.get("animationType").and_then(yaml_u32).unwrap_or(0) {
        1 => TextureSheetAnimationType::SingleRow,
        _ => TextureSheetAnimationType::WholeSheet,
    };
    let row_index = node.get("rowIndex").and_then(yaml_u32).unwrap_or(0);
    let cycles = yaml_scalar(node.get("cycles"), 1.0);
    let start_frame = yaml_scalar(node.get("startFrame"), 0.0);
    let frame_curve = parse_curve_1(node.get("frameOverTime"));
    Some(
        TextureSheetAnimation::new(tiles_x, tiles_y)
            .with_animation_type(animation_type)
            .with_row_index(row_index)
            .with_cycles(cycles)
            .with_start_frame(start_frame)
            .with_frame_over_time(frame_curve),
    )
}

fn parse_curve_1(node: Option<&Node>) -> Curve<1> {
    let Some(node) = node else {
        return Curve::new([0.0], [1.0]);
    };
    let start = yaml_scalar(node.get("start"), 0.0);
    let end = yaml_scalar(node.get("end"), 1.0);
    Curve::new([start], [end])
}

fn yaml_scalar(node: Option<&Node>, default: f32) -> f32 {
    match node.and_then(Node::as_f64) {
        Some(value) if value.is_finite() => value as f32,
        _ => match node
            .and_then(|inner| inner.get("m_Constant"))
            .and_then(Node::as_f64)
        {
            Some(value) if value.is_finite() => value as f32,
            _ => default,
        },
    }
}

fn yaml_u32(node: &Node) -> Option<u32> {
    let value = match node.as_i64() {
        Some(value) => value,
        None => node.as_f64()?.trunc() as i64,
    };
    u32::try_from(value).ok()
}

fn yaml_u64(node: &Node) -> Option<u64> {
    let value = match node.as_i64() {
        Some(value) => value,
        None => node.as_f64()?.trunc() as i64,
    };
    u64::try_from(value).ok()
}

fn yaml_flag(node: Option<&Node>, default: bool) -> bool {
    match node.and_then(Node::as_f64) {
        Some(value) => value != 0.0,
        None => match node.and_then(Node::as_str) {
            Some("true") => true,
            Some("false") => false,
            _ => default,
        },
    }
}

pub fn serialize_particle_system_modules(
    binding: &ParticleSystemBinding,
    modules: &ParticleSystemModulesBinding,
) -> String {
    let mut out = String::new();
    out.push_str("--- !u!198 &1000\nParticleSystem:\n");
    out.push_str(&format!("  m_Duration: {}\n", binding.duration));
    out.push_str(&format!("  m_Loop: {}\n", u8::from(binding.looping)));
    out.push_str(&format!("  m_MaxParticles: {}\n", binding.max_particles));
    out.push_str(&format!("  m_StartLifetime: {}\n", binding.start_lifetime));
    out.push_str(&format!("  m_StartSpeed: {}\n", binding.start_speed));
    out.push_str(&format!("  m_StartSize: {}\n", binding.start_size));
    out.push_str("  m_StartColor:\n");
    out.push_str(&format!("    r: {}\n", binding.start_color[0]));
    out.push_str(&format!("    g: {}\n", binding.start_color[1]));
    out.push_str(&format!("    b: {}\n", binding.start_color[2]));
    out.push_str(&format!("    a: {}\n", binding.start_color[3]));
    out.push_str(&format!("  m_EmissionRate: {}\n", binding.emission_rate));
    serialize_bursts(&mut out, binding);
    serialize_sub_module(&mut out, &modules.sub_emitters);
    serialize_uv_module(&mut out, modules.texture_sheet_animation);
    out
}

fn serialize_bursts(out: &mut String, binding: &ParticleSystemBinding) {
    if binding.bursts.is_empty() {
        out.push_str("  m_Bursts: []\n");
        return;
    }
    out.push_str("  m_Bursts:\n");
    for burst in &binding.bursts {
        out.push_str(&format!("  - m_Time: {}\n", burst.time));
        out.push_str(&format!("    m_Count: {}\n", burst.count));
        out.push_str(&format!("    m_Cycles: {}\n", burst.cycles));
        out.push_str(&format!("    m_RepeatInterval: {}\n", burst.interval));
    }
}

fn serialize_sub_module(out: &mut String, sub_emitters: &[SubEmitterConfig]) {
    out.push_str("  SubModule:\n");
    out.push_str(&format!("    enabled: {}\n", u8::from(!sub_emitters.is_empty())));
    if sub_emitters.is_empty() {
        out.push_str("    subEmitters: []\n");
        return;
    }
    out.push_str("    subEmitters:\n");
    for sub in sub_emitters {
        let code = match sub.trigger {
            SubEmitterTrigger::Birth => 0,
            SubEmitterTrigger::Collision => 1,
            SubEmitterTrigger::Death => 2,
            SubEmitterTrigger::Trigger => 3,
        };
        out.push_str(&format!(
            "    - subEmitter: {{fileID: {}}}\n",
            sub.sub_emitter.as_u64()
        ));
        out.push_str(&format!("      type: {code}\n"));
        out.push_str(&format!("      emitProbability: {}\n", sub.emit_probability));
        out.push_str(&format!("      emitCount: {}\n", sub.emit_count));
    }
}

fn serialize_uv_module(out: &mut String, sheet: Option<TextureSheetAnimation>) {
    out.push_str("  UVModule:\n");
    let Some(sheet) = sheet else {
        out.push_str("    enabled: 0\n");
        return;
    };
    let anim_code = match sheet.animation_type {
        TextureSheetAnimationType::WholeSheet => 0,
        TextureSheetAnimationType::SingleRow => 1,
    };
    out.push_str("    enabled: 1\n");
    out.push_str(&format!("    tilesX: {}\n", sheet.tiles_x));
    out.push_str(&format!("    tilesY: {}\n", sheet.tiles_y));
    out.push_str(&format!("    animationType: {anim_code}\n"));
    out.push_str(&format!("    rowIndex: {}\n", sheet.row_index));
    out.push_str(&format!("    cycles: {}\n", sheet.cycles));
    out.push_str(&format!("    startFrame: {}\n", sheet.start_frame));
    out.push_str("    frameOverTime:\n");
    out.push_str(&format!("      start: {}\n", sheet.frame_over_time.start[0]));
    out.push_str(&format!("      end: {}\n", sheet.frame_over_time.end[0]));
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleSystemSnapshot {
    pub entity: EntityId,
    pub seed: u64,
    pub duration: f32,
    pub looping: bool,
    pub state: ParticleSystemState,
    pub emitting: bool,
    pub emission_rate: f32,
    pub shape: ParticleShape,
    pub live_particle_count: usize,
    pub stopped_count: u32,
    pub disabled: bool,
    pub destroyed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleDrawItem {
    pub entity: EntityId,
    pub position: [f32; 3],
    pub size: f32,
    pub color: [f32; 4],
    pub uv_rect: [f32; 4],
}

#[derive(Default)]
pub struct ParticleRuntimeWorld {
    systems: BTreeMap<EntityId, ParticleSystem>,
    draw_list: Vec<ParticleDrawItem>,
}

impl ParticleRuntimeWorld {
    pub fn new() -> Self {
        Self {
            systems: BTreeMap::new(),
            draw_list: Vec::new(),
        }
    }

    pub fn register_system(&mut self, entity: EntityId, seed: u64) -> &mut ParticleSystem {
        self.systems
            .entry(entity)
            .or_insert_with(|| ParticleSystem::new(seed))
    }

    pub fn register_binding(
        &mut self,
        entity: EntityId,
        seed: u64,
        binding: &ParticleSystemBinding,
    ) -> &mut ParticleSystem {
        let system = self
            .systems
            .entry(entity)
            .or_insert_with(|| ParticleSystem::new(seed));
        apply_particle_binding(system, binding);
        system
    }

    pub fn register_binding_with_modules(
        &mut self,
        entity: EntityId,
        seed: u64,
        binding: &ParticleSystemBinding,
        modules: &ParticleSystemModulesBinding,
    ) -> &mut ParticleSystem {
        let system = self
            .systems
            .entry(entity)
            .or_insert_with(|| ParticleSystem::new(seed));
        apply_particle_binding(system, binding);
        system.set_sub_emitters(modules.sub_emitters.clone());
        system.set_texture_sheet_animation(modules.texture_sheet_animation);
        system
    }

    pub fn draw_list(&self) -> &[ParticleDrawItem] {
        &self.draw_list
    }

    pub fn insert_system(&mut self, entity: EntityId, system: ParticleSystem) {
        self.systems.insert(entity, system);
    }

    pub fn remove_system(&mut self, entity: EntityId) -> bool {
        self.systems.remove(&entity).is_some()
    }

    pub fn system(&self, entity: EntityId) -> Option<&ParticleSystem> {
        self.systems.get(&entity)
    }

    pub fn system_mut(&mut self, entity: EntityId) -> Option<&mut ParticleSystem> {
        self.systems.get_mut(&entity)
    }

    pub fn system_count(&self) -> usize {
        self.systems.len()
    }

    pub fn snapshot(&self, entity: EntityId) -> Option<ParticleSystemSnapshot> {
        let system = self.systems.get(&entity)?;
        Some(ParticleSystemSnapshot {
            entity,
            seed: system.seed(),
            duration: system.duration(),
            looping: system.is_looping(),
            state: system.state(),
            emitting: system.is_emitting(),
            emission_rate: system.emission_rate(),
            shape: system.shape(),
            live_particle_count: system.get_particles().len(),
            stopped_count: system.stopped_count(),
            disabled: system.is_disabled(),
            destroyed: system.is_destroyed(),
        })
    }

    pub fn play(&mut self, entity: EntityId) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.play();
        true
    }

    pub fn pause(&mut self, entity: EntityId) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.pause();
        true
    }

    pub fn stop(&mut self, entity: EntityId, action: StopAction) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.stop(action);
        true
    }

    pub fn clear(&mut self, entity: EntityId) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.clear();
        true
    }

    pub fn emit(&mut self, entity: EntityId, count: u32) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.emit(count);
        true
    }

    pub fn emit_with_params(&mut self, entity: EntityId, params: EmitParams, count: u32) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.emit_with_params(params, count);
        true
    }

    pub fn set_curves(&mut self, entity: EntityId, curves: ParticlePropertyCurves) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        if let Some(velocity) = curves.velocity {
            system.set_velocity_over_lifetime(velocity);
        }
        if let Some(force) = curves.force {
            system.set_force_over_lifetime(force);
        }
        if let Some(color) = curves.color {
            system.set_color_over_lifetime(color);
        }
        if let Some(size) = curves.size {
            system.set_size_over_lifetime(size);
        }
        true
    }

    pub fn set_velocity_curve(&mut self, entity: EntityId, curve: Curve<3>) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.set_velocity_over_lifetime(curve);
        true
    }

    pub fn particles(&self, entity: EntityId) -> Option<&[Particle]> {
        self.systems.get(&entity).map(ParticleSystem::get_particles)
    }

    pub fn set_particles(&mut self, entity: EntityId, particles: Vec<Particle>) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.set_particles(particles);
        true
    }

    pub fn set_custom_particle_data(
        &mut self,
        entity: EntityId,
        stream: CustomParticleDataStream,
        data: &[[f32; 4]],
    ) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.set_custom_particle_data(stream, data);
        true
    }

    pub fn get_custom_particle_data(
        &self,
        entity: EntityId,
        stream: CustomParticleDataStream,
    ) -> Option<&[[f32; 4]]> {
        self.systems
            .get(&entity)
            .map(|system| system.get_custom_particle_data(stream))
    }

    pub fn add_force_field(&mut self, entity: EntityId, field: ParticleSystemForceField) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.add_force_field(field);
        true
    }

    pub fn step_all(&mut self, delta_time: f32, physics_world: Option<&World>) {
        let mut destroyed_entities = Vec::new();
        let mut sub_events = Vec::new();
        for (entity, system) in &mut self.systems {
            if !system.is_disabled() && !system.is_destroyed() {
                system.simulate(delta_time);
                if let Some(world) = physics_world {
                    system.update_collisions(world);
                }
                sub_events.extend(system.drain_sub_emitter_events());
            }
            if system.is_destroyed() {
                destroyed_entities.push(*entity);
            }
        }
        self.dispatch_sub_emitter_events(&sub_events);
        for entity in destroyed_entities {
            self.systems.remove(&entity);
        }
        self.rebuild_draw_list();
    }

    fn dispatch_sub_emitter_events(&mut self, events: &[unitas_physics::particles::SubEmitterEvent]) {
        for event in events {
            let Some(child) = self.systems.get_mut(&event.sub_emitter) else {
                continue;
            };
            if child.is_disabled() || child.is_destroyed() {
                continue;
            }
            child.set_emit_position(event.position);
            child.emit(event.emit_count);
        }
    }

    fn rebuild_draw_list(&mut self) {
        let mut draw_list = Vec::new();
        for (entity, system) in &self.systems {
            let uvs = system.particle_uvs();
            for (index, particle) in system.get_particles().iter().enumerate() {
                let uv_rect = uvs
                    .get(index)
                    .map_or(ParticleUv::default().uv_rect, |uv| uv.uv_rect);
                draw_list.push(ParticleDrawItem {
                    entity: *entity,
                    position: particle.position,
                    size: particle.size,
                    color: particle.color,
                    uv_rect,
                });
            }
        }
        self.draw_list = draw_list;
    }

    pub fn collision_events(&self, entity: EntityId) -> Option<&[ParticleCollisionEvent]> {
        self.systems
            .get(&entity)
            .map(ParticleSystem::get_collision_events)
    }

    pub fn trigger_events(&self, entity: EntityId) -> Option<&[ParticleTriggerEvent]> {
        self.systems
            .get(&entity)
            .map(ParticleSystem::get_trigger_events)
    }
}

fn apply_particle_binding(system: &mut ParticleSystem, binding: &ParticleSystemBinding) {
    system.set_duration(binding.duration);
    system.set_looping(binding.looping);
    system.set_max_particles(binding.max_particles);
    system.set_emission_rate(binding.emission_rate);
    system.set_emit_params(EmitParams {
        start_speed: binding.start_speed,
        start_size: binding.start_size,
        start_lifetime: binding.start_lifetime,
        start_color: binding.start_color,
    });
    system.set_bursts(
        binding
            .bursts
            .iter()
            .map(|burst| ParticleBurst::new(burst.time, burst.count, burst.cycles, burst.interval))
            .collect(),
    );
}

#[derive(Default)]
pub struct ParticleRuntimeWorld2D {
    systems: BTreeMap<EntityId, ParticleSystem2D>,
}

impl ParticleRuntimeWorld2D {
    pub fn new() -> Self {
        Self {
            systems: BTreeMap::new(),
        }
    }

    pub fn register_system(&mut self, entity: EntityId, seed: u64) -> &mut ParticleSystem2D {
        self.systems
            .entry(entity)
            .or_insert_with(|| ParticleSystem2D::new(seed))
    }

    pub fn insert_system(&mut self, entity: EntityId, system: ParticleSystem2D) {
        self.systems.insert(entity, system);
    }

    pub fn remove_system(&mut self, entity: EntityId) -> bool {
        self.systems.remove(&entity).is_some()
    }

    pub fn system(&self, entity: EntityId) -> Option<&ParticleSystem2D> {
        self.systems.get(&entity)
    }

    pub fn system_mut(&mut self, entity: EntityId) -> Option<&mut ParticleSystem2D> {
        self.systems.get_mut(&entity)
    }

    pub fn system_count(&self) -> usize {
        self.systems.len()
    }

    pub fn step_all(&mut self, delta_time: f32, physics_world: Option<&World2D>) {
        let mut destroyed_entities = Vec::new();
        let mut sub_events = Vec::new();
        for (entity, system) in &mut self.systems {
            if !system.is_disabled() && !system.is_destroyed() {
                system.system_mut().simulate(delta_time);
                if let Some(world) = physics_world {
                    system.update_collisions(world);
                }
                sub_events.extend(system.system_mut().drain_sub_emitter_events());
            }
            if system.is_destroyed() {
                destroyed_entities.push(*entity);
            }
        }
        for event in sub_events {
            let Some(child) = self.systems.get_mut(&event.sub_emitter) else {
                continue;
            };
            if child.is_disabled() || child.is_destroyed() {
                continue;
            }
            child.system_mut().set_emit_position(event.position);
            child.system_mut().emit(event.emit_count);
        }
        for entity in destroyed_entities {
            self.systems.remove(&entity);
        }
    }

    pub fn collision_events(&self, entity: EntityId) -> Option<&[ParticleCollisionEvent2D]> {
        self.systems
            .get(&entity)
            .map(ParticleSystem2D::get_collision_events)
    }

    pub fn trigger_events(&self, entity: EntityId) -> Option<&[ParticleTriggerEvent2D]> {
        self.systems
            .get(&entity)
            .map(ParticleSystem2D::get_trigger_events)
    }
}
