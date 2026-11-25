use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use unitas_core::EntityId;
use unitas_physics::BodyRole;
use unitas_runtime::{Callbacks, Runtime};

const SAMPLE_SCENE_RELATIVE: &str = "../../samples/RollABall/Assets/Scenes/Main.unity";
const TRIGGER_FIXTURE_RELATIVE: &str = "../unitas-asset/tests/fixtures/physics_trigger.unity";
const BALL_NAME: &str = "Ball";
const ARENA_HALF_EXTENT: f64 = 5.0;
const BALL_RADIUS: f64 = 0.5;
const DRIVE_SPEED: f64 = 6.0;
const REACH_DISTANCE: f64 = 0.3;
const CONTAINMENT_TOLERANCE: f64 = 0.3;
const FRAME_DELTAS: [f64; 3] = [0.0166, 0.033, 0.008];
const MAX_TICKS_PER_PICKUP: usize = 4000;
const MAX_FIXTURE_TICKS: usize = 400;
const PHASE_ENTER: u32 = 0;
const PHASE_EXIT: u32 = 2;
const POSITION_TOLERANCE: f64 = 1e-9;
const PICKUP_POSITIONS: [[f64; 3]; 4] = [
    [0.0, 0.5, 3.0],
    [3.0, 0.5, 0.0],
    [0.0, 0.5, -3.0],
    [-3.0, 0.5, 0.0],
];

static SCENARIO_EVENTS: Mutex<Vec<(u64, u64, u32)>> = Mutex::new(Vec::new());
static SCENARIO_RUN_LOCK: Mutex<()> = Mutex::new(());

extern "C" fn scenario_create_script(_entity: u64, _guid_hi: u64, _guid_lo: u64) -> u64 {
    1
}

extern "C" fn scenario_noop_handle(_handle: u64) {}

extern "C" fn scenario_noop_handle_delta(_handle: u64, _delta: f64) {}

extern "C" fn scenario_noop_collision(_a: u64, _b: u64, _phase: u32) {}

extern "C" fn scenario_record_trigger(a: u64, b: u64, phase: u32) {
    if let Ok(mut events) = SCENARIO_EVENTS.lock() {
        events.push((a, b, phase));
    }
}

fn scenario_callbacks() -> Callbacks {
    Callbacks {
        create_script: scenario_create_script,
        awake: scenario_noop_handle,
        on_enable: scenario_noop_handle,
        start: scenario_noop_handle,
        fixed_update: scenario_noop_handle_delta,
        update: scenario_noop_handle_delta,
        late_update: scenario_noop_handle_delta,
        on_disable: scenario_noop_handle,
        destroy: scenario_noop_handle,
        collision: scenario_noop_collision,
        trigger: scenario_record_trigger,
    }
}

fn lock_scenario() -> Result<std::sync::MutexGuard<'static, ()>, Box<dyn Error>> {
    match SCENARIO_RUN_LOCK.lock() {
        Ok(guard) => Ok(guard),
        Err(_) => Err(String::from("the scenario run lock is poisoned").into()),
    }
}

fn clear_events() {
    if let Ok(mut events) = SCENARIO_EVENTS.lock() {
        events.clear();
    }
}

