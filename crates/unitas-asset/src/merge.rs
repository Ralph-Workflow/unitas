use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::binary_ser;
use crate::unity_yaml::{self, Document, Header, Node, Stream, YamlError};

const META_FILE_ID: i64 = 0;
const FLOW_REFERENCE_KEY: &str = "fileID";
const MAX_FLOW_ENTRIES: usize = 4;
const VECTOR_COMPONENTS: &str = "xyzwrgba";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeSide {
    Base,
    Ours,
    Theirs,
}

impl MergeSide {
    pub fn label(self) -> &'static str {
        match self {
            MergeSide::Base => "base",
            MergeSide::Ours => "ours",
            MergeSide::Theirs => "theirs",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuralKind {
    DeleteVersusChange {
        deleted_by: MergeSide,
    },
    ReparentCycle {
        cycle: Vec<i64>,
    },
    FileIdReuse {
        base_class: Option<u32>,
        ours_class: Option<u32>,
        theirs_class: Option<u32>,
    },
    ArrayReorder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictKind {
    Property,
    Structural(StructuralKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeConflict {
    pub kind: ConflictKind,
    pub object_path: String,
    pub file_id: i64,
    pub property: Option<String>,
    pub base: Option<String>,
    pub ours: Option<String>,
    pub theirs: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedReference {
    pub object_path: String,
    pub file_id: i64,
    pub property: String,
    pub referenced_file_id: i64,
    pub introduced: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeOutcome {
    pub merged: Option<String>,
    pub conflicts: Vec<MergeConflict>,
    pub references: Vec<UnresolvedReference>,
    pub objects: usize,
}

impl MergeOutcome {
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    pub fn merged_text(&self) -> Option<&str> {
        self.merged.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeError {
    NotMergeable {
        reason: String,
    },
    Yaml {
        side: MergeSide,
        error: YamlError,
    },
    Io {
        path: PathBuf,
        message: String,
    },
    UnresolvedConflicts {
        conflicts: Vec<MergeConflict>,
    },
    UnresolvedReferences {
        references: Vec<UnresolvedReference>,
    },
    InvalidMerge {
        message: String,
    },
}

impl fmt::Display for MergeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MergeError::NotMergeable { reason } => {
                write!(formatter, "asset is not semantically mergeable: {reason}")
            }
            MergeError::Yaml { side, error } => {
                write!(
                    formatter,
                    "{} side is not valid unity yaml: {error}",
                    side.label()
                )
            }
            MergeError::Io { path, message } => {
                write!(formatter, "cannot read {}: {}", path.display(), message)
            }
            MergeError::UnresolvedConflicts { conflicts } => {
                write!(
                    formatter,
                    "merge has {} unresolved conflicts",
                    conflicts.len()
                )
            }
            MergeError::UnresolvedReferences { references } => {
                write!(
                    formatter,
                    "merge has {} unresolved references",
                    references.len()
                )
            }
            MergeError::InvalidMerge { message } => {
                write!(formatter, "merge result is invalid: {message}")
            }
        }
    }
}

impl std::error::Error for MergeError {}

pub fn merge_bytes(base: &[u8], ours: &[u8], theirs: &[u8]) -> Result<MergeOutcome, MergeError> {
    let base_asset = parse_asset(base, MergeSide::Base)?;
    let ours_asset = parse_asset(ours, MergeSide::Ours)?;
    let theirs_asset = parse_asset(theirs, MergeSide::Theirs)?;
    merge_assets(&base_asset, &ours_asset, &theirs_asset)
}

pub fn merge_files(base: &Path, ours: &Path, theirs: &Path) -> Result<MergeOutcome, MergeError> {
    let base_bytes = read_file(base)?;
    let ours_bytes = read_file(ours)?;
    let theirs_bytes = read_file(theirs)?;
    merge_bytes(&base_bytes, &ours_bytes, &theirs_bytes)
}

pub fn merge_files_to_path(
    base: &Path,
    ours: &Path,
    theirs: &Path,
    output: &Path,
) -> Result<MergeOutcome, MergeError> {
    let outcome = merge_files(base, ours, theirs)?;
    if !outcome.conflicts.is_empty() {
        return Err(MergeError::UnresolvedConflicts {
            conflicts: outcome.conflicts,
        });
    }
    let introduced: Vec<UnresolvedReference> = outcome
        .references
        .iter()
        .filter(|reference| reference.introduced)
        .cloned()
        .collect();
    if !introduced.is_empty() {
        return Err(MergeError::UnresolvedReferences {
            references: introduced,
        });
    }
    let text = match outcome.merged.as_deref() {
        Some(text) => text,
        None => {
            return Err(MergeError::InvalidMerge {
                message: String::from("merge produced no result"),
            })
        }
    };
    fs::write(output, text).map_err(|error| MergeError::Io {
        path: output.to_path_buf(),
        message: error.to_string(),
    })?;
    Ok(outcome)
}

fn read_file(path: &Path) -> Result<Vec<u8>, MergeError> {
    fs::read(path).map_err(|error| MergeError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssetKind {
    Unity,
    Meta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Object {
    file_id: i64,
    class_id: Option<u32>,
    type_name: Option<String>,
    stripped: bool,
    properties: Node,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Asset {
    kind: AssetKind,
    header: Option<Header>,
    objects: Vec<Object>,
}

fn parse_asset(bytes: &[u8], side: MergeSide) -> Result<Asset, MergeError> {
    if binary_ser::is_text_serialized(bytes) {
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                return Err(MergeError::NotMergeable {
                    reason: String::from("asset is not valid utf-8 text"),
                })
            }
        };
        let stream = unity_yaml::parse(text).map_err(|error| MergeError::Yaml { side, error })?;
        return Ok(asset_from_stream(stream));
    }
    let text =
        match std::str::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => return Err(MergeError::NotMergeable {
                reason: String::from(
                    "asset uses a binary serialization form and carries no readable object data",
                ),
            }),
        };
    match parse_meta_properties(text, side) {
        Ok(properties) => Ok(Asset {
            kind: AssetKind::Meta,
            header: None,
            objects: vec![Object {
                file_id: META_FILE_ID,
                class_id: None,
                type_name: None,
                stripped: false,
                properties,
            }],
        }),
        Err(error) => {
            if binary_ser::read_serialized_file(bytes).is_ok() {
                Err(MergeError::NotMergeable {
                    reason: String::from(
                        "asset uses unity binary serialization and carries no readable object data",
                    ),
                })
            } else {
                Err(error)
            }
        }
    }
}

fn parse_meta_properties(text: &str, side: MergeSide) -> Result<Node, MergeError> {
    let mut wrapped = String::from("--- !u!1 &1\n");
    wrapped.push_str(text);
    let stream = unity_yaml::parse(&wrapped).map_err(|error| MergeError::Yaml { side, error })?;
    let document = match stream.documents.first() {
        Some(document) => document,
        None => {
            return Err(MergeError::NotMergeable {
                reason: String::from("asset carries no readable content"),
            })
        }
    };
    match &document.root {
        Node::Mapping(_) => Ok(document.root.clone()),
        _ => Err(MergeError::NotMergeable {
            reason: String::from("asset is not a unity yaml property document"),
        }),
    }
}

fn asset_from_stream(stream: Stream) -> Asset {
    let objects = stream
        .documents
        .iter()
        .map(|document| {
            let (type_name, properties) = split_document(document);
            Object {
                file_id: document.file_id,
                class_id: Some(document.class_id),
                type_name,
                stripped: document.stripped,
                properties,
            }
        })
        .collect();
    Asset {
        kind: AssetKind::Unity,
        header: stream.header,
        objects,
    }
}

fn split_document(document: &Document) -> (Option<String>, Node) {
    if let Node::Mapping(entries) = &document.root {
        if entries.len() == 1 {
            let (key, value) = &entries[0];
            if matches!(value, Node::Mapping(_)) {
                return (Some(key.clone()), value.clone());
            }
        }
        return (None, document.root.clone());
    }
    (None, Node::Mapping(Vec::new()))
}

fn merge_assets(base: &Asset, ours: &Asset, theirs: &Asset) -> Result<MergeOutcome, MergeError> {
    if base.kind != ours.kind || base.kind != theirs.kind {
        return Err(MergeError::NotMergeable {
            reason: String::from("asset sides use different serialization forms"),
        });
    }
    let base_objects = index_objects(&base.objects)?;
    let ours_objects = index_objects(&ours.objects)?;
    let theirs_objects = index_objects(&theirs.objects)?;
    let mut file_ids: BTreeSet<i64> = BTreeSet::new();
    for id in base_objects.keys() {
        file_ids.insert(*id);
    }
    for id in ours_objects.keys() {
        file_ids.insert(*id);
    }
    for id in theirs_objects.keys() {
        file_ids.insert(*id);
    }
    let mut merged: BTreeMap<i64, Object> = BTreeMap::new();
    let mut conflicts: Vec<MergeConflict> = Vec::new();
    for file_id in file_ids {
        let base_object = base_objects.get(&file_id);
        let ours_object = ours_objects.get(&file_id);
        let theirs_object = theirs_objects.get(&file_id);
        let (object, object_conflicts) =
            merge_object(file_id, base_object, ours_object, theirs_object);
        conflicts.extend(object_conflicts);
        if let Some(object) = object {
            merged.insert(file_id, object);
        }
    }
    let merged_objects = order_objects(&ours.objects, &theirs.objects, &merged);
    conflicts.extend(cycle_conflicts(
        &merged_objects,
        &base.objects,
        &ours.objects,
        &theirs.objects,
    ));
    let mut names: BTreeMap<i64, String> = BTreeMap::new();
    for objects in [&base.objects, &ours.objects, &theirs.objects] {
        for (id, name) in object_names(objects) {
            names.entry(id).or_insert(name);
        }
    }
    for (id, name) in object_names(&merged_objects) {
        names.insert(id, name);
    }
    let merged_paths = build_paths(&merged_objects, &names);
    let base_paths = build_paths(&base.objects, &names);
    for conflict in &mut conflicts {
        let path = merged_paths
            .get(&conflict.file_id)
            .or_else(|| base_paths.get(&conflict.file_id));
        conflict.object_path = match path {
            Some(path) => path.clone(),
            None => format!("fileID:{}", conflict.file_id),
        };
    }
    let references = unresolved_references(&merged_objects, &merged_paths);
    let base_references = unresolved_references(&base.objects, &base_paths);
    let mut introduced: BTreeSet<(i64, String, i64)> = BTreeSet::new();
    for reference in &base_references {
        introduced.insert((
            reference.file_id,
            reference.property.clone(),
            reference.referenced_file_id,
        ));
    }
    let mut references = references;
    for reference in &mut references {
        reference.introduced = !introduced.contains(&(
            reference.file_id,
            reference.property.clone(),
            reference.referenced_file_id,
        ));
    }
    let header = base
        .header
        .clone()
        .or_else(|| ours.header.clone())
        .or_else(|| theirs.header.clone());
    let merged_text = if conflicts.is_empty() {
        let text = serialize_asset(base.kind, &header, &merged_objects);
        validate_merged_text(base.kind, &text)?;
        Some(text)
    } else {
        None
    };
    Ok(MergeOutcome {
        merged: merged_text,
        conflicts,
        references,
        objects: merged_objects.len(),
    })
}

fn index_objects(objects: &[Object]) -> Result<BTreeMap<i64, Object>, MergeError> {
    let mut indexed: BTreeMap<i64, Object> = BTreeMap::new();
    for object in objects {
        if indexed.insert(object.file_id, object.clone()).is_some() {
            return Err(MergeError::NotMergeable {
                reason: format!("file id {} is used by more than one object", object.file_id),
            });
        }
    }
    Ok(indexed)
}

fn order_objects(
    ours: &[Object],
    theirs: &[Object],
    merged: &BTreeMap<i64, Object>,
) -> Vec<Object> {
    let mut ordered: Vec<Object> = Vec::new();
    let mut placed: BTreeSet<i64> = BTreeSet::new();
    for object in ours.iter().chain(theirs.iter()) {
        if !placed.contains(&object.file_id) {
            if let Some(merged_object) = merged.get(&object.file_id) {
                placed.insert(object.file_id);
                ordered.push(merged_object.clone());
            }
        }
    }
    for (file_id, object) in merged {
        if !placed.contains(file_id) {
            ordered.push(object.clone());
        }
    }
    ordered
}

fn objects_equal(left: Option<&Object>, right: Option<&Object>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.file_id == right.file_id
                && left.class_id == right.class_id
                && left.type_name == right.type_name
                && left.stripped == right.stripped
                && left.properties == right.properties
        }
        _ => false,
    }
}

fn merge_object(
    file_id: i64,
    base_object: Option<&Object>,
    ours_object: Option<&Object>,
    theirs_object: Option<&Object>,
) -> (Option<Object>, Vec<MergeConflict>) {
    if objects_equal(ours_object, theirs_object) {
        return (ours_object.cloned(), Vec::new());
    }
    if objects_equal(ours_object, base_object) {
        return (theirs_object.cloned(), Vec::new());
    }
    if objects_equal(theirs_object, base_object) {
        return (ours_object.cloned(), Vec::new());
    }
    match (ours_object, theirs_object) {
        (None, Some(theirs)) => {
            let conflict = MergeConflict {
                kind: ConflictKind::Structural(StructuralKind::DeleteVersusChange {
                    deleted_by: MergeSide::Ours,
                }),
                object_path: String::new(),
                file_id,
                property: None,
                base: base_object.map(|object| render_node(&object.properties)),
                ours: None,
                theirs: Some(render_node(&theirs.properties)),
                detail: Some(String::from(
                    "object deleted on one side and changed on the other",
                )),
            };
            (None, vec![conflict])
        }
        (Some(ours), None) => {
            let conflict = MergeConflict {
                kind: ConflictKind::Structural(StructuralKind::DeleteVersusChange {
                    deleted_by: MergeSide::Theirs,
                }),
                object_path: String::new(),
                file_id,
                property: None,
                base: base_object.map(|object| render_node(&object.properties)),
                ours: Some(render_node(&ours.properties)),
                theirs: None,
                detail: Some(String::from(
                    "object deleted on one side and changed on the other",
                )),
            };
            (None, vec![conflict])
        }
        (Some(ours), Some(theirs)) => {
            if ours.class_id != theirs.class_id || ours.type_name != theirs.type_name {
                let conflict = MergeConflict {
                    kind: ConflictKind::Structural(StructuralKind::FileIdReuse {
                        base_class: base_object.and_then(|object| object.class_id),
                        ours_class: ours.class_id,
                        theirs_class: theirs.class_id,
                    }),
                    object_path: String::new(),
                    file_id,
                    property: None,
                    base: base_object
                        .and_then(|object| object.class_id)
                        .map(|class_id| class_id.to_string()),
                    ours: ours.class_id.map(|class_id| class_id.to_string()),
                    theirs: theirs.class_id.map(|class_id| class_id.to_string()),
                    detail: Some(String::from("file id is used by different object types")),
                };
                return (Some(ours.clone()), vec![conflict]);
            }
            let base_properties = base_object.map(|object| &object.properties);
            let (properties, conflicts) = merge_values(
                base_properties,
                Some(&ours.properties),
                Some(&theirs.properties),
                "",
                file_id,
            );
            let merged = Object {
                file_id,
                class_id: ours.class_id,
                type_name: ours.type_name.clone(),
                stripped: ours.stripped,
                properties: properties.unwrap_or_else(|| Node::Mapping(Vec::new())),
            };
            (Some(merged), conflicts)
        }
        (None, None) => (None, Vec::new()),
    }
}

fn merge_values(
    base: Option<&Node>,
    ours: Option<&Node>,
    theirs: Option<&Node>,
    property: &str,
    file_id: i64,
) -> (Option<Node>, Vec<MergeConflict>) {
    if ours == theirs {
        return (ours.cloned(), Vec::new());
    }
    if ours == base {
        return (theirs.cloned(), Vec::new());
    }
    if theirs == base {
        return (ours.cloned(), Vec::new());
    }
    match (ours, theirs) {
        (Some(Node::Mapping(ours_entries)), Some(Node::Mapping(theirs_entries))) => {
            merge_mappings(base, ours_entries, theirs_entries, property, file_id)
        }
        (Some(Node::Sequence(_)), Some(Node::Sequence(_))) => {
            let conflict = MergeConflict {
                kind: ConflictKind::Structural(StructuralKind::ArrayReorder),
                object_path: String::new(),
                file_id,
                property: Some(String::from(property)),
                base: base.map(render_node),
                ours: ours.map(render_node),
                theirs: theirs.map(render_node),
                detail: Some(String::from(
                    "both sides reordered the same serialized array",
                )),
            };
            (None, vec![conflict])
        }
        _ => {
            let conflict = MergeConflict {
                kind: ConflictKind::Property,
                object_path: String::new(),
                file_id,
                property: Some(String::from(property)),
                base: base.map(render_node),
                ours: ours.map(render_node),
                theirs: theirs.map(render_node),
                detail: None,
            };
            (None, vec![conflict])
        }
    }
}

fn merge_mappings(
    base: Option<&Node>,
    ours: &[(String, Node)],
    theirs: &[(String, Node)],
    property: &str,
    file_id: i64,
) -> (Option<Node>, Vec<MergeConflict>) {
    let base_entries: &[(String, Node)] = match base {
        Some(Node::Mapping(entries)) => entries,
        _ => &[],
    };
    let mut keys: Vec<String> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for source in [base_entries, ours, theirs] {
        for (key, _) in source {
            if seen.insert(key.clone()) {
                keys.push(key.clone());
            }
        }
    }
    let mut entries: Vec<(String, Node)> = Vec::new();
    let mut conflicts: Vec<MergeConflict> = Vec::new();
    for key in keys {
        let base_value = lookup(base_entries, &key);
        let ours_value = lookup(ours, &key);
        let theirs_value = lookup(theirs, &key);
        let child_property = if property.is_empty() {
            key.clone()
        } else {
            format!("{property}.{key}")
        };
        let (merged, child_conflicts) = merge_values(
            base_value,
            ours_value,
            theirs_value,
            &child_property,
            file_id,
        );
        conflicts.extend(child_conflicts);
        if let Some(value) = merged {
            entries.push((key, value));
        }
    }
    (Some(Node::Mapping(entries)), conflicts)
}

fn lookup<'a>(entries: &'a [(String, Node)], key: &str) -> Option<&'a Node> {
    entries
        .iter()
        .find(|(entry_key, _)| entry_key == key)
        .map(|(_, value)| value)
}

fn render_node(node: &Node) -> String {
    match node {
        Node::Scalar(value) => value.clone(),
        Node::Mapping(entries) => {
            let mut rendered = String::from("{");
            for (index, (key, value)) in entries.iter().enumerate() {
                if index > 0 {
                    rendered.push_str(", ");
                }
                rendered.push_str(key);
                rendered.push_str(": ");
                rendered.push_str(&render_node(value));
            }
            rendered.push('}');
            rendered
        }
        Node::Sequence(items) => {
            let mut rendered = String::from("[");
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    rendered.push_str(", ");
                }
                rendered.push_str(&render_node(item));
            }
            rendered.push(']');
            rendered
        }
    }
}

fn is_game_object(object: &Object) -> bool {
    object.type_name.as_deref() == Some("GameObject") || object.class_id == Some(1)
}

fn is_transform(object: &Object) -> bool {
    object.type_name.as_deref() == Some("Transform") || object.class_id == Some(4)
}

fn reference_id(node: Option<&Node>) -> Option<i64> {
    let value = node?.get(FLOW_REFERENCE_KEY)?.as_scalar()?;
    value.trim().parse::<i64>().ok()
}

fn object_names(objects: &[Object]) -> BTreeMap<i64, String> {
    let mut names: BTreeMap<i64, String> = BTreeMap::new();
    for object in objects {
        if !is_game_object(object) {
            continue;
        }
        if let Some(name) = object.properties.get("m_Name").and_then(Node::as_scalar) {
            names.insert(object.file_id, name.to_string());
        }
    }
    names
}

#[derive(Debug, Clone, Copy)]
struct TransformInfo {
    game_object: Option<i64>,
    father: Option<i64>,
}

fn build_paths(objects: &[Object], names: &BTreeMap<i64, String>) -> BTreeMap<i64, String> {
    let mut transforms: BTreeMap<i64, TransformInfo> = BTreeMap::new();
    let mut transform_of_game_object: BTreeMap<i64, i64> = BTreeMap::new();
    let mut owner: BTreeMap<i64, i64> = BTreeMap::new();
    for object in objects {
        if is_game_object(object) {
            if let Some(Node::Sequence(items)) = object.properties.get("m_Component") {
                for item in items {
                    if let Some(component_id) = reference_id(item.get("component")) {
                        owner.insert(component_id, object.file_id);
                    }
                }
            }
        }
        if is_transform(object) {
            let info = TransformInfo {
                game_object: reference_id(object.properties.get("m_GameObject")),
                father: reference_id(object.properties.get("m_Father")),
            };
            if let Some(game_object) = info.game_object {
                transform_of_game_object.insert(game_object, object.file_id);
            }
            transforms.insert(object.file_id, info);
        }
    }
    let mut paths: BTreeMap<i64, String> = BTreeMap::new();
    let mut visiting: BTreeSet<i64> = BTreeSet::new();
    for (game_object, name) in names {
        let path = match transform_of_game_object.get(game_object) {
            Some(transform) => transform_path(*transform, &transforms, names, &mut visiting),
            None => name.clone(),
        };
        paths.insert(*game_object, path);
    }
    for transform in transforms.keys() {
        let path = transform_path(*transform, &transforms, names, &mut visiting);
        paths.insert(*transform, format!("{path}/Transform"));
    }
    for object in objects {
        if is_game_object(object) || is_transform(object) {
            continue;
        }
        let type_name = match &object.type_name {
            Some(type_name) => type_name.clone(),
            None => continue,
        };
        let path = match owner
            .get(&object.file_id)
            .and_then(|game_object| paths.get(game_object))
        {
            Some(game_object_path) => format!("{game_object_path}/{type_name}"),
            None => format!("{type_name}{}", object.file_id),
        };
        paths.insert(object.file_id, path);
    }
    paths
}

fn transform_path(
    transform: i64,
    transforms: &BTreeMap<i64, TransformInfo>,
    names: &BTreeMap<i64, String>,
    visiting: &mut BTreeSet<i64>,
) -> String {
    let info = match transforms.get(&transform) {
        Some(info) => info,
        None => return format!("Transform{transform}"),
    };
    let name = match info
        .game_object
        .and_then(|game_object| names.get(&game_object))
    {
        Some(name) => name.clone(),
        None => format!("Transform{transform}"),
    };
    if !visiting.insert(transform) {
        return name;
    }
    let prefix = match info.father {
        Some(father) if father != 0 && transforms.contains_key(&father) => {
            transform_path(father, transforms, names, visiting)
        }
        _ => String::new(),
    };
    visiting.remove(&transform);
    if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    }
}

