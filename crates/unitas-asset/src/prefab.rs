use std::collections::HashMap;
use std::path::{Path, PathBuf};

use unitas_core::{EntityId, SceneGraph};

use crate::meta::{GuidIndex, MetaError};
use crate::scene::GameObjectRecord;
use crate::unity_yaml::{Document, Node, Stream};

const GAME_OBJECT_CLASS: u32 = 1;
const TRANSFORM_CLASS: u32 = 4;
const NAME_PROPERTY: &str = "m_Name";
const POSITION_PROPERTY: &str = "m_LocalPosition";
const ROTATION_PROPERTY: &str = "m_LocalRotation";
const SCALE_PROPERTY: &str = "m_LocalScale";
const PROJECT_SETTINGS_DIR: &str = "ProjectSettings";
const ASSETS_DIR: &str = "Assets";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PrefabSource {
    pub guid: String,
    pub file_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrefabSourceReference {
    pub file_id: i64,
    pub guid: Option<String>,
    pub ref_type: Option<i64>,
}

impl PrefabSourceReference {
    pub fn local(file_id: i64) -> Self {
        Self {
            file_id,
            guid: None,
            ref_type: None,
        }
    }

    pub fn external(file_id: i64, guid: impl Into<String>, ref_type: i64) -> Self {
        Self {
            file_id,
            guid: Some(guid.into()),
            ref_type: Some(ref_type),
        }
    }

    pub fn to_node(&self) -> Node {
        let mut entries = vec![(
            String::from("fileID"),
            Node::Scalar(self.file_id.to_string()),
        )];
        if let Some(guid) = &self.guid {
            entries.push((String::from("guid"), Node::Scalar(guid.clone())));
        }
        if let Some(ref_type) = self.ref_type {
            entries.push((String::from("type"), Node::Scalar(ref_type.to_string())));
        }
        Node::Mapping(entries)
    }

    pub fn from_node(node: &Node) -> Option<Self> {
        let file_id = node.get("fileID").and_then(Node::as_i64)?;
        let guid = node
            .get("guid")
            .and_then(Node::as_str)
            .filter(|g| !g.is_empty())
            .map(str::to_string);
        let ref_type = node.get("type").and_then(Node::as_i64);
        if file_id == 0 && guid.is_none() {
            return None;
        }
        Some(Self {
            file_id,
            guid,
            ref_type,
        })
    }

    pub fn to_encoded_string(&self) -> String {
        match (&self.guid, self.ref_type) {
            (Some(guid), Some(ref_type)) => {
                format!(
                    "{{fileID: {}, guid: {}, type: {}}}",
                    self.file_id, guid, ref_type
                )
            }
            (Some(guid), None) => format!("{{fileID: {}, guid: {}}}", self.file_id, guid),
            (None, _) => format!("{{fileID: {}}}", self.file_id),
        }
    }

    pub fn from_encoded_string(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        let inner = trimmed.strip_prefix('{')?.strip_suffix('}')?;
        let mut file_id: Option<i64> = None;
        let mut guid: Option<String> = None;
        let mut ref_type: Option<i64> = None;
        for part in inner.split(',') {
            let (k, v) = part.split_once(':')?;
            let key = k.trim();
            let val = v.trim();
            match key {
                "fileID" => file_id = val.parse::<i64>().ok(),
                "guid" if !val.is_empty() => guid = Some(val.to_string()),
                "type" => ref_type = val.parse::<i64>().ok(),
                _ => {}
            }
        }
        let resolved_file_id = file_id?;
        if resolved_file_id == 0 && guid.is_none() {
            return None;
        }
        Some(Self {
            file_id: resolved_file_id,
            guid,
            ref_type,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modification {
    pub target_file_id: i64,
    pub property_path: String,
    pub value: String,
    pub object_reference: Option<PrefabSourceReference>,
}

impl Modification {
    pub fn new(
        target_file_id: i64,
        property_path: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            target_file_id,
            property_path: property_path.into(),
            value: value.into(),
            object_reference: None,
        }
    }

    pub fn with_object_reference(
        target_file_id: i64,
        property_path: impl Into<String>,
        object_reference: PrefabSourceReference,
    ) -> Self {
        Self {
            target_file_id,
            property_path: property_path.into(),
            value: String::new(),
            object_reference: Some(object_reference),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefabInstance {
    pub source: PrefabSource,
    pub transform_parent: i64,
    pub modifications: Vec<Modification>,
}

impl PrefabSource {
    pub fn new(guid: impl Into<String>, file_id: i64) -> Self {
        Self {
            guid: guid.into(),
            file_id,
        }
    }
}

impl PrefabInstance {
    pub fn new(
        source: PrefabSource,
        transform_parent: i64,
        modifications: Vec<Modification>,
    ) -> Self {
        Self {
            source,
            transform_parent,
            modifications,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
    Z,
    W,
}

impl Axis {
    fn index(self) -> usize {
        match self {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
            Axis::W => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Component {
    Translation,
    Rotation,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Property {
    Name,
    Transform(Component, Axis),
}

pub(crate) fn parse_instance(body: &Node) -> Option<PrefabInstance> {
    let source = body.get("m_SourcePrefab")?;
    let guid = source.get("guid")?.as_str()?.to_string();
    let file_id = source
        .get("fileID")
        .and_then(Node::as_i64)
        .unwrap_or_default();
    let modification = body.get("m_Modification");
    let transform_parent = modification
        .and_then(|body| body.get("m_TransformParent"))
        .and_then(|node| node.get("fileID"))
        .and_then(Node::as_i64)
        .unwrap_or_default();
    Some(PrefabInstance {
        source: PrefabSource { guid, file_id },
        transform_parent,
        modifications: parse_modifications(modification),
    })
}

pub(crate) fn apply_modifications(
    modifications: &[Modification],
    game_objects: &HashMap<i64, GameObjectRecord>,
    transforms: &HashMap<i64, EntityId>,
    graph: &mut SceneGraph,
) {
    for modification in modifications {
        let property = match property(&modification.property_path) {
            Some(property) => property,
            None => continue,
        };
        match property {
            Property::Name => {
                let record = match game_objects.get(&modification.target_file_id) {
                    Some(record) => record,
                    None => continue,
                };
                if let Some(entity) = graph.entity_mut(record.entity) {
                    entity.name = modification.value.clone();
                }
            }
            Property::Transform(component, axis) => {
                let value = match number(&modification.value) {
                    Some(value) => value,
                    None => continue,
                };
                let entity = match transforms.get(&modification.target_file_id) {
                    Some(entity) => *entity,
                    None => continue,
                };
                let transform = match graph.transform_mut(entity) {
                    Some(transform) => transform,
                    None => continue,
                };
                match component {
                    Component::Translation => transform.translation[axis.index()] = value,
                    Component::Rotation => transform.rotation[axis.index()] = value,
                    Component::Scale => transform.scale[axis.index()] = value,
                }
            }
        }
    }
}

pub fn apply_modification_to_document(
    document: &mut Document,
    property_path: &str,
    value: &str,
) -> bool {
    if property_path.is_empty() || property_path.contains('[') {
        return false;
    }
    let parts: Vec<&str> = property_path.split('.').collect();
    let body = match body_mut(&mut document.root) {
        Some(body) => body,
        None => return false,
    };
    set_nested(body, &parts, value)
}

pub fn corresponding_source(document: &Document) -> Option<(String, i64)> {
    let body = document_body(document)?;
    let source = body.get("m_CorrespondingSourceObject")?;
    let file_id = source.get("fileID").and_then(Node::as_i64)?;
    let guid = source
        .get("guid")
        .and_then(Node::as_str)
        .unwrap_or_default()
        .to_string();
    Some((guid, file_id))
}

pub fn remove_components(stream: &mut Stream, file_ids: &[i64]) {
    if file_ids.is_empty() {
        return;
    }
    let owners: Vec<i64> = stream
        .documents
        .iter()
        .filter(|document| file_ids.contains(&document.file_id))
        .filter_map(|document| {
            document_body(document)
                .and_then(|body| body.get("m_GameObject"))
                .and_then(|node| node.get("fileID"))
                .and_then(Node::as_i64)
        })
        .collect();
    for owner in owners {
        remove_component_reference(stream, owner, file_ids);
    }
    stream
        .documents
        .retain(|document| !file_ids.contains(&document.file_id));
}

pub fn remove_game_object(stream: &mut Stream, game_object_file_id: i64) {
    let mut doomed: Vec<i64> = vec![game_object_file_id];
    let mut pending: Vec<i64> = vec![game_object_file_id];
    while let Some(current) = pending.pop() {
        for component_id in component_ids(stream, current) {
            doomed.push(component_id);
        }
        let transform_id = match transform_for_game_object(stream, current) {
            Some(transform_id) => transform_id,
            None => continue,
        };
        doomed.push(transform_id);
        for child_id in children_of_transform(stream, transform_id) {
            match game_object_for_transform(stream, child_id) {
                Some(child_game_object) => pending.push(child_game_object),
                None => doomed.push(child_id),
            }
        }
    }
    for file_id in &doomed {
        detach_from_parent(stream, *file_id);
    }
    stream
        .documents
        .retain(|document| !doomed.contains(&document.file_id));
}

pub fn copy_added_game_object(
    target: &mut Stream,
    source: &Stream,
    added_game_object_file_id: i64,
    parent_transform_file_id: i64,
    sibling_index: Option<usize>,
) {
    let mut copies = Vec::new();
    let mut transform_file_id = None;
    for component_id in component_ids(source, added_game_object_file_id) {
        let mut document = match source
            .documents
            .iter()
            .find(|entry| entry.file_id == component_id && !entry.stripped)
        {
            Some(document) => document.clone(),
            None => continue,
        };
        if document.class_id == TRANSFORM_CLASS {
            transform_file_id = Some(component_id);
            set_reference(&mut document, "m_Father", parent_transform_file_id);
        }
        clear_prefab_instance(&mut document);
        copies.push(document);
    }
    let mut game_object = match source.documents.iter().find(|entry| {
        entry.class_id == GAME_OBJECT_CLASS
            && entry.file_id == added_game_object_file_id
            && !entry.stripped
    }) {
        Some(document) => document.clone(),
        None => return,
    };
    clear_prefab_instance(&mut game_object);
    copies.insert(0, game_object);
    let transform_id = match transform_file_id {
        Some(transform_id) => transform_id,
        None => return,
    };
    for copy in copies {
        target.documents.push(copy);
    }
    insert_child(
        target,
        parent_transform_file_id,
        transform_id,
        sibling_index,
    );
}

pub fn copy_added_component(
    target: &mut Stream,
    source: &Stream,
    component_file_id: i64,
    owner_game_object_file_id: i64,
) {
    let mut document = match source
        .documents
        .iter()
        .find(|entry| entry.file_id == component_file_id && !entry.stripped)
    {
        Some(document) => document.clone(),
        None => return,
    };
    clear_prefab_instance(&mut document);
    set_reference(&mut document, "m_GameObject", owner_game_object_file_id);
    target.documents.push(document);
    for owner in &mut target.documents {
        if owner.file_id != owner_game_object_file_id {
            continue;
        }
        let body = match body_mut(&mut owner.root) {
            Some(body) => body,
            None => continue,
        };
        let items = match mapping_entry_mut(body, "m_Component") {
            Some(Node::Sequence(items)) => items,
            _ => continue,
        };
        items.push(Node::Mapping(vec![(
            String::from("component"),
            Node::Mapping(vec![(
                String::from("fileID"),
                Node::Scalar(component_file_id.to_string()),
            )]),
        )]));
    }
}

pub fn resolve_transform_file_id(stream: &Stream, file_id: i64) -> Option<i64> {
    if stream
        .documents
        .iter()
        .any(|document| document.class_id == TRANSFORM_CLASS && document.file_id == file_id)
    {
        return Some(file_id);
    }
    transform_for_game_object(stream, file_id)
}

pub fn resolve_game_object_file_id(stream: &Stream, file_id: i64) -> Option<i64> {
    if stream
        .documents
        .iter()
        .any(|document| document.class_id == GAME_OBJECT_CLASS && document.file_id == file_id)
    {
        return Some(file_id);
    }
    game_object_for_transform(stream, file_id)
}

fn document_body(document: &Document) -> Option<&Node> {
    match &document.root {
        Node::Mapping(entries) => entries.first().map(|(_, body)| body),
        _ => None,
    }
}

fn body_mut(root: &mut Node) -> Option<&mut Node> {
    match root {
        Node::Mapping(entries) => entries.first_mut().map(|(_, body)| body),
        _ => None,
    }
}

fn mapping_entry_mut<'a>(node: &'a mut Node, key: &str) -> Option<&'a mut Node> {
    match node {
        Node::Mapping(entries) => entries
            .iter_mut()
            .find(|(entry_key, _)| entry_key == key)
            .map(|(_, value)| value),
        _ => None,
    }
}

fn set_nested(node: &mut Node, path: &[&str], value: &str) -> bool {
    let (head, tail) = match path.split_first() {
        Some(parts) => parts,
        None => return false,
    };
    match node {
        Node::Mapping(entries) => {
            let index = match entries.iter().position(|(entry_key, _)| entry_key == head) {
                Some(index) => index,
                None => return false,
            };
            if tail.is_empty() {
                entries[index].1 = Node::Scalar(value.to_string());
                return true;
            }
            set_nested(&mut entries[index].1, tail, value)
        }
        _ => false,
    }
}

fn component_ids(stream: &Stream, game_object_file_id: i64) -> Vec<i64> {
    stream
        .documents
        .iter()
        .find(|document| {
            document.class_id == GAME_OBJECT_CLASS
                && document.file_id == game_object_file_id
                && !document.stripped
        })
        .and_then(|document| document_body(document))
        .and_then(|body| body.get("m_Component"))
        .and_then(Node::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.get("component")
                        .and_then(|node| node.get("fileID"))
                        .and_then(Node::as_i64)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn transform_for_game_object(stream: &Stream, game_object_file_id: i64) -> Option<i64> {
    stream
        .documents
        .iter()
        .find(|document| {
            document.class_id == TRANSFORM_CLASS
                && !document.stripped
                && document_body(document)
                    .and_then(|body| body.get("m_GameObject"))
                    .and_then(|node| node.get("fileID"))
                    .and_then(Node::as_i64)
                    == Some(game_object_file_id)
        })
        .map(|document| document.file_id)
}

fn game_object_for_transform(stream: &Stream, transform_file_id: i64) -> Option<i64> {
    stream
        .documents
        .iter()
        .find(|document| document.file_id == transform_file_id)
        .and_then(|document| document_body(document))
        .and_then(|body| body.get("m_GameObject"))
        .and_then(|node| node.get("fileID"))
        .and_then(Node::as_i64)
}

fn children_of_transform(stream: &Stream, transform_file_id: i64) -> Vec<i64> {
    stream
        .documents
        .iter()
        .find(|document| document.file_id == transform_file_id)
        .and_then(|document| document_body(document))
        .and_then(|body| body.get("m_Children"))
        .and_then(Node::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("fileID").and_then(Node::as_i64))
                .collect()
        })
        .unwrap_or_default()
}

fn remove_component_reference(stream: &mut Stream, owner_file_id: i64, file_ids: &[i64]) {
    for document in &mut stream.documents {
        if document.file_id != owner_file_id {
            continue;
        }
        let body = match body_mut(&mut document.root) {
            Some(body) => body,
            None => continue,
        };
        let items = match mapping_entry_mut(body, "m_Component") {
            Some(Node::Sequence(items)) => items,
            _ => continue,
        };
        items.retain(|item| {
            match item
                .get("component")
                .and_then(|node| node.get("fileID"))
                .and_then(Node::as_i64)
            {
                Some(file_id) => !file_ids.contains(&file_id),
                None => true,
            }
        });
    }
}

fn detach_from_parent(stream: &mut Stream, transform_file_id: i64) {
    let father = match stream
        .documents
        .iter()
        .find(|document| document.file_id == transform_file_id)
        .and_then(|document| document_body(document))
        .and_then(|body| body.get("m_Father"))
        .and_then(|node| node.get("fileID"))
        .and_then(Node::as_i64)
    {
        Some(father) if father != 0 => father,
        _ => return,
    };
    for document in &mut stream.documents {
        if document.file_id != father {
            continue;
        }
        let body = match body_mut(&mut document.root) {
            Some(body) => body,
            None => continue,
        };
        let items = match mapping_entry_mut(body, "m_Children") {
            Some(Node::Sequence(items)) => items,
            _ => continue,
        };
        items.retain(|item| match item.get("fileID").and_then(Node::as_i64) {
            Some(file_id) => file_id != transform_file_id,
            None => true,
        });
    }
}

fn clear_prefab_instance(document: &mut Document) {
    if let Some(body) = body_mut(&mut document.root) {
        if let Some(node) = mapping_entry_mut(body, "m_PrefabInstance") {
            *node = Node::Mapping(vec![(
                String::from("fileID"),
                Node::Scalar(String::from("0")),
            )]);
        }
    }
}

fn set_reference(document: &mut Document, key: &str, file_id: i64) {
    if let Some(body) = body_mut(&mut document.root) {
        if let Some(node) = mapping_entry_mut(body, key) {
            *node = Node::Mapping(vec![(
                String::from("fileID"),
                Node::Scalar(file_id.to_string()),
            )]);
        }
    }
}

fn insert_child(
    stream: &mut Stream,
    parent_transform_file_id: i64,
    child_transform_file_id: i64,
    sibling_index: Option<usize>,
) {
    for document in &mut stream.documents {
        if document.file_id != parent_transform_file_id {
            continue;
        }
        let body = match body_mut(&mut document.root) {
            Some(body) => body,
            None => continue,
        };
        let items = match mapping_entry_mut(body, "m_Children") {
            Some(Node::Sequence(items)) => items,
            _ => continue,
        };
        let reference = Node::Mapping(vec![(
            String::from("fileID"),
            Node::Scalar(child_transform_file_id.to_string()),
        )]);
        match sibling_index {
            Some(index) if index <= items.len() => items.insert(index, reference),
            _ => items.push(reference),
        }
    }
}

pub(crate) fn project_root(scene_path: &Path) -> Option<PathBuf> {
    let parent = scene_path.parent()?;
    parent
        .ancestors()
        .find(|ancestor| ancestor.join(PROJECT_SETTINGS_DIR).is_dir())
        .map(Path::to_path_buf)
}

pub(crate) fn guid_index(scene_path: &Path) -> Result<Option<GuidIndex>, MetaError> {
    let root = match project_root(scene_path) {
        Some(root) => root,
        None => return Ok(None),
    };
    let assets = root.join(ASSETS_DIR);
    let index_root = if assets.is_dir() { assets } else { root };
    Ok(Some(GuidIndex::build(&index_root)?))
}

fn parse_modifications(modification: Option<&Node>) -> Vec<Modification> {
    let items = match modification
        .and_then(|body| body.get("m_Modifications"))
        .and_then(Node::as_sequence)
    {
        Some(items) => items,
        None => return Vec::new(),
    };
    let mut modifications = Vec::new();
    for item in items {
        let target_file_id = match item
            .get("target")
            .and_then(|target| target.get("fileID"))
            .and_then(Node::as_i64)
        {
            Some(file_id) => file_id,
            None => continue,
        };
        let property_path = match item.get("propertyPath").and_then(Node::as_str) {
            Some(path) => path.to_string(),
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
        modifications.push(Modification {
            target_file_id,
            property_path,
            value,
            object_reference,
        });
    }
    modifications
}

pub fn render_prefab_instance_body(instance: &PrefabInstance) -> Node {
    let mod_items: Vec<Node> = instance
        .modifications
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
                        (
                            String::from("guid"),
                            Node::Scalar(instance.source.guid.clone()),
                        ),
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
        .collect();

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
                            Node::Scalar(instance.transform_parent.to_string()),
                        )]),
                    ),
                    (String::from("m_Modifications"), Node::Sequence(mod_items)),
                    (
                        String::from("m_RemovedComponents"),
                        Node::Sequence(Vec::new()),
                    ),
                    (
                        String::from("m_RemovedGameObjects"),
                        Node::Sequence(Vec::new()),
                    ),
                    (
                        String::from("m_AddedGameObjects"),
                        Node::Sequence(Vec::new()),
                    ),
                    (
                        String::from("m_AddedComponents"),
                        Node::Sequence(Vec::new()),
                    ),
                ]),
            ),
            (
                String::from("m_SourcePrefab"),
                Node::Mapping(vec![
                    (
                        String::from("fileID"),
                        Node::Scalar(instance.source.file_id.to_string()),
                    ),
                    (
                        String::from("guid"),
                        Node::Scalar(instance.source.guid.clone()),
                    ),
                    (String::from("type"), Node::Scalar(String::from("3"))),
                ]),
            ),
        ]),
    )])
}

fn property(path: &str) -> Option<Property> {
    if path == NAME_PROPERTY {
        return Some(Property::Name);
    }
    let (name, component) = path.split_once('.')?;
    let axis = match component {
        "x" => Axis::X,
        "y" => Axis::Y,
        "z" => Axis::Z,
        "w" => Axis::W,
        _ => return None,
    };
    match name {
        POSITION_PROPERTY if axis != Axis::W => {
            Some(Property::Transform(Component::Translation, axis))
        }
        ROTATION_PROPERTY => Some(Property::Transform(Component::Rotation, axis)),
        SCALE_PROPERTY if axis != Axis::W => Some(Property::Transform(Component::Scale, axis)),
        _ => None,
    }
}

fn number(value: &str) -> Option<f32> {
    let number = value.trim().parse::<f32>().ok()?;
    if number.is_finite() {
        Some(number)
    } else {
        None
    }
}
