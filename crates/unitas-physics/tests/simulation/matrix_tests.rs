use unitas_core::EntityId;
use unitas_physics::matrix::{
    message_is_eligible, route_message, CollisionMatrix, CollisionMessage, FrameEvents,
    MessageDelivery, MessageTracker, PhysicsMaterial3D, PhysicsObject, TouchingPair, LAYER_COUNT,
};
use unitas_physics::{BodyDesc, BodyRole, ColliderDesc, ColliderShape, PhysicsError, Shape, World};

fn collider(object_id: u64, layer: usize) -> PhysicsObject {
    PhysicsObject::new(object_id, layer)
}

fn dynamic(object_id: u64, layer: usize) -> PhysicsObject {
    let mut object = PhysicsObject::new(object_id, layer);
    object.has_rigidbody = true;
    object
}

fn kinematic(object_id: u64, layer: usize) -> PhysicsObject {
    let mut object = dynamic(object_id, layer);
    object.is_kinematic = true;
    object
}

fn static_collider(object_id: u64, layer: usize) -> PhysicsObject {
    let mut object = PhysicsObject::new(object_id, layer);
    object.is_static = true;
    object
}

fn touching<'a>(a: &'a PhysicsObject, b: &'a PhysicsObject, is_trigger: bool) -> TouchingPair<'a> {
    TouchingPair { a, b, is_trigger }
}

fn receivers(deliveries: &[MessageDelivery]) -> Vec<u64> {
    let mut ids: Vec<u64> = deliveries
        .iter()
        .map(|delivery| delivery.receiver_object_id)
        .collect();
    ids.sort_unstable();
    ids
}

fn sphere_body(position: [f64; 3], velocity: [f64; 3], layer: u32) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity,
        use_gravity: false,
        is_trigger: false,
        collider_shape: ColliderShape::Primitive(Shape::Sphere { radius: 0.5 }),
        layer,
        role: BodyRole::Dynamic,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
    }
}

#[test]
fn matrix_has_32_layers_and_is_symmetric() {
    assert_eq!(LAYER_COUNT, 32);
    let matrix = CollisionMatrix::new();
    for a in 0..LAYER_COUNT {
        for b in 0..LAYER_COUNT {
            assert!(matrix.is_enabled(a, b), "layer pair {a} {b} starts enabled");
        }
    }

    let mut matrix = matrix;
    matrix.disable_pair(3, 7);
    assert!(!matrix.is_enabled(3, 7));
    assert!(!matrix.is_enabled(7, 3));
    assert!(matrix.is_enabled(3, 8));
    assert!(matrix.is_enabled(8, 3));
}

#[test]
fn matrix_removed_layer_pair_stops_callbacks() {
    let matrix = CollisionMatrix::new();
    let a = dynamic(1, 0);
    let b = dynamic(2, 1);
    assert!(message_is_eligible(&a, &b, &matrix));

    let mut restricted = CollisionMatrix::new();
    restricted.disable_pair(0, 1);
    assert!(!message_is_eligible(&a, &b, &restricted));

    let mut tracker = MessageTracker::new();
    let deliveries = tracker.update(
        &[touching(&a, &b, false)],
        &restricted,
        &FrameEvents::default(),
    );
    assert!(deliveries.is_empty());
}

#[test]
fn matrix_ignored_pair_exchanges_nothing() {
    let matrix = CollisionMatrix::new();
    let mut a = dynamic(1, 0);
    a.ignored_object_ids.push(2);
    let b = dynamic(2, 0);
    assert!(!message_is_eligible(&a, &b, &matrix));
}

#[test]
fn matrix_static_static_exchanges_nothing() {
    let matrix = CollisionMatrix::new();
    let a = static_collider(1, 0);
    let b = static_collider(2, 0);
    assert!(!message_is_eligible(&a, &b, &matrix));

    let mut tracker = MessageTracker::new();
    let deliveries = tracker.update(&[touching(&a, &b, false)], &matrix, &FrameEvents::default());
    assert!(deliveries.is_empty());
}

#[test]
fn matrix_without_rigidbody_exchanges_nothing() {
    let matrix = CollisionMatrix::new();
    let a = collider(1, 0);
    let b = collider(2, 0);
    assert!(!message_is_eligible(&a, &b, &matrix));

    let mut tracker = MessageTracker::new();
    assert!(tracker
        .update(&[touching(&a, &b, false)], &matrix, &FrameEvents::default())
        .is_empty());
    assert!(tracker
        .update(&[touching(&a, &b, true)], &matrix, &FrameEvents::default())
        .is_empty());
}

