//! The Unity 2022.3 player loop phase tree.
//!
//! The tree mirrors Unity's `UnityEngine.LowLevel.PlayerLoop` structure: a root
//! node with one child per player loop phase, and every phase holding the
//! subsystem nodes Unity's default loop runs inside it. The engine dispatches
//! its own work from the subsystem nodes, so a tree that drops a subsystem drops
//! the work that subsystem carried.

use std::collections::BTreeMap;
use std::sync::Arc;

/// The root node type, matching Unity's `UnityEngine.LowLevel.PlayerLoop`.
pub const ROOT_TYPE: &str = "UnityEngine.LowLevel.PlayerLoop";

/// The player loop phases, in Unity's order.
pub const INITIALIZATION: &str = "UnityEngine.PlayerLoop.Initialization";
pub const EARLY_UPDATE: &str = "UnityEngine.PlayerLoop.EarlyUpdate";
pub const FIXED_UPDATE: &str = "UnityEngine.PlayerLoop.FixedUpdate";
pub const PRE_UPDATE: &str = "UnityEngine.PlayerLoop.PreUpdate";
pub const UPDATE: &str = "UnityEngine.PlayerLoop.Update";
pub const PRE_LATE_UPDATE: &str = "UnityEngine.PlayerLoop.PreLateUpdate";
pub const POST_LATE_UPDATE: &str = "UnityEngine.PlayerLoop.PostLateUpdate";

/// The subsystem nodes the engine dispatches work from.
pub const PLAYER_UPDATE_TIME: &str = "UnityEngine.PlayerLoop.Initialization.PlayerUpdateTime";
pub const SYNCHRONIZE_INPUTS: &str = "UnityEngine.PlayerLoop.Initialization.SynchronizeInputs";
pub const PHYSICS_FIXED_UPDATE: &str = "UnityEngine.PlayerLoop.FixedUpdate.PhysicsFixedUpdate";
pub const PHYSICS_2D_FIXED_UPDATE: &str = "UnityEngine.PlayerLoop.FixedUpdate.Physics2DFixedUpdate";
pub const SCRIPT_RUN_BEHAVIOUR_FIXED_UPDATE: &str =
    "UnityEngine.PlayerLoop.FixedUpdate.ScriptRunBehaviourFixedUpdate";
pub const SCRIPT_RUN_BEHAVIOUR_UPDATE: &str =
    "UnityEngine.PlayerLoop.Update.ScriptRunBehaviourUpdate";
pub const LEGACY_ANIMATION_UPDATE: &str =
    "UnityEngine.PlayerLoop.PreLateUpdate.LegacyAnimationUpdate";
pub const VFX_UPDATE: &str = "UnityEngine.PlayerLoop.PostLateUpdate.VFXUpdate";
pub const UPDATE_AUDIO: &str = "UnityEngine.PlayerLoop.PostLateUpdate.UpdateAudio";
pub const UPDATE_CANVAS: &str = "UnityEngine.PlayerLoop.PostLateUpdate.UpdateCanvas";
pub const SCRIPT_RUN_DELAYED_TASKS: &str =
    "UnityEngine.PlayerLoop.PostLateUpdate.ScriptRunDelayedTasks";

/// The signature of a custom player loop system callback.
pub type PlayerLoopCallback = extern "C" fn(user_data: u64, delta_seconds: f64);

/// Where a custom system is inserted relative to the system it is anchored to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomSystemPosition {
    Before,
    After,
}

/// The failures the player loop API reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlayerLoopError {
    /// The anchored system is not part of the current tree.
    AnchorNotFound(String),
    /// The custom system type is already registered or names a built-in system.
    DuplicateType(String),
    /// The tree is not a usable player loop.
    InvalidTree(String),
}

impl std::fmt::Display for PlayerLoopError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerLoopError::AnchorNotFound(anchor) => {
                write!(formatter, "player loop anchor not found: {anchor}")
            }
            PlayerLoopError::DuplicateType(type_name) => {
                write!(
                    formatter,
                    "player loop system type already used: {type_name}"
                )
            }
            PlayerLoopError::InvalidTree(message) => {
                write!(formatter, "invalid player loop tree: {message}")
            }
        }
    }
}

impl std::error::Error for PlayerLoopError {}

/// One node of the player loop tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerLoopSystem {
    /// The human readable name of the node.
    pub name: String,
    /// The fully qualified Unity type name of the node.
    pub type_name: String,
    /// The systems this node runs.
    pub children: Vec<PlayerLoopSystem>,
}

impl PlayerLoopSystem {
    pub fn new(name: &str, type_name: &str) -> Self {
        Self {
            name: name.to_string(),
            type_name: type_name.to_string(),
            children: Vec::new(),
        }
    }

