use unitas_core::{compose_world, EntityId, SceneGraph, Transform};

// Brute-force reference: recomposes the ancestor chain from local transforms on
// every call, mirroring the pre-cache semantics. Rotations in these tests are
// all unit quaternions, so no normalization path is exercised here (that path
// is covered by the existing core tests).
fn brute_force_world(graph: &SceneGraph, id: EntityId) -> Option<Transform> {
    let mut chain = Vec::new();
    let mut current = id;
    loop {
        chain.push(current);
        match graph.parent(current) {
            Some(parent) => current = parent,
            None => break,
        }
        if chain.len() > graph.len() + 1 {
            return None;
        }
    }
    let mut world: Option<Transform> = None;
    for &entity in chain.iter().rev() {
        let local = graph.transform(entity)?;
        world = Some(match world {
            None => Transform {
                translation: local.translation,
                rotation: local.rotation,
                scale: local.scale,
                parent: None,
                children: Vec::new(),
            },
            Some(parent_world) => compose_world(&parent_world, local),
        });
    }
    world
}

fn set_transform(
    graph: &mut SceneGraph,
    id: EntityId,
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
) {
    let transform = graph.transform_mut(id).expect("entity must exist");
    transform.translation = translation;
    transform.rotation = rotation;
    transform.scale = scale;
}

// A quarter turn about Z: unit quaternion.
const QUARTER_TURN_Z: [f32; 4] = [0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2];
const FRAC_1_SQRT_2: f32 = std::f32::consts::FRAC_1_SQRT_2;
const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

fn assert_worlds_match(graph: &SceneGraph, ids: &[EntityId]) {
    for &id in ids {
        if !graph.contains(id) {
            assert!(graph.world_transform(id).is_none());
            continue;
        }
        let expected = brute_force_world(graph, id).expect("brute force must resolve live ids");
        let actual = graph.world_transform(id).expect("cache read must resolve live ids");
        assert_eq!(
            actual.translation, expected.translation,
            "translation mismatch for {id:?}"
        );
        assert_eq!(
            actual.rotation, expected.rotation,
            "rotation mismatch for {id:?}"
        );
        assert_eq!(
            actual.scale, expected.scale,
            "scale mismatch for {id:?}"
        );
        assert!(actual.parent.is_none() && actual.children.is_empty());
    }
}

#[test]
fn cache_matches_brute_force_after_every_mutation_kind() {
    let mut graph = SceneGraph::new();
    let root = graph.add_entity("root");
    let a = graph.add_entity("a");
    let b = graph.add_entity("b");
    let c = graph.add_entity("c");
    let other = graph.add_entity("other");
    graph.attach(a, root).unwrap();
    graph.attach(b, a).unwrap();
    graph.attach(c, root).unwrap();
    set_transform(&mut graph, root, [1.0, 2.0, 3.0], QUARTER_TURN_Z, [1.5, 1.0, 1.0]);
    set_transform(&mut graph, a, [0.5, -1.0, 2.0], IDENTITY, [1.0, 2.0, 1.0]);
    set_transform(&mut graph, b, [2.0, 0.0, -1.0], QUARTER_TURN_Z, [1.0, 1.0, 1.0]);
    set_transform(&mut graph, c, [-3.0, 1.0, 0.0], IDENTITY, [2.0, 1.0, 1.0]);
    set_transform(&mut graph, other, [10.0, 0.0, 0.0], IDENTITY, [1.0, 1.0, 1.0]);
    let mut live = vec![root, a, b, c, other];

    // Initial state: every node starts dirty.
    assert_eq!(graph.recompute_dirty_transforms(), live.len());
    assert_worlds_match(&graph, &live);

    // transform_mut on one node invalidates only its subtree (a, b).
    set_transform(&mut graph, a, [0.0, 4.0, 0.5], IDENTITY, [1.0, 1.0, 3.0]);
    assert_worlds_match(&graph, &live); // self-heal read without recompute
    assert_eq!(graph.recompute_dirty_transforms(), 2);
    assert_worlds_match(&graph, &live);

    // attach (new node under a dirty-checked parent).
    let d = graph.add_entity("d");
    set_transform(&mut graph, d, [1.0, 1.0, 1.0], IDENTITY, [1.0, 1.0, 1.0]);
    graph.attach(d, b).unwrap();
    live.push(d);
    assert_worlds_match(&graph, &live);
    assert_eq!(graph.recompute_dirty_transforms(), 1);
    assert_worlds_match(&graph, &live);

    // detach.
    graph.detach(c).unwrap();
    assert_worlds_match(&graph, &live);
    assert_eq!(graph.recompute_dirty_transforms(), 1);
    assert_worlds_match(&graph, &live);

    // reparent (move the subtree rooted at a under other).
    graph.attach(a, other).unwrap();
    assert_worlds_match(&graph, &live);
    assert_eq!(graph.recompute_dirty_transforms(), 3); // a, b, d
    assert_worlds_match(&graph, &live);

    // create (fresh entity is dirty but reads correctly immediately).
    let e = graph.add_entity("e");
    set_transform(&mut graph, e, [-1.0, -1.0, 5.0], IDENTITY, [1.0, 1.0, 1.0]);
    live.push(e);
    assert_worlds_match(&graph, &live);
    assert_eq!(graph.recompute_dirty_transforms(), 1); // e only

    // destroy + sweep (subtree rooted at a goes away).
    graph.mark_destroyed(a).unwrap();
    let swept = graph.sweep_destroyed();
    assert_eq!(swept.len(), 3); // a, b, d
    assert_worlds_match(&graph, &live);
    assert_eq!(graph.recompute_dirty_transforms(), 0);
    assert_worlds_match(&graph, &live);

    // Destroyed ids must read as None.
    for destroyed in [a, b, d] {
        assert!(graph.world_transform(destroyed).is_none());
    }
}

