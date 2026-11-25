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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetImportState {
    NotYetEvaluated,
    Processing,
    Ready,
    ReadyWithLimitations,
    Failed,
    Missing,
    BlockedByDependency,
    BlockedByTrust,
}

impl AssetImportState {
    pub fn label(self) -> &'static str {
        match self {
            AssetImportState::NotYetEvaluated => "not yet evaluated",
            AssetImportState::Processing => "processing",
            AssetImportState::Ready => "ready",
            AssetImportState::ReadyWithLimitations => "ready with limitations",
            AssetImportState::Failed => "failed",
            AssetImportState::Missing => "missing",
            AssetImportState::BlockedByDependency => "blocked by dependency",
            AssetImportState::BlockedByTrust => "blocked by trust",
        }
    }

    pub fn is_fully_usable(self) -> bool {
        matches!(self, AssetImportState::Ready)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDiagnostic {
    pub asset: String,
    pub sub_object: Option<String>,
    pub issue: String,
    pub consequence: String,
    pub partially_usable: bool,
    pub context: Option<String>,
}

impl ImportDiagnostic {
    pub fn new(
        asset: impl Into<String>,
        issue: impl Into<String>,
        consequence: impl Into<String>,
    ) -> Self {
        ImportDiagnostic {
            asset: asset.into(),
            sub_object: None,
            issue: issue.into(),
            consequence: consequence.into(),
            partially_usable: false,
            context: None,
        }
    }

    pub fn with_sub_object(mut self, name: impl Into<String>) -> Self {
        self.sub_object = Some(name.into());
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn partially_usable(mut self) -> Self {
        self.partially_usable = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportOutcome {
    pub state: AssetImportState,
    pub diagnostics: Vec<ImportDiagnostic>,
}

impl ImportOutcome {
    pub fn ready() -> Self {
        ImportOutcome {
            state: AssetImportState::Ready,
            diagnostics: Vec::new(),
        }
    }

    pub fn limited(diagnostic: ImportDiagnostic) -> Self {
        ImportOutcome {
            state: AssetImportState::ReadyWithLimitations,
            diagnostics: vec![diagnostic],
        }
    }

    pub fn failed(diagnostic: ImportDiagnostic) -> Self {
        ImportOutcome {
            state: AssetImportState::Failed,
            diagnostics: vec![diagnostic],
        }
    }

    pub fn missing(diagnostic: ImportDiagnostic) -> Self {
        ImportOutcome {
            state: AssetImportState::Missing,
            diagnostics: vec![diagnostic],
        }
    }

    pub fn blocked_by_dependency(diagnostic: ImportDiagnostic) -> Self {
        ImportOutcome {
            state: AssetImportState::BlockedByDependency,
            diagnostics: vec![diagnostic],
        }
    }

    pub fn blocked_by_trust(diagnostic: ImportDiagnostic) -> Self {
        ImportOutcome {
            state: AssetImportState::BlockedByTrust,
            diagnostics: vec![diagnostic],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportFingerprint {
    pub source: u64,
    pub metadata: u64,
    pub settings: u64,
    pub importer_version: u64,
    pub dependencies: u64,
}

impl ImportFingerprint {
    pub fn change_reason(&self, previous: &ImportFingerprint) -> Option<&'static str> {
        if self.source != previous.source {
            Some("source content changed")
        } else if self.metadata != previous.metadata {
            Some("metadata changed")
        } else if self.settings != previous.settings {
            Some("import settings changed")
        } else if self.importer_version != previous.importer_version {
            Some("importer version changed")
        } else if self.dependencies != previous.dependencies {
            Some("dependency set changed")
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportCandidate {
    pub path: PathBuf,
    pub guid: String,
    pub source: Vec<u8>,
    pub metadata: String,
    pub settings: String,
    pub importer_version: String,
    pub dependencies: Vec<String>,
}

impl ImportCandidate {
    pub fn fingerprint(&self) -> ImportFingerprint {
        ImportFingerprint {
            source: hash_bytes(&self.source),
            metadata: hash_bytes(self.metadata.as_bytes()),
            settings: hash_bytes(self.settings.as_bytes()),
            importer_version: hash_bytes(self.importer_version.as_bytes()),
            dependencies: hash_bytes(self.dependencies.join("\u{1}").as_bytes()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRecord {
    pub path: PathBuf,
    pub guid: String,
    pub state: AssetImportState,
    pub diagnostics: Vec<ImportDiagnostic>,
    pub fingerprint: ImportFingerprint,
    pub reused: bool,
    pub reason: String,
}

impl ImportRecord {
    pub fn consequence(&self) -> Option<&str> {
        self.diagnostics
            .iter()
            .find(|diagnostic| !diagnostic.consequence.is_empty())
            .map(|diagnostic| diagnostic.consequence.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSessionReport {
    pub records: Vec<ImportRecord>,
    pub imported: usize,
    pub reused: usize,
    pub failed: usize,
    pub succeeded: usize,
    pub partial_success: bool,
}

impl ImportSessionReport {
    pub fn record(&self, path: &Path) -> Option<&ImportRecord> {
        self.records.iter().find(|record| record.path == path)
    }

    pub fn imported_again(&self) -> Vec<&ImportRecord> {
        self.records
            .iter()
            .filter(|record| !record.reused)
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImportSession {
    records: BTreeMap<PathBuf, ImportRecord>,
}

impl ImportSession {
    pub fn new() -> Self {
        ImportSession::default()
    }

    pub fn record(&self, path: &Path) -> Option<&ImportRecord> {
        self.records.get(path)
    }

    pub fn run(
        &mut self,
        candidates: &[ImportCandidate],
        importer: &mut dyn FnMut(&ImportCandidate) -> ImportOutcome,
    ) -> ImportSessionReport {
        let mut changed: BTreeSet<String> = BTreeSet::new();
        let mut reasons: BTreeMap<PathBuf, String> = BTreeMap::new();

        for candidate in candidates {
            let fingerprint = candidate.fingerprint();
            let reason = match self.records.get(&candidate.path) {
                None => Some("first import"),
                Some(record) => fingerprint.change_reason(&record.fingerprint),
            };
            if let Some(reason) = reason {
                changed.insert(candidate.guid.clone());
                reasons.insert(candidate.path.clone(), reason.to_string());
            }
        }

        loop {
            let mut propagated = false;
            for candidate in candidates {
                if changed.contains(&candidate.guid) {
                    continue;
                }
                for dependency in &candidate.dependencies {
                    if changed.contains(dependency) {
                        changed.insert(candidate.guid.clone());
                        reasons.insert(
                            candidate.path.clone(),
                            format!("dependency changed: {dependency}"),
                        );
                        propagated = true;
                        break;
                    }
                }
            }
            if !propagated {
                break;
            }
        }

        let mut records = Vec::new();
        let mut imported = 0usize;
        let mut reused = 0usize;
        let mut failed = 0usize;

        for candidate in candidates {
            let fingerprint = candidate.fingerprint();
            match reasons.get(&candidate.path).cloned() {
                Some(reason) => {
                    let asset = candidate.path.display().to_string();
                    let outcome = state_with_consequence(importer(candidate), &asset);
                    let mut diagnostics = self
                        .records
                        .get(&candidate.path)
                        .map(|record| record.diagnostics.clone())
                        .unwrap_or_default();
                    diagnostics.extend(outcome.diagnostics);
                    let record = ImportRecord {
                        path: candidate.path.clone(),
                        guid: candidate.guid.clone(),
                        state: outcome.state,
                        diagnostics,
                        fingerprint,
                        reused: false,
                        reason,
                    };
                    if record.state == AssetImportState::Failed {
                        failed += 1;
                    }
                    imported += 1;
                    self.records.insert(candidate.path.clone(), record.clone());
                    records.push(record);
                }
                None => {
                    let previous = match self.records.get(&candidate.path) {
                        Some(previous) => previous.clone(),
                        None => continue,
                    };
                    let record = ImportRecord {
                        reused: true,
                        reason: "inputs unchanged; the recorded result was reused".to_string(),
                        ..previous
                    };
                    if record.state == AssetImportState::Failed {
                        failed += 1;
                    }
                    reused += 1;
                    self.records.insert(candidate.path.clone(), record.clone());
                    records.push(record);
                }
            }
        }

        let succeeded = imported + reused - failed;
        let partial_success = failed > 0 && succeeded > 0;
        ImportSessionReport {
            records,
            imported,
            reused,
            failed,
            succeeded,
            partial_success,
        }
    }
}

fn state_with_consequence(outcome: ImportOutcome, asset: &str) -> ImportOutcome {
    if outcome.state.is_fully_usable() {
        return outcome;
    }
    if outcome
        .diagnostics
        .iter()
        .any(|diagnostic| !diagnostic.consequence.is_empty())
    {
        return outcome;
    }
    ImportOutcome {
        state: outcome.state,
        diagnostics: {
            let mut diagnostics = outcome.diagnostics;
            diagnostics.push(ImportDiagnostic::new(
                asset,
                format!("the asset is {}", outcome.state.label()),
                format!("the asset stays {}", outcome.state.label()),
            ));
            diagnostics
        },
    }
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

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
