use unitas_core::{CoreError, Entity, EntityId, SceneGraph, SceneGraphError, Transform};

fn id_unknown_to(graph: &SceneGraph) -> EntityId {
    let mut spare = SceneGraph::new();
    loop {
        let candidate = spare.add_entity("spare");
        if graph.entity(candidate).is_none() {
            return candidate;
        }
    }
}

#[test]
fn attach_and_detach_updates_the_hierarchy() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let parent = graph.add_entity("parent");
    let child = graph.add_entity("child");

    graph.attach(child, parent)?;

    assert_eq!(graph.children(parent), &[child][..]);
    assert_eq!(graph.parent(child), Some(parent));

    graph.detach(child)?;

    assert!(graph.children(parent).is_empty());
    assert_eq!(graph.parent(child), None);
    assert_eq!(graph.roots(), vec![parent, child]);
    Ok(())
}

#[test]
fn children_keep_the_order_of_attachment() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let parent = graph.add_entity("parent");
    let first = graph.add_entity("first");
    let second = graph.add_entity("second");
    let third = graph.add_entity("third");

    graph.attach(third, parent)?;
    graph.attach(first, parent)?;
    graph.attach(second, parent)?;

    assert_eq!(graph.children(parent), &[third, first, second][..]);
    Ok(())
}

#[test]
fn attaching_again_moves_the_child() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let first_parent = graph.add_entity("first_parent");
    let second_parent = graph.add_entity("second_parent");
    let child = graph.add_entity("child");

    graph.attach(child, first_parent)?;
    graph.attach(child, second_parent)?;

    assert!(graph.children(first_parent).is_empty());
    assert_eq!(graph.children(second_parent), &[child][..]);
    assert_eq!(graph.parent(child), Some(second_parent));
    Ok(())
}

#[test]
fn traversal_orders_are_deterministic() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let root = graph.add_entity("root");
    let first = graph.add_entity("first");
    let second = graph.add_entity("second");
    let leaf = graph.add_entity("leaf");

    graph.attach(first, root)?;
    graph.attach(second, root)?;
    graph.attach(leaf, first)?;

    assert_eq!(graph.breadth_first_order(), vec![root, first, second, leaf]);
    assert_eq!(graph.depth_first_order(), vec![root, first, leaf, second]);
    assert_eq!(graph.breadth_first_order(), graph.breadth_first_order());
    assert_eq!(graph.depth_first_order(), graph.depth_first_order());
    Ok(())
}

#[test]
fn attach_rejects_unknown_entities() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let parent = graph.add_entity("parent");
    let unknown = id_unknown_to(&graph);

    assert!(matches!(
        graph.attach(unknown, parent),
        Err(SceneGraphError::UnknownEntity(id)) if id == unknown
    ));
    assert!(matches!(
        graph.attach(parent, unknown),
        Err(SceneGraphError::UnknownEntity(id)) if id == unknown
    ));
    assert!(graph.children(parent).is_empty());
    Ok(())
}

#[test]
fn attach_rejects_cycles() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let parent = graph.add_entity("parent");
    let child = graph.add_entity("child");
    let grandchild = graph.add_entity("grandchild");

    graph.attach(child, parent)?;
    graph.attach(grandchild, child)?;

    assert!(matches!(
        graph.attach(parent, child),
        Err(SceneGraphError::AttachCycle {
            child: cycle_child,
            parent: cycle_parent
        }) if cycle_child == parent && cycle_parent == child
    ));
    assert!(matches!(
        graph.attach(parent, parent),
        Err(SceneGraphError::AttachCycle { .. })
    ));
    assert!(matches!(
        graph.attach(child, grandchild),
        Err(SceneGraphError::AttachCycle { .. })
    ));
    assert_eq!(graph.children(parent), &[child][..]);
    assert_eq!(graph.children(child), &[grandchild][..]);
    assert_eq!(graph.parent(parent), None);
    Ok(())
}

#[test]
fn detach_rejects_unknown_entities() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let unknown = id_unknown_to(&graph);

    assert!(matches!(
        graph.detach(unknown),
        Err(SceneGraphError::UnknownEntity(id)) if id == unknown
    ));

    let root = graph.add_entity("root");
    graph.detach(root)?;

    assert_eq!(graph.parent(root), None);
    assert_eq!(graph.roots(), vec![root]);
    Ok(())
}

#[test]
fn entity_names_are_stored() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let id = graph.add_entity(String::from("player"));

    let entity = match graph.entity(id) {
        Some(entity) => entity,
        None => return Err(CoreError::UnknownEntity(id)),
    };

    assert_eq!(entity.id, id);
    assert_eq!(entity.name, "player");
    assert_eq!(entity.name(), "player");
    assert_eq!(entity.id(), id);
    Ok(())
}

