use std::borrow::Borrow;
use std::collections::{HashMap, VecDeque};

use crate::entity::{Entity, EntityId};
use crate::error::CoreError;
use crate::transform::{compose_world, usable_rotation, Transform};

#[derive(Debug)]
struct Node {
    entity: Entity,
    transform: Transform,
}

#[derive(Debug)]
pub struct SceneGraph {
    nodes: Vec<Node>,
    index: HashMap<EntityId, usize>,
    next_id: u64,
}

pub struct BreadthFirstIter<'a> {
    graph: &'a SceneGraph,
    queue: VecDeque<EntityId>,
}

impl<'a> Iterator for BreadthFirstIter<'a> {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.queue.pop_front()?;
        for child in self.graph.children(id) {
            self.queue.push_back(*child);
        }
        Some(id)
    }
}

pub struct DepthFirstIter<'a> {
    graph: &'a SceneGraph,
    stack: Vec<EntityId>,
}

impl<'a> Iterator for DepthFirstIter<'a> {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.stack.pop()?;
        for child in self.graph.children(id).iter().rev() {
            self.stack.push(*child);
        }
        Some(id)
    }
}

impl SceneGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            index: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn create_entity(&mut self, name: &str) -> EntityId {
        self.add_entity(name)
    }

    pub fn add_entity(&mut self, name: impl Into<String>) -> EntityId {
        let id = EntityId::new(self.next_id);
        self.next_id += 1;
        self.index.insert(id, self.nodes.len());
        self.nodes.push(Node {
            entity: Entity {
                id,
                name: name.into(),
            },
            transform: Transform::default(),
        });
        id
    }

    pub fn shift_entity_ids(&mut self, base: u64) {
        let mut shifted = HashMap::with_capacity(self.index.len());
        for (position, node) in self.nodes.iter_mut().enumerate() {
            let new_id = EntityId::new(base + node.entity.id.as_u64());
            node.entity.id = new_id;
            if let Some(parent) = node.transform.parent {
                node.transform.parent = Some(EntityId::new(base + parent.as_u64()));
            }
            for child in node.transform.children.iter_mut() {
                *child = EntityId::new(base + child.as_u64());
            }
            shifted.insert(new_id, position);
        }
        self.index = shifted;
        self.next_id += base;
    }

    pub fn entity(&self, id: impl Borrow<EntityId>) -> Option<&Entity> {
        let index = self.index.get(id.borrow())?;
        Some(&self.nodes[*index].entity)
    }

    pub fn entity_mut(&mut self, id: impl Borrow<EntityId>) -> Option<&mut Entity> {
        let index = self.index.get(id.borrow())?;
        Some(&mut self.nodes[*index].entity)
    }

    pub fn transform(&self, id: impl Borrow<EntityId>) -> Option<&Transform> {
        let index = self.index.get(id.borrow())?;
        Some(&self.nodes[*index].transform)
    }

    pub fn transform_mut(&mut self, id: impl Borrow<EntityId>) -> Option<&mut Transform> {
        let index = self.index.get(id.borrow())?;
        Some(&mut self.nodes[*index].transform)
    }

    pub fn world_transform(&self, id: impl Borrow<EntityId>) -> Option<Transform> {
        let mut chain: Vec<usize> = Vec::new();
        let mut current = *id.borrow();
        loop {
            let index = *self.index.get(&current)?;
            chain.push(index);
            match self.nodes[index].transform.parent {
                Some(parent) => current = parent,
                None => break,
            }
            if chain.len() > self.nodes.len() {
                return None;
            }
        }

        let mut world: Option<Transform> = None;
        for index in chain.into_iter().rev() {
            let local = &self.nodes[index].transform;
            world = Some(match world {
                None => Transform {
                    translation: local.translation,
                    rotation: usable_rotation(local.rotation),
                    scale: local.scale,
                    parent: None,
                    children: Vec::new(),
                },
                Some(parent_world) => compose_world(&parent_world, local),
            });
        }
        world
    }

    pub fn parent(&self, id: impl Borrow<EntityId>) -> Option<EntityId> {
        self.transform(id)?.parent
    }

    pub fn parent_of(&self, id: impl Borrow<EntityId>) -> Option<EntityId> {
        self.parent(id)
    }

    pub fn children(&self, id: impl Borrow<EntityId>) -> &[EntityId] {
        match self.index.get(id.borrow()) {
            Some(index) => &self.nodes[*index].transform.children,
            None => &[],
        }
    }

    pub fn children_of(&self, id: impl Borrow<EntityId>) -> &[EntityId] {
        self.children(id)
    }

    pub fn roots(&self) -> Vec<EntityId> {
        self.nodes
            .iter()
            .filter(|node| node.transform.parent.is_none())
            .map(|node| node.entity.id)
            .collect()
    }

    pub fn contains(&self, id: impl Borrow<EntityId>) -> bool {
        self.index.contains_key(id.borrow())
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn attach(&mut self, child: EntityId, parent: EntityId) -> Result<(), CoreError> {
        let child_index = match self.index.get(&child) {
            Some(index) => *index,
            None => return Err(CoreError::UnknownEntity(child)),
        };
        let parent_index = match self.index.get(&parent) {
            Some(index) => *index,
            None => return Err(CoreError::UnknownEntity(parent)),
        };
        if self.creates_cycle(child, parent) {
            return Err(CoreError::AttachCycle { child, parent });
        }
        self.detach_from_parent(child);
        self.nodes[child_index].transform.parent = Some(parent);
        self.nodes[parent_index].transform.children.push(child);
        Ok(())
    }

    pub fn detach(&mut self, child: EntityId) -> Result<(), CoreError> {
        if !self.index.contains_key(&child) {
            return Err(CoreError::UnknownEntity(child));
        }
        self.detach_from_parent(child);
        Ok(())
    }

    pub fn iter_breadth_first(&self) -> BreadthFirstIter<'_> {
        let mut queue = VecDeque::new();
        for root in self.roots() {
            queue.push_back(root);
        }
        BreadthFirstIter {
            graph: self,
            queue,
        }
    }

    pub fn iter_depth_first(&self) -> DepthFirstIter<'_> {
        let mut stack = self.roots();
        stack.reverse();
        DepthFirstIter {
            graph: self,
            stack,
        }
    }

    pub fn breadth_first_order(&self) -> Vec<EntityId> {
        self.iter_breadth_first().collect()
    }

    pub fn depth_first_order(&self) -> Vec<EntityId> {
        self.iter_depth_first().collect()
    }

    pub fn traverse_breadth_first<F>(&self, mut visitor: F)
    where
        F: FnMut(EntityId, &Entity, &Transform),
    {
        for id in self.iter_breadth_first() {
            if let (Some(entity), Some(transform)) = (self.entity(id), self.transform(id)) {
                visitor(id, entity, transform);
            }
        }
    }

    pub fn traverse_depth_first<F>(&self, mut visitor: F)
    where
        F: FnMut(EntityId, &Entity, &Transform),
    {
        for id in self.iter_depth_first() {
            if let (Some(entity), Some(transform)) = (self.entity(id), self.transform(id)) {
                visitor(id, entity, transform);
            }
        }
    }

    fn detach_from_parent(&mut self, child: EntityId) {
        let parent = match self.index.get(&child) {
            Some(index) => self.nodes[*index].transform.parent,
            None => return,
        };
        let parent = match parent {
            Some(parent) => parent,
            None => return,
        };
        if let Some(index) = self.index.get(&parent) {
            self.nodes[*index]
                .transform
                .children
                .retain(|candidate| *candidate != child);
        }
        if let Some(index) = self.index.get(&child) {
            self.nodes[*index].transform.parent = None;
        }
    }

    fn creates_cycle(&self, child: EntityId, parent: EntityId) -> bool {
        let mut current = parent;
        loop {
            if current == child {
                return true;
            }
            match self.parent(current) {
                Some(ancestor) => current = ancestor,
                None => return false,
            }
        }
    }
}

impl Default for SceneGraph {
    fn default() -> Self {
        Self::new()
    }
}