#[test]
fn matrix_dynamic_static_routes_to_the_dynamic_side() {
    let matrix = CollisionMatrix::new();
    let a = dynamic(1, 0);
    let b = static_collider(2, 0);
    assert!(message_is_eligible(&a, &b, &matrix));

    let deliveries = route_message(CollisionMessage::OnCollisionEnter, &a, &b);
    assert_eq!(receivers(&deliveries), vec![1]);
    assert_eq!(deliveries[0].from_object_id, 2);
}

#[test]
fn matrix_kinematic_and_dynamic_both_receive() {
    let matrix = CollisionMatrix::new();
    let platform = kinematic(1, 0);
    let body = dynamic(2, 0);
    assert!(message_is_eligible(&platform, &body, &matrix));

    let deliveries = route_message(CollisionMessage::OnCollisionEnter, &platform, &body);
    assert_eq!(receivers(&deliveries), vec![1, 2]);
}

#[test]
fn matrix_kinematic_static_exchanges_nothing() {
    let matrix = CollisionMatrix::new();
    let platform = kinematic(1, 0);
    let floor = static_collider(2, 0);
    assert!(!message_is_eligible(&platform, &floor, &matrix));
    assert!(route_message(CollisionMessage::OnCollisionEnter, &platform, &floor).is_empty());
    assert!(route_message(CollisionMessage::OnTriggerEnter, &platform, &floor).is_empty());
}

#[test]
fn matrix_routes_to_collider_and_rigidbody_game_objects() {
    let mut a = dynamic(1, 0);
    a.collider_object_id = Some(10);
    a.rigidbody_object_id = Some(11);
    let b = dynamic(2, 0);

    let deliveries = route_message(CollisionMessage::OnCollisionEnter, &a, &b);
    assert_eq!(receivers(&deliveries), vec![2, 10, 11]);
}

#[test]
fn matrix_disabled_behaviour_still_receives() {
    let mut a = dynamic(1, 0);
    a.behaviour_enabled = false;
    let b = dynamic(2, 0);

    let deliveries = route_message(CollisionMessage::OnTriggerEnter, &a, &b);
    assert_eq!(receivers(&deliveries), vec![1, 2]);
}

#[test]
fn matrix_six_message_kinds_fire_on_transitions() {
    let matrix = CollisionMatrix::new();
    let events = FrameEvents::default();
    let a = dynamic(1, 0);
    let b = dynamic(2, 0);

    let mut tracker = MessageTracker::new();
    let entered = tracker.update(&[touching(&a, &b, false)], &matrix, &events);
    assert!(entered
        .iter()
        .any(|delivery| delivery.message == CollisionMessage::OnCollisionEnter));
    let stayed = tracker.update(&[touching(&a, &b, false)], &matrix, &events);
    assert!(stayed
        .iter()
        .any(|delivery| delivery.message == CollisionMessage::OnCollisionStay));
    let exited = tracker.update(&[], &matrix, &events);
    assert!(exited
        .iter()
        .any(|delivery| delivery.message == CollisionMessage::OnCollisionExit));

    let mut tracker = MessageTracker::new();
    let entered = tracker.update(&[touching(&a, &b, true)], &matrix, &events);
    assert!(entered
        .iter()
        .any(|delivery| delivery.message == CollisionMessage::OnTriggerEnter));
    let stayed = tracker.update(&[touching(&a, &b, true)], &matrix, &events);
    assert!(stayed
        .iter()
        .any(|delivery| delivery.message == CollisionMessage::OnTriggerStay));
    let exited = tracker.update(&[], &matrix, &events);
    assert!(exited
        .iter()
        .any(|delivery| delivery.message == CollisionMessage::OnTriggerExit));
}

#[test]
fn matrix_teleport_emits_no_message() {
    let matrix = CollisionMatrix::new();
    let a = dynamic(1, 0);
    let b = dynamic(2, 0);

    let mut teleported = FrameEvents::default();
    teleported.teleported.insert(1);
    let mut tracker = MessageTracker::new();
    let deliveries = tracker.update(&[touching(&a, &b, false)], &matrix, &teleported);
    assert!(deliveries.is_empty());

    let mut tracker = MessageTracker::new();
    let events = FrameEvents::default();
    assert!(!tracker
        .update(&[touching(&a, &b, false)], &matrix, &events)
        .is_empty());
    let deliveries = tracker.update(&[touching(&a, &b, false)], &matrix, &teleported);
    assert!(deliveries.is_empty());
}

