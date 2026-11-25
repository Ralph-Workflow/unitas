use std::f32::consts::FRAC_1_SQRT_2;

use unitas_core::transform::{
    compose_world, multiply_rotation, normalize_rotation, rotate_vector, IDENTITY_ROTATION,
};
use unitas_core::{CoreError, Entity, EntityId, SceneGraph, SceneGraphError, Transform};

const TOLERANCE: f32 = 1e-6;
const QUARTER_TURN_Z: [f32; 4] = [0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2];
const HALF_TURN_X: [f32; 4] = [1.0, 0.0, 0.0, 0.0];
const HALF_TURN_Y: [f32; 4] = [0.0, 1.0, 0.0, 0.0];
const HALF_TURN_Z: [f32; 4] = [0.0, 0.0, 1.0, 0.0];

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
    assert_eq!(stored.rotation, [0.0, 0.0, 0.0, 1.0]);
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

fn set_transform(
    graph: &mut SceneGraph,
    id: EntityId,
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
) -> Result<(), CoreError> {
    let transform = match graph.transform_mut(id) {
        Some(transform) => transform,
        None => return Err(CoreError::UnknownEntity(id)),
    };
    transform.translation = translation;
    transform.rotation = rotation;
    transform.scale = scale;
    Ok(())
}

fn world_of(graph: &SceneGraph, id: EntityId) -> Result<Transform, CoreError> {
    match graph.world_transform(id) {
        Some(world) => Ok(world),
        None => Err(CoreError::UnknownEntity(id)),
    }
}

fn assert_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected.iter()).enumerate() {
        assert!(
            (actual - expected).abs() <= TOLERANCE,
            "component {index}: {actual} is not within {TOLERANCE} of {expected}"
        );
    }
}

#[test]
fn world_transform_of_a_root_equals_its_local_transform() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let root = graph.add_entity("root");
    set_transform(
        &mut graph,
        root,
        [1.0, 2.0, 3.0],
        QUARTER_TURN_Z,
        [2.0, 3.0, 4.0],
    )?;

    let local = match graph.transform(root) {
        Some(transform) => transform.clone(),
        None => return Err(CoreError::UnknownEntity(root)),
    };

    assert_eq!(graph.world_transform(root), Some(local));
    Ok(())
}

#[test]
fn child_world_translation_follows_the_parent_rotation_and_scale() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let parent = graph.add_entity("parent");
    let child = graph.add_entity("child");
    graph.attach(child, parent)?;
    set_transform(
        &mut graph,
        parent,
        [10.0, 20.0, 30.0],
        QUARTER_TURN_Z,
        [2.0, 3.0, 4.0],
    )?;
    set_transform(
        &mut graph,
        child,
        [1.0, 1.0, 1.0],
        IDENTITY_ROTATION,
        [1.0, 1.0, 1.0],
    )?;

    let world = world_of(&graph, child)?;

    assert_close(&world.translation, &[7.0, 22.0, 34.0]);
    assert_eq!(world.rotation, QUARTER_TURN_Z);
    assert_eq!(world.scale, [2.0, 3.0, 4.0]);
    assert_eq!(world.parent, None);
    assert!(world.children.is_empty());
    Ok(())
}

#[test]
fn child_under_a_quarter_turn_parent_lands_at_the_rotated_offset() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let parent = graph.add_entity("parent");
    let child = graph.add_entity("child");
    graph.attach(child, parent)?;
    set_transform(
        &mut graph,
        parent,
        [1.0, 2.0, 0.0],
        QUARTER_TURN_Z,
        [1.0, 1.0, 1.0],
    )?;
    set_transform(
        &mut graph,
        child,
        [1.0, 0.0, 0.0],
        IDENTITY_ROTATION,
        [1.0, 1.0, 1.0],
    )?;

    let parent_world = world_of(&graph, parent)?;
    let child_world = world_of(&graph, child)?;

    assert_close(&child_world.translation, &[1.0, 3.0, 0.0]);
    assert_eq!(parent_world.rotation, QUARTER_TURN_Z);
    assert_eq!(child_world.rotation, parent_world.rotation);
    assert_eq!(child_world.scale, [1.0, 1.0, 1.0]);
    Ok(())
}

#[test]
fn world_transform_composes_through_the_whole_chain() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let parent = graph.add_entity("parent");
    let child = graph.add_entity("child");
    let grandchild = graph.add_entity("grandchild");
    graph.attach(child, parent)?;
    graph.attach(grandchild, child)?;
    set_transform(
        &mut graph,
        parent,
        [1.0, 2.0, 0.0],
        HALF_TURN_Z,
        [1.0, 1.0, 1.0],
    )?;
    set_transform(
        &mut graph,
        child,
        [1.0, 0.0, 0.0],
        HALF_TURN_X,
        [1.0, 1.0, 1.0],
    )?;
    set_transform(
        &mut graph,
        grandchild,
        [1.0, 0.0, 0.0],
        IDENTITY_ROTATION,
        [1.0, 1.0, 1.0],
    )?;

    let child_world = world_of(&graph, child)?;
    let grandchild_world = world_of(&graph, grandchild)?;

    assert_eq!(child_world.translation, [0.0, 2.0, 0.0]);
    assert_eq!(child_world.rotation, HALF_TURN_Y);
    assert_eq!(grandchild_world.translation, [-1.0, 2.0, 0.0]);
    assert_eq!(grandchild_world.rotation, HALF_TURN_Y);
    assert_eq!(grandchild_world.scale, [1.0, 1.0, 1.0]);
    Ok(())
}

