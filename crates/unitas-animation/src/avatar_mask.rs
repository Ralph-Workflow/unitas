use crate::clip::SampledBinding;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AvatarMask {
    name: String,
    paths: Vec<String>,
}

impl AvatarMask {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            paths: Vec::new(),
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        let path = path.into();
        if !self.paths.contains(&path) {
            self.paths.push(path);
        }
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn includes_path(&self, path: &str) -> bool {
        self.paths.iter().any(|masked| path_matches(masked, path))
    }

    pub fn filter_bindings(&self, bindings: &[SampledBinding]) -> Vec<SampledBinding> {
        bindings
            .iter()
            .filter(|binding| self.includes_path(&binding.target.object_path))
            .cloned()
            .collect()
    }
}

fn path_matches(masked: &str, path: &str) -> bool {
    if masked == path {
        return true;
    }
    if masked.is_empty() {
        return false;
    }
    path.len() > masked.len() && path.starts_with(masked) && path.as_bytes()[masked.len()] == b'/'
}
