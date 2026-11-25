use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::binary_ser::{is_text_serialized, read_serialized_file, SerializedFile};
use crate::project::{AssetKind, Project};
use crate::unity_yaml::{self, Node};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyError {
    Io { path: PathBuf, message: String },
}

impl fmt::Display for DependencyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependencyError::Io { path, message } => {
                write!(formatter, "cannot read {}: {}", path.display(), message)
            }
        }
    }
}

impl std::error::Error for DependencyError {}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DependencyGraph {
    direct: BTreeMap<String, Vec<String>>,
    reverse: BTreeMap<String, Vec<String>>,
}

impl DependencyGraph {
    pub fn build(project: &Project) -> Result<Self, DependencyError> {
        let mut direct: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut reverse_sets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for entry in &project.inventory().entries {
            if entry.kind != AssetKind::Main {
                continue;
            }
            let asset_path = project.root().join(&entry.path);
            let mut deps = scan_asset_dependencies(&asset_path)?;
            deps.remove(&entry.guid);
            let sorted_deps: Vec<String> = deps.into_iter().collect();
            for target_guid in &sorted_deps {
                reverse_sets
                    .entry(target_guid.clone())
                    .or_default()
                    .insert(entry.guid.clone());
            }
            reverse_sets.entry(entry.guid.clone()).or_default();
            direct.insert(entry.guid.clone(), sorted_deps);
        }

        let reverse = reverse_sets
            .into_iter()
            .map(|(guid, sources)| (guid, sources.into_iter().collect()))
            .collect();

        Ok(Self { direct, reverse })
    }

    pub fn dependencies_of(&self, guid: &str) -> Vec<&str> {
        match self.direct.get(guid) {
            Some(targets) => targets.iter().map(String::as_str).collect(),
            None => Vec::new(),
        }
    }

    pub fn reverse_dependencies_of(&self, guid: &str, transitive: bool) -> Vec<&str> {
        if !transitive {
            return match self.reverse.get(guid) {
                Some(sources) => sources.iter().map(String::as_str).collect(),
                None => Vec::new(),
            };
        }

        let mut visited: BTreeSet<&str> = BTreeSet::new();
        let mut queue: Vec<&str> = match self.reverse.get(guid) {
            Some(sources) => sources.iter().map(String::as_str).collect(),
            None => Vec::new(),
        };

        while let Some(current) = queue.pop() {
            if current == guid || !visited.insert(current) {
                continue;
            }
            if let Some(parents) = self.reverse.get(current) {
                for parent in parents {
                    let parent_str = parent.as_str();
                    if parent_str != guid && !visited.contains(parent_str) {
                        queue.push(parent_str);
                    }
                }
            }
        }

        visited.into_iter().collect()
    }

    pub fn cycles(&self) -> Vec<Vec<String>> {
        let mut canonical_cycles: BTreeSet<Vec<String>> = BTreeSet::new();
        let nodes: Vec<&str> = self.direct.keys().map(String::as_str).collect();

        for &start in &nodes {
            let mut path: Vec<&str> = vec![start];
            let mut on_path: BTreeSet<&str> = BTreeSet::new();
            on_path.insert(start);
            self.collect_cycles_from(start, start, &mut path, &mut on_path, &mut canonical_cycles);
        }

        canonical_cycles.into_iter().collect()
    }

    fn collect_cycles_from<'a>(
        &'a self,
        start: &'a str,
        current: &'a str,
        path: &mut Vec<&'a str>,
        on_path: &mut BTreeSet<&'a str>,
        found: &mut BTreeSet<Vec<String>>,
    ) {
        let neighbors = match self.direct.get(current) {
            Some(neighbors) => neighbors,
            None => return,
        };

        for neighbor in neighbors {
            let next = neighbor.as_str();
            if next == start {
                found.insert(canonicalize_cycle(path));
            } else if next > start && !on_path.contains(next) && self.direct.contains_key(next) {
                path.push(next);
                on_path.insert(next);
                self.collect_cycles_from(start, next, path, on_path, found);
                on_path.remove(next);
                path.pop();
            }
        }
    }
}

