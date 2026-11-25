use std::time::{Duration, Instant};

use unitas_core::{ChangeKind, CoreError, EntityId, SceneGraph};

const SCALE_NODES: usize = 100_000;
const SCALE_LOOKUPS: usize = 10_000;
const SCALE_BUDGET: Duration = Duration::from_secs(5);
const LOOKUP_BUDGET: Duration = Duration::from_millis(500);

#[test]
fn marking_destroyed_rejects_lookups_through_the_stale_handle() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let id = graph.add_entity("doomed");

    graph.mark_destroyed(id)?;

    assert!(graph.entity(id).is_none());
    assert!(graph.transform(id).is_none());
    assert!(!graph.contains(id));
    assert!(graph.world_transform(id).is_none());
    assert!(graph.parent(id).is_none());
    assert!(graph.children(id).is_empty());
    assert!(graph.version(id).is_none());
    assert_eq!(graph.len(), 0);
    Ok(())
}

#[test]
fn marking_destroyed_destroys_the_children_with_their_parent() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let root = graph.add_entity("root");
    let child = graph.add_entity("child");
    let grandchild = graph.add_entity("grandchild");
    let survivor = graph.add_entity("survivor");
    graph.attach(child, root)?;
    graph.attach(grandchild, child)?;

    graph.mark_destroyed(root)?;

    assert!(!graph.contains(root));
    assert!(!graph.contains(child));
    assert!(!graph.contains(grandchild));
    assert!(graph.contains(survivor));
    assert!(graph.children(root).is_empty());
    assert_eq!(graph.len(), 1);
    assert_eq!(graph.roots(), vec![survivor]);
    Ok(())
}

#[test]
fn marking_destroyed_rejects_a_handle_it_already_destroyed() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let id = graph.add_entity("doomed");
    graph.mark_destroyed(id)?;

    assert_eq!(graph.mark_destroyed(id), Err(CoreError::UnknownEntity(id)));
    Ok(())
}

#[test]
fn marking_destroyed_rejects_an_unknown_entity() {
    let mut graph = SceneGraph::new();
    let unknown = EntityId::new(4_242);

    assert_eq!(
        graph.mark_destroyed(unknown),
        Err(CoreError::UnknownEntity(unknown))
    );
}

#[test]
fn hierarchy_operations_reject_a_destroyed_handle() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let doomed = graph.add_entity("doomed");
    let other = graph.add_entity("other");
    graph.mark_destroyed(doomed)?;

    assert_eq!(
        graph.attach(doomed, other),
        Err(CoreError::UnknownEntity(doomed))
    );
    assert_eq!(graph.detach(doomed), Err(CoreError::UnknownEntity(doomed)));
    Ok(())
}

#[test]
fn sweeping_reports_the_destroyed_entities_in_destruction_order() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let root = graph.add_entity("root");
    let child = graph.add_entity("child");
    graph.attach(child, root)?;
    graph.mark_destroyed(root)?;

    let swept = graph.sweep_destroyed();

    assert_eq!(swept, vec![root, child]);
    assert_eq!(graph.len(), 0);
    assert!(!graph.contains(root));
    assert!(!graph.contains(child));
    Ok(())
}

#[test]
fn sweeping_an_untouched_graph_reports_nothing() {
    let mut graph = SceneGraph::new();
    graph.add_entity("node");

    assert!(graph.sweep_destroyed().is_empty());
}

#[test]
fn a_destroyed_identity_is_never_reissued() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let first = graph.add_entity("first");
    graph.mark_destroyed(first)?;
    graph.sweep_destroyed();

    let second = graph.add_entity("second");

    assert_ne!(first, second);
    assert!(!graph.contains(first));
    assert!(graph.contains(second));
    assert_eq!(
        graph.entity(second).map(|entity| entity.name()),
        Some("second")
    );
    Ok(())
}

#[test]
fn taking_changes_reports_creation_once() {
    let mut graph = SceneGraph::new();
    let id = graph.add_entity("node");

    let changes = graph.take_changes();

    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].kind, ChangeKind::Created);
    assert_eq!(changes[0].entity, id);
    assert_eq!(changes[0].version, 0);
    assert!(graph.take_changes().is_empty());
}

#[test]
fn transform_edits_record_a_change_and_advance_the_version() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let id = graph.add_entity("node");
    let _ = graph.take_changes();

    assert_eq!(graph.version(id), Some(0));
    match graph.transform_mut(id) {
        Some(transform) => transform.translation = [1.0, 2.0, 3.0],
        None => return Err(CoreError::UnknownEntity(id)),
    }

    let changes = graph.take_changes();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].kind, ChangeKind::TransformChanged);
    assert_eq!(changes[0].entity, id);
    assert_eq!(changes[0].version, 1);
    assert_eq!(graph.version(id), Some(1));
    Ok(())
}

