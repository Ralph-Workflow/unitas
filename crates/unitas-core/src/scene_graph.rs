use std::borrow::Borrow;
use std::collections::{HashMap, VecDeque};

use crate::changes::{ChangeKind, SceneChange};
use crate::entity::{Entity, EntityId};
use crate::error::CoreError;
use crate::transform::{compose_world, usable_rotation, Transform};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeState {
    Live,
    PendingDestroy,
    Vacant,
}

#[derive(Debug)]
struct Node {
    entity: Entity,
    transform: Transform,
    // Cached world transform; valid only while `world_dirty` is false.
    world: Transform,
    world_dirty: bool,
    version: u64,
    state: NodeState,
}

#[derive(Debug)]
pub struct SceneGraph {
    nodes: Vec<Node>,
    index: HashMap<EntityId, usize>,
    next_id: u64,
    pending: Vec<usize>,
    vacant: Vec<usize>,
    changes: Vec<SceneChange>,
}

pub struct BreadthFirstIter<'a> {
    graph: &'a SceneGraph,
    queue: VecDeque<EntityId>,
}

impl Iterator for BreadthFirstIter<'_> {
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

impl Iterator for DepthFirstIter<'_> {
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
            pending: Vec::new(),
            vacant: Vec::new(),
            changes: Vec::new(),
        }
    }

    pub fn create_entity(&mut self, name: &str) -> EntityId {
        self.add_entity(name)
    }

    pub fn add_entity(&mut self, name: impl Into<String>) -> EntityId {
        let id = EntityId::new(self.next_id);
        self.next_id += 1;
        self.insert_entity(id, name.into());
        id
    }

    pub fn add_entity_with_id(
        &mut self,
        id: EntityId,
        name: impl Into<String>,
    ) -> Option<EntityId> {
        if self.index.contains_key(&id) {
            return None;
        }
        self.next_id = self.next_id.max(id.as_u64().saturating_add(1));
        self.insert_entity(id, name.into());
        Some(id)
    }

    pub fn reserve_entity_id(&mut self, id: u64) {
        self.next_id = self.next_id.max(id.saturating_add(1));
    }

    pub fn shift_entity_ids(&mut self, base: u64) {
        let mut shifted = HashMap::with_capacity(self.index.len());
        for (position, node) in self.nodes.iter_mut().enumerate() {
            let new_id = EntityId::new(base + node.entity.id.as_u64());
            node.entity.id = new_id;
            if let Some(parent) = node.transform.parent {
                node.transform.parent = Some(EntityId::new(base + parent.as_u64()));
            }
            for child in &mut node.transform.children {
                *child = EntityId::new(base + child.as_u64());
            }
            if node.state == NodeState::Live {
                shifted.insert(new_id, position);
            }
        }
        self.index = shifted;
        self.next_id += base;
        for change in &mut self.changes {
            change.entity = EntityId::new(base + change.entity.as_u64());
            if let Some(parent) = change.parent {
                change.parent = Some(EntityId::new(base + parent.as_u64()));
            }
        }
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
        let entity_id = *id.borrow();
        let index = *self.index.get(&entity_id)?;
        self.record(index, ChangeKind::TransformChanged, None);
        // Pessimistic checkout marking: the caller may rewrite parent/children
        // fields too, so the whole subtree must be invalidated.
        self.mark_subtree_dirty(index);
        Some(&mut self.nodes[index].transform)
    }

    pub fn world_transform(&self, id: impl Borrow<EntityId>) -> Option<Transform> {
        let index = *self.index.get(id.borrow())?;
        if !self.nodes[index].world_dirty {
            // Steady state: invalidation always marks whole subtrees, so a clean
            // node's cache is valid and needs no ancestor walk.
            return Some(self.nodes[index].world.clone());
        }

        // Self-heal: recompose only the dirty suffix below the deepest clean
        // ancestor. The result is identical to a full brute-force recomposition.
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

        let mut base: Option<Transform> = None;
        let mut suffix: Vec<usize> = Vec::new();
        for &index in chain.iter().rev() {
            if suffix.is_empty() && !self.nodes[index].world_dirty {
                base = Some(self.nodes[index].world.clone());
            } else {
                suffix.push(index);
            }
        }

        let mut world = base;
        for &index in &suffix {
            let local = &self.nodes[index].transform;
            world = Some(match world {
                None => local_world(local),
                Some(parent_world) => compose_world(&parent_world, local),
            });
        }
        world
    }

    /// Bulk fast path for the once-per-frame tick: recomputes every dirty
    /// node's cached world, parents before children, and returns how many
    /// nodes were recomputed. Clean subtrees are untouched.
    pub fn recompute_dirty_transforms(&mut self) -> usize {
        // Roots-first traversal: a node always appears after its parent, so a
        // parent's cache is final before its children compose on top of it.
        let mut order = Vec::with_capacity(self.index.len());
        let mut stack: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.state == NodeState::Live && node.transform.parent.is_none())
            .map(|(index, _)| index)
            .collect();
        while let Some(index) = stack.pop() {
            order.push(index);
            for &child in &self.nodes[index].transform.children {
                if let Some(&child_index) = self.index.get(&child) {
                    if self.nodes[child_index].state == NodeState::Live {
                        stack.push(child_index);
                    }
                }
            }
        }

        let mut recomputed = 0;
        for &index in &order {
            if !self.nodes[index].world_dirty {
                continue;
            }
            self.nodes[index].world = self.world_from_parent(index);
            self.nodes[index].world_dirty = false;
            recomputed += 1;
        }
        recomputed
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
            .filter(|node| node.state == NodeState::Live && node.transform.parent.is_none())
            .map(|node| node.entity.id)
            .collect()
    }

    pub fn contains(&self, id: impl Borrow<EntityId>) -> bool {
        self.index.contains_key(id.borrow())
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    pub fn version(&self, id: impl Borrow<EntityId>) -> Option<u64> {
        let index = *self.index.get(id.borrow())?;
        Some(self.nodes[index].version)
    }

    pub fn take_changes(&mut self) -> Vec<SceneChange> {
        std::mem::take(&mut self.changes)
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
        let previous_parent = self.nodes[child_index].transform.parent;
        self.detach_from_parent(child);
        self.nodes[child_index].transform.parent = Some(parent);
        self.nodes[parent_index].transform.children.push(child);
        self.mark_subtree_dirty(child_index);
        if previous_parent != Some(parent) {
            self.record(child_index, ChangeKind::Reparented, Some(parent));
        }
        Ok(())
    }

    pub fn detach(&mut self, child: EntityId) -> Result<(), CoreError> {
        let index = match self.index.get(&child) {
            Some(index) => *index,
            None => return Err(CoreError::UnknownEntity(child)),
        };
        if self.nodes[index].transform.parent.is_none() {
            return Ok(());
        }
        self.detach_from_parent(child);
        self.mark_subtree_dirty(index);
        self.record(index, ChangeKind::Reparented, None);
        Ok(())
    }

    pub fn mark_destroyed(&mut self, id: impl Borrow<EntityId>) -> Result<(), CoreError> {
        let entity_id = *id.borrow();
        let root = match self.index.get(&entity_id) {
            Some(index) => *index,
            None => return Err(CoreError::UnknownEntity(entity_id)),
        };
        let mut order = Vec::new();
        let mut stack = vec![root];
        while let Some(current) = stack.pop() {
            order.push(current);
            let children = self.nodes[current].transform.children.clone();
            for child in children.iter().rev() {
                if let Some(child_index) = self.index.get(child).copied() {
                    stack.push(child_index);
                }
            }
        }
        for index in order {
            let entity = self.nodes[index].entity.id;
            self.detach_from_parent(entity);
            self.index.remove(&entity);
            self.nodes[index].state = NodeState::PendingDestroy;
            self.nodes[index].transform.children.clear();
            self.record(index, ChangeKind::Destroyed, None);
            self.pending.push(index);
        }
        Ok(())
    }

    pub fn sweep_destroyed(&mut self) -> Vec<EntityId> {
        let pending = std::mem::take(&mut self.pending);
        let mut swept = Vec::with_capacity(pending.len());
        for index in pending {
            let entity = self.nodes[index].entity.id;
            swept.push(entity);
            self.nodes[index] = Node {
                entity: Entity {
                    id: entity,
                    name: String::new(),
                },
                transform: Transform::default(),
                world: Transform::default(),
                world_dirty: true,
                version: 0,
                state: NodeState::Vacant,
            };
            self.vacant.push(index);
        }
        swept
    }

    pub fn iter_breadth_first(&self) -> BreadthFirstIter<'_> {
        let mut queue = VecDeque::new();
        for root in self.roots() {
            queue.push_back(root);
        }
        BreadthFirstIter { graph: self, queue }
    }

    pub fn iter_depth_first(&self) -> DepthFirstIter<'_> {
        let mut stack = self.roots();
        stack.reverse();
        DepthFirstIter { graph: self, stack }
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

    fn insert_entity(&mut self, id: EntityId, name: String) {
        let node = Node {
            entity: Entity { id, name },
            transform: Transform::default(),
            world: Transform::default(),
            world_dirty: true,
            version: 0,
            state: NodeState::Live,
        };
        let index = match self.vacant.pop() {
            Some(slot) => {
                self.nodes[slot] = node;
                slot
            }
            None => {
                self.nodes.push(node);
                self.nodes.len() - 1
            }
        };
        self.index.insert(id, index);
        self.changes.push(SceneChange {
            entity: id,
            kind: ChangeKind::Created,
            version: 0,
            parent: None,
        });
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

    fn mark_subtree_dirty(&mut self, root: usize) {
        let mut stack = vec![root];
        while let Some(index) = stack.pop() {
            self.nodes[index].world_dirty = true;
            for &child in &self.nodes[index].transform.children {
                if let Some(&child_index) = self.index.get(&child) {
                    stack.push(child_index);
                }
            }
        }
    }

    fn world_from_parent(&self, index: usize) -> Transform {
        match self.nodes[index]
            .transform
            .parent
            .and_then(|parent| self.index.get(&parent).copied())
        {
            Some(parent_index) => compose_world(&self.nodes[parent_index].world, &self.nodes[index].transform),
            None => local_world(&self.nodes[index].transform),
        }
    }

    fn record(&mut self, index: usize, kind: ChangeKind, parent: Option<EntityId>) {
        let node = &mut self.nodes[index];
        node.version += 1;
        let version = node.version;
        let entity = node.entity.id;
        self.changes.push(SceneChange {
            entity,
            kind,
            version,
            parent,
        });
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

fn local_world(local: &Transform) -> Transform {
    Transform {
        translation: local.translation,
        rotation: usable_rotation(local.rotation),
        scale: local.scale,
        parent: None,
        children: Vec::new(),
    }
}
