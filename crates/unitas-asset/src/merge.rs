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
    DeleteVersusChange { deleted_by: MergeSide },
    ReparentCycle { cycle: Vec<i64> },
    FileIdReuse { base_class: Option<u32>, ours_class: Option<u32>, theirs_class: Option<u32> },
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
    NotMergeable { reason: String },
    Yaml { side: MergeSide, error: YamlError },
    Io { path: PathBuf, message: String },
    UnresolvedConflicts { conflicts: Vec<MergeConflict> },
    UnresolvedReferences { references: Vec<UnresolvedReference> },
    InvalidMerge { message: String },
}

impl fmt::Display for MergeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MergeError::NotMergeable { reason } => {
                write!(formatter, "asset is not semantically mergeable: {reason}")
            }
            MergeError::Yaml { side, error } => {
                write!(formatter, "{} side is not valid unity yaml: {error}", side.label())
            }
            MergeError::Io { path, message } => {
                write!(formatter, "cannot read {}: {}", path.display(), message)
            }
            MergeError::UnresolvedConflicts { conflicts } => {
                write!(formatter, "merge has {} unresolved conflicts", conflicts.len())
            }
            MergeError::UnresolvedReferences { references } => {
                write!(formatter, "merge has {} unresolved references", references.len())
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
        return Err(MergeError::UnresolvedConflicts { conflicts: outcome.conflicts });
    }
    let introduced: Vec<UnresolvedReference> = outcome
        .references
        .iter()
        .filter(|reference| reference.introduced)
        .cloned()
        .collect();
    if !introduced.is_empty() {
        return Err(MergeError::UnresolvedReferences { references: introduced });
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
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => {
            return Err(MergeError::NotMergeable {
                reason: String::from(
                    "asset uses a binary serialization form and carries no readable object data",
                ),
            })
        }
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
    Asset { kind: AssetKind::Unity, header: stream.header, objects }
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

fn merge_assets(
    base: &Asset,
    ours: &Asset,
    theirs: &Asset,
) -> Result<MergeOutcome, MergeError> {
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
    conflicts.extend(cycle_conflicts(&merged_objects, &base.objects, &ours.objects, &theirs.objects));
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
    for conflict in conflicts.iter_mut() {
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
    for reference in references.iter_mut() {
        reference.introduced = !introduced.contains(&(
            reference.file_id,
            reference.property.clone(),
            reference.referenced_file_id,
        ));
    }
    let header = base.header.clone().or_else(|| ours.header.clone()).or_else(|| theirs.header.clone());
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
                detail: Some(String::from("object deleted on one side and changed on the other")),
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
                detail: Some(String::from("object deleted on one side and changed on the other")),
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
                detail: Some(String::from("both sides reordered the same serialized array")),
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
            Some(transform) => transform_path(*transform, &transforms, &names, &mut visiting),
            None => name.clone(),
        };
        paths.insert(*game_object, path);
    }
    for transform in transforms.keys() {
        let path = transform_path(*transform, &transforms, &names, &mut visiting);
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
        let path = match owner.get(&object.file_id).and_then(|game_object| paths.get(game_object)) {
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
    let name = match info.game_object.and_then(|game_object| names.get(&game_object)) {
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
    left.sort();
    right.sort();
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
            members.sort();
            members
        })
        .collect();
    let mut conflicts: Vec<MergeConflict> = Vec::new();
    for cycle in find_cycles(&merged_fathers) {
        let mut members = cycle.clone();
        members.sort();
        if base_cycles.contains(&members) {
            continue;
        }
        let first = match cycle.first() {
            Some(first) => *first,
            None => continue,
        };
        conflicts.push(MergeConflict {
            kind: ConflictKind::Structural(StructuralKind::ReparentCycle { cycle: cycle.clone() }),
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
                    message: format!("merged result does not parse as a property document: {error}"),
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
    if !entries.iter().all(|(_, value)| matches!(value, Node::Scalar(_))) {
        return false;
    }
    if entries.iter().any(|(key, _)| key == FLOW_REFERENCE_KEY) {
        return true;
    }
    entries.iter().all(|(key, _)| {
        key.len() == 1 && key.chars().all(|character| VECTOR_COMPONENTS.contains(character))
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
        match value {
            Node::Scalar(scalar) => write_scalar(out, scalar),
            _ => {}
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
