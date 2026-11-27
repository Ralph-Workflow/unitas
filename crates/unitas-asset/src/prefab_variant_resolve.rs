use std::collections::{BTreeMap, HashMap, HashSet};

use crate::prefab::{Modification, PrefabSource, PrefabSourceReference};
use crate::unity_yaml::{Node, Stream};

use super::yaml::{
    emit_prefab_node_documents, lookup_property, set_property, upsert_prefab_instance_in_stream,
    PREFAB_INSTANCE_CLASS_ID,
};
use super::{
    validate_override, NestedPrefabReference, PrefabComponentRecord, PrefabNodeRecord,
    PrefabRegistry, PrefabVariantDefinition, PrefabVariantError, ScenePrefabInstance,
    SourceObjectId, StructuralOverrides, TargetedModification,
};

/// Chain of prefab guids resolved from a leaf, plus the first missing base if the chain broke.
type PrefabChain = (Vec<String>, Option<(String, OverrideLevel)>);

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
        let (mut raw_chain_from_leaf, missing_guid) =
            self.collect_prefab_chain(leaf_prefab_guid, scene_instance)?;

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
            return Ok(ResolvedPrefab::BrokenBase(self.broken_base_state(
                raw_chain_from_leaf,
                scene_instance,
                missing,
                broken_at_level,
            )));
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
                });
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
        let mut file_id_to_source: HashMap<i64, SourceObjectId> = HashMap::new();
        let mut nodes = build_base_nodes(&base_guid, base_def, &base_level, &mut file_id_to_source);

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

    fn collect_prefab_chain(
        &self,
        leaf_prefab_guid: &str,
        scene_instance: Option<&ScenePrefabInstance>,
    ) -> Result<PrefabChain, PrefabVariantError> {
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

        Ok((raw_chain_from_leaf, missing_guid))
    }

    fn broken_base_state(
        &self,
        raw_chain_from_leaf: Vec<String>,
        scene_instance: Option<&ScenePrefabInstance>,
        missing: String,
        broken_at_level: OverrideLevel,
    ) -> BrokenPrefabState {
        let mut raw_chain_from_leaf = raw_chain_from_leaf;
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

        BrokenPrefabState {
            missing_guid: missing,
            broken_at_level,
            chain_before_break,
            retained_modifications,
            retained_structural_overrides: retained_structural,
        }
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
            return self.apply_nested_override_to_level(
                owner,
                *instance_file_id,
                from_level,
                target_level,
                target_file_id,
                property_path,
            );
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
                    });
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
                });
            }
        };
        let target_idx = match full_chain.iter().position(|lvl| lvl == target_level) {
            Some(idx) => idx,
            None => {
                return Err(PrefabVariantError::InvalidTargetLevel {
                    from_level: from_level.as_id(),
                    target_level: target_level.as_id(),
                    reason: String::from("target level is not in the inheritance chain"),
                });
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
                        });
                    }
                };
                match inst.remove_modification(target_file_id, property_path) {
                    Some(val) => val,
                    None => {
                        return Err(PrefabVariantError::OverrideNotFound {
                            level: from_level.as_id(),
                            target_file_id,
                            property_path: property_path.to_string(),
                        });
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
                        });
                    }
                };
                match def.remove_modification(target_file_id, property_path) {
                    Some(val) => val,
                    None => {
                        return Err(PrefabVariantError::OverrideNotFound {
                            level: from_level.as_id(),
                            target_file_id,
                            property_path: property_path.to_string(),
                        });
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
                        });
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

    fn apply_nested_override_to_level(
        &mut self,
        owner: &String,
        instance_file_id: i64,
        from_level: &OverrideLevel,
        target_level: &OverrideLevel,
        target_file_id: i64,
        property_path: &str,
    ) -> Result<(), PrefabVariantError> {
        let nested_source_guid = {
            let owner_def = match self.prefabs.get(owner) {
                Some(def) => def,
                None => {
                    return Err(PrefabVariantError::MissingBasePrefab {
                        missing_guid: owner.clone(),
                        dependent_guid: owner.clone(),
                    });
                }
            };
            let nested_ref = match owner_def
                .nested_prefabs
                .iter()
                .find(|n| n.instance_file_id == instance_file_id)
            {
                Some(n) => n,
                None => {
                    return Err(PrefabVariantError::OverrideNotFound {
                        level: from_level.as_id(),
                        target_file_id,
                        property_path: property_path.to_string(),
                    });
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
                });
            }
        };
        let removed_mod = {
            let owner_def = match self.prefabs.get_mut(owner) {
                Some(def) => def,
                None => {
                    return Err(PrefabVariantError::MissingBasePrefab {
                        missing_guid: owner.clone(),
                        dependent_guid: owner.clone(),
                    });
                }
            };
            let nested_ref = match owner_def
                .nested_prefabs
                .iter_mut()
                .find(|n| n.instance_file_id == instance_file_id)
            {
                Some(n) => n,
                None => {
                    return Err(PrefabVariantError::OverrideNotFound {
                        level: from_level.as_id(),
                        target_file_id,
                        property_path: property_path.to_string(),
                    });
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
                    });
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
                });
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
                        });
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
                        });
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
                        });
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
                        });
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
                        });
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
                        });
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
                        });
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
                        });
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
                });
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

fn build_base_nodes(
    base_guid: &str,
    base_def: &PrefabVariantDefinition,
    base_level: &OverrideLevel,
    file_id_to_source: &mut HashMap<i64, SourceObjectId>,
) -> BTreeMap<SourceObjectId, ResolvedNode> {
    let mut nodes: BTreeMap<SourceObjectId, ResolvedNode> = BTreeMap::new();

    for node_rec in &base_def.nodes {
        let identity = SourceObjectId::new(base_guid, node_rec.file_id);
        file_id_to_source.insert(node_rec.file_id, identity.clone());
        let parent = node_rec
            .parent_file_id
            .map(|pid| SourceObjectId::new(base_guid, pid));
        let children = node_rec
            .children
            .iter()
            .map(|cid| SourceObjectId::new(base_guid, *cid))
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
            let comp_id = SourceObjectId::new(base_guid, comp_rec.file_id);
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

    nodes
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
                        });
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

    apply_added_children(level, level_source_id, structural, nodes, file_id_to_source);

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

    apply_added_components(level, level_source_id, structural, nodes, file_id_to_source);

    apply_modifications(
        level,
        modifications,
        nodes,
        file_id_to_source,
        orphaned_overrides,
    );
}

fn apply_added_children(
    level: &OverrideLevel,
    level_source_id: &str,
    structural: &StructuralOverrides,
    nodes: &mut BTreeMap<SourceObjectId, ResolvedNode>,
    file_id_to_source: &mut HashMap<i64, SourceObjectId>,
) {
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
}

fn apply_added_components(
    level: &OverrideLevel,
    level_source_id: &str,
    structural: &StructuralOverrides,
    nodes: &mut BTreeMap<SourceObjectId, ResolvedNode>,
    file_id_to_source: &mut HashMap<i64, SourceObjectId>,
) {
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
}

fn apply_modifications(
    level: &OverrideLevel,
    modifications: &[Modification],
    nodes: &mut BTreeMap<SourceObjectId, ResolvedNode>,
    file_id_to_source: &mut HashMap<i64, SourceObjectId>,
    orphaned_overrides: &mut Vec<OrphanedOverride>,
) {
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