fn father_map(objects: &[Object]) -> BTreeMap<i64, i64> {
    let mut fathers: BTreeMap<i64, i64> = BTreeMap::new();
    for object in objects {
        if !is_transform(object) {
            continue;
        }
        if let Some(father) = reference_id(object.properties.get("m_Father")) {
            fathers.insert(object.file_id, father);
        }
    }
    fathers
}

fn find_cycles(fathers: &BTreeMap<i64, i64>) -> Vec<Vec<i64>> {
    let mut cycles: Vec<Vec<i64>> = Vec::new();
    for start in fathers.keys() {
        let mut path: Vec<i64> = Vec::new();
        let mut current = *start;
        let mut steps = 0usize;
        while steps <= fathers.len() {
            steps += 1;
            if let Some(index) = path.iter().position(|id| *id == current) {
                let cycle: Vec<i64> = path[index..].to_vec();
                if !cycles.iter().any(|existing| same_members(existing, &cycle)) {
                    cycles.push(cycle);
                }
                break;
            }
            path.push(current);
            match fathers.get(&current) {
                Some(parent) if *parent != 0 && fathers.contains_key(parent) => current = *parent,
                _ => break,
            }
        }
    }
    cycles
}

fn same_members(left: &[i64], right: &[i64]) -> bool {
    let mut left: Vec<i64> = left.to_vec();
    let mut right: Vec<i64> = right.to_vec();
    left.sort_unstable();
    right.sort_unstable();
    left == right
}