#[test]
fn hierarchy_operations_record_reparenting() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let parent = graph.add_entity("parent");
    let child = graph.add_entity("child");
    let _ = graph.take_changes();

    graph.attach(child, parent)?;
    let changes = graph.take_changes();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].kind, ChangeKind::Reparented);
    assert_eq!(changes[0].entity, child);
    assert_eq!(changes[0].parent, Some(parent));
    assert_eq!(graph.version(child), Some(1));
    assert_eq!(graph.version(parent), Some(0));

    graph.detach(child)?;
    let changes = graph.take_changes();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].kind, ChangeKind::Reparented);
    assert_eq!(changes[0].entity, child);
    assert_eq!(changes[0].parent, None);
    assert_eq!(graph.version(child), Some(2));
    Ok(())
}

#[test]
fn destroying_records_a_destroyed_change_for_the_whole_subtree() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let root = graph.add_entity("root");
    let child = graph.add_entity("child");
    graph.attach(child, root)?;
    let _ = graph.take_changes();

    graph.mark_destroyed(root)?;

    let changes = graph.take_changes();
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[0].kind, ChangeKind::Destroyed);
    assert_eq!(changes[0].entity, root);
    assert_eq!(changes[1].kind, ChangeKind::Destroyed);
    assert_eq!(changes[1].entity, child);
    Ok(())
}

#[test]
fn a_subsystem_can_consume_the_change_log_without_polling() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let parent = graph.add_entity("parent");
    let child = graph.add_entity("child");
    graph.attach(child, parent)?;

    let mut live = Vec::new();
    let mut parents = Vec::new();
    for change in graph.take_changes() {
        match change.kind {
            ChangeKind::Created => live.push(change.entity),
            ChangeKind::Destroyed => live.retain(|entity| *entity != change.entity),
            ChangeKind::Reparented => parents.push((change.entity, change.parent)),
            ChangeKind::TransformChanged => {}
        }
    }

    assert_eq!(live, vec![parent, child]);
    assert_eq!(parents, vec![(child, Some(parent))]);
    Ok(())
}

#[test]
fn shifting_identities_keeps_the_change_log_consistent() {
    let mut graph = SceneGraph::new();
    let id = graph.add_entity("node");

    graph.shift_entity_ids(1_000);

    let changes = graph.take_changes();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].entity, EntityId::new(1_000));
    assert!(graph.contains(EntityId::new(1_000)));
    assert!(!graph.contains(id));
}

#[test]
fn a_hundred_thousand_nodes_build_look_up_iterate_and_destroy_within_the_budget(
) -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let started = Instant::now();

    let mut ids = Vec::with_capacity(SCALE_NODES);
    for _ in 0..SCALE_NODES {
        ids.push(graph.add_entity("node"));
    }
    let root = ids[0];
    for id in ids.iter().skip(1) {
        graph.attach(*id, root)?;
    }
    let built = started.elapsed();

    let lookup_started = Instant::now();
    let mut found = 0usize;
    for step in 0..SCALE_LOOKUPS {
        let index = (step * 7_919) % SCALE_NODES;
        if graph.entity(ids[index]).is_some() {
            found += 1;
        }
    }
    let lookup_elapsed = lookup_started.elapsed();

    let mut visited = 0usize;
    let mut links = 0usize;
    graph.traverse_breadth_first(|_id, _entity, transform| {
        visited += 1;
        links += transform.children.len();
    });

    let destroyed: Vec<EntityId> = ids.iter().skip(1).step_by(1_000).copied().collect();
    for id in &destroyed {
        graph.mark_destroyed(*id)?;
    }
    let swept = graph.sweep_destroyed();

    let changes = graph.take_changes();
    let elapsed = started.elapsed();

    assert_eq!(found, SCALE_LOOKUPS);
    assert_eq!(visited, SCALE_NODES);
    assert_eq!(links, SCALE_NODES - 1);
    assert_eq!(swept.len(), destroyed.len());
    assert_eq!(graph.len(), SCALE_NODES - destroyed.len());
    assert_eq!(
        changes.len(),
        SCALE_NODES + (SCALE_NODES - 1) + destroyed.len()
    );
    assert!(
        built < SCALE_BUDGET,
        "building {SCALE_NODES} nodes took {built:?}"
    );
    assert!(
        lookup_elapsed < LOOKUP_BUDGET,
        "{SCALE_LOOKUPS} lookups took {lookup_elapsed:?}"
    );
    assert!(elapsed < SCALE_BUDGET, "the scale run took {elapsed:?}");
    Ok(())
}
