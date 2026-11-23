use crate::entity::EntityId;

#[derive(Clone, Debug, PartialEq)]
pub struct Transform {
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub parent: Option<EntityId>,
    pub children: Vec<EntityId>,
}

impl Transform {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parent(&self) -> Option<EntityId> {
        self.parent
    }

    pub fn children(&self) -> &[EntityId] {
        &self.children
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
            parent: None,
            children: Vec::new(),
        }
    }
}