fn cycle_conflicts(
    merged: &[Object],
    base: &[Object],
    ours: &[Object],
    theirs: &[Object],
) -> Vec<MergeConflict> {
    let merged_fathers = father_map(merged);
    let base_fathers = father_map(base);
    let ours_fathers = father_map(ours);
    let theirs_fathers = father_map(theirs);
    let base_cycles: Vec<Vec<i64>> = find_cycles(&base_fathers)
        .iter()
        .map(|cycle| {
            let mut members = cycle.clone();
            members.sort_unstable();
            members
        })
        .collect();
    let mut conflicts: Vec<MergeConflict> = Vec::new();
    for cycle in find_cycles(&merged_fathers) {
        let mut members = cycle.clone();
        members.sort_unstable();
        if base_cycles.contains(&members) {
            continue;
        }
        let first = match cycle.first() {
            Some(first) => *first,
            None => continue,
        };
        conflicts.push(MergeConflict {
            kind: ConflictKind::Structural(StructuralKind::ReparentCycle {
                cycle: cycle.clone(),
            }),
            object_path: String::new(),
            file_id: first,
            property: Some(String::from("m_Father")),
            base: base_fathers.get(&first).map(|father| father.to_string()),
            ours: ours_fathers.get(&first).map(|father| father.to_string()),
            theirs: theirs_fathers.get(&first).map(|father| father.to_string()),
            detail: Some(format!(
                "merged hierarchy would repeat {}",
                cycle
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<String>>()
                    .join(" -> ")
            )),
        });
    }
    conflicts
}

fn unresolved_references(
    objects: &[Object],
    paths: &BTreeMap<i64, String>,
) -> Vec<UnresolvedReference> {
    let mut known: BTreeSet<i64> = BTreeSet::new();
    for object in objects {
        known.insert(object.file_id);
    }
    let mut unresolved: Vec<UnresolvedReference> = Vec::new();
    for object in objects {
        let mut references: Vec<(String, i64)> = Vec::new();
        collect_references(&object.properties, "", &mut references);
        for (property, referenced) in references {
            if referenced == 0 || known.contains(&referenced) {
                continue;
            }
            let object_path = match paths.get(&object.file_id) {
                Some(path) => path.clone(),
                None => format!("fileID:{}", object.file_id),
            };
            unresolved.push(UnresolvedReference {
                object_path,
                file_id: object.file_id,
                property,
                referenced_file_id: referenced,
                introduced: false,
            });
        }
    }
    unresolved
}

fn collect_references(node: &Node, property: &str, references: &mut Vec<(String, i64)>) {
    match node {
        Node::Mapping(entries) => {
            let file_id = entries
                .iter()
                .find(|(key, _)| key == FLOW_REFERENCE_KEY)
                .and_then(|(_, value)| value.as_scalar())
                .and_then(|value| value.trim().parse::<i64>().ok());
            if let Some(file_id) = file_id {
                let external = entries.iter().any(|(key, _)| key == "guid");
                if !external {
                    references.push((property.to_string(), file_id));
                }
                return;
            }
            for (key, value) in entries {
                let child = if property.is_empty() {
                    key.clone()
                } else {
                    format!("{property}.{key}")
                };
                collect_references(value, &child, references);
            }
        }
        Node::Sequence(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_references(item, &format!("{property}[{index}]"), references);
            }
        }
        Node::Scalar(_) => {}
    }
}

