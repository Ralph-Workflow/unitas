use unitas_core::{EntityId, Transform};

use crate::Runtime;

impl Runtime {
    pub fn world_transform_of(&self, entity: EntityId) -> Option<Transform> {
        self.entity_transform(entity)
    }

    pub fn find_entity_by_name(&self, name: &str) -> Option<EntityId> {
        let graph = self.scene_graph()?;
        for id in graph.iter_breadth_first() {
            if let Some(entity) = graph.entity(id) {
                if entity.name == name {
                    return Some(id);
                }
            }
        }
        None
    }
}
