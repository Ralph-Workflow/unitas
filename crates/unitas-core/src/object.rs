#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectKind {
    GameObject,
    Component,
    ScriptableObject,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldObject {
    id: u64,
    kind: ObjectKind,
    name: String,
    destroyed: bool,
}

impl WorldObject {
    pub fn new(id: u64, kind: ObjectKind, name: impl Into<String>) -> Self {
        Self {
            id,
            kind,
            name: name.into(),
            destroyed: false,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn kind(&self) -> ObjectKind {
        self.kind
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    pub fn destroy(&mut self) -> bool {
        if self.destroyed {
            return false;
        }

        self.destroyed = true;
        true
    }
}