#[test]
fn matrix_deactivation_and_activation_report_no_message() {
    let matrix = CollisionMatrix::new();
    let a = dynamic(1, 0);
    let mut b = dynamic(2, 0);
    let events = FrameEvents::default();

    let mut tracker = MessageTracker::new();
    assert!(!tracker
        .update(&[touching(&a, &b, false)], &matrix, &events)
        .is_empty());

    let mut deactivated = FrameEvents::default();
    deactivated.deactivated.insert(2);
    b.active = false;
    let deliveries = tracker.update(&[], &matrix, &deactivated);
    assert!(deliveries.is_empty());

    let mut activated = FrameEvents::default();
    activated.activated.insert(2);
    b.active = true;
    let deliveries = tracker.update(&[touching(&a, &b, false)], &matrix, &activated);
    assert!(deliveries.is_empty());
}

#[test]
fn matrix_shape_change_does_not_re_fire_enter() {
    let matrix = CollisionMatrix::new();
    let events = FrameEvents::default();
    let a = dynamic(1, 0);
    let b = dynamic(2, 0);

    let mut tracker = MessageTracker::new();
    let entered = tracker.update(&[touching(&a, &b, false)], &matrix, &events);
    assert_eq!(entered.len(), 2);
    assert!(entered
        .iter()
        .all(|delivery| delivery.message == CollisionMessage::OnCollisionEnter));

    let stayed = tracker.update(&[touching(&a, &b, false)], &matrix, &events);
    assert_eq!(stayed.len(), 2);
    assert!(stayed
        .iter()
        .all(|delivery| delivery.message == CollisionMessage::OnCollisionStay));
}

#[test]
fn matrix_trigger_without_rigidbody_is_silent() {
    let matrix = CollisionMatrix::new();
    let mut a = collider(1, 0);
    a.is_trigger = true;
    let mut b = collider(2, 0);
    b.is_trigger = true;
    assert!(!message_is_eligible(&a, &b, &matrix));

    let mut tracker = MessageTracker::new();
    let deliveries = tracker.update(&[touching(&a, &b, true)], &matrix, &FrameEvents::default());
    assert!(deliveries.is_empty());
}

#[test]
fn matrix_trigger_reaches_trigger_and_rigidbody_game_objects() {
    let mut trigger = collider(1, 0);
    trigger.is_trigger = true;
    trigger.behaviour_enabled = false;
    let mut body = dynamic(2, 0);
    body.collider_object_id = Some(20);
    body.rigidbody_object_id = Some(21);

    let deliveries = route_message(CollisionMessage::OnTriggerEnter, &trigger, &body);
    assert_eq!(receivers(&deliveries), vec![1, 20, 21]);
}

#[test]
fn matrix_world_layer_matrix_stops_collision_callbacks() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let left = EntityId::new(1);
    let right = EntityId::new(2);
    world.add_body(left, sphere_body([-1.0, 0.0, 0.0], [0.6, 0.0, 0.0], 0))?;
    world.add_body(right, sphere_body([1.0, 0.0, 0.0], [-0.6, 0.0, 0.0], 1))?;

    let mut matrix = CollisionMatrix::new();
    matrix.disable_pair(0, 1);
    world.set_collision_matrix(matrix);
    world.step(1.0)?;
    assert!(world.drain_events().is_empty());
    assert!(world.drain_trigger_events().is_empty());

    let mut world = World::new();
    world.add_body(left, sphere_body([-1.0, 0.0, 0.0], [0.6, 0.0, 0.0], 0))?;
    world.add_body(right, sphere_body([1.0, 0.0, 0.0], [-0.6, 0.0, 0.0], 1))?;
    world.step(1.0)?;
    assert_eq!(world.drain_events().len(), 1);
    assert!(world.collision_matrix().is_enabled(0, 1));
    Ok(())
}

