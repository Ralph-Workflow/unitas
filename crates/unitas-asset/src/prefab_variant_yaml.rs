use std::collections::BTreeMap;

use crate::prefab::{Modification, PrefabInstance, PrefabSource, PrefabSourceReference};
use crate::unity_yaml::{Document, Node, Stream};

use super::{
    AddedChildOverride, AddedComponentOverride, PrefabComponentRecord, PrefabNodeRecord,
    SourceObjectId, StructuralOverrides, TargetedModification,
};

pub(super) const PREFAB_INSTANCE_CLASS_ID: u32 = 1001;
pub(super) const GAME_OBJECT_CLASS_ID: u32 = 1;
pub(super) const DEFAULT_VARIANT_INSTANCE_FILE_ID: i64 = 100100000;

pub(super) struct ParsedInstanceData {
    pub instance: PrefabInstance,
    pub targeted_modifications: Vec<TargetedModification>,
    pub structural_overrides: StructuralOverrides,
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

pub(super) fn lookup_property<'a>(
    properties: &'a BTreeMap<String, Node>,
    path: &str,
) -> Option<&'a Node> {
    if let Some(node) = properties.get(path) {
        return Some(node);
    }
    let (parent, leaf) = path.rsplit_once('.')?;
    lookup_property(properties, parent)?.get(leaf)
}

pub(super) fn set_property(properties: &mut BTreeMap<String, Node>, path: &str, value: Node) {
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

pub(super) fn emit_prefab_node_documents(
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

pub(super) fn upsert_prefab_instance_in_stream(
    stream: &mut Stream,
    instance_file_id: i64,
    source: &PrefabSource,
    transform_parent: i64,
    targeted_modifications: &[TargetedModification],
    raw_modifications: &[Modification],
    structural: &StructuralOverrides,
) {
    let mod_items = modification_nodes(targeted_modifications, raw_modifications, source);

    let removed_components = removed_component_nodes(structural);

    let removed_game_objects = removed_game_object_nodes(structural);

    let added_game_objects = added_game_object_nodes(structural);

    let added_components = added_component_nodes(structural);

    let prefab_root = prefab_instance_root(
        source,
        transform_parent,
        mod_items,
        removed_components,
        removed_game_objects,
        added_game_objects,
        added_components,
    );

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

fn modification_nodes(
    targeted_modifications: &[TargetedModification],
    raw_modifications: &[Modification],
    source: &PrefabSource,
) -> Vec<Node> {
    if !targeted_modifications.is_empty() || raw_modifications.is_empty() {
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
    }
}

fn removed_component_nodes(structural: &StructuralOverrides) -> Vec<Node> {
    structural
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
        .collect()
}

fn removed_game_object_nodes(structural: &StructuralOverrides) -> Vec<Node> {
    structural
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
        .collect()
}

fn added_game_object_nodes(structural: &StructuralOverrides) -> Vec<Node> {
    structural
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
        .collect()
}

fn added_component_nodes(structural: &StructuralOverrides) -> Vec<Node> {
    structural
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
        .collect()
}

fn prefab_instance_root(
    source: &PrefabSource,
    transform_parent: i64,
    mod_items: Vec<Node>,
    removed_components: Vec<Node>,
    removed_game_objects: Vec<Node>,
    added_game_objects: Vec<Node>,
    added_components: Vec<Node>,
) -> Node {
    Node::Mapping(vec![(
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
    )])
}

pub(super) fn parse_prefab_instance_from_stream(
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

pub(super) fn parse_base_nodes_from_stream(stream: &Stream) -> Vec<PrefabNodeRecord> {
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