fn drain_events() -> Vec<(u64, u64, u32)> {
    match SCENARIO_EVENTS.lock() {
        Ok(mut events) => std::mem::take(&mut *events),
        Err(_) => Vec::new(),
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sample_scene_path() -> Option<PathBuf> {
    let path = crate_root().join(SAMPLE_SCENE_RELATIVE);
    if path.is_file() { Some(path) } else { None }
}

fn trigger_fixture_path() -> PathBuf {
    crate_root().join(TRIGGER_FIXTURE_RELATIVE)
}

#[derive(Clone)]
struct BodyInfo {
    entity: EntityId,
    name: String,
    role: Option<BodyRole>,
    trigger: bool,
    position: Option<[f64; 3]>,
}

fn scene_bodies(runtime: &Runtime, path: &Path) -> Result<Vec<BodyInfo>, Box<dyn Error>> {
    let assembled = unitas_asset::scene::assemble_scene_file_bundle(path)?;
    let mut triggers: BTreeMap<String, bool> = BTreeMap::new();
    for binding in &assembled.physics {
        let trigger = binding
            .sphere_collider
            .as_ref()
            .is_some_and(|c| c.is_trigger)
            || binding.box_collider.as_ref().is_some_and(|c| c.is_trigger)
            || binding
                .capsule_collider
                .as_ref()
                .is_some_and(|c| c.is_trigger);
        if let Some(entity) = assembled.graph.entity(binding.entity) {
            triggers.insert(entity.name.clone(), trigger);
        }
    }
    let graph = match runtime.scene_graph() {
        Some(graph) => graph,
        None => return Err(String::from("the runtime loaded no scene graph").into()),
    };
    let mut bodies = Vec::new();
    for id in graph.breadth_first_order() {
        let entity = match graph.entity(id) {
            Some(entity) => entity,
            None => continue,
        };
        bodies.push(BodyInfo {
            entity: id,
            name: entity.name.clone(),
            role: runtime.body_role(id),
            trigger: triggers.get(&entity.name).copied().unwrap_or(false),
            position: runtime.body_position(id),
        });
    }
    Ok(bodies)
}

fn describe_bodies(bodies: &[BodyInfo]) -> String {
    let mut text = String::new();
    for body in bodies {
        let role = match body.role {
            Some(BodyRole::Dynamic) => "dynamic",
            Some(BodyRole::Kinematic) => "kinematic",
            Some(BodyRole::Static) => "static",
            None => "no-body",
        };
        let kind = if body.trigger { "trigger" } else { "solid" };
        text.push_str(&format!("{}={role}/{kind} ", body.name));
    }
    text
}

fn select_ball(bodies: &[BodyInfo]) -> Result<EntityId, Box<dyn Error>> {
    let dynamic: Vec<&BodyInfo> = bodies
        .iter()
        .filter(|body| body.role == Some(BodyRole::Dynamic) && !body.trigger)
        .collect();
    if let Some(named) = bodies.iter().find(|body| body.name == BALL_NAME) {
        return Ok(named.entity);
    }
    if let Some(named) = dynamic
        .iter()
        .find(|body| body.name.to_lowercase().contains("ball"))
    {
        return Ok(named.entity);
    }
    if dynamic.len() == 1 {
        return Ok(dynamic[0].entity);
    }
    Err(format!(
        "the scene has no unambiguous physics ball: {}",
        describe_bodies(bodies)
    )
    .into())
}

#[derive(Clone, Copy)]
struct Arena {
    min_x: f64,
    max_x: f64,
    min_z: f64,
    max_z: f64,
    floor_y: f64,
}

fn arena_from_bodies(bodies: &[BodyInfo]) -> Option<Arena> {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    let mut floor_y = f64::INFINITY;
    let mut found = false;
    for body in bodies {
        let Some(position) = body.position else {
            continue;
        };
        if body.trigger {
            continue;
        }
        found = true;
        min_x = min_x.min(position[0]);
        max_x = max_x.max(position[0]);
        min_z = min_z.min(position[2]);
        max_z = max_z.max(position[2]);
        floor_y = floor_y.min(position[1]);
    }
    if !found {
        return None;
    }
    Some(Arena {
        min_x,
        max_x,
        min_z,
        max_z,
        floor_y,
    })
}

struct SceneBuilder {
    yaml: String,
    next_id: u64,
}

impl SceneBuilder {
    fn new() -> Self {
        Self {
            yaml: String::from("%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n"),
            next_id: 100,
        }
    }

    fn game_object(&mut self, name: &str, position: [f64; 3]) -> u64 {
        let game_object = self.next_id;
        let transform = self.next_id + 1;
        self.next_id += 2;
        self.yaml.push_str(&format!(
            "--- !u!1 &{game_object}\nGameObject:\n  m_Name: {name}\n  m_Layer: 0\n"
        ));
        self.yaml.push_str(&format!(
            "--- !u!4 &{transform}\nTransform:\n  m_GameObject: {{fileID: {game_object}}}\n  \
             m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  \
             m_LocalPosition: {{x: {x}, y: {y}, z: {z}}}\n  \
             m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Children: []\n  m_Father: {{fileID: 0}}\n",
            x = position[0],
            y = position[1],
            z = position[2]
        ));
        game_object
    }

    fn box_collider(&mut self, game_object: u64, size: [f64; 3], trigger: bool) {
        let collider = self.next_id;
        self.next_id += 1;
        let is_trigger = u8::from(trigger);
        self.yaml.push_str(&format!(
            "--- !u!65 &{collider}\nBoxCollider:\n  m_GameObject: {{fileID: {game_object}}}\n  \
             m_IsTrigger: {is_trigger}\n  m_Size: {{x: {x}, y: {y}, z: {z}}}\n  \
             m_Center: {{x: 0, y: 0, z: 0}}\n",
            x = size[0],
            y = size[1],
            z = size[2]
        ));
    }

    fn sphere_collider(&mut self, game_object: u64, radius: f64, trigger: bool) {
        let collider = self.next_id;
        self.next_id += 1;
        let is_trigger = u8::from(trigger);
        self.yaml.push_str(&format!(
            "--- !u!135 &{collider}\nSphereCollider:\n  m_GameObject: {{fileID: {game_object}}}\n  \
             m_IsTrigger: {is_trigger}\n  m_Radius: {radius}\n  m_Center: {{x: 0, y: 0, z: 0}}\n"
        ));
    }

    fn rigidbody(&mut self, game_object: u64, gravity: bool) {
        let rigidbody = self.next_id;
        self.next_id += 1;
        let use_gravity = u8::from(gravity);
        self.yaml.push_str(&format!(
            "--- !u!54 &{rigidbody}\nRigidbody:\n  m_GameObject: {{fileID: {game_object}}}\n  \
             m_Mass: 1\n  m_UseGravity: {use_gravity}\n  m_IsKinematic: 0\n"
        ));
    }
}

fn arena_scene_yaml() -> String {
    let mut builder = SceneBuilder::new();
    let floor = builder.game_object("Floor", [0.0, -0.5, 0.0]);
    builder.box_collider(floor, [12.0, 1.0, 12.0], false);
    let north = builder.game_object("WallNorth", [0.0, 0.5, -5.5]);
    builder.box_collider(north, [12.0, 1.0, 1.0], false);
    let south = builder.game_object("WallSouth", [0.0, 0.5, 5.5]);
    builder.box_collider(south, [12.0, 1.0, 1.0], false);
    let east = builder.game_object("WallEast", [5.5, 0.5, 0.0]);
    builder.box_collider(east, [1.0, 1.0, 12.0], false);
    let west = builder.game_object("WallWest", [-5.5, 0.5, 0.0]);
    builder.box_collider(west, [1.0, 1.0, 12.0], false);
    let ball = builder.game_object(BALL_NAME, [0.0, 0.5, 0.0]);
    builder.rigidbody(ball, true);
    builder.sphere_collider(ball, BALL_RADIUS, false);
    for (index, position) in PICKUP_POSITIONS.iter().enumerate() {
        let pickup = builder.game_object(&format!("Pickup{index}"), *position);
        builder.box_collider(pickup, [1.0, 1.0, 1.0], true);
    }
    builder.yaml
}

fn write_temp_scene(label: &str, yaml: &str) -> Result<PathBuf, Box<dyn Error>> {
    let directory =
        std::env::temp_dir().join(format!("unitas_roll_a_ball_{label}_{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let path = directory.join("arena.unity");
    std::fs::write(&path, yaml)?;
    Ok(path)
}

struct Scenario {
    runtime: Runtime,
    ball: EntityId,
    pickups: Vec<EntityId>,
    collected: BTreeSet<EntityId>,
    enter_counts: BTreeMap<EntityId, usize>,
    bodies: Vec<BodyInfo>,
    arena: Option<Arena>,
}

impl Scenario {
    fn load(path: &Path, arena: Option<Arena>) -> Result<Self, Box<dyn Error>> {
        let mut runtime = Runtime::new()?;
        runtime.load_scene(path)?;
        runtime.set_callbacks(scenario_callbacks());
        let bodies = scene_bodies(&runtime, path)?;
        let ball = select_ball(&bodies)?;
        let pickups: Vec<EntityId> = bodies
            .iter()
            .filter(|body| body.trigger)
            .map(|body| body.entity)
            .collect();
        let arena = arena.or_else(|| arena_from_bodies(&bodies));
        Ok(Self {
            runtime,
            ball,
            pickups,
            collected: BTreeSet::new(),
            enter_counts: BTreeMap::new(),
            bodies,
            arena,
        })
    }

    fn restart(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        self.runtime.load_scene(path)?;
        self.bodies = scene_bodies(&self.runtime, path)?;
        self.ball = select_ball(&self.bodies)?;
        self.pickups = self
            .bodies
            .iter()
            .filter(|body| body.trigger)
            .map(|body| body.entity)
            .collect();
        self.collected.clear();
        self.enter_counts.clear();
        Ok(())
    }

    fn score(&self) -> usize {
        self.collected.len()
    }

    fn enter_count(&self, pickup: EntityId) -> usize {
        self.enter_counts.get(&pickup).copied().unwrap_or(0)
    }

    fn body_position(&self, entity: EntityId) -> Result<[f64; 3], Box<dyn Error>> {
        match self.runtime.body_position(entity) {
            Some(position) => Ok(position),
            None => Err(String::from("the runtime reports no body position").into()),
        }
    }

    fn record_event(&mut self, a: u64, b: u64, phase: u32) {
        if phase != PHASE_ENTER {
            return;
        }
        let first = EntityId::new(a);
        let second = EntityId::new(b);
        let pickup = if first == self.ball {
            Some(second)
        } else if second == self.ball {
            Some(first)
        } else {
            None
        };
        let pickup = match pickup {
            Some(pickup) if self.pickups.contains(&pickup) => pickup,
            _ => return,
        };
        *self.enter_counts.entry(pickup).or_insert(0) += 1;
        self.collected.insert(pickup);
    }

    fn assert_contained(&self, position: [f64; 3]) {
        let Some(arena) = self.arena else {
            return;
        };
        assert!(
            position[1] >= arena.floor_y - 1.0,
            "the ball fell through the arena floor at {position:?}: {}",
            describe_bodies(&self.bodies)
        );
        assert!(
            (arena.min_x - CONTAINMENT_TOLERANCE..=arena.max_x + CONTAINMENT_TOLERANCE)
                .contains(&position[0])
                && (arena.min_z - CONTAINMENT_TOLERANCE..=arena.max_z + CONTAINMENT_TOLERANCE)
                    .contains(&position[2]),
            "the ball left the arena at {position:?}: {}",
            describe_bodies(&self.bodies)
        );
    }

    fn tick(&mut self, delta: f64) -> Result<(), Box<dyn Error>> {
        self.runtime.tick(delta)?;
        for (a, b, phase) in drain_events() {
            self.record_event(a, b, phase);
        }
        let position = self.body_position(self.ball)?;
        self.assert_contained(position);
        Ok(())
    }

    fn steer(&mut self, target: EntityId) -> Result<(), Box<dyn Error>> {
        let position = self.body_position(self.ball)?;
        let destination = self.body_position(target)?;
        let offset_x = destination[0] - position[0];
        let offset_z = destination[2] - position[2];
        let distance = (offset_x * offset_x + offset_z * offset_z).sqrt();
        if distance <= REACH_DISTANCE {
            self.runtime.set_body_velocity(self.ball, [0.0; 3])?;
            return Ok(());
        }
        let velocity = self.runtime.body_velocity(self.ball).unwrap_or([0.0; 3]);
        self.runtime.set_body_velocity(
            self.ball,
            [
                offset_x / distance * DRIVE_SPEED,
                velocity[1],
                offset_z / distance * DRIVE_SPEED,
            ],
        )?;
        Ok(())
    }

    fn drive_to(&mut self, pickup: EntityId, delta: f64) -> Result<(), Box<dyn Error>> {
        for _ in 0..MAX_TICKS_PER_PICKUP {
            self.tick(delta)?;
            if self.collected.contains(&pickup) {
                self.runtime.set_body_velocity(self.ball, [0.0; 3])?;
                return Ok(());
            }
            self.steer(pickup)?;
        }
        Err(format!(
            "the ball did not reach a pickup within {MAX_TICKS_PER_PICKUP} ticks: {}",
            describe_bodies(&self.bodies)
        )
        .into())
    }

    fn run(&mut self, delta: f64) -> Result<(), Box<dyn Error>> {
        let pickups = self.pickups.clone();
        for pickup in pickups {
            self.drive_to(pickup, delta)?;
        }
        self.runtime.set_body_velocity(self.ball, [0.0; 3])?;
        for _ in 0..20 {
            self.tick(delta)?;
        }
        Ok(())
    }
}

#[test]
fn roll_a_ball_fixture_harness_collects_the_trigger_zone_once() -> Result<(), Box<dyn Error>> {
    let _guard = lock_scenario()?;
    clear_events();
    let path = trigger_fixture_path();
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&path)?;
    runtime.set_callbacks(scenario_callbacks());
    let bodies = scene_bodies(&runtime, &path)?;
    let ball = select_ball(&bodies)?;
    let pickups: Vec<EntityId> = bodies
        .iter()
        .filter(|body| body.trigger)
        .map(|body| body.entity)
        .collect();
    assert!(
        !pickups.is_empty(),
        "the trigger fixture must declare pickup triggers: {}",
        describe_bodies(&bodies)
    );
    let mut enters = 0usize;
    let mut exits = 0usize;
    for _ in 0..MAX_FIXTURE_TICKS {
        runtime.tick(0.02)?;
        for (a, b, phase) in drain_events() {
            let first = EntityId::new(a);
            let second = EntityId::new(b);
            if first != ball && second != ball {
                continue;
            }
            if phase == PHASE_ENTER {
                enters += 1;
            }
            if phase == PHASE_EXIT {
                exits += 1;
            }
        }
        if exits > 0 {
            break;
        }
    }
    let velocity = match runtime.body_velocity(ball) {
        Some(velocity) => velocity,
        None => return Err(String::from("the fixture ball has no body").into()),
    };
    assert!(
        velocity[1] < 0.0,
        "the real tick must integrate gravity for the fixture ball: {velocity:?}"
    );
    assert_eq!(
        enters,
        1,
        "the ball must enter the fixture pickup exactly once: {}",
        describe_bodies(&bodies)
    );
    assert_eq!(
        exits,
        1,
        "the ball must leave the fixture pickup exactly once: {}",
        describe_bodies(&bodies)
    );
    Ok(())
}

#[test]
fn roll_a_ball_scenario_collects_each_pickup_exactly_once_at_every_frame_rate()
-> Result<(), Box<dyn Error>> {
    let _guard = lock_scenario()?;
    let yaml = arena_scene_yaml();
    let path = write_temp_scene("scenario", &yaml)?;
    let mut failures = Vec::new();
    for delta in FRAME_DELTAS {
        clear_events();
        let arena = Some(Arena {
            min_x: -ARENA_HALF_EXTENT,
            max_x: ARENA_HALF_EXTENT,
            min_z: -ARENA_HALF_EXTENT,
            max_z: ARENA_HALF_EXTENT,
            floor_y: -0.5,
        });
        let mut scenario = match Scenario::load(&path, arena) {
            Ok(scenario) => scenario,
            Err(error) => {
                failures.push(format!("delta {delta}: {error}"));
                continue;
            }
        };
        if let Err(error) = scenario.run(delta) {
            failures.push(format!("delta {delta}: {error}"));
            continue;
        }
        let pickups = scenario.pickups.clone();
        if pickups.len() != PICKUP_POSITIONS.len() {
            failures.push(format!(
                "delta {delta}: the arena declares {} pickups, expected {}",
                pickups.len(),
                PICKUP_POSITIONS.len()
            ));
        }
        for pickup in &pickups {
            let enters = scenario.enter_count(*pickup);
            if enters != 1 {
                failures.push(format!(
                    "delta {delta}: a pickup was entered {enters} times, expected exactly once"
                ));
            }
        }
        eprintln!(
            "delta {delta}: the score {} reached the pickup count {}",
            scenario.score(),
            pickups.len()
        );
        if scenario.score() != pickups.len() {
            failures.push(format!(
                "delta {delta}: the score {} did not reach the pickup count {}",
                scenario.score(),
                pickups.len()
            ));
        }
    }
    let _ = std::fs::remove_dir_all(path.parent().unwrap_or(Path::new(".")));
    assert!(failures.is_empty(), "{failures:?}");
    Ok(())
}

#[test]
fn roll_a_ball_scenario_restart_restores_the_initial_state() -> Result<(), Box<dyn Error>> {
    let _guard = lock_scenario()?;
    clear_events();
    let yaml = arena_scene_yaml();
    let path = write_temp_scene("restart", &yaml)?;
    let arena = Some(Arena {
        min_x: -ARENA_HALF_EXTENT,
        max_x: ARENA_HALF_EXTENT,
        min_z: -ARENA_HALF_EXTENT,
        max_z: ARENA_HALF_EXTENT,
        floor_y: -0.5,
    });
    let mut scenario = Scenario::load(&path, arena)?;
    scenario.run(0.0166)?;
    let pickups = scenario.pickups.clone();
    assert_eq!(
        pickups.len(),
        PICKUP_POSITIONS.len(),
        "the arena must declare every pickup"
    );
    assert_eq!(
        scenario.score(),
        pickups.len(),
        "the scenario must collect every pickup before the restart"
    );
    scenario.restart(&path)?;
    let position = scenario.body_position(scenario.ball)?;
    assert!(
        (position[0] - 0.0).abs() <= POSITION_TOLERANCE
            && (position[1] - 0.5).abs() <= POSITION_TOLERANCE
            && (position[2] - 0.0).abs() <= POSITION_TOLERANCE,
        "the restart must restore the ball position, got {position:?}"
    );
    let velocity = match scenario.runtime.body_velocity(scenario.ball) {
        Some(velocity) => velocity,
        None => return Err(String::from("the restarted ball has no body").into()),
    };
    assert_eq!(
        velocity, [0.0; 3],
        "the restart must clear the ball velocity, got {velocity:?}"
    );
    assert_eq!(scenario.score(), 0, "the restart must clear the score");
    assert!(
        scenario.collected.is_empty(),
        "the restart must clear the collected pickups"
    );
    for _ in 0..10 {
        scenario.tick(0.0166)?;
    }
    assert!(
        scenario.enter_counts.is_empty(),
        "the restart must not retain trigger state, got {:?}",
        scenario.enter_counts
    );
    let _ = std::fs::remove_dir_all(path.parent().unwrap_or(Path::new(".")));
    Ok(())
}

#[test]
fn roll_a_ball_sample_scene_runs_the_scenario_headless() -> Result<(), Box<dyn Error>> {
    let path = match sample_scene_path() {
        Some(path) => path,
        None => {
            eprintln!(
                "the sample scene is absent at {SAMPLE_SCENE_RELATIVE}: the sample binding waits \
                 for the sample content and the harness scenario covers the acceptance criteria"
            );
            return Ok(());
        }
    };
    let _guard = lock_scenario()?;
    clear_events();
    let mut scenario = Scenario::load(&path, None)?;
    eprintln!(
        "the sample scene bodies: {}",
        describe_bodies(&scenario.bodies)
    );
    assert!(
        !scenario.pickups.is_empty(),
        "the sample scene must declare pickup triggers: {}",
        describe_bodies(&scenario.bodies)
    );
    scenario.run(0.0166)?;
    let pickups = scenario.pickups.clone();
    for pickup in &pickups {
        let enters = scenario.enter_count(*pickup);
        assert_eq!(
            enters,
            1,
            "the sample pickup must be collected exactly once, got {enters}: {}",
            describe_bodies(&scenario.bodies)
        );
    }
    assert_eq!(
        scenario.score(),
        pickups.len(),
        "the sample score must reach the pickup count"
    );
    Ok(())
}