#[test]
fn matrix_removed_layer_pair_stops_default_mask_queries_and_default_material_applies(
) -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let (body_l0, body_l1, body_l2) = setup_default_material_bodies(&mut world)?;
    assert_default_material_updates(&mut world, (body_l0, body_l1, body_l2));

    let initial_hits = world.raycast_default_layer([-2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 20.0, 0);
    let initial_entities: Vec<EntityId> = initial_hits.iter().map(|h| h.entity).collect();
    assert_eq!(initial_entities, vec![body_l0, body_l1, body_l2]);

    world.ignore_layer_collision(0, 1, true);
    assert!(world.get_ignore_layer_collision(0, 1));
    assert!(world.get_ignore_layer_collision(1, 0));

    let filtered_hits = world.raycast_default_layer([-2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 20.0, 0);
    let filtered_entities: Vec<EntityId> = filtered_hits.iter().map(|h| h.entity).collect();
    assert_eq!(filtered_entities, vec![body_l0, body_l2]);
    assert!(!world
        .overlap_sphere_default_layer([3.0, 0.0, 0.0], 1.0, 0)
        .contains(&body_l1));
    assert!(world
        .overlap_sphere_default_layer([6.0, 0.0, 0.0], 1.0, 0)
        .contains(&body_l2));
    Ok(())
}

fn setup_default_material_bodies(
    world: &mut World,
) -> Result<(EntityId, EntityId, EntityId), PhysicsError> {
    let body_l0 = EntityId::new(10);
    let body_l1 = EntityId::new(11);
    let body_l2 = EntityId::new(12);
    world.add_body_with_default_material(
        body_l0,
        sphere_body([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0),
    )?;
    world.add_body_with_material(
        body_l1,
        sphere_body([3.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1),
        None,
    )?;
    world.add_body_with_material(
        body_l2,
        sphere_body([6.0, 0.0, 0.0], [0.0, 0.0, 0.0], 2),
        Some(PhysicsMaterial3D::new(0.25, 0.9)),
    )?;
    Ok((body_l0, body_l1, body_l2))
}

fn assert_default_material_updates(
    world: &mut World,
    bodies: (EntityId, EntityId, EntityId),
) {
    let (body_l0, body_l1, body_l2) = bodies;
    assert_eq!(
        world.body_material(body_l0),
        Some(PhysicsMaterial3D::default())
    );
    assert_eq!(
        world.body_material(body_l1),
        Some(PhysicsMaterial3D::default())
    );
    assert_eq!(
        world.body_material(body_l2),
        Some(PhysicsMaterial3D::new(0.25, 0.9))
    );
    world.set_default_material(PhysicsMaterial3D::new(0.8, 0.4));
    assert_eq!(
        world.body_material(body_l0),
        Some(PhysicsMaterial3D::new(0.8, 0.4))
    );
    assert_eq!(
        world.body_material(body_l1),
        Some(PhysicsMaterial3D::new(0.8, 0.4))
    );
    assert_eq!(
        world.body_material(body_l2),
        Some(PhysicsMaterial3D::new(0.25, 0.9))
    );
}

#[test]
fn auto_sync_transforms_off_defers_moved_collider_visibility_until_sync_transforms_or_step(
) -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    assert!(world.auto_sync_transforms());
    let (wall, crate_body) = setup_auto_sync_bodies(&mut world)?;

    world.set_auto_sync_transforms(false);
    assert!(!world.auto_sync_transforms());

    world.set_position(wall, [2.0, 0.0, 0.0])?;
    assert_eq!(world.position(wall), Some([2.0, 0.0, 0.0]));
    assert_eq!(world.query_position(wall), Some([0.0, 10.0, 0.0]));
    assert!(
        world
            .raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 20.0, 1)
            .is_empty(),
        "with auto_sync_transforms off, raycast must not see moved static collider before sync_transforms"
    );
    assert!(
        world.overlap_sphere([2.0, 0.0, 0.0], 0.25, 1).is_empty(),
        "with auto_sync_transforms off, overlap_sphere must not see moved static collider before sync_transforms"
    );
    assert_eq!(
        world.overlap_sphere([0.0, 10.0, 0.0], 0.25, 1),
        vec![wall],
        "before sync_transforms, queries still observe the last synchronized position"
    );

    assert_sync_and_step_visibility(&mut world, wall, crate_body)?;
    Ok(())
}

fn setup_auto_sync_bodies(world: &mut World) -> Result<(EntityId, EntityId), PhysicsError> {
    let wall = EntityId::new(1);
    let crate_body = EntityId::new(2);
    world.add_body(
        wall,
        BodyDesc {
            role: BodyRole::Static,
            collider_shape: ColliderShape::Primitive(Shape::Box {
                half_extents: [0.5, 0.5, 0.5],
            }),
            position: [0.0, 10.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0, 0.0, 0.0],
            use_gravity: false,
            is_trigger: false,
            layer: 1,
            friction: 0.6,
            restitution: 0.0,
            collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
        },
    )?;
    world.add_body(
        crate_body,
        BodyDesc {
            role: BodyRole::Dynamic,
            collider_shape: ColliderShape::Primitive(Shape::Sphere { radius: 0.5 }),
            position: [0.0, -10.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 1.0,
            velocity: [0.0, 0.0, 0.0],
            use_gravity: false,
            is_trigger: false,
            layer: 1,
            friction: 0.6,
            restitution: 0.0,
            collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
        },
    )?;
    Ok((wall, crate_body))
}

fn assert_sync_and_step_visibility(
    world: &mut World,
    wall: EntityId,
    crate_body: EntityId,
) -> Result<(), PhysicsError> {
    world.sync_transforms();
    assert_eq!(world.query_position(wall), Some([2.0, 0.0, 0.0]));
    let hits_after_sync = world.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 20.0, 1);
    assert_eq!(hits_after_sync.len(), 1);
    assert_eq!(hits_after_sync[0].entity, wall);
    assert_eq!(world.overlap_sphere([2.0, 0.0, 0.0], 0.25, 1), vec![wall]);

    world.teleport_position(crate_body, [4.0, 0.0, 0.0])?;
    assert_eq!(world.position(crate_body), Some([4.0, 0.0, 0.0]));
    assert_eq!(world.query_position(crate_body), Some([0.0, -10.0, 0.0]));
    let hits_before_step = world.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 20.0, 1);
    assert_eq!(
        hits_before_step
            .iter()
            .map(|h| h.entity)
            .collect::<Vec<_>>(),
        vec![wall],
        "teleported dynamic body is not visible along the ray before sync_transforms or step"
    );

    world.step(0.02)?;
    let hits_after_step = world.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 20.0, 1);
    assert_eq!(
        hits_after_step.iter().map(|h| h.entity).collect::<Vec<_>>(),
        vec![wall, crate_body],
        "simulation step synchronizes pending transforms into query visibility"
    );

    world.set_auto_sync_transforms(true);
    world.set_position(wall, [0.0, 20.0, 0.0])?;
    assert!(
        world.overlap_sphere([2.0, 0.0, 0.0], 0.25, 1).is_empty(),
        "with auto_sync_transforms on, moving collider updates query visibility immediately"
    );
    assert_eq!(world.overlap_sphere([0.0, 20.0, 0.0], 0.25, 1), vec![wall]);
    Ok(())
}