    /// The first node with the given type, anywhere in the tree.
    pub fn find(&self, type_name: &str) -> Option<&Self> {
        if self.type_name == type_name {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(type_name))
    }

    /// The first node with the given type, anywhere in the tree.
    pub fn find_mut(&mut self, type_name: &str) -> Option<&mut Self> {
        if self.type_name == type_name {
            return Some(self);
        }
        self.children
            .iter_mut()
            .find_map(|child| child.find_mut(type_name))
    }

    /// Whether the tree holds a node with the given type.
    pub fn contains(&self, type_name: &str) -> bool {
        self.find(type_name).is_some()
    }

    /// Removes the first node with the given type, with its children.
    pub fn remove(&mut self, type_name: &str) -> bool {
        if let Some(index) = self
            .children
            .iter()
            .position(|child| child.type_name == type_name)
        {
            self.children.remove(index);
            return true;
        }
        self.children
            .iter_mut()
            .any(|child| child.remove(type_name))
    }

    fn insert_sibling(
        &mut self,
        anchor_type: &str,
        new_node: &mut Option<PlayerLoopSystem>,
        position: CustomSystemPosition,
    ) -> bool {
        if let Some(index) = self
            .children
            .iter()
            .position(|child| child.type_name == anchor_type)
        {
            let Some(child) = new_node.take() else {
                return false;
            };
            let at = match position {
                CustomSystemPosition::Before => index,
                CustomSystemPosition::After => index + 1,
            };
            self.children.insert(at, child);
            return true;
        }
        self.children
            .iter_mut()
            .any(|child| child.insert_sibling(anchor_type, new_node, position))
    }
}

/// The engine work a built-in subsystem node carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinWork {
    PlayerUpdateTime,
    SynchronizeInputs,
    PhysicsFixedUpdate,
    Physics2DFixedUpdate,
    ScriptRunBehaviourFixedUpdate,
    ScriptRunBehaviourUpdate,
    LegacyAnimationUpdate,
    VFXUpdate,
    UpdateAudio,
    UpdateCanvas,
    ScriptLateUpdate,
    InitPendingScripts,
}

/// The work a node of the compiled tree carries.
#[derive(Clone, Debug)]
pub(crate) enum PlayerLoopStep {
    Builtin(BuiltinWork),
    Custom {
        callback: PlayerLoopCallback,
        user_data: u64,
    },
    /// The fixed update phase: its systems run once per fixed step.
    FixedPhase {
        steps: Vec<PlayerLoopStep>,
    },
}

#[derive(Clone, Copy)]
struct CustomSystem {
    callback: PlayerLoopCallback,
    user_data: u64,
}

/// The player loop tree the runtime walks every frame.
pub struct PlayerLoop {
    root: PlayerLoopSystem,
    customs: BTreeMap<String, CustomSystem>,
    plan: Arc<Vec<PlayerLoopStep>>,
}

impl Default for PlayerLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerLoop {
    /// A player loop holding Unity's default phase tree.
    pub fn new() -> Self {
        let root = default_tree();
        let plan = Arc::new(compile(&root, &BTreeMap::new()));
        Self {
            root,
            customs: BTreeMap::new(),
            plan,
        }
    }

    /// The current tree.
    pub fn root(&self) -> &PlayerLoopSystem {
        &self.root
    }

    /// Replaces the current tree. The custom systems stay registered, so a tree
    /// that keeps their nodes keeps running them.
    pub fn set_root(&mut self, root: PlayerLoopSystem) -> Result<(), PlayerLoopError> {
        if root.type_name.is_empty() {
            return Err(PlayerLoopError::InvalidTree(String::from(
                "the root node needs a type name",
            )));
        }
        self.root = root;
        self.rebuild_plan();
        Ok(())
    }

    /// Registers a custom system next to the system it is anchored to.
    pub fn register_custom_system(
        &mut self,
        name: &str,
        type_name: &str,
        anchor_type: &str,
        position: CustomSystemPosition,
        callback: PlayerLoopCallback,
        user_data: u64,
    ) -> Result<(), PlayerLoopError> {
        if type_name.is_empty() {
            return Err(PlayerLoopError::InvalidTree(String::from(
                "a custom system needs a type name",
            )));
        }
        if builtin_work_for(type_name).is_some() || self.customs.contains_key(type_name) {
            return Err(PlayerLoopError::DuplicateType(type_name.to_string()));
        }
        let node = PlayerLoopSystem::new(name, type_name);
        if self.root.type_name == anchor_type {
            match position {
                CustomSystemPosition::Before => self.root.children.insert(0, node),
                CustomSystemPosition::After => self.root.children.push(node),
            }
        } else {
            let mut pending = Some(node);
            if !self
                .root
                .insert_sibling(anchor_type, &mut pending, position)
            {
                return Err(PlayerLoopError::AnchorNotFound(anchor_type.to_string()));
            }
        }
        self.customs.insert(
            type_name.to_string(),
            CustomSystem {
                callback,
                user_data,
            },
        );
        self.rebuild_plan();
        Ok(())
    }

