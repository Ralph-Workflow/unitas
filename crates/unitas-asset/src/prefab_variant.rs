use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::Path;

use crate::prefab::{Modification, PrefabInstance, PrefabSource, PrefabSourceReference};
use crate::scene_save::{self, SceneSaveError};
use crate::unity_yaml::{self, Header, Node, Stream, YamlError};

#[path = "prefab_variant_resolve.rs"]
mod resolve;
#[path = "prefab_variant_yaml.rs"]
mod yaml;

pub use self::resolve::{
    BrokenPrefabState, OrphanedOverride, OrphanedOverrideResolution, OverrideLevel,
    ResolvedComponent, ResolvedHierarchy, ResolvedNestedPrefab, ResolvedNode, ResolvedPrefab,
    UnpackMode, UnpackedPrefabHierarchy, ValueProvenanceEntry,
};

use self::yaml::{
    emit_prefab_node_documents, parse_base_nodes_from_stream, parse_prefab_instance_from_stream,
    set_property, upsert_prefab_instance_in_stream, DEFAULT_VARIANT_INSTANCE_FILE_ID,
    GAME_OBJECT_CLASS_ID, PREFAB_INSTANCE_CLASS_ID,
};

const UNRECORDABLE_PROPERTIES: [(&str, &str); 8] = [
    ("m_ObjectHideFlags", "immutable object hide flags"),
    ("fileID", "immutable local file identifier"),
    ("m_Script", "immutable script asset binding"),
    (
        "m_CorrespondingSourceObject",
        "immutable prefab source correspondence",
    ),
    ("m_PrefabInstance", "immutable prefab instance link"),
    ("m_PrefabAsset", "immutable prefab asset link"),
    ("m_GameObject", "immutable component owner reference"),
    (
        "m_Father",
        "structural transform parent cannot be recorded as a property override",
    ),
];

const UNRECORDABLE_PREFIXES: [(&str, &str); 3] = [
    (
        "m_Children",
        "source child ordering cannot be overridden in an instance or variant",
    ),
    (
        "m_Component",
        "component table cannot be mutated via property override",
    ),
    (
        "cross_prefab_ref:",
        "cross-prefab non-serializable reference",
    ),
];

const CROSS_SCENE_TOKEN: &str = "cross_scene:";
const NON_SERIALIZABLE_TOKEN: &str = "non_serializable:";
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceObjectId {
    pub source_guid: String,
    pub target_file_id: i64,
}