#[test]
fn mesh_colliders_follow_the_unity_rules_for_convex_static_and_dynamic_bodies(
) -> Result<(), PhysicsError> {
    let mut world = World::new();
    let valid_convex_dynamic = EntityId::new(3);
    let valid_non_convex_static = EntityId::new(4);
    setup_mesh_collider_world(&mut world, valid_convex_dynamic, valid_non_convex_static)?;

    assert!(world.is_body_simulated(valid_convex_dynamic));
    let hull_diags = world.entity_diagnostics(valid_convex_dynamic);
    assert_eq!(hull_diags.len(), 1);
    assert_eq!(hull_diags[0].code, "PHY-MESH-CONVEX-HULL-LIMIT");
    assert!(hull_diags[0].message.contains("255"));

    assert!(world.is_body_simulated(valid_non_convex_static));
    assert!(world.entity_diagnostics(valid_non_convex_static).is_empty());

    world.step(0.1)?;
    let convex_y = world.position(valid_convex_dynamic).map_or(2.0, |p| p[1]);
    assert!(
        convex_y < 2.0,
        "convex dynamic mesh collider simulates normally"
    );

    let attach_err = world.add_collider(
        valid_convex_dynamic,
        ColliderDesc {
            shape: ColliderShape::Mesh {
                vertices: plate_vertices(),
                indices: plate_indices(),
                convex: false,
            },
            offset: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            is_trigger: false,
            material: None,
            layer: 1,
        },
    );
    assert!(
        matches!(
            attach_err,
            Err(PhysicsError::NonConvexMeshColliderOnDynamicBody { entity, .. })
                if entity == valid_convex_dynamic
        ),
        "a non-convex mesh collider must be refused on a dynamic body: got {attach_err:?}"
    );
    assert!(
        world.is_body_simulated(valid_convex_dynamic),
        "the refused collider must leave the body it was refused for simulating"
    );
    Ok(())
}