    /// Removes a custom system and the node it runs in.
    pub fn unregister_custom_system(&mut self, type_name: &str) -> bool {
        let removed = self.customs.remove(type_name).is_some();
        self.root.remove(type_name);
        if removed {
            self.rebuild_plan();
        }
        removed
    }

    pub(crate) fn plan(&self) -> Arc<Vec<PlayerLoopStep>> {
        Arc::clone(&self.plan)
    }

    fn rebuild_plan(&mut self) {
        self.plan = Arc::new(compile(&self.root, &self.customs));
    }
}

/// The work a built-in system node carries.
pub fn builtin_work_for(type_name: &str) -> Option<BuiltinWork> {
    match type_name {
        PLAYER_UPDATE_TIME => Some(BuiltinWork::PlayerUpdateTime),
        SYNCHRONIZE_INPUTS => Some(BuiltinWork::SynchronizeInputs),
        PHYSICS_FIXED_UPDATE => Some(BuiltinWork::PhysicsFixedUpdate),
        PHYSICS_2D_FIXED_UPDATE => Some(BuiltinWork::Physics2DFixedUpdate),
        SCRIPT_RUN_BEHAVIOUR_FIXED_UPDATE => Some(BuiltinWork::ScriptRunBehaviourFixedUpdate),
        SCRIPT_RUN_BEHAVIOUR_UPDATE => Some(BuiltinWork::ScriptRunBehaviourUpdate),
        LEGACY_ANIMATION_UPDATE => Some(BuiltinWork::LegacyAnimationUpdate),
        VFX_UPDATE => Some(BuiltinWork::VFXUpdate),
        UPDATE_AUDIO => Some(BuiltinWork::UpdateAudio),
        UPDATE_CANVAS => Some(BuiltinWork::UpdateCanvas),
        SCRIPT_RUN_DELAYED_TASKS => Some(BuiltinWork::ScriptLateUpdate),
        _ => None,
    }
}

/// Unity's default phase tree.
///
/// The phase order and the subsystem type names follow Unity 2022.3. The three
/// post late update systems the engine drives are ordered the way the engine
/// ran them before the tree existed, so the migration keeps the observable
/// frame order.
pub fn default_tree() -> PlayerLoopSystem {
    let phases: [(&str, &[&str]); 7] = [
        (
            INITIALIZATION,
            &[
                "PlayerUpdateTime",
                "AsyncUploadTimeSlicedUpdate",
                "SynchronizeInputs",
                "SynchronizeState",
                "XREarlyUpdate",
            ],
        ),
        (
            EARLY_UPDATE,
            &[
                "PollPlayerConnection",
                "GpuTimestamp",
                "AnalyticsDirector",
                "AsyncUploadTimeSlicedUpdate",
                "UpdateRectTransform",
                "UpdateCanvasRectTransform",
                "ClearLines",
                "DirectorUpdateAnimationBegin",
                "LegacyAnimationUpdate",
                "DirectorUpdateAnimationEnd",
                "DirectorDeferredEvaluate",
                "AudioUpdate",
                "ScriptRunBehaviourEarlyUpdate",
            ],
        ),
        (
            FIXED_UPDATE,
            &[
                "PhysicsFixedUpdate",
                "Physics2DFixedUpdate",
                "ScriptRunBehaviourFixedUpdate",
                "DirectorFixedUpdate",
                "LegacyFixedAnimationUpdate",
                "XRFixedUpdate",
                "NewInputFixedUpdate",
            ],
        ),
        (
            PRE_UPDATE,
            &["AIUpdate", "NewInputUpdate", "ScriptRunBehaviourPreUpdate"],
        ),
        (
            UPDATE,
            &[
                "ScriptRunBehaviourUpdate",
                "ScriptRunDelayedTasks",
                "DirectorUpdate",
                "AudioUpdate",
            ],
        ),
        (
            PRE_LATE_UPDATE,
            &[
                "DirectorUpdateAnimationBegin",
                "LegacyAnimationUpdate",
                "DirectorUpdateAnimationEnd",
                "DirectorDeferredEvaluate",
                "ScriptRunBehaviourLateUpdate",
                "NewInputLateUpdate",
            ],
        ),
        (
            POST_LATE_UPDATE,
            &[
                "PlayerSendFrameStarted",
                "UpdateRectTransform",
                "UpdateCanvasRectTransform",
                "VFXUpdate",
                "UpdateAudio",
                "UpdateCanvas",
                "UpdateVideo",
                "NewInputPostLateUpdate",
                "DirectorLateUpdate",
                "ScriptRunDelayedTasks",
            ],
        ),
    ];
    let mut root = PlayerLoopSystem::new("PlayerLoop", ROOT_TYPE);
    for (phase, systems) in phases {
        let mut node = PlayerLoopSystem::new(short_name(phase), phase);
        for system in systems {
            node.children
                .push(PlayerLoopSystem::new(system, &format!("{phase}.{system}")));
        }
        root.children.push(node);
    }
    root
}