fn validate_merged_text(kind: AssetKind, text: &str) -> Result<(), MergeError> {
    match kind {
        AssetKind::Unity => {
            unity_yaml::parse(text).map_err(|error| MergeError::InvalidMerge {
                message: format!("merged result does not parse as unity yaml: {error}"),
            })?;
        }
        AssetKind::Meta => {
            parse_meta_properties(text, MergeSide::Base).map_err(|error| {
                MergeError::InvalidMerge {
                    message: format!(
                        "merged result does not parse as a property document: {error}"
                    ),
                }
            })?;
        }
    }
    Ok(())
}

fn serialize_asset(kind: AssetKind, header: &Option<Header>, objects: &[Object]) -> String {
    let mut out = String::new();
    if let Some(header) = header {
        out.push_str(&format!("%YAML {}\n", header.yaml_version));
        if let (Some(handle), Some(prefix)) = (&header.tag_handle, &header.tag_prefix) {
            out.push_str(&format!("%TAG {handle} {prefix}\n"));
        }
    }
    for object in objects {
        if kind == AssetKind::Unity {
            let class_id = match object.class_id {
                Some(class_id) => class_id,
                None => continue,
            };
            out.push_str(&format!("--- !u!{class_id} &{}", object.file_id));
            if object.stripped {
                out.push_str(" stripped");
            }
            out.push('\n');
            match &object.type_name {
                Some(type_name) => {
                    out.push_str(type_name);
                    out.push_str(":\n");
                    if let Node::Mapping(entries) = &object.properties {
                        write_mapping_entries(&mut out, entries, 2);
                    }
                }
                None => {
                    if let Node::Mapping(entries) = &object.properties {
                        write_mapping_entries(&mut out, entries, 0);
                    }
                }
            }
        } else if let Node::Mapping(entries) = &object.properties {
            write_mapping_entries(&mut out, entries, 0);
        }
    }
    out
}

fn write_mapping_entries(out: &mut String, entries: &[(String, Node)], indent: usize) {
    for (key, value) in entries {
        out.push_str(&" ".repeat(indent));
        write_key_value(out, key, value, indent);
    }
}

fn write_key_value(out: &mut String, key: &str, value: &Node, indent: usize) {
    write_scalar(out, key);
    out.push(':');
    match value {
        Node::Mapping(entries) => {
            if entries.is_empty() {
                out.push_str(" {}\n");
            } else if is_flow_mapping(entries) {
                out.push(' ');
                write_flow_mapping(out, entries);
                out.push('\n');
            } else {
                out.push('\n');
                write_mapping_entries(out, entries, indent + 2);
            }
        }
        Node::Sequence(items) => {
            if items.is_empty() {
                out.push_str(" []\n");
            } else {
                out.push('\n');
                write_sequence(out, items, indent);
            }
        }
        Node::Scalar(scalar) => {
            if scalar.is_empty() {
                out.push('\n');
            } else {
                out.push(' ');
                write_scalar(out, scalar);
                out.push('\n');
            }
        }
    }
}

fn write_sequence(out: &mut String, items: &[Node], indent: usize) {
    for item in items {
        match item {
            Node::Mapping(entries) => {
                if entries.is_empty() {
                    out.push_str(&format!("{}- {{}}\n", " ".repeat(indent)));
                    continue;
                }
                if is_flow_mapping(entries) {
                    out.push_str(&format!("{}- ", " ".repeat(indent)));
                    write_flow_mapping(out, entries);
                    out.push('\n');
                    continue;
                }
                let mut remaining = entries.iter();
                if let Some((key, value)) = remaining.next() {
                    out.push_str(&format!("{}- ", " ".repeat(indent)));
                    write_key_value(out, key, value, indent + 2);
                }
                for (key, value) in remaining {
                    out.push_str(&" ".repeat(indent + 2));
                    write_key_value(out, key, value, indent + 2);
                }
            }
            Node::Sequence(inner) => {
                if inner.is_empty() {
                    out.push_str(&format!("{}- []\n", " ".repeat(indent)));
                } else {
                    out.push_str(&format!("{}-\n", " ".repeat(indent)));
                    write_sequence(out, inner, indent + 2);
                }
            }
            Node::Scalar(scalar) => {
                out.push_str(&format!("{}- ", " ".repeat(indent)));
                write_scalar(out, scalar);
                out.push('\n');
            }
        }
    }
}

fn is_flow_mapping(entries: &[(String, Node)]) -> bool {
    if entries.is_empty() || entries.len() > MAX_FLOW_ENTRIES {
        return false;
    }
    if !entries
        .iter()
        .all(|(_, value)| matches!(value, Node::Scalar(_)))
    {
        return false;
    }
    if entries.iter().any(|(key, _)| key == FLOW_REFERENCE_KEY) {
        return true;
    }
    entries.iter().all(|(key, _)| {
        key.len() == 1
            && key
                .chars()
                .all(|character| VECTOR_COMPONENTS.contains(character))
    })
}

fn write_flow_mapping(out: &mut String, entries: &[(String, Node)]) {
    out.push('{');
    for (index, (key, value)) in entries.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write_scalar(out, key);
        out.push_str(": ");
        if let Node::Scalar(scalar) = value {
            write_scalar(out, scalar);
        }
    }
    out.push('}');
}

fn write_scalar(out: &mut String, text: &str) {
    if !needs_quotes(text) {
        out.push_str(text);
        return;
    }
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
}

fn needs_quotes(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    if text.trim() != text {
        return true;
    }
    if text.starts_with("- ")
        || text == "-"
        || text.starts_with("? ")
        || text == "?"
        || text.starts_with("! ")
        || text == "!"
        || text.starts_with('&')
        || text.starts_with('*')
        || text.starts_with('%')
        || text.starts_with('@')
        || text.starts_with('`')
    {
        return true;
    }
    text.chars().any(|character| {
        matches!(
            character,
            ':' | '#'
                | '{'
                | '}'
                | '['
                | ']'
                | ','
                | '"'
                | '\''
                | '\\'
                | '\n'
                | '\r'
                | '\t'
                | '|'
                | '>'
        )
    })
}