fn canonicalize_cycle(path: &[&str]) -> Vec<String> {
    if path.is_empty() {
        return Vec::new();
    }
    let mut min_index = 0;
    for (index, item) in path.iter().enumerate() {
        if item < &path[min_index] {
            min_index = index;
        }
    }
    let mut cycle = Vec::with_capacity(path.len());
    for index in 0..path.len() {
        let rotated = (min_index + index) % path.len();
        cycle.push(path[rotated].to_string());
    }
    cycle
}

fn scan_asset_dependencies(path: &Path) -> Result<BTreeSet<String>, DependencyError> {
    let bytes = fs::read(path).map_err(|error| DependencyError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;

    let mut guids = BTreeSet::new();

    if is_text_serialized(&bytes) {
        if let Ok(text) = std::str::from_utf8(&bytes) {
            if let Ok(stream) = unity_yaml::parse(text) {
                for document in &stream.documents {
                    collect_guids_from_yaml_node(&document.root, &mut guids);
                }
            }
            collect_guids_from_text(text, &mut guids);
        }
        return Ok(guids);
    }

    if let Ok(serialized) = read_serialized_file(&bytes) {
        collect_guids_from_binary_file(&serialized, &mut guids);
        return Ok(guids);
    }

    if let Ok(text) = std::str::from_utf8(&bytes) {
        collect_guids_from_text(text, &mut guids);
    }

    Ok(guids)
}

fn collect_guids_from_yaml_node(node: &Node, guids: &mut BTreeSet<String>) {
    match node {
        Node::Scalar(_) => {}
        Node::Mapping(entries) => {
            for (key, value) in entries {
                if key == "guid" {
                    if let Some(raw) = value.as_str() {
                        if let Some(normalized) = normalize_guid(raw) {
                            guids.insert(normalized);
                        }
                    }
                }
                collect_guids_from_yaml_node(value, guids);
            }
        }
        Node::Sequence(items) => {
            for item in items {
                collect_guids_from_yaml_node(item, guids);
            }
        }
    }
}

fn collect_guids_from_binary_file(file: &SerializedFile, guids: &mut BTreeSet<String>) {
    for index in 0..file.objects.len() {
        if let Ok(payload) = file.object_bytes(index) {
            collect_guids_from_bytes(payload, guids);
        }
    }
    collect_guids_from_bytes(&file.raw, guids);
}

fn collect_guids_from_bytes(bytes: &[u8], guids: &mut BTreeSet<String>) {
    let text = String::from_utf8_lossy(bytes);
    collect_guids_from_text(&text, guids);
}

fn collect_guids_from_text(text: &str, guids: &mut BTreeSet<String>) {
    for line in text.lines() {
        let mut remainder = line;
        while let Some(pos) = find_guid_key(remainder) {
            let after_key = &remainder[pos..];
            let candidate = extract_guid_token(after_key);
            if let Some(normalized) = normalize_guid(candidate) {
                guids.insert(normalized);
            }
            remainder = after_key;
        }
    }
}

fn find_guid_key(line: &str) -> Option<usize> {
    for (index, _) in line.match_indices("guid") {
        let after = &line[index + 4..];
        let trimmed = after.trim_start_matches([' ', '\t', '"', '\'']);
        if let Some(after_colon) = trimmed.strip_prefix(':') {
            let offset = line.len() - after_colon.len();
            return Some(offset);
        }
    }
    None
}

fn extract_guid_token(after_colon: &str) -> &str {
    let trimmed = after_colon.trim_start_matches([' ', '\t', '"', '\'']);
    let end = trimmed
        .find(|character: char| !character.is_ascii_hexdigit())
        .unwrap_or(trimmed.len());
    &trimmed[..end]
}

fn normalize_guid(candidate: &str) -> Option<String> {
    let trimmed = candidate.trim();
    if trimmed.len() == 32
        && trimmed
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        let lower = trimmed.to_ascii_lowercase();
        if lower.chars().any(|character| character != '0') {
            return Some(lower);
        }
    }
    None
}