fn setup_mesh_collider_world(
    world: &mut World,
    valid_convex_dynamic: EntityId,
    valid_non_convex_static: EntityId,
) -> Result<(), PhysicsError> {
    let floor = EntityId::new(1);
    world.add_body(
        floor,
        BodyDesc {
            role: BodyRole::Static,
            collider_shape: ColliderShape::Primitive(Shape::Box {
                half_extents: [10.0, 0.5, 10.0],
            }),
            position: [0.0, -0.5, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0, 0.0, 0.0],
            use_gravity: false,
            is_trigger: false,
            layer: 1,
            friction: 0.6,
            restitution: 0.0,
            collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
        },
    )?;

    let dynamic_desc = BodyDesc {
        role: BodyRole::Dynamic,
        collider_shape: ColliderShape::Primitive(Shape::Box {
            half_extents: [0.5, 0.5, 0.5],
        }),
        position: [0.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity: [0.0, -1.0, 0.0],
        use_gravity: true,
        is_trigger: false,
        layer: 1,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
    };

    let (cone_vertices, cone_indices) = cone_mesh(300);
    world.add_body(
        valid_convex_dynamic,
        BodyDesc {
            position: [2.0, 2.0, 0.0],
            collider_shape: ColliderShape::Mesh {
                vertices: cone_vertices,
                indices: cone_indices,
                convex: true,
            },
            ..dynamic_desc
        },
    )?;
    world.add_body(
        valid_non_convex_static,
        BodyDesc {
            role: BodyRole::Static,
            mass: 0.0,
            use_gravity: false,
            position: [-3.0, 0.5, 0.0],
            collider_shape: ColliderShape::Mesh {
                vertices: plate_vertices(),
                indices: plate_indices(),
                convex: false,
            },
            ..dynamic_desc
        },
    )?;
    Ok(())
}

fn plate_vertices() -> Vec<[f64; 3]> {
    vec![
        [-1.0, 0.0, -1.0],
        [1.0, 0.0, -1.0],
        [1.0, 0.0, 1.0],
        [-1.0, 0.0, 1.0],
    ]
}

fn plate_indices() -> Vec<u32> {
    vec![0, 2, 1, 0, 3, 2]
}

fn cone_mesh(segments: usize) -> (Vec<[f64; 3]>, Vec<u32>) {
    let mut vertices = vec![[0.0, 1.0, 0.0]];
    for index in 0..segments {
        let angle = 2.0 * std::f64::consts::PI * index as f64 / segments as f64;
        vertices.push([angle.cos(), 0.0, angle.sin()]);
    }
    let mut indices = Vec::new();
    for index in 0..segments {
        let next = (index + 1) % segments;
        indices.push(0);
        indices.push((index + 1) as u32);
        indices.push((next + 1) as u32);
    }
    (vertices, indices)
}

#[test]
fn phy_001_story_2_removing_layer_pair_stops_collision_and_trigger_callbacks_and_default_mask_queries(
) -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let dynamic_l0 = EntityId::new(1);
    let dynamic_l1 = EntityId::new(2);
    let trigger_l1 = EntityId::new(3);

    world.add_body(
        dynamic_l0,
        sphere_body([-0.3, 0.0, 0.0], [0.0, 0.0, 0.0], 0),
    )?;
    world.add_body(dynamic_l1, sphere_body([0.3, 0.0, 0.0], [0.0, 0.0, 0.0], 1))?;
    world.add_body(
        trigger_l1,
        BodyDesc {
            is_trigger: true,
            role: BodyRole::Static,
            mass: 0.0,
            ..sphere_body([-0.2, 0.0, 0.0], [0.0, 0.0, 0.0], 1)
        },
    )?;

    world.ignore_layer_collision(0, 1, true);
    world.ignore_layer_collision(1, 1, true);
    world.step(0.02)?;

    assert!(
        world.drain_events().is_empty(),
        "PHY-001 story 2: removing layer pair (0, 1) must stop 3D collision callbacks"
    );
    assert!(
        world.drain_trigger_events().is_empty(),
        "PHY-001 story 2: removing layer pair (0, 1) must stop 3D trigger callbacks"
    );
    let ray_hits = world.raycast_default_layer([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 20.0, 0);
    assert_eq!(
        ray_hits.iter().map(|h| h.entity).collect::<Vec<_>>(),
        vec![dynamic_l0],
        "PHY-001 story 2: default-mask raycast from layer 0 must exclude layer 1 bodies"
    );
    let overlap_hits = world.overlap_sphere_default_layer([0.0, 0.0, 0.0], 5.0, 0);
    assert_eq!(
        overlap_hits,
        vec![dynamic_l0],
        "PHY-001 story 2: default-mask overlap query from layer 0 must exclude layer 1 bodies"
    );
    Ok(())
}

#[test]
fn phy_001_story_13_kinematic_move_position_pushes_dynamic_body_and_both_receive_collision_messages_while_rigidbodyless_triggers_are_silent_and_disabled_monobehaviour_receives_on_trigger_enter(
) -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let kinematic_id = EntityId::new(1);
    let dynamic_id = EntityId::new(2);
    world.add_body(
        kinematic_id,
        BodyDesc {
            role: BodyRole::Kinematic,
            mass: 0.0,
            ..sphere_body([-1.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0)
        },
    )?;
    world.add_body(dynamic_id, sphere_body([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0))?;

    world.move_position(kinematic_id, [-0.5, 0.0, 0.0])?;
    assert_eq!(
        world.position(kinematic_id),
        Some([-1.0, 0.0, 0.0]),
        "PHY-001 story 13: MovePosition on kinematic body must defer movement until the next simulation step"
    );

    world.step(0.1)?;
    assert_eq!(
        world.position(kinematic_id),
        Some([-0.5, 0.0, 0.0]),
        "PHY-001 story 13: kinematic body reaches MovePosition target on the simulation step"
    );
    let dynamic_pos = world.position(dynamic_id).unwrap_or([0.0; 3]);
    assert!(
        dynamic_pos[0] > 0.49,
        "PHY-001 story 13: kinematic body moved with MovePosition must push the dynamic body, got {dynamic_pos:?}"
    );
    let step_events = world.drain_events();
    assert_eq!(
        step_events,
        vec![unitas_physics::CollisionEvent {
            a: kinematic_id,
            b: dynamic_id,
            phase: unitas_physics::Phase::Enter,
        }],
        "PHY-001 story 13: kinematic MovePosition into dynamic body must emit OnCollisionEnter"
    );

    assert_story_13_message_routing(world.collision_matrix(), kinematic_id, dynamic_id);
    Ok(())
}

fn assert_story_13_message_routing(
    matrix: &CollisionMatrix,
    kinematic_id: EntityId,
    dynamic_id: EntityId,
) {
    let kinematic_obj = kinematic(kinematic_id.as_u64(), 0);
    let dynamic_obj = dynamic(dynamic_id.as_u64(), 0);
    let mut tracker = MessageTracker::new();
    let deliveries = tracker.update(
        &[touching(&kinematic_obj, &dynamic_obj, false)],
        matrix,
        &FrameEvents::default(),
    );
    assert_eq!(
        receivers(&deliveries),
        vec![1, 2],
        "PHY-001 story 13: both kinematic and dynamic bodies must receive OnCollisionEnter"
    );
    assert!(deliveries
        .iter()
        .all(|d| d.message == CollisionMessage::OnCollisionEnter));

    let mut trigger_a = collider(10, 0);
    trigger_a.is_trigger = true;
    let mut trigger_b = collider(11, 0);
    trigger_b.is_trigger = true;
    let mut rigidbodyless_tracker = MessageTracker::new();
    let silent = rigidbodyless_tracker.update(
        &[touching(&trigger_a, &trigger_b, true)],
        matrix,
        &FrameEvents::default(),
    );
    assert!(
        silent.is_empty(),
        "PHY-001 story 13: trigger between two colliders without a Rigidbody must produce no trigger messages"
    );

    let mut disabled_trigger = collider(20, 0);
    disabled_trigger.is_trigger = true;
    disabled_trigger.behaviour_enabled = false;
    let trigger_deliveries = tracker.update(
        &[touching(&disabled_trigger, &dynamic_obj, true)],
        matrix,
        &FrameEvents::default(),
    );
    assert!(
        trigger_deliveries.iter().any(|d| {
            d.receiver_object_id == 20 && d.message == CollisionMessage::OnTriggerEnter
        }),
        "PHY-001 story 13: disabled MonoBehaviour on the trigger collider's GameObject must still receive OnTriggerEnter"
    );
}