const PREFAB_INSTANCE_CLASS_ID: u32 = 1001;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffChangeKind {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDiff {
    pub object_path: String,
    pub file_id: i64,
    pub property_path: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectDiff {
    pub change: DiffChangeKind,
    pub file_id: i64,
    pub class_id: Option<u32>,
    pub type_name: Option<String>,
    pub object_path: String,
    pub properties: Vec<PropertyDiff>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentDiff {
    pub change: DiffChangeKind,
    pub file_id: i64,
    pub class_id: Option<u32>,
    pub type_name: Option<String>,
    pub owner_file_id: Option<i64>,
    pub object_path: String,
    pub properties: Vec<PropertyDiff>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverrideSection {
    Modifications,
    RemovedComponents,
    AddedComponents,
    AddedGameObjects,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefabOverrideDiff {
    pub change: DiffChangeKind,
    pub section: OverrideSection,
    pub prefab_instance_file_id: i64,
    pub object_path: String,
    pub target_file_id: Option<i64>,
    pub target_guid: Option<String>,
    pub property_path: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub old_object_reference: Option<String>,
    pub new_object_reference: Option<String>,
    pub added_object_file_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SemanticDiff {
    pub objects: Vec<ObjectDiff>,
    pub components: Vec<ComponentDiff>,
    pub properties: Vec<PropertyDiff>,
    pub prefab_overrides: Vec<PrefabOverrideDiff>,
}

impl SemanticDiff {
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
            && self.components.is_empty()
            && self.properties.is_empty()
            && self.prefab_overrides.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffOutcome {
    Diff(SemanticDiff),
    NotSemanticallyDiffable { reason: String },
}

impl DiffOutcome {
    pub fn is_diffable(&self) -> bool {
        matches!(self, DiffOutcome::Diff(_))
    }

    pub fn as_diff(&self) -> Option<&SemanticDiff> {
        match self {
            DiffOutcome::Diff(diff) => Some(diff),
            DiffOutcome::NotSemanticallyDiffable { .. } => None,
        }
    }
}

pub fn semantic_diff_bytes(old: &[u8], new: &[u8]) -> Result<DiffOutcome, MergeError> {
    let old_asset = match parse_asset(old, MergeSide::Ours) {
        Ok(asset) => asset,
        Err(MergeError::NotMergeable { reason }) => {
            return Ok(DiffOutcome::NotSemanticallyDiffable { reason })
        }
        Err(error) => return Err(error),
    };
    let new_asset = match parse_asset(new, MergeSide::Theirs) {
        Ok(asset) => asset,
        Err(MergeError::NotMergeable { reason }) => {
            return Ok(DiffOutcome::NotSemanticallyDiffable { reason })
        }
        Err(error) => return Err(error),
    };
    if old_asset.kind != new_asset.kind {
        return Ok(DiffOutcome::NotSemanticallyDiffable {
            reason: String::from("asset revisions use different serialization forms"),
        });
    }
    let diff = diff_parsed_assets(&old_asset, &new_asset)?;
    Ok(DiffOutcome::Diff(diff))
}

pub fn semantic_diff(old: &str, new: &str) -> Result<DiffOutcome, MergeError> {
    semantic_diff_bytes(old.as_bytes(), new.as_bytes())
}

pub fn semantic_diff_files(old: &Path, new: &Path) -> Result<DiffOutcome, MergeError> {
    let old_bytes = read_file(old)?;
    let new_bytes = read_file(new)?;
    semantic_diff_bytes(&old_bytes, &new_bytes)
}

pub fn diff_streams(old: &Stream, new: &Stream) -> Result<SemanticDiff, MergeError> {
    let old_asset = asset_from_stream(old.clone());
    let new_asset = asset_from_stream(new.clone());
    diff_parsed_assets(&old_asset, &new_asset)
}

pub fn diff_assets(old: &[u8], new: &[u8]) -> Result<DiffOutcome, MergeError> {
    semantic_diff_bytes(old, new)
}

fn is_prefab_instance(object: &Object) -> bool {
    object.type_name.as_deref() == Some("PrefabInstance")
        || object.class_id == Some(PREFAB_INSTANCE_CLASS_ID)
}

fn build_owner_map(objects: &[Object]) -> BTreeMap<i64, i64> {
    let mut owner: BTreeMap<i64, i64> = BTreeMap::new();
    for object in objects {
        if is_game_object(object) {
            if let Some(Node::Sequence(items)) = object.properties.get("m_Component") {
                for item in items {
                    if let Some(component_id) = reference_id(item.get("component")) {
                        owner.insert(component_id, object.file_id);
                    }
                }
            }
        }
        if is_transform(object) {
            if let Some(game_object) = reference_id(object.properties.get("m_GameObject")) {
                if game_object != 0 {
                    owner.insert(object.file_id, game_object);
                }
            }
        }
        if let Some(game_object) = reference_id(object.properties.get("m_GameObject")) {
            if game_object != 0 && !is_game_object(object) {
                owner.entry(object.file_id).or_insert(game_object);
            }
        }
    }
    owner
}

fn is_component_object(object: &Object, owners: &BTreeMap<i64, i64>) -> bool {
    if is_game_object(object) || is_prefab_instance(object) {
        return false;
    }
    if is_transform(object) || owners.contains_key(&object.file_id) {
        return true;
    }
    if let Some(game_object) = reference_id(object.properties.get("m_GameObject")) {
        if game_object != 0 {
            return true;
        }
    }
    false
}

fn resolve_object_path(
    file_id: i64,
    object: &Object,
    primary_paths: &BTreeMap<i64, String>,
    fallback_paths: &BTreeMap<i64, String>,
) -> String {
    if let Some(path) = primary_paths
        .get(&file_id)
        .or_else(|| fallback_paths.get(&file_id))
    {
        return path.clone();
    }
    if is_prefab_instance(object) {
        if let Some(modifications) = object
            .properties
            .get("m_Modification")
            .and_then(|node| node.get("m_Modifications"))
            .and_then(Node::as_sequence)
        {
            for item in modifications {
                if item.get("propertyPath").and_then(Node::as_scalar) == Some("m_Name") {
                    if let Some(val) = item.get("value").and_then(Node::as_scalar) {
                        if !val.is_empty() {
                            return format!("PrefabInstance({val})");
                        }
                    }
                }
            }
        }
        return format!("PrefabInstance{file_id}");
    }
    if let Some(type_name) = &object.type_name {
        return format!("{type_name}{file_id}");
    }
    if file_id == META_FILE_ID {
        return String::from("Meta");
    }
    format!("fileID:{file_id}")
}

fn diff_parsed_assets(old: &Asset, new: &Asset) -> Result<SemanticDiff, MergeError> {
    let old_objects = index_objects(&old.objects)?;
    let new_objects = index_objects(&new.objects)?;

    let old_names = object_names(&old.objects);
    let new_names = object_names(&new.objects);
    let old_paths = build_paths(&old.objects, &old_names);
    let new_paths = build_paths(&new.objects, &new_names);

    let old_owners = build_owner_map(&old.objects);
    let new_owners = build_owner_map(&new.objects);

    let mut file_ids: BTreeSet<i64> = BTreeSet::new();
    for id in old_objects.keys().chain(new_objects.keys()) {
        file_ids.insert(*id);
    }

    let mut objects = Vec::new();
    let mut components = Vec::new();
    let mut properties = Vec::new();
    let mut prefab_overrides = Vec::new();

    for file_id in file_ids {
        let old_obj = old_objects.get(&file_id);
        let new_obj = new_objects.get(&file_id);
        match (old_obj, new_obj) {
            (None, Some(added)) => {
                let object_path = resolve_object_path(file_id, added, &new_paths, &old_paths);
                if is_component_object(added, &new_owners) {
                    components.push(ComponentDiff {
                        change: DiffChangeKind::Added,
                        file_id,
                        class_id: added.class_id,
                        type_name: added.type_name.clone(),
                        owner_file_id: new_owners.get(&file_id).copied(),
                        object_path,
                        properties: Vec::new(),
                    });
                } else {
                    if is_prefab_instance(added) {
                        prefab_overrides.extend(diff_prefab_instance_overrides(
                            file_id,
                            &object_path,
                            None,
                            Some(&added.properties),
                        ));
                    }
                    objects.push(ObjectDiff {
                        change: DiffChangeKind::Added,
                        file_id,
                        class_id: added.class_id,
                        type_name: added.type_name.clone(),
                        object_path,
                        properties: Vec::new(),
                    });
                }
            }
            (Some(removed), None) => {
                let object_path = resolve_object_path(file_id, removed, &old_paths, &new_paths);
                if is_component_object(removed, &old_owners) {
                    components.push(ComponentDiff {
                        change: DiffChangeKind::Removed,
                        file_id,
                        class_id: removed.class_id,
                        type_name: removed.type_name.clone(),
                        owner_file_id: old_owners.get(&file_id).copied(),
                        object_path,
                        properties: Vec::new(),
                    });
                } else {
                    if is_prefab_instance(removed) {
                        prefab_overrides.extend(diff_prefab_instance_overrides(
                            file_id,
                            &object_path,
                            Some(&removed.properties),
                            None,
                        ));
                    }
                    objects.push(ObjectDiff {
                        change: DiffChangeKind::Removed,
                        file_id,
                        class_id: removed.class_id,
                        type_name: removed.type_name.clone(),
                        object_path,
                        properties: Vec::new(),
                    });
                }
            }
            (Some(old_item), Some(new_item)) => {
                let object_path = resolve_object_path(file_id, new_item, &new_paths, &old_paths);
                let is_comp = is_component_object(new_item, &new_owners)
                    || is_component_object(old_item, &old_owners);
                let is_go = is_game_object(new_item) || is_game_object(old_item);
                let is_prefab = is_prefab_instance(new_item) || is_prefab_instance(old_item);

                if is_prefab {
                    prefab_overrides.extend(diff_prefab_instance_overrides(
                        file_id,
                        &object_path,
                        Some(&old_item.properties),
                        Some(&new_item.properties),
                    ));
                }

                let mut prop_diffs = Vec::new();
                diff_object_properties(
                    file_id,
                    &object_path,
                    old_item,
                    new_item,
                    is_go,
                    is_prefab,
                    &mut prop_diffs,
                );

                if !prop_diffs.is_empty() {
                    properties.extend(prop_diffs.clone());
                    if is_comp {
                        components.push(ComponentDiff {
                            change: DiffChangeKind::Changed,
                            file_id,
                            class_id: new_item.class_id,
                            type_name: new_item.type_name.clone(),
                            owner_file_id: new_owners
                                .get(&file_id)
                                .or_else(|| old_owners.get(&file_id))
                                .copied(),
                            object_path,
                            properties: prop_diffs,
                        });
                    } else {
                        objects.push(ObjectDiff {
                            change: DiffChangeKind::Changed,
                            file_id,
                            class_id: new_item.class_id,
                            type_name: new_item.type_name.clone(),
                            object_path,
                            properties: prop_diffs,
                        });
                    }
                }
            }
            (None, None) => {}
        }
    }

    Ok(SemanticDiff {
        objects,
        components,
        properties,
        prefab_overrides,
    })
}

fn diff_object_properties(
    file_id: i64,
    object_path: &str,
    old_item: &Object,
    new_item: &Object,
    is_go: bool,
    is_prefab: bool,
    out: &mut Vec<PropertyDiff>,
) {
    if old_item.class_id != new_item.class_id {
        out.push(PropertyDiff {
            object_path: object_path.to_string(),
            file_id,
            property_path: String::from("class_id"),
            old_value: old_item.class_id.map(|id| id.to_string()),
            new_value: new_item.class_id.map(|id| id.to_string()),
        });
    }
    if old_item.type_name != new_item.type_name {
        out.push(PropertyDiff {
            object_path: object_path.to_string(),
            file_id,
            property_path: String::from("type_name"),
            old_value: old_item.type_name.clone(),
            new_value: new_item.type_name.clone(),
        });
    }
    let ctx = DiffNodeContext {
        file_id,
        object_path,
        is_go,
        is_prefab,
    };
    diff_nodes(
        &ctx,
        "",
        Some(&old_item.properties),
        Some(&new_item.properties),
        out,
    );
}

fn is_prefab_override_property(is_prefab: bool, path: &str) -> bool {
    is_prefab
        && matches!(
            path,
            "m_Modification.m_Modifications"
                | "m_Modification.m_RemovedComponents"
                | "m_Modification.m_AddedComponents"
                | "m_Modification.m_AddedGameObjects"
        )
}

fn is_reference_mapping(entries: &[(String, Node)]) -> bool {
    !entries.is_empty()
        && entries.len() <= MAX_FLOW_ENTRIES
        && entries
            .iter()
            .all(|(_, value)| matches!(value, Node::Scalar(_)))
        && entries.iter().any(|(key, _)| key == FLOW_REFERENCE_KEY)
}

fn canonicalize_node(node: &Node) -> Node {
    match node {
        Node::Scalar(val) => Node::Scalar(val.clone()),
        Node::Mapping(entries) => {
            let mut sorted: Vec<(String, Node)> = entries
                .iter()
                .map(|(k, v)| (k.clone(), canonicalize_node(v)))
                .collect();
            sorted.sort_by(|(a, _), (b, _)| a.cmp(b));
            Node::Mapping(sorted)
        }
        Node::Sequence(items) => Node::Sequence(items.iter().map(canonicalize_node).collect()),
    }
}

fn nodes_semantically_equal(left: Option<&Node>, right: Option<&Node>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(l), Some(r)) => canonicalize_node(l) == canonicalize_node(r),
        _ => false,
    }
}

struct DiffNodeContext<'a> {
    file_id: i64,
    object_path: &'a str,
    is_go: bool,
    is_prefab: bool,
}

fn diff_nodes(
    ctx: &DiffNodeContext<'_>,
    property_path: &str,
    old: Option<&Node>,
    new: Option<&Node>,
    out: &mut Vec<PropertyDiff>,
) {
    if ctx.is_go && property_path == "m_Component" {
        return;
    }
    if is_prefab_override_property(ctx.is_prefab, property_path) {
        return;
    }
    if nodes_semantically_equal(old, new) {
        return;
    }
    match (old, new) {
        (Some(Node::Mapping(old_entries)), Some(Node::Mapping(new_entries))) => {
            if is_reference_mapping(old_entries) || is_reference_mapping(new_entries) {
                out.push(PropertyDiff {
                    object_path: ctx.object_path.to_string(),
                    file_id: ctx.file_id,
                    property_path: property_path.to_string(),
                    old_value: old.map(render_node),
                    new_value: new.map(render_node),
                });
                return;
            }
            let mut keys: Vec<String> = Vec::new();
            let mut seen: BTreeSet<String> = BTreeSet::new();
            for source in [old_entries.as_slice(), new_entries.as_slice()] {
                for (key, _) in source {
                    if seen.insert(key.clone()) {
                        keys.push(key.clone());
                    }
                }
            }
            for key in keys {
                let old_child = lookup(old_entries, &key);
                let new_child = lookup(new_entries, &key);
                let child_path = if property_path.is_empty() {
                    key
                } else {
                    format!("{property_path}.{key}")
                };
                diff_nodes(ctx, &child_path, old_child, new_child, out);
            }
        }
        _ => {
            out.push(PropertyDiff {
                object_path: ctx.object_path.to_string(),
                file_id: ctx.file_id,
                property_path: property_path.to_string(),
                old_value: old.map(render_node),
                new_value: new.map(render_node),
            });
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ModificationKey {
    target_file_id: Option<i64>,
    target_guid: Option<String>,
    property_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModificationEntry {
    value: Option<String>,
    object_reference: Option<String>,
}

fn parse_target_ref(node: Option<&Node>) -> (Option<i64>, Option<String>) {
    let target = match node {
        Some(target) => target,
        None => return (None, None),
    };
    let file_id = reference_id(Some(target));
    let guid = target
        .get("guid")
        .and_then(Node::as_scalar)
        .map(String::from);
    (file_id, guid)
}

fn extract_modification_map(
    properties: Option<&Node>,
) -> BTreeMap<ModificationKey, ModificationEntry> {
    let mut map = BTreeMap::new();
    let items = match properties
        .and_then(|p| p.get("m_Modification"))
        .and_then(|m| m.get("m_Modifications"))
        .and_then(Node::as_sequence)
    {
        Some(items) => items,
        None => return map,
    };
    for item in items {
        let (target_file_id, target_guid) = parse_target_ref(item.get("target"));
        let property_path = item
            .get("propertyPath")
            .and_then(Node::as_scalar)
            .unwrap_or_default()
            .to_string();
        let value = item.get("value").map(render_node);
        let object_reference = item.get("objectReference").map(render_node);
        map.insert(
            ModificationKey {
                target_file_id,
                target_guid,
                property_path,
            },
            ModificationEntry {
                value,
                object_reference,
            },
        );
    }
    map
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TargetRefKey {
    target_file_id: Option<i64>,
    target_guid: Option<String>,
}

fn extract_removed_components(properties: Option<&Node>) -> BTreeSet<TargetRefKey> {
    let mut set = BTreeSet::new();
    let items = match properties
        .and_then(|p| p.get("m_Modification"))
        .and_then(|m| m.get("m_RemovedComponents"))
        .and_then(Node::as_sequence)
    {
        Some(items) => items,
        None => return set,
    };
    for item in items {
        let target_node = item.get("target").unwrap_or(item);
        let (target_file_id, target_guid) = parse_target_ref(Some(target_node));
        set.insert(TargetRefKey {
            target_file_id,
            target_guid,
        });
    }
    set
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct AddedObjectKey {
    target_file_id: Option<i64>,
    target_guid: Option<String>,
    added_object_file_id: Option<i64>,
}

fn extract_added_objects(
    properties: Option<&Node>,
    field: &str,
) -> BTreeMap<AddedObjectKey, Option<String>> {
    let mut map = BTreeMap::new();
    let items = match properties
        .and_then(|p| p.get("m_Modification"))
        .and_then(|m| m.get(field))
        .and_then(Node::as_sequence)
    {
        Some(items) => items,
        None => return map,
    };
    for item in items {
        let target_node = item
            .get("targetCorrespondingSourceObject")
            .or_else(|| item.get("target"));
        let (target_file_id, target_guid) = parse_target_ref(target_node);
        let added_object_file_id =
            reference_id(item.get("addedObject")).or_else(|| reference_id(Some(item)));
        let insert_index = item
            .get("insertIndex")
            .and_then(Node::as_scalar)
            .map(String::from);
        map.insert(
            AddedObjectKey {
                target_file_id,
                target_guid,
                added_object_file_id,
            },
            insert_index,
        );
    }
    map
}

fn diff_prefab_instance_overrides(
    prefab_instance_file_id: i64,
    object_path: &str,
    old_props: Option<&Node>,
    new_props: Option<&Node>,
) -> Vec<PrefabOverrideDiff> {
    let mut diffs = Vec::new();

    let old_mods = extract_modification_map(old_props);
    let new_mods = extract_modification_map(new_props);
    let mut mod_keys: BTreeSet<ModificationKey> = BTreeSet::new();
    for key in old_mods.keys().chain(new_mods.keys()) {
        mod_keys.insert(key.clone());
    }
    for key in mod_keys {
        match (old_mods.get(&key), new_mods.get(&key)) {
            (None, Some(added)) => {
                diffs.push(PrefabOverrideDiff {
                    change: DiffChangeKind::Added,
                    section: OverrideSection::Modifications,
                    prefab_instance_file_id,
                    object_path: object_path.to_string(),
                    target_file_id: key.target_file_id,
                    target_guid: key.target_guid,
                    property_path: Some(key.property_path),
                    old_value: None,
                    new_value: added.value.clone(),
                    old_object_reference: None,
                    new_object_reference: added.object_reference.clone(),
                    added_object_file_id: None,
                });
            }
            (Some(removed), None) => {
                diffs.push(PrefabOverrideDiff {
                    change: DiffChangeKind::Removed,
                    section: OverrideSection::Modifications,
                    prefab_instance_file_id,
                    object_path: object_path.to_string(),
                    target_file_id: key.target_file_id,
                    target_guid: key.target_guid,
                    property_path: Some(key.property_path),
                    old_value: removed.value.clone(),
                    new_value: None,
                    old_object_reference: removed.object_reference.clone(),
                    new_object_reference: None,
                    added_object_file_id: None,
                });
            }
            (Some(old_entry), Some(new_entry)) => {
                if old_entry != new_entry {
                    diffs.push(PrefabOverrideDiff {
                        change: DiffChangeKind::Changed,
                        section: OverrideSection::Modifications,
                        prefab_instance_file_id,
                        object_path: object_path.to_string(),
                        target_file_id: key.target_file_id,
                        target_guid: key.target_guid,
                        property_path: Some(key.property_path),
                        old_value: old_entry.value.clone(),
                        new_value: new_entry.value.clone(),
                        old_object_reference: old_entry.object_reference.clone(),
                        new_object_reference: new_entry.object_reference.clone(),
                        added_object_file_id: None,
                    });
                }
            }
            (None, None) => {}
        }
    }

    let old_removed = extract_removed_components(old_props);
    let new_removed = extract_removed_components(new_props);
    for added_rem in new_removed.difference(&old_removed) {
        diffs.push(PrefabOverrideDiff {
            change: DiffChangeKind::Added,
            section: OverrideSection::RemovedComponents,
            prefab_instance_file_id,
            object_path: object_path.to_string(),
            target_file_id: added_rem.target_file_id,
            target_guid: added_rem.target_guid.clone(),
            property_path: None,
            old_value: None,
            new_value: None,
            old_object_reference: None,
            new_object_reference: None,
            added_object_file_id: None,
        });
    }
    for removed_rem in old_removed.difference(&new_removed) {
        diffs.push(PrefabOverrideDiff {
            change: DiffChangeKind::Removed,
            section: OverrideSection::RemovedComponents,
            prefab_instance_file_id,
            object_path: object_path.to_string(),
            target_file_id: removed_rem.target_file_id,
            target_guid: removed_rem.target_guid.clone(),
            property_path: None,
            old_value: None,
            new_value: None,
            old_object_reference: None,
            new_object_reference: None,
            added_object_file_id: None,
        });
    }

    for (field, section) in [
        ("m_AddedComponents", OverrideSection::AddedComponents),
        ("m_AddedGameObjects", OverrideSection::AddedGameObjects),
    ] {
        let old_added = extract_added_objects(old_props, field);
        let new_added = extract_added_objects(new_props, field);
        let mut keys: BTreeSet<AddedObjectKey> = BTreeSet::new();
        for key in old_added.keys().chain(new_added.keys()) {
            keys.insert(key.clone());
        }
        for key in keys {
            match (old_added.get(&key), new_added.get(&key)) {
                (None, Some(new_idx)) => {
                    diffs.push(PrefabOverrideDiff {
                        change: DiffChangeKind::Added,
                        section,
                        prefab_instance_file_id,
                        object_path: object_path.to_string(),
                        target_file_id: key.target_file_id,
                        target_guid: key.target_guid,
                        property_path: None,
                        old_value: None,
                        new_value: new_idx.clone(),
                        old_object_reference: None,
                        new_object_reference: None,
                        added_object_file_id: key.added_object_file_id,
                    });
                }
                (Some(old_idx), None) => {
                    diffs.push(PrefabOverrideDiff {
                        change: DiffChangeKind::Removed,
                        section,
                        prefab_instance_file_id,
                        object_path: object_path.to_string(),
                        target_file_id: key.target_file_id,
                        target_guid: key.target_guid,
                        property_path: None,
                        old_value: old_idx.clone(),
                        new_value: None,
                        old_object_reference: None,
                        new_object_reference: None,
                        added_object_file_id: key.added_object_file_id,
                    });
                }
                (Some(old_idx), Some(new_idx)) if old_idx != new_idx => {
                    diffs.push(PrefabOverrideDiff {
                        change: DiffChangeKind::Changed,
                        section,
                        prefab_instance_file_id,
                        object_path: object_path.to_string(),
                        target_file_id: key.target_file_id,
                        target_guid: key.target_guid,
                        property_path: None,
                        old_value: old_idx.clone(),
                        new_value: new_idx.clone(),
                        old_object_reference: None,
                        new_object_reference: None,
                        added_object_file_id: key.added_object_file_id,
                    });
                }
                _ => {}
            }
        }
    }

    diffs
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictResolutionChoice {
    Ours,
    Theirs,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictResolution {
    pub file_id: i64,
    pub property: Option<String>,
    pub choice: ConflictResolutionChoice,
}

pub fn resolve_conflicts_bytes(
    base: &[u8],
    ours: &[u8],
    theirs: &[u8],
    resolutions: &[ConflictResolution],
) -> Result<MergeOutcome, MergeError> {
    let base_asset = parse_asset(base, MergeSide::Base)?;
    let mut ours_asset = parse_asset(ours, MergeSide::Ours)?;
    let mut theirs_asset = parse_asset(theirs, MergeSide::Theirs)?;
    for resolution in resolutions {
        apply_resolution(&mut ours_asset, &mut theirs_asset, resolution)?;
    }
    merge_assets(&base_asset, &ours_asset, &theirs_asset)
}

pub fn resolve_conflicts_to_path(
    base: &Path,
    ours: &Path,
    theirs: &Path,
    resolutions: &[ConflictResolution],
    output: &Path,
) -> Result<MergeOutcome, MergeError> {
    let base_bytes = read_file(base)?;
    let ours_bytes = read_file(ours)?;
    let theirs_bytes = read_file(theirs)?;
    let outcome = resolve_conflicts_bytes(&base_bytes, &ours_bytes, &theirs_bytes, resolutions)?;
    if !outcome.conflicts.is_empty() {
        return Err(MergeError::UnresolvedConflicts {
            conflicts: outcome.conflicts,
        });
    }
    let introduced: Vec<UnresolvedReference> = outcome
        .references
        .iter()
        .filter(|reference| reference.introduced)
        .cloned()
        .collect();
    if !introduced.is_empty() {
        return Err(MergeError::UnresolvedReferences {
            references: introduced,
        });
    }
    let text = match outcome.merged.as_deref() {
        Some(text) => text,
        None => {
            return Err(MergeError::InvalidMerge {
                message: String::from("merge produced no result"),
            })
        }
    };
    fs::write(output, text).map_err(|error| MergeError::Io {
        path: output.to_path_buf(),
        message: error.to_string(),
    })?;
    Ok(outcome)
}

fn apply_resolution(
    ours: &mut Asset,
    theirs: &mut Asset,
    resolution: &ConflictResolution,
) -> Result<(), MergeError> {
    match &resolution.property {
        Some(property_path) => {
            let ours_obj = ours
                .objects
                .iter()
                .find(|obj| obj.file_id == resolution.file_id)
                .cloned();
            let theirs_obj = theirs
                .objects
                .iter()
                .find(|obj| obj.file_id == resolution.file_id)
                .cloned();
            let chosen_node = match &resolution.choice {
                ConflictResolutionChoice::Ours => ours_obj
                    .as_ref()
                    .and_then(|obj| get_nested_node(&obj.properties, property_path))
                    .cloned(),
                ConflictResolutionChoice::Theirs => theirs_obj
                    .as_ref()
                    .and_then(|obj| get_nested_node(&obj.properties, property_path))
                    .cloned(),
                ConflictResolutionChoice::Custom(val) => Some(Node::Scalar(val.clone())),
            };
            if let Some(node) = chosen_node {
                if let Some(obj) = ours
                    .objects
                    .iter_mut()
                    .find(|obj| obj.file_id == resolution.file_id)
                {
                    set_nested_node(&mut obj.properties, property_path, node.clone());
                }
                if let Some(obj) = theirs
                    .objects
                    .iter_mut()
                    .find(|obj| obj.file_id == resolution.file_id)
                {
                    set_nested_node(&mut obj.properties, property_path, node);
                }
            }
        }
        None => match &resolution.choice {
            ConflictResolutionChoice::Ours => {
                let ours_obj = ours
                    .objects
                    .iter()
                    .find(|obj| obj.file_id == resolution.file_id)
                    .cloned();
                theirs
                    .objects
                    .retain(|obj| obj.file_id != resolution.file_id);
                if let Some(obj) = ours_obj {
                    theirs.objects.push(obj);
                }
            }
            ConflictResolutionChoice::Theirs => {
                let theirs_obj = theirs
                    .objects
                    .iter()
                    .find(|obj| obj.file_id == resolution.file_id)
                    .cloned();
                ours.objects
                    .retain(|obj| obj.file_id != resolution.file_id);
                if let Some(obj) = theirs_obj {
                    ours.objects.push(obj);
                }
            }
            ConflictResolutionChoice::Custom(custom_yaml) => {
                let parsed = parse_asset(custom_yaml.as_bytes(), MergeSide::Ours)?;
                if let Some(custom_obj) = parsed
                    .objects
                    .into_iter()
                    .find(|obj| obj.file_id == resolution.file_id)
                {
                    ours.objects
                        .retain(|obj| obj.file_id != resolution.file_id);
                    theirs
                        .objects
                        .retain(|obj| obj.file_id != resolution.file_id);
                    ours.objects.push(custom_obj.clone());
                    theirs.objects.push(custom_obj);
                }
            }
        },
    }
    Ok(())
}

fn get_nested_node<'a>(node: &'a Node, path: &str) -> Option<&'a Node> {
    let mut current = node;
    for segment in path.split('.') {
        if segment.is_empty() {
            continue;
        }
        current = current.get(segment)?;
    }
    Some(current)
}

fn set_nested_node(node: &mut Node, path: &str, value: Node) {
    let segments: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
    set_nested_segments(node, &segments, value);
}

fn set_nested_segments(node: &mut Node, segments: &[&str], value: Node) {
    let (first, rest) = match segments.split_first() {
        Some(pair) => pair,
        None => {
            *node = value;
            return;
        }
    };
    if let Node::Mapping(entries) = node {
        if rest.is_empty() {
            if let Some((_, existing)) = entries.iter_mut().find(|(k, _)| k == first) {
                *existing = value;
            } else {
                entries.push(((*first).to_string(), value));
            }
        } else if let Some((_, child)) = entries.iter_mut().find(|(k, _)| k == first) {
            set_nested_segments(child, rest, value);
        } else {
            let mut child = Node::Mapping(Vec::new());
            set_nested_segments(&mut child, rest, value);
            entries.push(((*first).to_string(), child));
        }
    }
}

pub fn mcp_merge_asset(base: &str, ours: &str, theirs: &str) -> Result<MergeOutcome, MergeError> {
    merge_bytes(base.as_bytes(), ours.as_bytes(), theirs.as_bytes())
}

pub fn mcp_semantic_diff_asset(old: &str, new: &str) -> Result<DiffOutcome, MergeError> {
    semantic_diff_bytes(old.as_bytes(), new.as_bytes())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileChangeOrigin {
    Person,
    AgentSession { session_id: String },
    ExternalSourceControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAccountabilityState {
    Clean,
    Unsaved,
    ExternallyChanged,
    Conflicted,
    AccountedKeepUnsaved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileResolutionAction {
    Save,
    DiscardAndReload,
    KeepUnsavedWork,
    ResolveConflict { resolved_bytes: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackedFileAccountability {
    pub path: PathBuf,
    pub state: FileAccountabilityState,
    pub origins: Vec<FileChangeOrigin>,
    pub unresolved_conflicts: Vec<MergeConflict>,
    pub saved_bytes: Vec<u8>,
    pub working_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceGuardError {
    UnaccountedFiles {
        files: Vec<TrackedFileAccountability>,
    },
    UntrackedFile {
        path: PathBuf,
    },
    UnresolvedMergeConflict {
        path: PathBuf,
        conflicts: Vec<MergeConflict>,
    },
    MergeFailed {
        path: PathBuf,
        error: MergeError,
    },
}

impl fmt::Display for WorkspaceGuardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkspaceGuardError::UnaccountedFiles { files } => {
                write!(
                    formatter,
                    "cannot close or reload: {} unsaved or conflicting file(s) unaccounted for",
                    files.len()
                )
            }
            WorkspaceGuardError::UntrackedFile { path } => {
                write!(formatter, "file is not tracked in workspace guard: {}", path.display())
            }
            WorkspaceGuardError::UnresolvedMergeConflict { path, conflicts } => {
                write!(
                    formatter,
                    "file {} still has {} unresolved conflict(s)",
                    path.display(),
                    conflicts.len()
                )
            }
            WorkspaceGuardError::MergeFailed { path, error } => {
                write!(formatter, "merge validation failed for {}: {error}", path.display())
            }
        }
    }
}

impl std::error::Error for WorkspaceGuardError {}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceMergeGuard {
    files: BTreeMap<PathBuf, TrackedFileAccountability>,
}

impl WorkspaceMergeGuard {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
        }
    }

    pub fn record_saved(&mut self, path: &Path, bytes: &[u8]) {
        self.files.insert(
            path.to_path_buf(),
            TrackedFileAccountability {
                path: path.to_path_buf(),
                state: FileAccountabilityState::Clean,
                origins: Vec::new(),
                unresolved_conflicts: Vec::new(),
                saved_bytes: bytes.to_vec(),
                working_bytes: bytes.to_vec(),
            },
        );
    }

    pub fn record_unsaved_edit(&mut self, path: &Path, origin: FileChangeOrigin, bytes: &[u8]) {
        let entry = self
            .files
            .entry(path.to_path_buf())
            .or_insert_with(|| TrackedFileAccountability {
                path: path.to_path_buf(),
                state: FileAccountabilityState::Clean,
                origins: Vec::new(),
                unresolved_conflicts: Vec::new(),
                saved_bytes: Vec::new(),
                working_bytes: Vec::new(),
            });
        entry.working_bytes = bytes.to_vec();
        if !entry.origins.contains(&origin) {
            entry.origins.push(origin);
        }
        if entry.state != FileAccountabilityState::Conflicted {
            entry.state = FileAccountabilityState::Unsaved;
        }
    }

    pub fn record_external_change_or_conflict(
        &mut self,
        path: &Path,
        external_bytes: &[u8],
        conflicts: Vec<MergeConflict>,
    ) {
        let entry = self
            .files
            .entry(path.to_path_buf())
            .or_insert_with(|| TrackedFileAccountability {
                path: path.to_path_buf(),
                state: FileAccountabilityState::Clean,
                origins: Vec::new(),
                unresolved_conflicts: Vec::new(),
                saved_bytes: external_bytes.to_vec(),
                working_bytes: external_bytes.to_vec(),
            });
        if !entry
            .origins
            .contains(&FileChangeOrigin::ExternalSourceControl)
        {
            entry.origins.push(FileChangeOrigin::ExternalSourceControl);
        }
        if !conflicts.is_empty() {
            entry.state = FileAccountabilityState::Conflicted;
            entry.unresolved_conflicts = conflicts;
        } else if entry.state == FileAccountabilityState::Unsaved
            || entry.state == FileAccountabilityState::AccountedKeepUnsaved
        {
            let outcome = merge_bytes(&entry.saved_bytes, &entry.working_bytes, external_bytes);
            match outcome {
                Ok(merged) if merged.conflicts.is_empty() => {
                    entry.state = FileAccountabilityState::ExternallyChanged;
                }
                Ok(merged) => {
                    entry.state = FileAccountabilityState::Conflicted;
                    entry.unresolved_conflicts = merged.conflicts;
                }
                Err(_) => {
                    entry.state = FileAccountabilityState::Conflicted;
                }
            }
        } else {
            entry.state = FileAccountabilityState::ExternallyChanged;
            entry.saved_bytes = external_bytes.to_vec();
        }
    }

    pub fn unaccounted_files(&self) -> Vec<TrackedFileAccountability> {
        self.files
            .values()
            .filter(|entry| {
                matches!(
                    entry.state,
                    FileAccountabilityState::Unsaved
                        | FileAccountabilityState::ExternallyChanged
                        | FileAccountabilityState::Conflicted
                )
            })
            .cloned()
            .collect()
    }

    pub fn can_close_workspace(&self) -> Result<(), WorkspaceGuardError> {
        let unaccounted = self.unaccounted_files();
        if unaccounted.is_empty() {
            Ok(())
        } else {
            Err(WorkspaceGuardError::UnaccountedFiles { files: unaccounted })
        }
    }

    pub fn can_reload_file(&self, path: &Path) -> Result<(), WorkspaceGuardError> {
        let entry = self
            .files
            .get(path)
            .ok_or_else(|| WorkspaceGuardError::UntrackedFile {
                path: path.to_path_buf(),
            })?;
        match entry.state {
            FileAccountabilityState::Clean => Ok(()),
            FileAccountabilityState::Unsaved
            | FileAccountabilityState::ExternallyChanged
            | FileAccountabilityState::Conflicted
            | FileAccountabilityState::AccountedKeepUnsaved => {
                Err(WorkspaceGuardError::UnaccountedFiles {
                    files: vec![entry.clone()],
                })
            }
        }
    }

    pub fn account_for_file(
        &mut self,
        path: &Path,
        action: FileResolutionAction,
    ) -> Result<TrackedFileAccountability, WorkspaceGuardError> {
        let entry = self
            .files
            .get_mut(path)
            .ok_or_else(|| WorkspaceGuardError::UntrackedFile {
                path: path.to_path_buf(),
            })?;
        match action {
            FileResolutionAction::Save => {
                if !entry.unresolved_conflicts.is_empty() {
                    return Err(WorkspaceGuardError::UnresolvedMergeConflict {
                        path: path.to_path_buf(),
                        conflicts: entry.unresolved_conflicts.clone(),
                    });
                }
                entry.saved_bytes = entry.working_bytes.clone();
                entry.origins.clear();
                entry.state = FileAccountabilityState::Clean;
            }
            FileResolutionAction::DiscardAndReload => {
                entry.working_bytes = entry.saved_bytes.clone();
                entry.origins.clear();
                entry.unresolved_conflicts.clear();
                entry.state = FileAccountabilityState::Clean;
            }
            FileResolutionAction::KeepUnsavedWork => {
                if !entry.unresolved_conflicts.is_empty() {
                    return Err(WorkspaceGuardError::UnresolvedMergeConflict {
                        path: path.to_path_buf(),
                        conflicts: entry.unresolved_conflicts.clone(),
                    });
                }
                entry.state = FileAccountabilityState::AccountedKeepUnsaved;
            }
            FileResolutionAction::ResolveConflict { resolved_bytes } => {
                let parsed = parse_asset(&resolved_bytes, MergeSide::Ours).map_err(|error| {
                    WorkspaceGuardError::MergeFailed {
                        path: path.to_path_buf(),
                        error,
                    }
                })?;
                let names = object_names(&parsed.objects);
                let paths = build_paths(&parsed.objects, &names);
                let dangling = unresolved_references(&parsed.objects, &paths);
                if !dangling.is_empty() {
                    return Err(WorkspaceGuardError::MergeFailed {
                        path: path.to_path_buf(),
                        error: MergeError::UnresolvedReferences {
                            references: dangling,
                        },
                    });
                }
                entry.saved_bytes = resolved_bytes.clone();
                entry.working_bytes = resolved_bytes;
                entry.unresolved_conflicts.clear();
                entry.origins.clear();
                entry.state = FileAccountabilityState::Clean;
            }
        }
        Ok(entry.clone())
    }
}