fn short_name(type_name: &str) -> &str {
    type_name.rsplit('.').next().unwrap_or(type_name)
}

/// Compiles the tree into the flat list of steps the runtime walks.
fn compile(
    root: &PlayerLoopSystem,
    customs: &BTreeMap<String, CustomSystem>,
) -> Vec<PlayerLoopStep> {
    let mut steps = Vec::new();
    compile_node(root, customs, &mut steps);
    steps
}

fn compile_node(
    node: &PlayerLoopSystem,
    customs: &BTreeMap<String, CustomSystem>,
    steps: &mut Vec<PlayerLoopStep>,
) {
    if node.type_name == FIXED_UPDATE {
        let mut inner = Vec::new();
        for child in &node.children {
            compile_node(child, customs, &mut inner);
        }
        steps.push(PlayerLoopStep::FixedPhase { steps: inner });
        steps.push(PlayerLoopStep::Builtin(BuiltinWork::InitPendingScripts));
        return;
    }
    if let Some(work) = builtin_work_for(&node.type_name) {
        steps.push(PlayerLoopStep::Builtin(work));
    } else if let Some(custom) = customs.get(&node.type_name) {
        steps.push(PlayerLoopStep::Custom {
            callback: custom.callback,
            user_data: custom.user_data,
        });
    }
    for child in &node.children {
        compile_node(child, customs, steps);
    }
    if node.type_name == POST_LATE_UPDATE {
        steps.push(PlayerLoopStep::Builtin(BuiltinWork::InitPendingScripts));
    }
}

const TREE_MAGIC: u32 = 0x314C_5055;
const TREE_VERSION: u32 = 1;

/// Serializes a tree into the byte form the managed bridge round trips.
pub fn serialize(root: &PlayerLoopSystem) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&TREE_MAGIC.to_le_bytes());
    bytes.extend_from_slice(&TREE_VERSION.to_le_bytes());
    bytes.extend_from_slice(&count_nodes(root).to_le_bytes());
    serialize_node(root, &mut bytes);
    bytes
}

fn count_nodes(node: &PlayerLoopSystem) -> u32 {
    let mut count = 1u32;
    for child in &node.children {
        count += count_nodes(child);
    }
    count
}

fn serialize_node(node: &PlayerLoopSystem, bytes: &mut Vec<u8>) {
    write_bytes(&node.name, bytes);
    write_bytes(&node.type_name, bytes);
    bytes.extend_from_slice(&(node.children.len() as u32).to_le_bytes());
    for child in &node.children {
        serialize_node(child, bytes);
    }
}

fn write_bytes(value: &str, bytes: &mut Vec<u8>) {
    let raw = value.as_bytes();
    bytes.extend_from_slice(&(raw.len() as u32).to_le_bytes());
    bytes.extend_from_slice(raw);
}

/// Reads a tree back from the byte form [`serialize`] writes.
pub fn deserialize(bytes: &[u8]) -> Option<PlayerLoopSystem> {
    let mut cursor = 0usize;
    let magic = read_u32(bytes, &mut cursor)?;
    if magic != TREE_MAGIC {
        return None;
    }
    let version = read_u32(bytes, &mut cursor)?;
    if version != TREE_VERSION {
        return None;
    }
    let nodes = read_u32(bytes, &mut cursor)?;
    let root = read_node(bytes, &mut cursor)?;
    if count_nodes(&root) != nodes {
        return None;
    }
    Some(root)
}

fn read_node(bytes: &[u8], cursor: &mut usize) -> Option<PlayerLoopSystem> {
    let name = read_string(bytes, cursor)?;
    let type_name = read_string(bytes, cursor)?;
    let child_count = read_u32(bytes, cursor)?;
    let mut children = Vec::new();
    for _ in 0..child_count {
        children.push(read_node(bytes, cursor)?);
    }
    Some(PlayerLoopSystem {
        name,
        type_name,
        children,
    })
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Option<u32> {
    let end = cursor.checked_add(4)?;
    let raw = bytes.get(*cursor..end)?;
    *cursor = end;
    Some(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn read_string(bytes: &[u8], cursor: &mut usize) -> Option<String> {
    let length = read_u32(bytes, cursor)? as usize;
    let end = cursor.checked_add(length)?;
    let raw = bytes.get(*cursor..end)?;
    *cursor = end;
    String::from_utf8(raw.to_vec()).ok()
}