#[test]
fn child_under_a_non_uniformly_scaled_parent_uses_the_parent_scale() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let parent = graph.add_entity("parent");
    let child = graph.add_entity("child");
    graph.attach(child, parent)?;
    set_transform(
        &mut graph,
        parent,
        [1.0, 1.0, 1.0],
        IDENTITY_ROTATION,
        [2.0, 3.0, 4.0],
    )?;
    set_transform(
        &mut graph,
        child,
        [1.0, 1.0, 1.0],
        IDENTITY_ROTATION,
        [2.0, 2.0, 2.0],
    )?;

    let world = world_of(&graph, child)?;

    assert_eq!(world.translation, [3.0, 4.0, 5.0]);
    assert_eq!(world.scale, [4.0, 6.0, 8.0]);
    Ok(())
}

#[test]
fn a_degenerate_rotation_falls_back_to_the_identity_rotation() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let parent = graph.add_entity("parent");
    let child = graph.add_entity("child");
    graph.attach(child, parent)?;
    set_transform(
        &mut graph,
        parent,
        [1.0, 0.0, 0.0],
        IDENTITY_ROTATION,
        [1.0, 1.0, 1.0],
    )?;
    set_transform(
        &mut graph,
        child,
        [0.0, 2.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
    )?;

    let child_world = world_of(&graph, child)?;
    assert_eq!(child_world.rotation, IDENTITY_ROTATION);
    assert_eq!(child_world.translation, [1.0, 2.0, 0.0]);

    let root = graph.add_entity("root");
    set_transform(
        &mut graph,
        root,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
    )?;
    assert_eq!(world_of(&graph, root)?.rotation, IDENTITY_ROTATION);
    Ok(())
}

#[test]
fn a_non_unit_rotation_is_normalized_before_it_rotates() -> Result<(), CoreError> {
    let mut graph = SceneGraph::new();
    let entity = graph.add_entity("entity");
    set_transform(
        &mut graph,
        entity,
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
    )?;

    let world = match graph.world_transform(entity) {
        Some(world) => world,
        None => return Err(CoreError::UnknownEntity(entity)),
    };

    assert_close(
        &world.rotation,
        &[0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2],
    );
    assert_close(
        &rotate_vector([0.0, 0.0, 1.0, 1.0], [1.0, 0.0, 0.0]),
        &[0.0, 1.0, 0.0],
    );
    Ok(())
}

#[test]
fn world_transform_of_an_unknown_entity_is_none() -> Result<(), CoreError> {
    let graph = SceneGraph::new();
    let unknown = id_unknown_to(&graph);

    assert!(graph.world_transform(unknown).is_none());
    Ok(())
}

#[test]
fn rotation_helpers_follow_the_quaternion_contract() {
    assert_eq!(normalize_rotation([0.0, 0.0, 0.0, 0.0]), IDENTITY_ROTATION);
    assert_eq!(
        normalize_rotation([0.0, 0.0, 0.0, 3.0]),
        [0.0, 0.0, 0.0, 1.0]
    );
    assert_eq!(
        multiply_rotation(IDENTITY_ROTATION, QUARTER_TURN_Z),
        QUARTER_TURN_Z
    );
    assert_eq!(multiply_rotation(HALF_TURN_Z, HALF_TURN_X), HALF_TURN_Y);
    assert_eq!(
        rotate_vector(IDENTITY_ROTATION, [1.0, 2.0, 3.0]),
        [1.0, 2.0, 3.0]
    );
    assert_eq!(
        rotate_vector(HALF_TURN_Z, [1.0, 0.0, 0.0]),
        [-1.0, 0.0, 0.0]
    );
    assert_eq!(
        rotate_vector([0.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        [1.0, 0.0, 0.0]
    );
}

#[test]
fn compose_world_uses_the_unity_trs_order() {
    let parent = Transform {
        translation: [1.0, 2.0, 0.0],
        rotation: QUARTER_TURN_Z,
        scale: [1.0, 1.0, 1.0],
        parent: None,
        children: Vec::new(),
    };
    let local = Transform {
        translation: [1.0, 0.0, 0.0],
        rotation: IDENTITY_ROTATION,
        scale: [1.0, 1.0, 1.0],
        parent: None,
        children: Vec::new(),
    };

    let world = compose_world(&parent, &local);

    assert_close(&world.translation, &[1.0, 3.0, 0.0]);
    assert_eq!(world.rotation, QUARTER_TURN_Z);
    assert_eq!(world.scale, [1.0, 1.0, 1.0]);
    assert_eq!(world.parent, None);
    assert!(world.children.is_empty());
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

#[test]
fn scene_graph_shift_entity_ids_moves_keys_ids_and_links() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = SceneGraph::new();
    let root = graph.add_entity("Root");
    let child = graph.add_entity("Child");
    graph.attach(child, root)?;

    graph.shift_entity_ids(10);

    let shifted_root = EntityId::new(10);
    let shifted_child = EntityId::new(11);
    assert!(graph.entity(root).is_none());
    assert!(graph.entity(child).is_none());
    assert_eq!(
        graph.entity(shifted_root).map(|entity| entity.name.clone()),
        Some(String::from("Root"))
    );
    assert_eq!(
        graph.entity(shifted_child).map(|entity| entity.name.clone()),
        Some(String::from("Child"))
    );
    assert_eq!(graph.parent_of(shifted_child), Some(shifted_root));
    assert_eq!(graph.children_of(shifted_root), &[shifted_child][..]);
    assert_eq!(graph.breadth_first_order(), vec![shifted_root, shifted_child]);
    let added = graph.add_entity("Added");
    assert!(added.as_u64() > shifted_child.as_u64());
    Ok(())
}
