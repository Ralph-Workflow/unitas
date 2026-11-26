use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::Path;

use crate::prefab::{Modification, PrefabInstance, PrefabSource, PrefabSourceReference};
use crate::scene_save::{self, SceneSaveError};
use crate::unity_yaml::{self, Document, Header, Node, Stream, YamlError};

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
pub enum OverrideLevel {
    Prefab(String),
    SceneInstance(String),
    NestedPrefab {
        owner: String,
        instance_file_id: i64,
    },
}

impl OverrideLevel {
    pub fn as_id(&self) -> String {
        match self {
            OverrideLevel::Prefab(guid) => guid.clone(),
            OverrideLevel::SceneInstance(id) => id.clone(),
            OverrideLevel::NestedPrefab {
                owner,
                instance_file_id,
            } => format!("nested:{owner}:{instance_file_id}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueProvenanceEntry {
    pub level: OverrideLevel,
    pub value: String,
    pub is_effective: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedComponent {
    pub identity: SourceObjectId,
    pub defined_at: OverrideLevel,
    pub class_name: String,
    pub properties: BTreeMap<String, Node>,
    pub object_references: BTreeMap<String, PrefabSourceReference>,
    pub provenance: BTreeMap<String, Vec<ValueProvenanceEntry>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedNode {
    pub identity: SourceObjectId,
    pub defined_at: OverrideLevel,
    pub name: String,
    pub parent: Option<SourceObjectId>,
    pub children: Vec<SourceObjectId>,
    pub properties: BTreeMap<String, Node>,
    pub object_references: BTreeMap<String, PrefabSourceReference>,
    pub provenance: BTreeMap<String, Vec<ValueProvenanceEntry>>,
    pub components: Vec<ResolvedComponent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrphanedOverrideResolution {
    Discard,
    Retain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrphanedOverride {
    pub level: OverrideLevel,
    pub target_file_id: i64,
    pub property_path: String,
    pub value: String,
}

impl OrphanedOverride {
    pub fn resolution_options(&self) -> &'static [OrphanedOverrideResolution] {
        &[
            OrphanedOverrideResolution::Discard,
            OrphanedOverrideResolution::Retain,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnpackMode {
    OneLevel,
    Completely,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnpackedPrefabHierarchy {
    pub nodes: Vec<PrefabNodeRecord>,
    pub nested_prefabs: Vec<NestedPrefabReference>,
}

impl UnpackedPrefabHierarchy {
    pub fn effective_property(&self, target_file_id: i64, property_path: &str) -> Option<&str> {
        for node in &self.nodes {
            if node.file_id == target_file_id {
                return lookup_property(&node.properties, property_path).and_then(Node::as_str);
            }
            for comp in &node.components {
                if comp.file_id == target_file_id {
                    return lookup_property(&comp.properties, property_path).and_then(Node::as_str);
                }
            }
        }
        None
    }

    pub fn apply_to_scene_stream(&self, stream: &mut Stream, unpacked_instance_file_id: i64) {
        stream.documents.retain(|doc| {
            if doc.class_id == PREFAB_INSTANCE_CLASS_ID && doc.file_id == unpacked_instance_file_id
            {
                return false;
            }
            let owned_by_instance = match &doc.root {
                Node::Mapping(entries) => entries.first().and_then(|(_, body)| {
                    body.get("m_PrefabInstance")
                        .and_then(|n| n.get("fileID"))
                        .and_then(Node::as_i64)
                }),
                _ => None,
            };
            owned_by_instance != Some(unpacked_instance_file_id)
        });
        for node in &self.nodes {
            stream
                .documents
                .extend(emit_prefab_node_documents(node, None));
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHierarchy {
    pub chain: Vec<OverrideLevel>,
    pub base_guid: String,
    pub root: Option<SourceObjectId>,
    pub nodes: BTreeMap<SourceObjectId, ResolvedNode>,
    pub ordered_nodes: Vec<SourceObjectId>,
    pub orphaned_overrides: Vec<OrphanedOverride>,
    pub nested: Vec<ResolvedNestedPrefab>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedNestedPrefab {
    pub instance_file_id: i64,
    pub parent_target_file_id: i64,
    pub source: PrefabSource,
    pub declared_at: OverrideLevel,
    pub resolved: ResolvedPrefab,
}

impl ResolvedHierarchy {
    pub fn node_by_file_id(&self, target_file_id: i64) -> Option<&ResolvedNode> {
        self.nodes
            .values()
            .find(|node| node.identity.target_file_id == target_file_id)
    }

    pub fn node(&self, id: &SourceObjectId) -> Option<&ResolvedNode> {
        self.nodes.get(id)
    }

    pub fn effective_property(&self, target_file_id: i64, property_path: &str) -> Option<&str> {
        for node in self.nodes.values() {
            if node.identity.target_file_id == target_file_id {
                return lookup_property(&node.properties, property_path).and_then(Node::as_str);
            }
            for component in &node.components {
                if component.identity.target_file_id == target_file_id {
                    return lookup_property(&component.properties, property_path)
                        .and_then(Node::as_str);
                }
            }
        }
        None
    }

    pub fn effective_object_reference(
        &self,
        target_file_id: i64,
        property_path: &str,
    ) -> Option<&PrefabSourceReference> {
        for node in self.nodes.values() {
            if node.identity.target_file_id == target_file_id {
                return node.object_references.get(property_path);
            }
            for component in &node.components {
                if component.identity.target_file_id == target_file_id {
                    return component.object_references.get(property_path);
                }
            }
        }
        None
    }

    pub fn property_inheritance_path(
        &self,
        target_file_id: i64,
        property_path: &str,
    ) -> Option<&[ValueProvenanceEntry]> {
        for node in self.nodes.values() {
            if node.identity.target_file_id == target_file_id {
                return node.provenance.get(property_path).map(Vec::as_slice);
            }
            for component in &node.components {
                if component.identity.target_file_id == target_file_id {
                    return component.provenance.get(property_path).map(Vec::as_slice);
                }
            }
        }
        None
    }

    pub fn effective_owner(
        &self,
        target_file_id: i64,
        property_path: &str,
    ) -> Option<&OverrideLevel> {
        let entries = self.property_inheritance_path(target_file_id, property_path)?;
        entries
            .iter()
            .find(|entry| entry.is_effective)
            .map(|entry| &entry.level)
    }

    pub fn nested_by_instance_file_id(
        &self,
        instance_file_id: i64,
    ) -> Option<&ResolvedNestedPrefab> {
        self.nested
            .iter()
            .find(|nested| nested.instance_file_id == instance_file_id)
    }

    pub fn eligible_apply_targets(&self, from_level: &OverrideLevel) -> Vec<OverrideLevel> {
        let pos = self.chain.iter().position(|lvl| lvl == from_level);
        match pos {
            Some(idx) => self.chain[..idx].to_vec(),
            None => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenPrefabState {
    pub missing_guid: String,
    pub broken_at_level: OverrideLevel,
    pub chain_before_break: Vec<OverrideLevel>,
    pub retained_modifications: Vec<(OverrideLevel, Modification)>,
    pub retained_structural_overrides: Vec<(OverrideLevel, StructuralOverrides)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedPrefab {
    Complete(ResolvedHierarchy),
    BrokenBase(BrokenPrefabState),
}

impl ResolvedPrefab {
    pub fn as_complete(&self) -> Option<&ResolvedHierarchy> {
        match self {
            ResolvedPrefab::Complete(hierarchy) => Some(hierarchy),
            ResolvedPrefab::BrokenBase(_) => None,
        }
    }

    pub fn as_broken(&self) -> Option<&BrokenPrefabState> {
        match self {
            ResolvedPrefab::Complete(_) => None,
            ResolvedPrefab::BrokenBase(broken) => Some(broken),
        }
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

impl PrefabRegistry {
    pub fn new() -> Self {
        Self {
            prefabs: HashMap::new(),
        }
    }

    pub fn register(&mut self, definition: PrefabVariantDefinition) {
        self.prefabs.insert(definition.guid.clone(), definition);
    }

    pub fn remove(&mut self, guid: &str) -> Option<PrefabVariantDefinition> {
        self.prefabs.remove(guid)
    }

    pub fn get(&self, guid: &str) -> Option<&PrefabVariantDefinition> {
        self.prefabs.get(guid)
    }

    pub fn get_mut(&mut self, guid: &str) -> Option<&mut PrefabVariantDefinition> {
        self.prefabs.get_mut(guid)
    }

    pub fn inheritance_chain(&self, leaf_guid: &str) -> Result<Vec<String>, PrefabVariantError> {
        let mut chain_from_leaf = Vec::new();
        let mut visited = Vec::new();
        let mut current = leaf_guid.to_string();
        loop {
            if visited.contains(&current) {
                visited.push(current);
                return Err(PrefabVariantError::InheritanceCycle { chain: visited });
            }
            visited.push(current.clone());
            chain_from_leaf.push(current.clone());
            let def = match self.prefabs.get(&current) {
                Some(def) => def,
                None => break,
            };
            match &def.base {
                Some(base) => {
                    current = base.guid.clone();
                }
                None => break,
            }
        }
        chain_from_leaf.reverse();
        Ok(chain_from_leaf)
    }

    pub fn validate_no_cycles(&self, start_guid: &str) -> Result<(), PrefabVariantError> {
        let _ = self.inheritance_chain(start_guid)?;
        let mut stack = Vec::new();
        self.check_nesting_recursive(start_guid, &mut stack)
    }

    fn check_nesting_recursive(
        &self,
        guid: &str,
        stack: &mut Vec<String>,
    ) -> Result<(), PrefabVariantError> {
        if stack.contains(&guid.to_string()) {
            let mut cycle = stack.clone();
            cycle.push(guid.to_string());
            return Err(PrefabVariantError::NestingCycle { chain: cycle });
        }
        let chain = self.inheritance_chain(guid)?;
        stack.push(guid.to_string());
        for ancestor_guid in &chain {
            if let Some(def) = self.prefabs.get(ancestor_guid) {
                for nested in &def.nested_prefabs {
                    self.check_nesting_recursive(&nested.source.guid, stack)?;
                }
            }
        }
        stack.pop();
        Ok(())
    }

    pub fn resolve_prefab(&self, guid: &str) -> Result<ResolvedPrefab, PrefabVariantError> {
        self.resolve_internal(guid, None)
    }

    pub fn resolve_instance(
        &self,
        instance: &ScenePrefabInstance,
    ) -> Result<ResolvedPrefab, PrefabVariantError> {
        self.resolve_internal(&instance.instance.source.guid, Some(instance))
    }

    fn resolve_internal(
        &self,
        leaf_prefab_guid: &str,
        scene_instance: Option<&ScenePrefabInstance>,
    ) -> Result<ResolvedPrefab, PrefabVariantError> {
        let mut raw_chain_from_leaf = Vec::new();
        let mut visited = Vec::new();
        let mut current = leaf_prefab_guid.to_string();
        let mut missing_guid: Option<(String, OverrideLevel)> = None;

        loop {
            if visited.contains(&current) {
                visited.push(current);
                return Err(PrefabVariantError::InheritanceCycle { chain: visited });
            }
            visited.push(current.clone());
            match self.prefabs.get(&current) {
                Some(def) => {
                    for mod_item in &def.modifications {
                        validate_override(
                            &mod_item.property_path,
                            &mod_item.value,
                            mod_item.target_file_id,
                        )?;
                    }
                    raw_chain_from_leaf.push(current.clone());
                    match &def.base {
                        Some(base) => {
                            current = base.guid.clone();
                        }
                        None => break,
                    }
                }
                None => {
                    let broken_at = match raw_chain_from_leaf.last() {
                        Some(prev) => OverrideLevel::Prefab(prev.clone()),
                        None => match scene_instance {
                            Some(inst) => OverrideLevel::SceneInstance(inst.instance_id.clone()),
                            None => OverrideLevel::Prefab(current.clone()),
                        },
                    };
                    missing_guid = Some((current.clone(), broken_at));
                    break;
                }
            }
        }

        if let Some(inst) = scene_instance {
            for mod_item in &inst.instance.modifications {
                validate_override(
                    &mod_item.property_path,
                    &mod_item.value,
                    mod_item.target_file_id,
                )?;
            }
        }

        if let Some((missing, broken_at_level)) = missing_guid {
            raw_chain_from_leaf.reverse();
            let mut chain_before_break: Vec<OverrideLevel> = raw_chain_from_leaf
                .iter()
                .map(|g| OverrideLevel::Prefab(g.clone()))
                .collect();
            let mut retained_modifications = Vec::new();
            let mut retained_structural = Vec::new();

            for g in &raw_chain_from_leaf {
                if let Some(def) = self.prefabs.get(g) {
                    let lvl = OverrideLevel::Prefab(g.clone());
                    for m in &def.modifications {
                        retained_modifications.push((lvl.clone(), m.clone()));
                    }
                    retained_structural.push((lvl, def.structural_overrides.clone()));
                }
            }
            if let Some(inst) = scene_instance {
                let lvl = OverrideLevel::SceneInstance(inst.instance_id.clone());
                chain_before_break.push(lvl.clone());
                for m in &inst.instance.modifications {
                    retained_modifications.push((lvl.clone(), m.clone()));
                }
                retained_structural.push((lvl, inst.structural_overrides.clone()));
            }

            return Ok(ResolvedPrefab::BrokenBase(BrokenPrefabState {
                missing_guid: missing,
                broken_at_level,
                chain_before_break,
                retained_modifications,
                retained_structural_overrides: retained_structural,
            }));
        }

        self.validate_no_cycles(leaf_prefab_guid)?;

        raw_chain_from_leaf.reverse();
        let base_guid = match raw_chain_from_leaf.first() {
            Some(g) => g.clone(),
            None => leaf_prefab_guid.to_string(),
        };

        let base_def = match self.prefabs.get(&base_guid) {
            Some(def) => def,
            None => {
                return Err(PrefabVariantError::MissingBasePrefab {
                    missing_guid: base_guid.clone(),
                    dependent_guid: leaf_prefab_guid.to_string(),
                })
            }
        };

        let mut chain_levels: Vec<OverrideLevel> = raw_chain_from_leaf
            .iter()
            .map(|g| OverrideLevel::Prefab(g.clone()))
            .collect();
        if let Some(inst) = scene_instance {
            chain_levels.push(OverrideLevel::SceneInstance(inst.instance_id.clone()));
        }

        let base_level = OverrideLevel::Prefab(base_guid.clone());
        let mut nodes: BTreeMap<SourceObjectId, ResolvedNode> = BTreeMap::new();
        let mut file_id_to_source: HashMap<i64, SourceObjectId> = HashMap::new();

        for node_rec in &base_def.nodes {
            let identity = SourceObjectId::new(&base_guid, node_rec.file_id);
            file_id_to_source.insert(node_rec.file_id, identity.clone());
            let parent = node_rec
                .parent_file_id
                .map(|pid| SourceObjectId::new(&base_guid, pid));
            let children = node_rec
                .children
                .iter()
                .map(|cid| SourceObjectId::new(&base_guid, *cid))
                .collect();

            let mut provenance: BTreeMap<String, Vec<ValueProvenanceEntry>> = BTreeMap::new();
            for (k, v) in &node_rec.properties {
                let value = match v.as_str() {
                    Some(text) => text.to_string(),
                    None => continue,
                };
                provenance.insert(
                    k.clone(),
                    vec![ValueProvenanceEntry {
                        level: base_level.clone(),
                        value,
                        is_effective: true,
                    }],
                );
            }

            let mut components = Vec::new();
            for comp_rec in &node_rec.components {
                let comp_id = SourceObjectId::new(&base_guid, comp_rec.file_id);
                file_id_to_source.insert(comp_rec.file_id, comp_id.clone());
                let mut comp_prov: BTreeMap<String, Vec<ValueProvenanceEntry>> = BTreeMap::new();
                for (k, v) in &comp_rec.properties {
                    let value = match v.as_str() {
                        Some(text) => text.to_string(),
                        None => continue,
                    };
                    comp_prov.insert(
                        k.clone(),
                        vec![ValueProvenanceEntry {
                            level: base_level.clone(),
                            value,
                            is_effective: true,
                        }],
                    );
                }
                components.push(ResolvedComponent {
                    identity: comp_id,
                    defined_at: base_level.clone(),
                    class_name: comp_rec.class_name.clone(),
                    properties: comp_rec.properties.clone(),
                    object_references: BTreeMap::new(),
                    provenance: comp_prov,
                });
            }

            nodes.insert(
                identity.clone(),
                ResolvedNode {
                    identity,
                    defined_at: base_level.clone(),
                    name: node_rec.name.clone(),
                    parent,
                    children,
                    properties: node_rec.properties.clone(),
                    object_references: BTreeMap::new(),
                    provenance,
                    components,
                },
            );
        }

        let mut orphaned_overrides = Vec::new();

        for variant_guid in raw_chain_from_leaf.iter().skip(1) {
            if let Some(var_def) = self.prefabs.get(variant_guid) {
                let level = OverrideLevel::Prefab(variant_guid.clone());
                apply_level_overrides(
                    &level,
                    variant_guid,
                    &var_def.structural_overrides,
                    &var_def.modifications,
                    &mut nodes,
                    &mut file_id_to_source,
                    &mut orphaned_overrides,
                );
            }
        }

        if let Some(inst) = scene_instance {
            let level = OverrideLevel::SceneInstance(inst.instance_id.clone());
            apply_level_overrides(
                &level,
                &inst.instance_id,
                &inst.structural_overrides,
                &inst.instance.modifications,
                &mut nodes,
                &mut file_id_to_source,
                &mut orphaned_overrides,
            );
        }

        let root_id = if base_def.root_file_id != 0 {
            file_id_to_source.get(&base_def.root_file_id).cloned()
        } else {
            nodes
                .values()
                .find(|n| n.parent.is_none())
                .map(|n| n.identity.clone())
        };

        let mut ordered_nodes = Vec::new();
        if let Some(root) = &root_id {
            collect_depth_first(root, &nodes, &mut ordered_nodes);
        }
        for id in nodes.keys() {
            if !ordered_nodes.contains(id) {
                ordered_nodes.push(id.clone());
            }
        }

        let nested = self.resolve_nested_prefabs(&raw_chain_from_leaf)?;

        Ok(ResolvedPrefab::Complete(ResolvedHierarchy {
            chain: chain_levels,
            base_guid,
            root: root_id,
            nodes,
            ordered_nodes,
            orphaned_overrides,
            nested,
        }))
    }

    fn resolve_nested_prefabs(
        &self,
        chain: &[String],
    ) -> Result<Vec<ResolvedNestedPrefab>, PrefabVariantError> {
        let mut resolved_nested = Vec::new();
        for guid in chain {
            let def = match self.prefabs.get(guid) {
                Some(def) => def,
                None => continue,
            };
            let declared_at = OverrideLevel::Prefab(guid.clone());
            for nested in &def.nested_prefabs {
                let level = OverrideLevel::NestedPrefab {
                    owner: guid.clone(),
                    instance_file_id: nested.instance_file_id,
                };
                let resolved = match self.resolve_internal(&nested.source.guid, None)? {
                    ResolvedPrefab::Complete(mut hierarchy) => {
                        hierarchy.chain.push(level.clone());
                        apply_modifications_to_hierarchy(
                            &mut hierarchy,
                            &level,
                            &nested.source.guid,
                            &nested.modifications,
                        );
                        ResolvedPrefab::Complete(hierarchy)
                    }
                    broken @ ResolvedPrefab::BrokenBase(_) => broken,
                };
                resolved_nested.push(ResolvedNestedPrefab {
                    instance_file_id: nested.instance_file_id,
                    parent_target_file_id: nested.parent_target_file_id,
                    source: nested.source.clone(),
                    declared_at: declared_at.clone(),
                    resolved,
                });
            }
        }
        Ok(resolved_nested)
    }

    pub fn apply_override_to_level(
        &mut self,
        scene_instance: Option<&mut ScenePrefabInstance>,
        from_level: &OverrideLevel,
        target_level: &OverrideLevel,
        target_file_id: i64,
        property_path: &str,
    ) -> Result<(), PrefabVariantError> {
        if let OverrideLevel::NestedPrefab {
            owner,
            instance_file_id,
        } = from_level
        {
            let nested_source_guid = {
                let owner_def = match self.prefabs.get(owner) {
                    Some(def) => def,
                    None => {
                        return Err(PrefabVariantError::MissingBasePrefab {
                            missing_guid: owner.clone(),
                            dependent_guid: owner.clone(),
                        })
                    }
                };
                let nested_ref = match owner_def
                    .nested_prefabs
                    .iter()
                    .find(|n| n.instance_file_id == *instance_file_id)
                {
                    Some(n) => n,
                    None => {
                        return Err(PrefabVariantError::OverrideNotFound {
                            level: from_level.as_id(),
                            target_file_id,
                            property_path: property_path.to_string(),
                        })
                    }
                };
                nested_ref.source.guid.clone()
            };
            let nested_chain = self.inheritance_chain(&nested_source_guid)?;
            let target_guid = match target_level {
                OverrideLevel::Prefab(g) if nested_chain.contains(g) => g.clone(),
                _ => {
                    return Err(PrefabVariantError::InvalidTargetLevel {
                        from_level: from_level.as_id(),
                        target_level: target_level.as_id(),
                        reason: String::from(
                            "nested prefab override target must be in the nested prefab inheritance chain",
                        ),
                    })
                }
            };
            let removed_mod = {
                let owner_def = match self.prefabs.get_mut(owner) {
                    Some(def) => def,
                    None => {
                        return Err(PrefabVariantError::MissingBasePrefab {
                            missing_guid: owner.clone(),
                            dependent_guid: owner.clone(),
                        })
                    }
                };
                let nested_ref = match owner_def
                    .nested_prefabs
                    .iter_mut()
                    .find(|n| n.instance_file_id == *instance_file_id)
                {
                    Some(n) => n,
                    None => {
                        return Err(PrefabVariantError::OverrideNotFound {
                            level: from_level.as_id(),
                            target_file_id,
                            property_path: property_path.to_string(),
                        })
                    }
                };
                let mut found: Option<Modification> = None;
                nested_ref.modifications.retain(|m| {
                    let matches =
                        m.target_file_id == target_file_id && m.property_path == property_path;
                    if matches {
                        found = Some(m.clone());
                    }
                    !matches
                });
                match found {
                    Some(m) => m,
                    None => {
                        return Err(PrefabVariantError::OverrideNotFound {
                            level: from_level.as_id(),
                            target_file_id,
                            property_path: property_path.to_string(),
                        })
                    }
                }
            };
            if let Some(target_idx) = nested_chain.iter().position(|g| g == &target_guid) {
                for inter_guid in nested_chain.iter().skip(target_idx + 1) {
                    if let Some(inter_def) = self.prefabs.get_mut(inter_guid) {
                        let _ = inter_def.remove_modification(target_file_id, property_path);
                    }
                }
            }
            let target_def = match self.prefabs.get_mut(&target_guid) {
                Some(def) => def,
                None => {
                    return Err(PrefabVariantError::MissingBasePrefab {
                        missing_guid: target_guid.clone(),
                        dependent_guid: nested_source_guid,
                    })
                }
            };
            if target_def.base.is_none() {
                let val = match &removed_mod.object_reference {
                    Some(r) if removed_mod.value.is_empty() => r.to_encoded_string(),
                    _ => removed_mod.value,
                };
                target_def.set_base_property(target_file_id, property_path, val)?;
            } else {
                target_def.record_override(removed_mod)?;
            }
            return Ok(());
        }

        let leaf_guid = match &scene_instance {
            Some(inst) => inst.instance.source.guid.clone(),
            None => match from_level {
                OverrideLevel::Prefab(guid) => guid.clone(),
                OverrideLevel::SceneInstance(id) => {
                    return Err(PrefabVariantError::InvalidTargetLevel {
                        from_level: id.clone(),
                        target_level: target_level.as_id(),
                        reason: String::from("scene instance not provided"),
                    })
                }
                OverrideLevel::NestedPrefab { .. } => {
                    return Err(PrefabVariantError::InvalidTargetLevel {
                        from_level: from_level.as_id(),
                        target_level: target_level.as_id(),
                        reason: String::from(
                            "nested prefab instance overrides cannot be applied from their own level",
                        ),
                    })
                }
            },
        };

        let prefab_chain = self.inheritance_chain(&leaf_guid)?;
        let mut full_chain: Vec<OverrideLevel> = prefab_chain
            .iter()
            .map(|g| OverrideLevel::Prefab(g.clone()))
            .collect();
        if let Some(inst) = &scene_instance {
            full_chain.push(OverrideLevel::SceneInstance(inst.instance_id.clone()));
        }

        let from_idx = match full_chain.iter().position(|lvl| lvl == from_level) {
            Some(idx) => idx,
            None => {
                return Err(PrefabVariantError::InvalidTargetLevel {
                    from_level: from_level.as_id(),
                    target_level: target_level.as_id(),
                    reason: String::from("source level is not in the inheritance chain"),
                })
            }
        };
        let target_idx = match full_chain.iter().position(|lvl| lvl == target_level) {
            Some(idx) => idx,
            None => {
                return Err(PrefabVariantError::InvalidTargetLevel {
                    from_level: from_level.as_id(),
                    target_level: target_level.as_id(),
                    reason: String::from("target level is not in the inheritance chain"),
                })
            }
        };

        if target_idx >= from_idx {
            return Err(PrefabVariantError::InvalidTargetLevel {
                from_level: from_level.as_id(),
                target_level: target_level.as_id(),
                reason: String::from("target level must be an ancestor of source level"),
            });
        }

        let removed_mod = match from_level {
            OverrideLevel::SceneInstance(_) => {
                let inst = match scene_instance {
                    Some(inst) => inst,
                    None => {
                        return Err(PrefabVariantError::InvalidTargetLevel {
                            from_level: from_level.as_id(),
                            target_level: target_level.as_id(),
                            reason: String::from("scene instance is required"),
                        })
                    }
                };
                match inst.remove_modification(target_file_id, property_path) {
                    Some(val) => val,
                    None => {
                        return Err(PrefabVariantError::OverrideNotFound {
                            level: from_level.as_id(),
                            target_file_id,
                            property_path: property_path.to_string(),
                        })
                    }
                }
            }
            OverrideLevel::Prefab(guid) => {
                let def = match self.prefabs.get_mut(guid) {
                    Some(def) => def,
                    None => {
                        return Err(PrefabVariantError::MissingBasePrefab {
                            missing_guid: guid.clone(),
                            dependent_guid: leaf_guid,
                        })
                    }
                };
                match def.remove_modification(target_file_id, property_path) {
                    Some(val) => val,
                    None => {
                        return Err(PrefabVariantError::OverrideNotFound {
                            level: from_level.as_id(),
                            target_file_id,
                            property_path: property_path.to_string(),
                        })
                    }
                }
            }
            OverrideLevel::NestedPrefab { .. } => {
                return Err(PrefabVariantError::InvalidTargetLevel {
                    from_level: from_level.as_id(),
                    target_level: target_level.as_id(),
                    reason: String::from(
                        "nested prefab instance overrides cannot be applied from their own level",
                    ),
                });
            }
        };

        for level_entry in full_chain.iter().take(from_idx).skip(target_idx + 1) {
            if let OverrideLevel::Prefab(inter_guid) = level_entry {
                if let Some(inter_def) = self.prefabs.get_mut(inter_guid) {
                    let _ = inter_def.remove_modification(target_file_id, property_path);
                }
            }
        }

        match target_level {
            OverrideLevel::Prefab(target_guid) => {
                let target_def = match self.prefabs.get_mut(target_guid) {
                    Some(def) => def,
                    None => {
                        return Err(PrefabVariantError::MissingBasePrefab {
                            missing_guid: target_guid.clone(),
                            dependent_guid: leaf_guid,
                        })
                    }
                };
                if target_def.base.is_none() {
                    let val = match &removed_mod.object_reference {
                        Some(r) if removed_mod.value.is_empty() => r.to_encoded_string(),
                        _ => removed_mod.value,
                    };
                    target_def.set_base_property(target_file_id, property_path, val)?;
                } else {
                    target_def.record_override(removed_mod)?;
                }
            }
            OverrideLevel::SceneInstance(_) => {
                return Err(PrefabVariantError::InvalidTargetLevel {
                    from_level: from_level.as_id(),
                    target_level: target_level.as_id(),
                    reason: String::from("cannot apply override downward to a scene instance"),
                });
            }
            OverrideLevel::NestedPrefab { .. } => {
                return Err(PrefabVariantError::InvalidTargetLevel {
                    from_level: from_level.as_id(),
                    target_level: target_level.as_id(),
                    reason: String::from("a nested prefab instance level is not an apply target"),
                });
            }
        }

        Ok(())
    }

    pub fn apply_all_to_level(
        &mut self,
        mut scene_instance: Option<&mut ScenePrefabInstance>,
        from_level: &OverrideLevel,
        target_level: &OverrideLevel,
    ) -> Result<(), PrefabVariantError> {
        let keys: Vec<(i64, String)> = match from_level {
            OverrideLevel::SceneInstance(_) => match &scene_instance {
                Some(inst) => inst
                    .instance
                    .modifications
                    .iter()
                    .map(|m| (m.target_file_id, m.property_path.clone()))
                    .collect(),
                None => Vec::new(),
            },
            OverrideLevel::Prefab(guid) => match self.prefabs.get(guid) {
                Some(def) => def
                    .modifications
                    .iter()
                    .map(|m| (m.target_file_id, m.property_path.clone()))
                    .collect(),
                None => Vec::new(),
            },
            OverrideLevel::NestedPrefab { .. } => {
                return Err(PrefabVariantError::InvalidTargetLevel {
                    from_level: from_level.as_id(),
                    target_level: target_level.as_id(),
                    reason: String::from(
                        "nested prefab instance overrides cannot be applied from their own level",
                    ),
                });
            }
        };

        for (target_file_id, property_path) in keys {
            let inst_ref = scene_instance.as_deref_mut();
            self.apply_override_to_level(
                inst_ref,
                from_level,
                target_level,
                target_file_id,
                &property_path,
            )?;
        }
        Ok(())
    }

    pub fn revert_override_at_level(
        &mut self,
        scene_instance: Option<&mut ScenePrefabInstance>,
        level: &OverrideLevel,
        target_file_id: i64,
        property_path: &str,
    ) -> Result<(), PrefabVariantError> {
        match level {
            OverrideLevel::SceneInstance(_) => {
                let inst = match scene_instance {
                    Some(inst) => inst,
                    None => {
                        return Err(PrefabVariantError::OverrideNotFound {
                            level: level.as_id(),
                            target_file_id,
                            property_path: property_path.to_string(),
                        })
                    }
                };
                if inst
                    .remove_modification(target_file_id, property_path)
                    .is_none()
                {
                    return Err(PrefabVariantError::OverrideNotFound {
                        level: level.as_id(),
                        target_file_id,
                        property_path: property_path.to_string(),
                    });
                }
                Ok(())
            }
            OverrideLevel::Prefab(guid) => {
                let def = match self.prefabs.get_mut(guid) {
                    Some(def) => def,
                    None => {
                        return Err(PrefabVariantError::MissingBasePrefab {
                            missing_guid: guid.clone(),
                            dependent_guid: guid.clone(),
                        })
                    }
                };
                if def
                    .remove_modification(target_file_id, property_path)
                    .is_none()
                {
                    return Err(PrefabVariantError::OverrideNotFound {
                        level: level.as_id(),
                        target_file_id,
                        property_path: property_path.to_string(),
                    });
                }
                Ok(())
            }
            OverrideLevel::NestedPrefab {
                owner,
                instance_file_id,
            } => {
                let def = match self.prefabs.get_mut(owner) {
                    Some(def) => def,
                    None => {
                        return Err(PrefabVariantError::MissingBasePrefab {
                            missing_guid: owner.clone(),
                            dependent_guid: owner.clone(),
                        })
                    }
                };
                let nested = match def
                    .nested_prefabs
                    .iter_mut()
                    .find(|nested| nested.instance_file_id == *instance_file_id)
                {
                    Some(nested) => nested,
                    None => {
                        return Err(PrefabVariantError::OverrideNotFound {
                            level: level.as_id(),
                            target_file_id,
                            property_path: property_path.to_string(),
                        })
                    }
                };
                let before = nested.modifications.len();
                nested.modifications.retain(|item| {
                    !(item.target_file_id == target_file_id && item.property_path == property_path)
                });
                if nested.modifications.len() == before {
                    return Err(PrefabVariantError::OverrideNotFound {
                        level: level.as_id(),
                        target_file_id,
                        property_path: property_path.to_string(),
                    });
                }
                Ok(())
            }
        }
    }

    pub fn revert_all_at_level(
        &mut self,
        scene_instance: Option<&mut ScenePrefabInstance>,
        level: &OverrideLevel,
    ) -> Result<(), PrefabVariantError> {
        match level {
            OverrideLevel::SceneInstance(_) => {
                let inst = match scene_instance {
                    Some(inst) => inst,
                    None => {
                        return Err(PrefabVariantError::InvalidTargetLevel {
                            from_level: level.as_id(),
                            target_level: level.as_id(),
                            reason: String::from("scene instance not provided"),
                        })
                    }
                };
                inst.instance.modifications.clear();
                inst.targeted_modifications.clear();
                inst.structural_overrides = StructuralOverrides::default();
                Ok(())
            }
            OverrideLevel::Prefab(guid) => {
                let def = match self.prefabs.get_mut(guid) {
                    Some(def) => def,
                    None => {
                        return Err(PrefabVariantError::MissingBasePrefab {
                            missing_guid: guid.clone(),
                            dependent_guid: guid.clone(),
                        })
                    }
                };
                def.modifications.clear();
                def.targeted_modifications.clear();
                def.structural_overrides = StructuralOverrides::default();
                Ok(())
            }
            OverrideLevel::NestedPrefab {
                owner,
                instance_file_id,
            } => {
                let def = match self.prefabs.get_mut(owner) {
                    Some(def) => def,
                    None => {
                        return Err(PrefabVariantError::MissingBasePrefab {
                            missing_guid: owner.clone(),
                            dependent_guid: owner.clone(),
                        })
                    }
                };
                let nested = match def
                    .nested_prefabs
                    .iter_mut()
                    .find(|nested| nested.instance_file_id == *instance_file_id)
                {
                    Some(nested) => nested,
                    None => {
                        return Err(PrefabVariantError::OverrideNotFound {
                            level: level.as_id(),
                            target_file_id: 0,
                            property_path: String::from("nested prefab instance"),
                        })
                    }
                };
                nested.modifications.clear();
                Ok(())
            }
        }
    }

    pub fn resolve_orphaned_override(
        &mut self,
        scene_instance: Option<&mut ScenePrefabInstance>,
        level: &OverrideLevel,
        target_file_id: i64,
        property_path: &str,
        resolution: OrphanedOverrideResolution,
    ) -> Result<(), PrefabVariantError> {
        match resolution {
            OrphanedOverrideResolution::Retain => Ok(()),
            OrphanedOverrideResolution::Discard => {
                self.revert_override_at_level(scene_instance, level, target_file_id, property_path)
            }
        }
    }

    pub fn unpack_instance(
        &self,
        instance: &ScenePrefabInstance,
        mode: UnpackMode,
    ) -> Result<UnpackedPrefabHierarchy, PrefabVariantError> {
        let resolved = self.resolve_instance(instance)?;
        let hierarchy = match resolved {
            ResolvedPrefab::Complete(h) => h,
            ResolvedPrefab::BrokenBase(broken) => {
                return Err(PrefabVariantError::MissingBasePrefab {
                    missing_guid: broken.missing_guid,
                    dependent_guid: instance.instance.source.guid.clone(),
                })
            }
        };
        let mut nodes = Vec::new();
        let mut nested_prefabs = Vec::new();
        flatten_resolved_hierarchy(&hierarchy, mode, None, &mut nodes, &mut nested_prefabs)?;
        Ok(UnpackedPrefabHierarchy {
            nodes,
            nested_prefabs,
        })
    }
}

fn flatten_resolved_hierarchy(
    hierarchy: &ResolvedHierarchy,
    mode: UnpackMode,
    parent_override: Option<i64>,
    out_nodes: &mut Vec<PrefabNodeRecord>,
    out_nested: &mut Vec<NestedPrefabReference>,
) -> Result<(), PrefabVariantError> {
    for node_id in &hierarchy.ordered_nodes {
        let rnode = match hierarchy.nodes.get(node_id) {
            Some(n) => n,
            None => continue,
        };
        let parent_file_id = match &rnode.parent {
            Some(p) => Some(p.target_file_id),
            None => parent_override,
        };
        let children_ids: Vec<i64> = rnode.children.iter().map(|c| c.target_file_id).collect();
        let components: Vec<PrefabComponentRecord> = rnode
            .components
            .iter()
            .map(|rc| PrefabComponentRecord {
                file_id: rc.identity.target_file_id,
                class_name: rc.class_name.clone(),
                properties: rc.properties.clone(),
            })
            .collect();
        out_nodes.push(PrefabNodeRecord {
            file_id: rnode.identity.target_file_id,
            name: rnode.name.clone(),
            parent_file_id,
            children: children_ids,
            properties: rnode.properties.clone(),
            components,
        });
    }
    match mode {
        UnpackMode::OneLevel => {
            for nested in &hierarchy.nested {
                let modifications = extract_nested_modifications(nested);
                out_nested.push(NestedPrefabReference {
                    instance_file_id: nested.instance_file_id,
                    parent_target_file_id: nested.parent_target_file_id,
                    source: nested.source.clone(),
                    modifications,
                });
            }
        }
        UnpackMode::Completely => {
            for nested in &hierarchy.nested {
                let nested_hierarchy = match &nested.resolved {
                    ResolvedPrefab::Complete(h) => h,
                    ResolvedPrefab::BrokenBase(broken) => {
                        return Err(PrefabVariantError::MissingBasePrefab {
                            missing_guid: broken.missing_guid.clone(),
                            dependent_guid: nested.source.guid.clone(),
                        })
                    }
                };
                flatten_resolved_hierarchy(
                    nested_hierarchy,
                    UnpackMode::Completely,
                    Some(nested.parent_target_file_id),
                    out_nodes,
                    out_nested,
                )?;
            }
        }
    }
    Ok(())
}

fn extract_nested_modifications(nested: &ResolvedNestedPrefab) -> Vec<Modification> {
    let mut mods = Vec::new();
    let hierarchy = match &nested.resolved {
        ResolvedPrefab::Complete(h) => h,
        ResolvedPrefab::BrokenBase(_) => return mods,
    };
    let expected_level = OverrideLevel::NestedPrefab {
        owner: match &nested.declared_at {
            OverrideLevel::Prefab(g) => g.clone(),
            OverrideLevel::SceneInstance(id) => id.clone(),
            OverrideLevel::NestedPrefab { owner, .. } => owner.clone(),
        },
        instance_file_id: nested.instance_file_id,
    };
    for node in hierarchy.nodes.values() {
        for (prop, entries) in &node.provenance {
            for entry in entries {
                if entry.level == expected_level && entry.is_effective {
                    mods.push(Modification::new(
                        node.identity.target_file_id,
                        prop.clone(),
                        entry.value.clone(),
                    ));
                }
            }
        }
        for comp in &node.components {
            for (prop, entries) in &comp.provenance {
                for entry in entries {
                    if entry.level == expected_level && entry.is_effective {
                        mods.push(Modification::new(
                            comp.identity.target_file_id,
                            prop.clone(),
                            entry.value.clone(),
                        ));
                    }
                }
            }
        }
    }
    mods
}

fn apply_level_overrides(
    level: &OverrideLevel,
    level_source_id: &str,
    structural: &StructuralOverrides,
    modifications: &[Modification],
    nodes: &mut BTreeMap<SourceObjectId, ResolvedNode>,
    file_id_to_source: &mut HashMap<i64, SourceObjectId>,
    orphaned_overrides: &mut Vec<OrphanedOverride>,
) {
    for removed_child in &structural.removed_children {
        let resolved_id = file_id_to_source
            .get(&removed_child.target_file_id)
            .cloned()
            .unwrap_or_else(|| removed_child.clone());
        remove_node_recursive(&resolved_id, nodes, file_id_to_source);
    }

    for removed_comp in &structural.removed_components {
        file_id_to_source.remove(&removed_comp.target_file_id);
        for node in nodes.values_mut() {
            node.components
                .retain(|c| c.identity.target_file_id != removed_comp.target_file_id);
        }
    }

    for added_child in &structural.added_children {
        let parent_id = file_id_to_source
            .get(&added_child.parent_target.target_file_id)
            .cloned()
            .unwrap_or_else(|| added_child.parent_target.clone());
        if !nodes.contains_key(&parent_id) {
            continue;
        }
        let child_id = SourceObjectId::new(level_source_id, added_child.child.file_id);
        file_id_to_source.insert(added_child.child.file_id, child_id.clone());

        let mut provenance = BTreeMap::new();
        for (k, v) in &added_child.child.properties {
            let value = match v.as_str() {
                Some(text) => text.to_string(),
                None => continue,
            };
            provenance.insert(
                k.clone(),
                vec![ValueProvenanceEntry {
                    level: level.clone(),
                    value,
                    is_effective: true,
                }],
            );
        }

        let mut components = Vec::new();
        for comp_rec in &added_child.child.components {
            let comp_id = SourceObjectId::new(level_source_id, comp_rec.file_id);
            file_id_to_source.insert(comp_rec.file_id, comp_id.clone());
            let mut comp_prov = BTreeMap::new();
            for (k, v) in &comp_rec.properties {
                let value = match v.as_str() {
                    Some(text) => text.to_string(),
                    None => continue,
                };
                comp_prov.insert(
                    k.clone(),
                    vec![ValueProvenanceEntry {
                        level: level.clone(),
                        value,
                        is_effective: true,
                    }],
                );
            }
            components.push(ResolvedComponent {
                identity: comp_id,
                defined_at: level.clone(),
                class_name: comp_rec.class_name.clone(),
                properties: comp_rec.properties.clone(),
                object_references: BTreeMap::new(),
                provenance: comp_prov,
            });
        }

        let resolved_child = ResolvedNode {
            identity: child_id.clone(),
            defined_at: level.clone(),
            name: added_child.child.name.clone(),
            parent: Some(parent_id.clone()),
            children: Vec::new(),
            properties: added_child.child.properties.clone(),
            object_references: BTreeMap::new(),
            provenance,
            components,
        };
        nodes.insert(child_id.clone(), resolved_child);

        if let Some(parent_node) = nodes.get_mut(&parent_id) {
            if !parent_node.children.contains(&child_id) {
                match added_child.sibling_index {
                    Some(idx) if idx <= parent_node.children.len() => {
                        parent_node.children.insert(idx, child_id);
                    }
                    _ => parent_node.children.push(child_id),
                }
            }
        }
    }

    for (parent_target, ordered_added_ids) in &structural.added_sibling_order {
        let parent_id = file_id_to_source
            .get(&parent_target.target_file_id)
            .cloned()
            .unwrap_or_else(|| parent_target.clone());
        if let Some(parent_node) = nodes.get_mut(&parent_id) {
            let added_set: HashSet<i64> = ordered_added_ids.iter().copied().collect();
            let mut reordered: Vec<SourceObjectId> = parent_node
                .children
                .iter()
                .filter(|cid| !added_set.contains(&cid.target_file_id))
                .cloned()
                .collect();
            for file_id in ordered_added_ids {
                if let Some(cid) = file_id_to_source.get(file_id) {
                    if parent_node.children.contains(cid) && !reordered.contains(cid) {
                        reordered.push(cid.clone());
                    }
                }
            }
            parent_node.children = reordered;
        }
    }

    for added_comp in &structural.added_components {
        let owner_id = file_id_to_source
            .get(&added_comp.owner_target.target_file_id)
            .cloned()
            .unwrap_or_else(|| added_comp.owner_target.clone());
        if let Some(owner_node) = nodes.get_mut(&owner_id) {
            let comp_id = SourceObjectId::new(level_source_id, added_comp.component.file_id);
            file_id_to_source.insert(added_comp.component.file_id, comp_id.clone());
            let mut comp_prov = BTreeMap::new();
            for (k, v) in &added_comp.component.properties {
                let value = match v.as_str() {
                    Some(text) => text.to_string(),
                    None => continue,
                };
                comp_prov.insert(
                    k.clone(),
                    vec![ValueProvenanceEntry {
                        level: level.clone(),
                        value,
                        is_effective: true,
                    }],
                );
            }
            owner_node.components.push(ResolvedComponent {
                identity: comp_id,
                defined_at: level.clone(),
                class_name: added_comp.component.class_name.clone(),
                properties: added_comp.component.properties.clone(),
                object_references: BTreeMap::new(),
                provenance: comp_prov,
            });
        }
    }

    for modification in modifications {
        let mut matched = false;
        let effective_display_value = match &modification.object_reference {
            Some(reference) if modification.value.is_empty() => reference.to_encoded_string(),
            _ => modification.value.clone(),
        };
        if let Some(target_id) = file_id_to_source.get(&modification.target_file_id).cloned() {
            if let Some(node) = nodes.get_mut(&target_id) {
                matched = true;
                if modification.property_path == "m_Name" {
                    node.name = effective_display_value.clone();
                }
                set_property(
                    &mut node.properties,
                    &modification.property_path,
                    Node::Scalar(effective_display_value.clone()),
                );
                if let Some(reference) = &modification.object_reference {
                    node.object_references
                        .insert(modification.property_path.clone(), reference.clone());
                } else if let Some(parsed_ref) =
                    PrefabSourceReference::from_encoded_string(&modification.value)
                {
                    node.object_references
                        .insert(modification.property_path.clone(), parsed_ref);
                }
                let entries = node
                    .provenance
                    .entry(modification.property_path.clone())
                    .or_default();
                for entry in entries.iter_mut() {
                    entry.is_effective = false;
                }
                entries.push(ValueProvenanceEntry {
                    level: level.clone(),
                    value: effective_display_value.clone(),
                    is_effective: true,
                });
            }
        }
        if !matched {
            for node in nodes.values_mut() {
                for comp in &mut node.components {
                    if comp.identity.target_file_id == modification.target_file_id {
                        matched = true;
                        set_property(
                            &mut comp.properties,
                            &modification.property_path,
                            Node::Scalar(effective_display_value.clone()),
                        );
                        if let Some(reference) = &modification.object_reference {
                            comp.object_references
                                .insert(modification.property_path.clone(), reference.clone());
                        } else if let Some(parsed_ref) =
                            PrefabSourceReference::from_encoded_string(&modification.value)
                        {
                            comp.object_references
                                .insert(modification.property_path.clone(), parsed_ref);
                        }
                        let entries = comp
                            .provenance
                            .entry(modification.property_path.clone())
                            .or_default();
                        for entry in entries.iter_mut() {
                            entry.is_effective = false;
                        }
                        entries.push(ValueProvenanceEntry {
                            level: level.clone(),
                            value: effective_display_value.clone(),
                            is_effective: true,
                        });
                        break;
                    }
                }
                if matched {
                    break;
                }
            }
        }
        if !matched {
            orphaned_overrides.push(OrphanedOverride {
                level: level.clone(),
                target_file_id: modification.target_file_id,
                property_path: modification.property_path.clone(),
                value: effective_display_value,
            });
        }
    }
}

fn apply_modifications_to_hierarchy(
    hierarchy: &mut ResolvedHierarchy,
    level: &OverrideLevel,
    level_source_id: &str,
    modifications: &[Modification],
) {
    let mut file_id_to_source = HashMap::new();
    for node in hierarchy.nodes.values() {
        file_id_to_source.insert(node.identity.target_file_id, node.identity.clone());
        for component in &node.components {
            file_id_to_source.insert(
                component.identity.target_file_id,
                component.identity.clone(),
            );
        }
    }
    let mut orphaned = Vec::new();
    let structural = StructuralOverrides::default();
    apply_level_overrides(
        level,
        level_source_id,
        &structural,
        modifications,
        &mut hierarchy.nodes,
        &mut file_id_to_source,
        &mut orphaned,
    );
    hierarchy.orphaned_overrides.extend(orphaned);
}
fn remove_node_recursive(
    target_id: &SourceObjectId,
    nodes: &mut BTreeMap<SourceObjectId, ResolvedNode>,
    file_id_to_source: &mut HashMap<i64, SourceObjectId>,
) {
    let removed = match nodes.remove(target_id) {
        Some(node) => node,
        None => return,
    };
    file_id_to_source.remove(&removed.identity.target_file_id);
    for comp in &removed.components {
        file_id_to_source.remove(&comp.identity.target_file_id);
    }
    if let Some(parent_id) = &removed.parent {
        if let Some(parent_node) = nodes.get_mut(parent_id) {
            parent_node.children.retain(|cid| cid != target_id);
        }
    }
    for child_id in &removed.children {
        remove_node_recursive(child_id, nodes, file_id_to_source);
    }
}

fn collect_depth_first(
    current: &SourceObjectId,
    nodes: &BTreeMap<SourceObjectId, ResolvedNode>,
    out: &mut Vec<SourceObjectId>,
) {
    if out.contains(current) {
        return;
    }
    if let Some(node) = nodes.get(current) {
        out.push(current.clone());
        for child in &node.children {
            collect_depth_first(child, nodes, out);
        }
    }
}

const PREFAB_INSTANCE_CLASS_ID: u32 = 1001;
const GAME_OBJECT_CLASS_ID: u32 = 1;
const DEFAULT_VARIANT_INSTANCE_FILE_ID: i64 = 100100000;

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

struct ParsedInstanceData {
    instance: PrefabInstance,
    targeted_modifications: Vec<TargetedModification>,
    structural_overrides: StructuralOverrides,
}

fn class_id_for_component(class_name: &str) -> u32 {
    match class_name {
        "GameObject" => 1,
        "Transform" => 4,
        "Camera" => 20,
        "MeshRenderer" => 23,
        "MeshFilter" => 33,
        "Rigidbody" => 54,
        "BoxCollider" => 65,
        "AudioSource" => 82,
        "Light" => 108,
        "MonoBehaviour" => 114,
        "SphereCollider" => 135,
        "CapsuleCollider" => 136,
        "CharacterController" => 143,
        _ => 114,
    }
}

fn lookup_property<'a>(properties: &'a BTreeMap<String, Node>, path: &str) -> Option<&'a Node> {
    if let Some(node) = properties.get(path) {
        return Some(node);
    }
    let (parent, leaf) = path.rsplit_once('.')?;
    lookup_property(properties, parent)?.get(leaf)
}

fn set_property(properties: &mut BTreeMap<String, Node>, path: &str, value: Node) {
    if properties.contains_key(path) {
        properties.insert(path.to_string(), value);
        return;
    }
    if let Some((parent, leaf)) = path.rsplit_once('.') {
        if let Some(Node::Mapping(parent_entries)) = properties.get_mut(parent) {
            set_mapping_value(parent_entries, leaf, value);
            return;
        }
    }
    properties.insert(path.to_string(), value);
}

fn set_mapping_value(entries: &mut Vec<(String, Node)>, key: &str, value: Node) {
    match entries.iter_mut().find(|(entry_key, _)| entry_key == key) {
        Some(entry) => entry.1 = value,
        None => entries.push((key.to_string(), value)),
    }
}

fn property_entries(properties: &BTreeMap<String, Node>) -> Vec<(String, Node)> {
    let mut entries: Vec<(String, Node)> = Vec::new();
    for (key, value) in properties {
        let merged = match key.rsplit_once('.') {
            Some((parent, leaf)) => {
                match entries
                    .iter()
                    .position(|(entry_key, _)| entry_key == parent)
                {
                    Some(index) => match &mut entries[index].1 {
                        Node::Mapping(parent_entries) => {
                            set_mapping_value(parent_entries, leaf, value.clone());
                            true
                        }
                        _ => false,
                    },
                    None => false,
                }
            }
            None => false,
        };
        if !merged {
            entries.push((key.clone(), value.clone()));
        }
    }
    entries
}

fn emit_prefab_node_documents(
    node: &PrefabNodeRecord,
    prefab_instance_file_id: Option<i64>,
) -> Vec<Document> {
    let mut docs = Vec::new();
    let mut go_entries = Vec::new();
    go_entries.push((
        String::from("m_ObjectHideFlags"),
        Node::Scalar(String::from("0")),
    ));
    if let Some(inst_id) = prefab_instance_file_id {
        go_entries.push((
            String::from("m_PrefabInstance"),
            Node::Mapping(vec![(
                String::from("fileID"),
                Node::Scalar(inst_id.to_string()),
            )]),
        ));
    }
    let comp_refs: Vec<Node> = node
        .components
        .iter()
        .map(|c| {
            Node::Mapping(vec![(
                String::from("component"),
                Node::Mapping(vec![(
                    String::from("fileID"),
                    Node::Scalar(c.file_id.to_string()),
                )]),
            )])
        })
        .collect();
    go_entries.push((String::from("m_Component"), Node::Sequence(comp_refs)));
    if let Some(parent_id) = node.parent_file_id {
        go_entries.push((
            String::from("m_Father"),
            Node::Mapping(vec![(
                String::from("fileID"),
                Node::Scalar(parent_id.to_string()),
            )]),
        ));
    } else {
        go_entries.push((
            String::from("m_Father"),
            Node::Mapping(vec![(
                String::from("fileID"),
                Node::Scalar(String::from("0")),
            )]),
        ));
    }
    let child_refs: Vec<Node> = node
        .children
        .iter()
        .map(|cid| {
            Node::Mapping(vec![(
                String::from("fileID"),
                Node::Scalar(cid.to_string()),
            )])
        })
        .collect();
    go_entries.push((String::from("m_Children"), Node::Sequence(child_refs)));
    go_entries.push((String::from("m_Name"), Node::Scalar(node.name.clone())));
    for (key, value) in property_entries(&node.properties) {
        if key == "m_Name"
            || key == "m_ObjectHideFlags"
            || key == "m_Component"
            || key == "m_Father"
            || key == "m_Children"
            || key == "m_PrefabInstance"
        {
            continue;
        }
        go_entries.push((key, value));
    }

    docs.push(Document {
        class_id: GAME_OBJECT_CLASS_ID,
        file_id: node.file_id,
        stripped: false,
        root: Node::Mapping(vec![(
            String::from("GameObject"),
            Node::Mapping(go_entries),
        )]),
    });

    for comp in &node.components {
        docs.push(emit_component_document(
            comp,
            node.file_id,
            prefab_instance_file_id,
        ));
    }

    docs
}

fn emit_component_document(
    comp: &PrefabComponentRecord,
    owner_go_file_id: i64,
    prefab_instance_file_id: Option<i64>,
) -> Document {
    let class_id = class_id_for_component(&comp.class_name);
    let mut entries = Vec::new();
    entries.push((
        String::from("m_ObjectHideFlags"),
        Node::Scalar(String::from("0")),
    ));
    if let Some(inst_id) = prefab_instance_file_id {
        entries.push((
            String::from("m_PrefabInstance"),
            Node::Mapping(vec![(
                String::from("fileID"),
                Node::Scalar(inst_id.to_string()),
            )]),
        ));
    }
    entries.push((
        String::from("m_GameObject"),
        Node::Mapping(vec![(
            String::from("fileID"),
            Node::Scalar(owner_go_file_id.to_string()),
        )]),
    ));
    for (key, value) in property_entries(&comp.properties) {
        if key == "m_ObjectHideFlags" || key == "m_GameObject" || key == "m_PrefabInstance" {
            continue;
        }
        entries.push((key, value));
    }
    Document {
        class_id,
        file_id: comp.file_id,
        stripped: false,
        root: Node::Mapping(vec![(comp.class_name.clone(), Node::Mapping(entries))]),
    }
}

fn upsert_prefab_instance_in_stream(
    stream: &mut Stream,
    instance_file_id: i64,
    source: &PrefabSource,
    transform_parent: i64,
    targeted_modifications: &[TargetedModification],
    raw_modifications: &[Modification],
    structural: &StructuralOverrides,
) {
    let mod_items: Vec<Node> = if !targeted_modifications.is_empty() || raw_modifications.is_empty()
    {
        targeted_modifications
            .iter()
            .map(|tm| {
                let obj_ref_node = match &tm.object_reference {
                    Some(reference) => reference.to_node(),
                    None => Node::Mapping(vec![(
                        String::from("fileID"),
                        Node::Scalar(String::from("0")),
                    )]),
                };
                Node::Mapping(vec![
                    (
                        String::from("target"),
                        Node::Mapping(vec![
                            (
                                String::from("fileID"),
                                Node::Scalar(tm.target.target_file_id.to_string()),
                            ),
                            (
                                String::from("guid"),
                                Node::Scalar(tm.target.source_guid.clone()),
                            ),
                            (String::from("type"), Node::Scalar(String::from("3"))),
                        ]),
                    ),
                    (
                        String::from("propertyPath"),
                        Node::Scalar(tm.property_path.clone()),
                    ),
                    (String::from("value"), Node::Scalar(tm.value.clone())),
                    (String::from("objectReference"), obj_ref_node),
                ])
            })
            .collect()
    } else {
        raw_modifications
            .iter()
            .map(|m| {
                let obj_ref_node = match &m.object_reference {
                    Some(reference) => reference.to_node(),
                    None => Node::Mapping(vec![(
                        String::from("fileID"),
                        Node::Scalar(String::from("0")),
                    )]),
                };
                Node::Mapping(vec![
                    (
                        String::from("target"),
                        Node::Mapping(vec![
                            (
                                String::from("fileID"),
                                Node::Scalar(m.target_file_id.to_string()),
                            ),
                            (String::from("guid"), Node::Scalar(source.guid.clone())),
                            (String::from("type"), Node::Scalar(String::from("3"))),
                        ]),
                    ),
                    (
                        String::from("propertyPath"),
                        Node::Scalar(m.property_path.clone()),
                    ),
                    (String::from("value"), Node::Scalar(m.value.clone())),
                    (String::from("objectReference"), obj_ref_node),
                ])
            })
            .collect()
    };

    let removed_components: Vec<Node> = structural
        .removed_components
        .iter()
        .map(|rc| {
            Node::Mapping(vec![
                (
                    String::from("fileID"),
                    Node::Scalar(rc.target_file_id.to_string()),
                ),
                (String::from("guid"), Node::Scalar(rc.source_guid.clone())),
                (String::from("type"), Node::Scalar(String::from("3"))),
            ])
        })
        .collect();

    let removed_game_objects: Vec<Node> = structural
        .removed_children
        .iter()
        .map(|rg| {
            Node::Mapping(vec![
                (
                    String::from("fileID"),
                    Node::Scalar(rg.target_file_id.to_string()),
                ),
                (String::from("guid"), Node::Scalar(rg.source_guid.clone())),
                (String::from("type"), Node::Scalar(String::from("3"))),
            ])
        })
        .collect();

    let added_game_objects: Vec<Node> = structural
        .added_children
        .iter()
        .map(|ac| {
            let mut entries = vec![
                (
                    String::from("targetCorrespondingSourceObject"),
                    Node::Mapping(vec![
                        (
                            String::from("fileID"),
                            Node::Scalar(ac.parent_target.target_file_id.to_string()),
                        ),
                        (
                            String::from("guid"),
                            Node::Scalar(ac.parent_target.source_guid.clone()),
                        ),
                        (String::from("type"), Node::Scalar(String::from("3"))),
                    ]),
                ),
                (
                    String::from("insertIndex"),
                    Node::Scalar(
                        ac.sibling_index
                            .map(|idx| idx as i64)
                            .unwrap_or(-1)
                            .to_string(),
                    ),
                ),
                (
                    String::from("addedObject"),
                    Node::Mapping(vec![(
                        String::from("fileID"),
                        Node::Scalar(ac.child.file_id.to_string()),
                    )]),
                ),
            ];
            if let Some(order) = structural.added_sibling_order.get(&ac.parent_target) {
                let order_str = order
                    .iter()
                    .map(i64::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                entries.push((String::from("siblingOrder"), Node::Scalar(order_str)));
            }
            Node::Mapping(entries)
        })
        .collect();

    let added_components: Vec<Node> = structural
        .added_components
        .iter()
        .map(|ac| {
            Node::Mapping(vec![
                (
                    String::from("targetCorrespondingSourceObject"),
                    Node::Mapping(vec![
                        (
                            String::from("fileID"),
                            Node::Scalar(ac.owner_target.target_file_id.to_string()),
                        ),
                        (
                            String::from("guid"),
                            Node::Scalar(ac.owner_target.source_guid.clone()),
                        ),
                        (String::from("type"), Node::Scalar(String::from("3"))),
                    ]),
                ),
                (
                    String::from("insertIndex"),
                    Node::Scalar(String::from("-1")),
                ),
                (
                    String::from("addedObject"),
                    Node::Mapping(vec![(
                        String::from("fileID"),
                        Node::Scalar(ac.component.file_id.to_string()),
                    )]),
                ),
            ])
        })
        .collect();

    let prefab_root = Node::Mapping(vec![(
        String::from("PrefabInstance"),
        Node::Mapping(vec![
            (
                String::from("m_ObjectHideFlags"),
                Node::Scalar(String::from("0")),
            ),
            (
                String::from("serializedVersion"),
                Node::Scalar(String::from("2")),
            ),
            (
                String::from("m_Modification"),
                Node::Mapping(vec![
                    (
                        String::from("serializedVersion"),
                        Node::Scalar(String::from("3")),
                    ),
                    (
                        String::from("m_TransformParent"),
                        Node::Mapping(vec![(
                            String::from("fileID"),
                            Node::Scalar(transform_parent.to_string()),
                        )]),
                    ),
                    (String::from("m_Modifications"), Node::Sequence(mod_items)),
                    (
                        String::from("m_RemovedComponents"),
                        Node::Sequence(removed_components),
                    ),
                    (
                        String::from("m_RemovedGameObjects"),
                        Node::Sequence(removed_game_objects),
                    ),
                    (
                        String::from("m_AddedGameObjects"),
                        Node::Sequence(added_game_objects),
                    ),
                    (
                        String::from("m_AddedComponents"),
                        Node::Sequence(added_components),
                    ),
                ]),
            ),
            (
                String::from("m_SourcePrefab"),
                Node::Mapping(vec![
                    (
                        String::from("fileID"),
                        Node::Scalar(source.file_id.to_string()),
                    ),
                    (String::from("guid"), Node::Scalar(source.guid.clone())),
                    (String::from("type"), Node::Scalar(String::from("3"))),
                ]),
            ),
        ]),
    )]);

    let mut companion_docs = Vec::new();
    for ac in &structural.added_children {
        companion_docs.extend(emit_prefab_node_documents(
            &ac.child,
            Some(instance_file_id),
        ));
    }
    for ac in &structural.added_components {
        companion_docs.push(emit_component_document(
            &ac.component,
            ac.owner_target.target_file_id,
            Some(instance_file_id),
        ));
    }

    stream.documents.retain(|doc| {
        if doc.class_id == PREFAB_INSTANCE_CLASS_ID {
            return true;
        }
        let owned_by_instance = match &doc.root {
            Node::Mapping(entries) => entries.first().and_then(|(_, body)| {
                body.get("m_PrefabInstance")
                    .and_then(|n| n.get("fileID"))
                    .and_then(Node::as_i64)
            }),
            _ => None,
        };
        owned_by_instance != Some(instance_file_id)
    });

    if let Some(existing_doc) = stream
        .documents
        .iter_mut()
        .find(|d| d.class_id == PREFAB_INSTANCE_CLASS_ID && d.file_id == instance_file_id)
    {
        existing_doc.root = prefab_root;
    } else {
        stream.documents.push(Document {
            class_id: PREFAB_INSTANCE_CLASS_ID,
            file_id: instance_file_id,
            stripped: false,
            root: prefab_root,
        });
    }
    stream.documents.extend(companion_docs);
}

fn parse_prefab_instance_from_stream(
    stream: &Stream,
    instance_file_id: i64,
) -> Option<ParsedInstanceData> {
    let doc = stream
        .documents
        .iter()
        .find(|d| d.class_id == PREFAB_INSTANCE_CLASS_ID && d.file_id == instance_file_id)?;
    let body = doc.root.get("PrefabInstance")?;
    let instance = crate::prefab::parse_instance(body)?;
    let default_guid = instance.source.guid.clone();

    let modification_node = body.get("m_Modification");
    let mut targeted_modifications = Vec::new();
    if let Some(items) = modification_node
        .and_then(|m| m.get("m_Modifications"))
        .and_then(Node::as_sequence)
    {
        for item in items {
            let target_node = match item.get("target") {
                Some(n) => n,
                None => continue,
            };
            let target_file_id = match target_node.get("fileID").and_then(Node::as_i64) {
                Some(id) => id,
                None => continue,
            };
            let target_guid = target_node
                .get("guid")
                .and_then(Node::as_str)
                .filter(|g| !g.is_empty())
                .unwrap_or(&default_guid)
                .to_string();
            let property_path = match item.get("propertyPath").and_then(Node::as_str) {
                Some(p) => p.to_string(),
                None => continue,
            };
            let value = item
                .get("value")
                .and_then(Node::as_str)
                .unwrap_or_default()
                .to_string();
            let object_reference = item
                .get("objectReference")
                .and_then(PrefabSourceReference::from_node);
            targeted_modifications.push(TargetedModification {
                target: SourceObjectId::new(target_guid, target_file_id),
                property_path,
                value,
                object_reference,
            });
        }
    }

    let mut structural = StructuralOverrides::default();

    if let Some(items) = modification_node
        .and_then(|m| m.get("m_RemovedComponents"))
        .and_then(Node::as_sequence)
    {
        for item in items {
            if let Some(file_id) = item.get("fileID").and_then(Node::as_i64) {
                let guid = item
                    .get("guid")
                    .and_then(Node::as_str)
                    .filter(|g| !g.is_empty())
                    .unwrap_or(&default_guid)
                    .to_string();
                structural
                    .removed_components
                    .push(SourceObjectId::new(guid, file_id));
            }
        }
    }

    if let Some(items) = modification_node
        .and_then(|m| m.get("m_RemovedGameObjects"))
        .and_then(Node::as_sequence)
    {
        for item in items {
            if let Some(file_id) = item.get("fileID").and_then(Node::as_i64) {
                let guid = item
                    .get("guid")
                    .and_then(Node::as_str)
                    .filter(|g| !g.is_empty())
                    .unwrap_or(&default_guid)
                    .to_string();
                structural
                    .removed_children
                    .push(SourceObjectId::new(guid, file_id));
            }
        }
    }

    if let Some(items) = modification_node
        .and_then(|m| m.get("m_AddedGameObjects"))
        .and_then(Node::as_sequence)
    {
        for item in items {
            let parent_node = match item.get("targetCorrespondingSourceObject") {
                Some(n) => n,
                None => continue,
            };
            let parent_file_id = match parent_node.get("fileID").and_then(Node::as_i64) {
                Some(id) => id,
                None => continue,
            };
            let parent_guid = parent_node
                .get("guid")
                .and_then(Node::as_str)
                .filter(|g| !g.is_empty())
                .unwrap_or(&default_guid)
                .to_string();
            let parent_target = SourceObjectId::new(parent_guid, parent_file_id);

            let insert_idx = item.get("insertIndex").and_then(Node::as_i64).unwrap_or(-1);
            let sibling_index = if insert_idx >= 0 {
                Some(insert_idx as usize)
            } else {
                None
            };

            let added_file_id = match item
                .get("addedObject")
                .and_then(|n| n.get("fileID"))
                .and_then(Node::as_i64)
            {
                Some(id) => id,
                None => continue,
            };

            if let Some(order_str) = item.get("siblingOrder").and_then(Node::as_str) {
                let order_ids: Vec<i64> = order_str
                    .split(',')
                    .filter_map(|part| part.trim().parse::<i64>().ok())
                    .collect();
                if !order_ids.is_empty() {
                    structural
                        .added_sibling_order
                        .insert(parent_target.clone(), order_ids);
                }
            }

            let child = parse_node_record_from_stream(
                stream,
                added_file_id,
                Some(parent_file_id),
                Some(instance_file_id),
            )
            .unwrap_or_else(|| {
                PrefabNodeRecord::new(
                    added_file_id,
                    format!("AddedNode_{added_file_id}"),
                    Some(parent_file_id),
                )
            });

            structural.added_children.push(AddedChildOverride {
                parent_target,
                child,
                sibling_index,
            });
        }
    }

    if let Some(items) = modification_node
        .and_then(|m| m.get("m_AddedComponents"))
        .and_then(Node::as_sequence)
    {
        for item in items {
            let owner_node = match item.get("targetCorrespondingSourceObject") {
                Some(n) => n,
                None => continue,
            };
            let owner_file_id = match owner_node.get("fileID").and_then(Node::as_i64) {
                Some(id) => id,
                None => continue,
            };
            let owner_guid = owner_node
                .get("guid")
                .and_then(Node::as_str)
                .filter(|g| !g.is_empty())
                .unwrap_or(&default_guid)
                .to_string();
            let owner_target = SourceObjectId::new(owner_guid, owner_file_id);

            let added_comp_file_id = match item
                .get("addedObject")
                .and_then(|n| n.get("fileID"))
                .and_then(Node::as_i64)
            {
                Some(id) => id,
                None => continue,
            };

            let component = parse_component_record_from_stream(stream, added_comp_file_id)
                .unwrap_or_else(|| PrefabComponentRecord::new(added_comp_file_id, "MonoBehaviour"));

            structural.added_components.push(AddedComponentOverride {
                owner_target,
                component,
            });
        }
    }

    Some(ParsedInstanceData {
        instance,
        targeted_modifications,
        structural_overrides: structural,
    })
}

fn parse_base_nodes_from_stream(stream: &Stream) -> Vec<PrefabNodeRecord> {
    let mut nodes = Vec::new();
    for doc in &stream.documents {
        if doc.class_id != GAME_OBJECT_CLASS_ID || doc.stripped {
            continue;
        }
        let go_body = match doc.root.get("GameObject") {
            Some(b) => b,
            None => continue,
        };
        let prefab_inst = go_body
            .get("m_PrefabInstance")
            .and_then(|n| n.get("fileID"))
            .and_then(Node::as_i64)
            .unwrap_or(0);
        if prefab_inst != 0 {
            continue;
        }
        if let Some(node) = parse_node_record_from_stream(stream, doc.file_id, None, None) {
            nodes.push(node);
        }
    }
    nodes
}

fn parse_node_record_from_stream(
    stream: &Stream,
    go_file_id: i64,
    fallback_parent: Option<i64>,
    expected_instance_id: Option<i64>,
) -> Option<PrefabNodeRecord> {
    let doc = stream
        .documents
        .iter()
        .find(|d| d.class_id == GAME_OBJECT_CLASS_ID && d.file_id == go_file_id && !d.stripped)?;
    let go_body = doc.root.get("GameObject")?;
    if let Some(exp_inst) = expected_instance_id {
        let actual_inst = go_body
            .get("m_PrefabInstance")
            .and_then(|n| n.get("fileID"))
            .and_then(Node::as_i64)
            .unwrap_or(0);
        if actual_inst != 0 && actual_inst != exp_inst {
            return None;
        }
    }

    let name = go_body
        .get("m_Name")
        .and_then(Node::as_str)
        .unwrap_or("GameObject")
        .to_string();

    let parent_file_id = go_body
        .get("m_Father")
        .and_then(|n| n.get("fileID"))
        .and_then(Node::as_i64)
        .and_then(|id| if id == 0 { None } else { Some(id) })
        .or(fallback_parent);

    let mut children = Vec::new();
    if let Some(seq) = go_body.get("m_Children").and_then(Node::as_sequence) {
        for item in seq {
            if let Some(cid) = item.get("fileID").and_then(Node::as_i64) {
                children.push(cid);
            }
        }
    }

    let mut properties = BTreeMap::new();
    properties.insert(String::from("m_Name"), Node::Scalar(name.clone()));
    if let Some(entries) = go_body.as_mapping() {
        for (k, v) in entries {
            if k == "m_ObjectHideFlags"
                || k == "m_Component"
                || k == "m_Father"
                || k == "m_Children"
                || k == "m_PrefabInstance"
            {
                continue;
            }
            properties.insert(k.clone(), v.clone());
        }
    }

    let mut comp_ids = Vec::new();
    if let Some(seq) = go_body.get("m_Component").and_then(Node::as_sequence) {
        for item in seq {
            if let Some(cid) = item
                .get("component")
                .and_then(|c| c.get("fileID"))
                .and_then(Node::as_i64)
            {
                comp_ids.push(cid);
            }
        }
    }

    let mut components = Vec::new();
    for cid in comp_ids {
        if let Some(comp) = parse_component_record_from_stream(stream, cid) {
            components.push(comp);
        }
    }

    Some(PrefabNodeRecord {
        file_id: go_file_id,
        name,
        parent_file_id,
        children,
        properties,
        components,
    })
}

fn parse_component_record_from_stream(
    stream: &Stream,
    comp_file_id: i64,
) -> Option<PrefabComponentRecord> {
    let doc = stream.documents.iter().find(|d| {
        d.file_id == comp_file_id
            && d.class_id != GAME_OBJECT_CLASS_ID
            && d.class_id != PREFAB_INSTANCE_CLASS_ID
            && !d.stripped
    })?;
    let entries = doc.root.as_mapping()?;
    let (class_name, body) = entries.first()?;
    let mut properties = BTreeMap::new();
    if let Some(body_entries) = body.as_mapping() {
        for (k, v) in body_entries {
            if k == "m_ObjectHideFlags" || k == "m_GameObject" || k == "m_PrefabInstance" {
                continue;
            }
            properties.insert(k.clone(), v.clone());
        }
    }
    Some(PrefabComponentRecord {
        file_id: comp_file_id,
        class_name: class_name.clone(),
        properties,
    })
}
