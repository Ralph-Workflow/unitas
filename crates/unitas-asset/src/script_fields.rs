use std::collections::HashMap;

use crate::unity_yaml::{Node, Stream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldRename {
    pub current: String,
    pub former: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldRenameMap {
    entries: HashMap<String, Vec<FieldRename>>,
}

impl FieldRenameMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn add_rename(&mut self, script_key: &str, current: &str, former: &[&str]) {
        if current.is_empty() || former.is_empty() {
            return;
        }
        let former: Vec<String> = former
            .iter()
            .filter(|name| !name.is_empty())
            .map(|name| (*name).to_string())
            .collect();
        if former.is_empty() {
            return;
        }
        let renames = self.entries.entry(script_key.to_string()).or_default();
        if let Some(existing) = renames.iter_mut().find(|rename| rename.current == current) {
            for name in former {
                if !existing.former.contains(&name) {
                    existing.former.push(name);
                }
            }
            return;
        }
        renames.push(FieldRename {
            current: current.to_string(),
            former,
        });
    }

    pub fn renames_for(&self, script_key: &str) -> &[FieldRename] {
        self.entries
            .get(script_key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

// NOTE: the bridge symbol JSON (Unitas.Scripting SymbolQuery) identifies types
// by name/namespace/assembly and carries no Unity script GUID, so it cannot be
// parsed directly into the GUID-keyed map scene documents use (m_Script guid).
// Callers that hold symbol data resolve fullName -> GUID at the call site and
// feed this map through add_rename; see save_stream_with_renames.

pub fn resolve_formerly_serialized_as<'a>(
    body: &'a Node,
    current_field: &str,
    former_names: &[&str],
) -> Option<&'a Node> {
    if let Some(value) = body.get(current_field) {
        return Some(value);
    }
    former_names.iter().find_map(|former| body.get(former))
}

// Only the first matching former alias is migrated; real Unity renames never
// leave two aliases of one field live in the same document.
fn apply_rename(body_entries: &mut [(String, Node)], rename: &FieldRename) -> bool {
    if body_entries.iter().any(|(key, _)| key == &rename.current) {
        return true;
    }
    for (key, _) in body_entries.iter_mut() {
        if rename.former.iter().any(|former| former == key) {
            *key = rename.current.clone();
            return true;
        }
    }
    false
}

pub fn migrate_stream(stream: &mut Stream, map: &FieldRenameMap) {
    if map.is_empty() {
        return;
    }
    for document in &mut stream.documents {
        let Node::Mapping(entries) = &mut document.root else {
            continue;
        };
        let Some((_, body)) = entries.iter_mut().find(|(key, _)| key == "MonoBehaviour") else {
            continue;
        };
        let Node::Mapping(body_entries) = body else {
            continue;
        };
        let Some(guid) = body_entries
            .iter()
            .find(|(key, _)| key == "m_Script")
            .and_then(|(_, value)| value.get("guid"))
            .and_then(Node::as_str)
            .map(str::to_string)
        else {
            continue;
        };
        let renames = map.renames_for(&guid);
        for rename in renames {
            apply_rename(body_entries, rename);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedReferenceType<'a> {
    pub class: &'a str,
    pub ns: &'a str,
    pub asm: &'a str,
}

pub fn ref_ids(body: &Node) -> Option<&[Node]> {
    body.get("references")?.get("RefIds")?.as_sequence()
}

pub fn resolve_managed_reference(body: &Node, rid: i64) -> Option<&Node> {
    ref_ids(body)?
        .iter()
        .find(|entry| entry.get("rid").and_then(Node::as_i64) == Some(rid))
}

pub fn missing_managed_types<'a>(
    body: &'a Node,
    known_types: &[&str],
) -> Vec<ManagedReferenceType<'a>> {
    let mut missing = Vec::new();
    for entry in ref_ids(body).unwrap_or(&[]) {
        let Some(type_node) = entry.get("type") else {
            continue;
        };
        let class = type_node.get("class").and_then(Node::as_str);
        let ns = type_node.get("ns").and_then(Node::as_str);
        let asm = type_node.get("asm").and_then(Node::as_str);
        let (Some(class), Some(ns), Some(asm)) = (class, ns, asm) else {
            continue;
        };
        if !known_types.contains(&class) {
            missing.push(ManagedReferenceType { class, ns, asm });
        }
    }
    missing
}

#[cfg(test)]
#[path = "script_fields_tests.rs"]
mod tests;