#[test]
fn cache_matches_brute_force_on_a_deep_chain() {
    const DEPTH: usize = 64; // well beyond the 32-level concern
    let mut graph = SceneGraph::new();
    let mut ids = Vec::with_capacity(DEPTH);
    let mut parent: Option<EntityId> = None;
    for level in 0..DEPTH {
        let id = graph.add_entity(format!("node_{level}"));
        if let Some(parent) = parent {
            graph.attach(id, parent).unwrap();
        }
        set_transform(
            &mut graph,
            id,
            [level as f32, (level as f32) * 0.5, -(level as f32)],
            if level % 3 == 0 { QUARTER_TURN_Z } else { IDENTITY },
            [1.0 + level as f32 * 0.1, 1.0, 1.0],
        );
        ids.push(id);
        parent = Some(id);
    }

    assert_eq!(graph.recompute_dirty_transforms(), DEPTH);
    assert_worlds_match(&graph, &ids);

    // Mutate a node mid-chain without recomputing: every read must self-heal.
    let mid = ids[DEPTH / 2];
    set_transform(&mut graph, mid, [100.0, -50.0, 25.0], QUARTER_TURN_Z, [2.0, 2.0, 2.0]);
    assert_worlds_match(&graph, &ids);
    assert_eq!(graph.recompute_dirty_transforms(), DEPTH - DEPTH / 2);
    assert_worlds_match(&graph, &ids);
}

#[test]
fn recompute_touches_only_the_dirty_subtree() {
    let mut graph = SceneGraph::new();
    let root = graph.add_entity("root");
    let left = graph.add_entity("left");
    let right = graph.add_entity("right");
    let left_child = graph.add_entity("left_child");
    let right_child = graph.add_entity("right_child");
    graph.attach(left, root).unwrap();
    graph.attach(right, root).unwrap();
    graph.attach(left_child, left).unwrap();
    graph.attach(right_child, right).unwrap();
    let all = [root, left, right, left_child, right_child];

    // First bulk pass recomputes every dirty node.
    assert_eq!(graph.recompute_dirty_transforms(), all.len());
    assert_worlds_match(&graph, &all);
    // A fully clean graph recomputes nothing.
    assert_eq!(graph.recompute_dirty_transforms(), 0);

    // transform_mut on a leaf: exactly one node recomputed.
    set_transform(&mut graph, right_child, [1.0, 1.0, 1.0], IDENTITY, [1.0; 3]);
    assert_eq!(graph.recompute_dirty_transforms(), 1);
    assert_worlds_match(&graph, &all);
    assert_eq!(graph.recompute_dirty_transforms(), 0);

    // transform_mut on an internal node: exactly its subtree recomputed.
    set_transform(&mut graph, left, [0.0, 1.0, 0.0], IDENTITY, [1.0; 3]);
    assert_eq!(graph.recompute_dirty_transforms(), 2); // left + left_child
    assert_worlds_match(&graph, &all);

    // attach: exactly the moved subtree recomputed.
    let orphan = graph.add_entity("orphan");
    set_transform(&mut graph, orphan, [3.0, 2.0, 1.0], IDENTITY, [1.0; 3]);
    assert_eq!(graph.recompute_dirty_transforms(), 1); // orphan only
    graph.attach(orphan, left_child).unwrap();
    assert_eq!(graph.recompute_dirty_transforms(), 1); // orphan only, again
    let all = [root, left, right, left_child, right_child, orphan];
    assert_worlds_match(&graph, &all);
}

#[test]
fn read_after_transform_mut_without_recompute_returns_the_new_value() {
    let mut graph = SceneGraph::new();
    let parent = graph.add_entity("parent");
    let child = graph.add_entity("child");
    graph.attach(child, parent).unwrap();
    set_transform(&mut graph, parent, [1.0, 0.0, 0.0], IDENTITY, [1.0; 3]);
    set_transform(&mut graph, child, [0.0, 2.0, 0.0], IDENTITY, [1.0; 3]);
    assert_eq!(graph.recompute_dirty_transforms(), 2);
    assert_eq!(
        graph.world_transform(child).unwrap().translation,
        [1.0, 2.0, 0.0]
    );

    // Write through the checkout and read back immediately, no recompute call.
    graph.transform_mut(parent).unwrap().translation = [5.0, 6.0, 7.0];
    let world = graph.world_transform(child).expect("child world must be fresh");
    assert_eq!(world.translation, [5.0, 8.0, 7.0]);
    assert_eq!(
        graph.world_transform(parent).unwrap().translation,
        [5.0, 6.0, 7.0]
    );
}