#[test]
fn transform_holds_local_placement_data() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let id = graph.add_entity("entity");

    let transform = match graph.transform_mut(id) {
        Some(transform) => transform,
        None => return Err(CoreError::UnknownEntity(id)),
    };
    transform.translation = [1.0, 2.0, 3.0];

    let stored = match graph.transform(id) {
        Some(transform) => transform,
        None => return Err(CoreError::UnknownEntity(id)),
    };

    assert_eq!(stored.translation, [1.0, 2.0, 3.0]);
    assert_eq!(stored.rotation, [0.0; 3]);
    assert_eq!(stored.scale, [1.0; 3]);
    assert_eq!(stored.parent(), None);
    assert!(stored.children().is_empty());
    Ok(())
}

#[test]
fn create_entity_and_parent_of_children_of_queries() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    assert!(graph.is_empty());
    assert_eq!(graph.len(), 0);

    let parent = graph.create_entity("parent_node");
    let child = graph.create_entity("child_node");
    assert!(!graph.is_empty());
    assert_eq!(graph.len(), 2);
    assert!(graph.contains(parent));
    assert!(graph.contains(child));

    graph.attach(child, parent)?;

    assert_eq!(graph.parent_of(child), Some(parent));
    assert_eq!(graph.children_of(parent), &[child][..]);

    let non_existent = id_unknown_to(&graph);
    assert!(!graph.contains(non_existent));
    assert_eq!(graph.children_of(non_existent), &[][..]);
    assert_eq!(graph.parent_of(non_existent), None);
    Ok(())
}

#[test]
fn traversal_iterators_and_visitors() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let r1 = graph.create_entity("r1");
    let c1 = graph.create_entity("c1");
    let c2 = graph.create_entity("c2");
    let r2 = graph.create_entity("r2");

    graph.attach(c1, r1)?;
    graph.attach(c2, c1)?;

    let bfs_iter_collected: Vec<EntityId> = graph.iter_breadth_first().collect();
    assert_eq!(bfs_iter_collected, graph.breadth_first_order());
    assert_eq!(bfs_iter_collected, vec![r1, r2, c1, c2]);

    let dfs_iter_collected: Vec<EntityId> = graph.iter_depth_first().collect();
    assert_eq!(dfs_iter_collected, graph.depth_first_order());
    assert_eq!(dfs_iter_collected, vec![r1, c1, c2, r2]);

    let mut bfs_visited = Vec::new();
    graph.traverse_breadth_first(|id, entity, _transform| {
        bfs_visited.push((id, entity.name.clone()));
    });
    assert_eq!(
        bfs_visited,
        vec![
            (r1, String::from("r1")),
            (r2, String::from("r2")),
            (c1, String::from("c1")),
            (c2, String::from("c2")),
        ]
    );

    let mut dfs_visited = Vec::new();
    graph.traverse_depth_first(|id, entity, _transform| {
        dfs_visited.push((id, entity.name.clone()));
    });
    assert_eq!(
        dfs_visited,
        vec![
            (r1, String::from("r1")),
            (c1, String::from("c1")),
            (c2, String::from("c2")),
            (r2, String::from("r2")),
        ]
    );
    Ok(())
}

#[test]
fn entity_id_conversions_and_display() -> Result<(), CoreError> {
    let id = EntityId::new(42);
    assert_eq!(id.as_u64(), 42);
    assert_eq!(id.value(), 42);

    let from_u64: EntityId = 100.into();
    assert_eq!(from_u64.as_u64(), 100);

    let into_u64: u64 = from_u64.into();
    assert_eq!(into_u64, 100);

    let formatted = format!("{id}");
    assert_eq!(formatted, "EntityId(42)");

    let entity = Entity::new(id, "hero");
    assert_eq!(entity.id(), id);
    assert_eq!(entity.name(), "hero");

    let transform = Transform::new();
    assert_eq!(transform, Transform::default());
    Ok(())
}

#[test]
fn core_error_display_and_trait() -> Result<(), CoreError> {
    let err1 = CoreError::UnknownEntity(EntityId::new(7));
    assert_eq!(format!("{err1}"), "unknown entity EntityId(7)");

    let err2 = CoreError::AttachCycle {
        child: EntityId::new(1),
        parent: EntityId::new(2),
    };
    assert_eq!(
        format!("{err2}"),
        "attaching EntityId(1) under EntityId(2) creates a cycle"
    );

    let error_trait: &dyn std::error::Error = &err1;
    assert!(error_trait.source().is_none());
    Ok(())
}