impl SourceObjectId {
    pub fn new(source_guid: impl Into<String>, target_file_id: i64) -> Self {
        Self {
            source_guid: source_guid.into(),
            target_file_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefabComponentRecord {
    pub file_id: i64,
    pub class_name: String,
    pub properties: BTreeMap<String, Node>,
}

impl PrefabComponentRecord {
    pub fn new(file_id: i64, class_name: impl Into<String>) -> Self {
        Self {
            file_id,
            class_name: class_name.into(),
            properties: BTreeMap::new(),
        }
    }

    pub fn with_property(
        mut self,
        property_path: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.properties
            .insert(property_path.into(), Node::Scalar(value.into()));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefabNodeRecord {
    pub file_id: i64,
    pub name: String,
    pub parent_file_id: Option<i64>,
    pub children: Vec<i64>,
    pub properties: BTreeMap<String, Node>,
    pub components: Vec<PrefabComponentRecord>,
}

impl PrefabNodeRecord {
    pub fn new(file_id: i64, name: impl Into<String>, parent_file_id: Option<i64>) -> Self {
        let name_str = name.into();
        let mut properties = BTreeMap::new();
        properties.insert(String::from("m_Name"), Node::Scalar(name_str.clone()));
        Self {
            file_id,
            name: name_str,
            parent_file_id,
            children: Vec::new(),
            properties,
            components: Vec::new(),
        }
    }

    pub fn with_children(mut self, children: Vec<i64>) -> Self {
        self.children = children;
        self
    }

    pub fn with_property(
        mut self,
        property_path: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        let key = property_path.into();
        let val = value.into();
        if key == "m_Name" {
            self.name = val.clone();
        }
        self.properties.insert(key, Node::Scalar(val));
        self
    }

    pub fn with_component(mut self, component: PrefabComponentRecord) -> Self {
        self.components.push(component);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedChildOverride {
    pub parent_target: SourceObjectId,
    pub child: PrefabNodeRecord,
    pub sibling_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedComponentOverride {
    pub owner_target: SourceObjectId,
    pub component: PrefabComponentRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedPrefabReference {
    pub instance_file_id: i64,
    pub parent_target_file_id: i64,
    pub source: PrefabSource,
    pub modifications: Vec<Modification>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StructuralOverrides {
    pub added_children: Vec<AddedChildOverride>,
    pub removed_children: Vec<SourceObjectId>,
    pub added_components: Vec<AddedComponentOverride>,
    pub removed_components: Vec<SourceObjectId>,
    pub added_sibling_order: BTreeMap<SourceObjectId, Vec<i64>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetedModification {
    pub target: SourceObjectId,
    pub property_path: String,
    pub value: String,
    pub object_reference: Option<PrefabSourceReference>,
}

impl TargetedModification {
    pub fn new(
        target: SourceObjectId,
        property_path: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            target,
            property_path: property_path.into(),
            value: value.into(),
            object_reference: None,
        }
    }

    pub fn with_object_reference(
        target: SourceObjectId,
        property_path: impl Into<String>,
        object_reference: PrefabSourceReference,
    ) -> Self {
        Self {
            target,
            property_path: property_path.into(),
            value: String::new(),
            object_reference: Some(object_reference),
        }
    }

    pub fn from_modification(source_guid: impl Into<String>, modification: &Modification) -> Self {
        Self {
            target: SourceObjectId::new(source_guid, modification.target_file_id),
            property_path: modification.property_path.clone(),
            value: modification.value.clone(),
            object_reference: modification.object_reference.clone(),
        }
    }

    pub fn to_modification(&self) -> Modification {
        Modification {
            target_file_id: self.target.target_file_id,
            property_path: self.property_path.clone(),
            value: self.value.clone(),
            object_reference: self.object_reference.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefabVariantDefinition {
    pub guid: String,
    pub name: String,
    pub base: Option<PrefabSource>,
    pub root_file_id: i64,
    pub nodes: Vec<PrefabNodeRecord>,
    pub modifications: Vec<Modification>,
    pub targeted_modifications: Vec<TargetedModification>,
    pub structural_overrides: StructuralOverrides,
    pub nested_prefabs: Vec<NestedPrefabReference>,
}

impl PrefabVariantDefinition {
    pub fn base_prefab(
        guid: impl Into<String>,
        name: impl Into<String>,
        root_file_id: i64,
        nodes: Vec<PrefabNodeRecord>,
    ) -> Self {
        Self {
            guid: guid.into(),
            name: name.into(),
            base: None,
            root_file_id,
            nodes,
            modifications: Vec::new(),
            targeted_modifications: Vec::new(),
            structural_overrides: StructuralOverrides::default(),
            nested_prefabs: Vec::new(),
        }
    }

    pub fn variant(guid: impl Into<String>, name: impl Into<String>, base: PrefabSource) -> Self {
        Self {
            guid: guid.into(),
            name: name.into(),
            base: Some(base),
            root_file_id: 0,
            nodes: Vec::new(),
            modifications: Vec::new(),
            targeted_modifications: Vec::new(),
            structural_overrides: StructuralOverrides::default(),
            nested_prefabs: Vec::new(),
        }
    }

    pub fn record_override(
        &mut self,
        modification: Modification,
    ) -> Result<(), PrefabVariantError> {
        validate_override(
            &modification.property_path,
            &modification.value,
            modification.target_file_id,
        )?;
        let default_source_guid = match &self.base {
            Some(base) => base.guid.clone(),
            None => self.guid.clone(),
        };
        self.upsert_raw_modification(modification.clone());
        self.upsert_targeted_modification(TargetedModification::from_modification(
            default_source_guid,
            &modification,
        ));
        Ok(())
    }

    pub fn record_targeted_override(
        &mut self,
        modification: TargetedModification,
    ) -> Result<(), PrefabVariantError> {
        validate_override(
            &modification.property_path,
            &modification.value,
            modification.target.target_file_id,
        )?;
        self.upsert_raw_modification(modification.to_modification());
        self.upsert_targeted_modification(modification);
        Ok(())
    }

    pub fn add_child_override(
        &mut self,
        parent_target: SourceObjectId,
        mut child: PrefabNodeRecord,
        sibling_index: Option<usize>,
    ) -> Result<(), PrefabVariantError> {
        child.parent_file_id = Some(parent_target.target_file_id);
        self.structural_overrides
            .added_children
            .push(AddedChildOverride {
                parent_target,
                child,
                sibling_index,
            });
        Ok(())
    }

    pub fn remove_child_override(&mut self, target: SourceObjectId) {
        if !self.structural_overrides.removed_children.contains(&target) {
            self.structural_overrides.removed_children.push(target);
        }
    }

    pub fn add_component_override(
        &mut self,
        owner_target: SourceObjectId,
        component: PrefabComponentRecord,
    ) {
        self.structural_overrides
            .added_components
            .push(AddedComponentOverride {
                owner_target,
                component,
            });
    }

    pub fn remove_component_override(&mut self, target: SourceObjectId) {
        if !self
            .structural_overrides
            .removed_components
            .contains(&target)
        {
            self.structural_overrides.removed_components.push(target);
        }
    }

    pub fn reorder_added_siblings(
        &mut self,
        parent_target: SourceObjectId,
        added_child_file_ids: Vec<i64>,
    ) -> Result<(), PrefabVariantError> {
        let known_added: HashSet<i64> = self
            .structural_overrides
            .added_children
            .iter()
            .filter(|added| added.parent_target == parent_target)
            .map(|added| added.child.file_id)
            .collect();
        for file_id in &added_child_file_ids {
            if !known_added.contains(file_id) {
                return Err(PrefabVariantError::UnrecordableOverride {
                    target_file_id: *file_id,
                    property_path: String::from("m_Children"),
                    reason: String::from(
                        "source children cannot be reordered in a variant or instance; only added children can be ordered",
                    ),
                });
            }
        }
        self.structural_overrides
            .added_sibling_order
            .insert(parent_target, added_child_file_ids);
        Ok(())
    }

    pub fn reject_source_child_reorder(
        &self,
        parent_file_id: i64,
    ) -> Result<(), PrefabVariantError> {
        Err(PrefabVariantError::UnrecordableOverride {
            target_file_id: parent_file_id,
            property_path: String::from("m_Children"),
            reason: String::from("reordering source children inside an instance or variant is not recordable by Unity"),
        })
    }

    pub fn set_base_property(
        &mut self,
        target_file_id: i64,
        property_path: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), PrefabVariantError> {
        let prop = property_path.into();
        let val = value.into();
        validate_override(&prop, &val, target_file_id)?;
        for node in &mut self.nodes {
            if node.file_id == target_file_id {
                if prop == "m_Name" {
                    node.name = val.clone();
                }
                set_property(&mut node.properties, &prop, Node::Scalar(val));
                return Ok(());
            }
            for component in &mut node.components {
                if component.file_id == target_file_id {
                    set_property(&mut component.properties, &prop, Node::Scalar(val));
                    return Ok(());
                }
            }
        }
        Err(PrefabVariantError::TargetNotFound { target_file_id })
    }

    pub fn reorder_source_children(
        &mut self,
        parent_file_id: i64,
        new_children_order: Vec<i64>,
    ) -> Result<(), PrefabVariantError> {
        for node in &mut self.nodes {
            if node.file_id == parent_file_id {
                node.children = new_children_order;
                return Ok(());
            }
        }
        Err(PrefabVariantError::TargetNotFound {
            target_file_id: parent_file_id,
        })
    }

    fn upsert_raw_modification(&mut self, modification: Modification) {
        if let Some(existing) = self.modifications.iter_mut().find(|item| {
            item.target_file_id == modification.target_file_id
                && item.property_path == modification.property_path
        }) {
            existing.value = modification.value;
            existing.object_reference = modification.object_reference;
        } else {
            self.modifications.push(modification);
        }
    }

    fn upsert_targeted_modification(&mut self, modification: TargetedModification) {
        if let Some(existing) = self.targeted_modifications.iter_mut().find(|item| {
            item.target.target_file_id == modification.target.target_file_id
                && item.property_path == modification.property_path
        }) {
            existing.target = modification.target;
            existing.value = modification.value;
            existing.object_reference = modification.object_reference;
        } else {
            self.targeted_modifications.push(modification);
        }
    }

    fn remove_modification(
        &mut self,
        target_file_id: i64,
        property_path: &str,
    ) -> Option<Modification> {
        let mut removed = None;
        self.modifications.retain(|item| {
            let matches =
                item.target_file_id == target_file_id && item.property_path == property_path;
            if matches {
                removed = Some(item.clone());
            }
            !matches
        });
        self.targeted_modifications.retain(|item| {
            let matches =
                item.target.target_file_id == target_file_id && item.property_path == property_path;
            if matches && removed.is_none() {
                removed = Some(item.to_modification());
            }
            !matches
        });
        removed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenePrefabInstance {
    pub instance_id: String,
    pub instance: PrefabInstance,
    pub targeted_modifications: Vec<TargetedModification>,
    pub structural_overrides: StructuralOverrides,
}

impl ScenePrefabInstance {
    pub fn new(instance_id: impl Into<String>, instance: PrefabInstance) -> Self {
        let source_guid = instance.source.guid.clone();
        let targeted_modifications = instance
            .modifications
            .iter()
            .map(|modification| TargetedModification::from_modification(&source_guid, modification))
            .collect();
        Self {
            instance_id: instance_id.into(),
            instance,
            targeted_modifications,
            structural_overrides: StructuralOverrides::default(),
        }
    }

    pub fn record_override(
        &mut self,
        modification: Modification,
    ) -> Result<(), PrefabVariantError> {
        validate_override(
            &modification.property_path,
            &modification.value,
            modification.target_file_id,
        )?;
        let source_guid = self.instance.source.guid.clone();
        if let Some(existing) = self.instance.modifications.iter_mut().find(|item| {
            item.target_file_id == modification.target_file_id
                && item.property_path == modification.property_path
        }) {
            existing.value = modification.value.clone();
            existing.object_reference = modification.object_reference.clone();
        } else {
            self.instance.modifications.push(modification.clone());
        }
        let targeted = TargetedModification::from_modification(source_guid, &modification);
        if let Some(existing) = self.targeted_modifications.iter_mut().find(|item| {
            item.target.target_file_id == targeted.target.target_file_id
                && item.property_path == targeted.property_path
        }) {
            existing.value = targeted.value;
            existing.object_reference = targeted.object_reference;
        } else {
            self.targeted_modifications.push(targeted);
        }
        Ok(())
    }

    pub fn add_child_override(
        &mut self,
        parent_target: SourceObjectId,
        mut child: PrefabNodeRecord,
        sibling_index: Option<usize>,
    ) {
        child.parent_file_id = Some(parent_target.target_file_id);
        self.structural_overrides
            .added_children
            .push(AddedChildOverride {
                parent_target,
                child,
                sibling_index,
            });
    }

    pub fn remove_child_override(&mut self, target: SourceObjectId) {
        if !self.structural_overrides.removed_children.contains(&target) {
            self.structural_overrides.removed_children.push(target);
        }
    }

    pub fn add_component_override(
        &mut self,
        owner_target: SourceObjectId,
        component: PrefabComponentRecord,
    ) {
        self.structural_overrides
            .added_components
            .push(AddedComponentOverride {
                owner_target,
                component,
            });
    }

    pub fn remove_component_override(&mut self, target: SourceObjectId) {
        if !self
            .structural_overrides
            .removed_components
            .contains(&target)
        {
            self.structural_overrides.removed_components.push(target);
        }
    }

    fn remove_modification(
        &mut self,
        target_file_id: i64,
        property_path: &str,
    ) -> Option<Modification> {
        let mut removed = None;
        self.instance.modifications.retain(|item| {
            let matches =
                item.target_file_id == target_file_id && item.property_path == property_path;
            if matches {
                removed = Some(item.clone());
            }
            !matches
        });
        self.targeted_modifications.retain(|item| {
            let matches =
                item.target.target_file_id == target_file_id && item.property_path == property_path;
            if matches && removed.is_none() {
                removed = Some(item.to_modification());
            }
            !matches
        });
        removed
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrefabVariantError {
    InheritanceCycle {
        chain: Vec<String>,
    },
    NestingCycle {
        chain: Vec<String>,
    },
    MissingBasePrefab {
        missing_guid: String,
        dependent_guid: String,
    },
    UnrecordableOverride {
        target_file_id: i64,
        property_path: String,
        reason: String,
    },
    InvalidTargetLevel {
        from_level: String,
        target_level: String,
        reason: String,
    },
    OverrideNotFound {
        level: String,
        target_file_id: i64,
        property_path: String,
    },
    TargetNotFound {
        target_file_id: i64,
    },
}

impl fmt::Display for PrefabVariantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrefabVariantError::InheritanceCycle { chain } => {
                write!(
                    formatter,
                    "prefab variant inheritance cycle detected: {}",
                    chain.join(" -> ")
                )
            }
            PrefabVariantError::NestingCycle { chain } => {
                write!(
                    formatter,
                    "prefab nesting cycle detected: {}",
                    chain.join(" -> ")
                )
            }
            PrefabVariantError::MissingBasePrefab {
                missing_guid,
                dependent_guid,
            } => {
                write!(
                    formatter,
                    "prefab {dependent_guid} references missing base prefab {missing_guid}"
                )
            }
            PrefabVariantError::UnrecordableOverride {
                target_file_id,
                property_path,
                reason,
            } => {
                write!(
                    formatter,
                    "unrecordable override on target {target_file_id} path '{property_path}': {reason}"
                )
            }
            PrefabVariantError::InvalidTargetLevel {
                from_level,
                target_level,
                reason,
            } => {
                write!(
                    formatter,
                    "cannot apply override from {from_level} to {target_level}: {reason}"
                )
            }
            PrefabVariantError::OverrideNotFound {
                level,
                target_file_id,
                property_path,
            } => {
                write!(
                    formatter,
                    "override on target {target_file_id} path '{property_path}' not found at level {level}"
                )
            }
            PrefabVariantError::TargetNotFound { target_file_id } => {
                write!(formatter, "target fileID {target_file_id} not found")
            }
        }
    }
}

impl std::error::Error for PrefabVariantError {}

pub fn validate_override(
    property_path: &str,
    value: &str,
    target_file_id: i64,
) -> Result<(), PrefabVariantError> {
    let trimmed = property_path.trim();
    if trimmed.is_empty() {
        return Err(PrefabVariantError::UnrecordableOverride {
            target_file_id,
            property_path: property_path.to_string(),
            reason: String::from("property path cannot be empty"),
        });
    }
    for (forbidden, reason) in UNRECORDABLE_PROPERTIES {
        if trimmed == forbidden || trimmed.starts_with(&format!("{forbidden}.")) {
            return Err(PrefabVariantError::UnrecordableOverride {
                target_file_id,
                property_path: property_path.to_string(),
                reason: String::from(reason),
            });
        }
    }
    for (prefix, reason) in UNRECORDABLE_PREFIXES {
        if trimmed == prefix || trimmed.starts_with(prefix) {
            return Err(PrefabVariantError::UnrecordableOverride {
                target_file_id,
                property_path: property_path.to_string(),
                reason: String::from(reason),
            });
        }
    }
    if value.contains(CROSS_SCENE_TOKEN) {
        return Err(PrefabVariantError::UnrecordableOverride {
            target_file_id,
            property_path: property_path.to_string(),
            reason: String::from(
                "persistent cross-scene references cannot be recorded in prefab overrides",
            ),
        });
    }
    if value.contains(NON_SERIALIZABLE_TOKEN) || value.starts_with("cross_prefab_ref:") {
        return Err(PrefabVariantError::UnrecordableOverride {
            target_file_id,
            property_path: property_path.to_string(),
            reason: String::from("cross-prefab non-serializable reference cannot be recorded"),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct PrefabRegistry {
    prefabs: HashMap<String, PrefabVariantDefinition>,
}

impl PrefabVariantDefinition {
    pub fn to_stream(&self) -> Stream {
        let mut stream = Stream {
            header: Some(Header {
                yaml_version: String::from("1.1"),
                tag_handle: Some(String::from("!u!")),
                tag_prefix: Some(String::from("tag:unity3d.com,2011:")),
            }),
            documents: Vec::new(),
        };
        self.apply_to_stream(&mut stream);
        stream
    }

    pub fn apply_to_stream(&self, stream: &mut Stream) {
        if stream.header.is_none() {
            stream.header = Some(Header {
                yaml_version: String::from("1.1"),
                tag_handle: Some(String::from("!u!")),
                tag_prefix: Some(String::from("tag:unity3d.com,2011:")),
            });
        }
        if let Some(base) = &self.base {
            let instance_file_id = if self.root_file_id != 0 {
                self.root_file_id
            } else {
                DEFAULT_VARIANT_INSTANCE_FILE_ID
            };
            upsert_prefab_instance_in_stream(
                stream,
                instance_file_id,
                base,
                0,
                &self.targeted_modifications,
                &self.modifications,
                &self.structural_overrides,
            );
        } else {
            let mut docs = Vec::new();
            for node in &self.nodes {
                docs.extend(emit_prefab_node_documents(node, None));
            }
            stream.documents = docs;
        }
        for nested in &self.nested_prefabs {
            let targeted: Vec<TargetedModification> = nested
                .modifications
                .iter()
                .map(|m| TargetedModification::from_modification(&nested.source.guid, m))
                .collect();
            let empty_structural = StructuralOverrides::default();
            upsert_prefab_instance_in_stream(
                stream,
                nested.instance_file_id,
                &nested.source,
                nested.parent_target_file_id,
                &targeted,
                &nested.modifications,
                &empty_structural,
            );
        }
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), SceneSaveError> {
        let stream = self.to_stream();
        scene_save::save_stream(path, &stream)
    }

    pub fn from_stream(
        stream: &Stream,
        guid: impl Into<String>,
        name: impl Into<String>,
    ) -> Option<Self> {
        let guid_str = guid.into();
        let name_str = name.into();

        let primary_prefab_doc = stream
            .documents
            .iter()
            .find(|doc| doc.class_id == PREFAB_INSTANCE_CLASS_ID && !doc.stripped);

        let has_base_game_objects = stream.documents.iter().any(|doc| {
            if doc.class_id != GAME_OBJECT_CLASS_ID || doc.stripped {
                return false;
            }
            let prefab_inst_ref = doc
                .root
                .get("GameObject")
                .and_then(|go| go.get("m_PrefabInstance"))
                .and_then(|n| n.get("fileID"))
                .and_then(Node::as_i64)
                .unwrap_or(0);
            prefab_inst_ref == 0
        });

        if !has_base_game_objects {
            if let Some(doc) = primary_prefab_doc {
                let parsed = parse_prefab_instance_from_stream(stream, doc.file_id)?;
                let root_file_id = if doc.file_id == DEFAULT_VARIANT_INSTANCE_FILE_ID {
                    0
                } else {
                    doc.file_id
                };
                let mut nested_prefabs = Vec::new();
                for other_doc in &stream.documents {
                    if other_doc.class_id == PREFAB_INSTANCE_CLASS_ID
                        && !other_doc.stripped
                        && other_doc.file_id != doc.file_id
                    {
                        if let Some(nested_parsed) =
                            parse_prefab_instance_from_stream(stream, other_doc.file_id)
                        {
                            nested_prefabs.push(NestedPrefabReference {
                                instance_file_id: other_doc.file_id,
                                parent_target_file_id: nested_parsed.instance.transform_parent,
                                source: nested_parsed.instance.source,
                                modifications: nested_parsed.instance.modifications,
                            });
                        }
                    }
                }
                return Some(PrefabVariantDefinition {
                    guid: guid_str,
                    name: name_str,
                    base: Some(parsed.instance.source),
                    root_file_id,
                    nodes: Vec::new(),
                    modifications: parsed.instance.modifications,
                    targeted_modifications: parsed.targeted_modifications,
                    structural_overrides: parsed.structural_overrides,
                    nested_prefabs,
                });
            }
        }

        let nodes = parse_base_nodes_from_stream(stream);
        let root_file_id = nodes
            .iter()
            .find(|n| n.parent_file_id.is_none())
            .map(|n| n.file_id)
            .unwrap_or(0);
        let mut nested_prefabs = Vec::new();
        for doc in &stream.documents {
            if doc.class_id == PREFAB_INSTANCE_CLASS_ID && !doc.stripped {
                if let Some(nested_parsed) = parse_prefab_instance_from_stream(stream, doc.file_id)
                {
                    nested_prefabs.push(NestedPrefabReference {
                        instance_file_id: doc.file_id,
                        parent_target_file_id: nested_parsed.instance.transform_parent,
                        source: nested_parsed.instance.source,
                        modifications: nested_parsed.instance.modifications,
                    });
                }
            }
        }

        Some(PrefabVariantDefinition {
            guid: guid_str,
            name: name_str,
            base: None,
            root_file_id,
            nodes,
            modifications: Vec::new(),
            targeted_modifications: Vec::new(),
            structural_overrides: StructuralOverrides::default(),
            nested_prefabs,
        })
    }

    pub fn load_from_file(
        path: &Path,
        guid: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Self, YamlError> {
        let stream = unity_yaml::parse_file(path)?;
        Self::from_stream(&stream, guid, name).ok_or_else(|| YamlError::UnexpectedContent {
            line: 1,
            text: String::from("stream does not contain a valid prefab or variant definition"),
        })
    }
}

impl ScenePrefabInstance {
    pub fn to_stream(&self, instance_file_id: i64) -> Stream {
        let mut stream = Stream {
            header: Some(Header {
                yaml_version: String::from("1.1"),
                tag_handle: Some(String::from("!u!")),
                tag_prefix: Some(String::from("tag:unity3d.com,2011:")),
            }),
            documents: Vec::new(),
        };
        self.apply_to_stream(&mut stream, instance_file_id);
        stream
    }

    pub fn apply_to_stream(&self, stream: &mut Stream, instance_file_id: i64) {
        if stream.header.is_none() {
            stream.header = Some(Header {
                yaml_version: String::from("1.1"),
                tag_handle: Some(String::from("!u!")),
                tag_prefix: Some(String::from("tag:unity3d.com,2011:")),
            });
        }
        upsert_prefab_instance_in_stream(
            stream,
            instance_file_id,
            &self.instance.source,
            self.instance.transform_parent,
            &self.targeted_modifications,
            &self.instance.modifications,
            &self.structural_overrides,
        );
    }

    pub fn save_to_file(&self, path: &Path, instance_file_id: i64) -> Result<(), SceneSaveError> {
        let stream = self.to_stream(instance_file_id);
        scene_save::save_stream(path, &stream)
    }

    pub fn from_stream(
        stream: &Stream,
        instance_id: impl Into<String>,
        instance_file_id: i64,
    ) -> Option<Self> {
        let parsed = parse_prefab_instance_from_stream(stream, instance_file_id)?;
        Some(ScenePrefabInstance {
            instance_id: instance_id.into(),
            instance: parsed.instance,
            targeted_modifications: parsed.targeted_modifications,
            structural_overrides: parsed.structural_overrides,
        })
    }

    pub fn load_from_file(
        path: &Path,
        instance_id: impl Into<String>,
        instance_file_id: i64,
    ) -> Result<Self, YamlError> {
        let stream = unity_yaml::parse_file(path)?;
        Self::from_stream(&stream, instance_id, instance_file_id).ok_or_else(|| {
            YamlError::UnexpectedContent {
                line: 1,
                text: format!("prefab instance {instance_file_id} not found in stream"),
            }
        })
    }
}
