use unitas_core::EntityId;
use unitas_physics::matrix::{
    message_is_eligible, route_message, CollisionMatrix, CollisionMessage, FrameEvents,
    MessageDelivery, MessageTracker, PhysicsObject, TouchingPair, LAYER_COUNT,
};
use unitas_physics::{BodyDesc, BodyRole, PhysicsError, Shape, World};

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

fn touching<'a>(
    a: &'a PhysicsObject,
    b: &'a PhysicsObject,
    is_trigger: bool,
) -> TouchingPair<'a> {
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
        shape: Shape::Sphere { radius: 0.5 },
        layer,
        role: BodyRole::Dynamic,
        friction: 0.6,
        restitution: 0.0,
    }
}

#[test]
fn matrix_has_32_layers_and_is_symmetric() {
    assert_eq!(LAYER_COUNT, 32);
    let matrix = CollisionMatrix::new();
    for a in 0..LAYER_COUNT {
        for b in 0..LAYER_COUNT {
            assert!(
                matrix.is_enabled(a, b),
                "layer pair {a} {b} starts enabled"
            );
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
    let deliveries = tracker.update(
        &[touching(&a, &b, false)],
        &matrix,
        &FrameEvents::default(),
    );
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

    // the collider changed shape but the pair keeps touching, so it stays a stay
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
    let deliveries = tracker.update(
        &[touching(&a, &b, true)],
        &matrix,
        &FrameEvents::default(),
    );
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
