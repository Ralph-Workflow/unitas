use unitas_core::EntityId;
use unitas_physics::two_d::{
    BodyDesc2D, BodyType2D, ColliderDesc2D, ContactFilter2D, MessageKind2D, PhysicsMaterial2D,
    Shape2D, World2D,
};

#[test]
fn dynamic_body_hitting_static_box_delivers_enter_touching_and_ignore_collision() {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    let floor_go = EntityId::new(1);
    let box_go = EntityId::new(2);

    let floor_body = world.add_body(BodyDesc2D::static_body(floor_go, [0.0, 0.0]));
    let floor_col = world.add_collider(
        ColliderDesc2D::box_collider(floor_go, [5.0, 0.5]).with_attached_body(floor_body),
    );

    let dyn_body = world.add_body(
        BodyDesc2D::dynamic(box_go, [0.0, 0.9]).with_velocity([0.0, -2.0]),
    );
    let dyn_col = world.add_collider(
        ColliderDesc2D::box_collider(box_go, [0.5, 0.5]).with_attached_body(dyn_body),
    );

    world.step(0.05);

    assert!(world.is_touching(dyn_col, floor_col));
    assert!(world.is_touching_layers(dyn_col, 1 << 0));
    assert!(!world.is_touching_layers(dyn_col, 1 << 3));

    let contacts = world.get_contacts(dyn_col);
    assert_eq!(contacts.len(), 1);
    assert!(contacts[0].normal[1] > 0.9);
    assert!(contacts[0].normal_impulse > 0.0);

    let messages = world.drain_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].recipient, box_go);
    assert_eq!(messages[0].kind, MessageKind2D::CollisionEnter2D);
    assert_eq!(messages[0].self_collider, dyn_col);
    assert_eq!(messages[0].other_collider, floor_col);
    assert_eq!(messages[0].other_body, Some(floor_body));
    assert_eq!(messages[0].contacts.len(), 1);

    world.step(0.05);
    let stay_messages = world.drain_messages();
    assert_eq!(stay_messages.len(), 1);
    assert_eq!(stay_messages[0].kind, MessageKind2D::CollisionStay2D);

    world.ignore_collision(dyn_col, floor_col, true);
    assert!(world.get_ignore_collision(dyn_col, floor_col));
    world.step(0.05);
    assert!(!world.is_touching(dyn_col, floor_col));
    assert!(world.drain_messages().is_empty());
}

#[test]
fn layer_collision_matrix_gates_callbacks_and_default_mask_queries() {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    let go_a = EntityId::new(10);
    let go_b = EntityId::new(20);

    let body_a = world.add_body(
        BodyDesc2D::dynamic(go_a, [-1.0, 0.0])
            .with_layer(1)
            .with_velocity([20.0, 0.0]),
    );
    let col_a = world.add_collider(
        ColliderDesc2D::circle_collider(go_a, 0.6)
            .with_attached_body(body_a)
            .with_layer(1),
    );

    let body_b = world.add_body(
        BodyDesc2D::dynamic(go_b, [1.0, 0.0])
            .with_layer(2)
            .with_velocity([-20.0, 0.0]),
    );
    let col_b = world.add_collider(
        ColliderDesc2D::circle_collider(go_b, 0.6)
            .with_attached_body(body_b)
            .with_layer(2),
    );

    world.ignore_layer_collision(1, 2, true);
    assert!(world.get_ignore_layer_collision(1, 2));
    assert!(world.get_ignore_layer_collision(2, 1));

    let hits_ignored = world.raycast_default_layer([-3.0, 0.0], [1.0, 0.0], 10.0, 1);
    assert!(hits_ignored.iter().all(|h| h.collider != col_b));
    assert!(hits_ignored.iter().any(|h| h.collider == col_a));

    world.step(0.1);
    assert!(!world.is_touching(col_a, col_b));
    assert!(world.drain_messages().is_empty());

    world.ignore_layer_collision(1, 2, false);
    assert!(!world.get_ignore_layer_collision(1, 2));
    let hits_enabled = world.raycast_default_layer([-3.0, 0.0], [1.0, 0.0], 10.0, 1);
    assert!(hits_enabled.iter().any(|h| h.collider == col_b));
}

#[test]
fn message_routing_follows_action_matrix_and_child_collider_game_objects() {
    let mut world = World2D::with_gravity([0.0, 0.0]);

    let s1_go = EntityId::new(1);
    let s2_go = EntityId::new(2);
    world.add_collider(ColliderDesc2D::box_collider(s1_go, [1.0, 1.0]));
    world.add_collider(ColliderDesc2D::box_collider(s2_go, [1.0, 1.0]));
    world.step(0.02);
    assert!(world.drain_messages().is_empty());

    let k_go = EntityId::new(3);
    let k_body = world.add_body(BodyDesc2D::kinematic(k_go, [0.0, 0.0]));
    world.add_collider(
        ColliderDesc2D::box_collider(k_go, [1.0, 1.0]).with_attached_body(k_body),
    );
    world.step(0.02);
    assert!(world.drain_messages().is_empty());

    let parent_rb_go = EntityId::new(100);
    let child_col_go = EntityId::new(101);
    let dyn_body = world.add_body(BodyDesc2D::dynamic(parent_rb_go, [0.0, 0.0]));
    world.add_collider(
        ColliderDesc2D::circle_collider(child_col_go, 0.5).with_attached_body(dyn_body),
    );
    world.step(0.02);

    let messages = world.drain_messages();
    let recipients: Vec<EntityId> = messages.iter().map(|m| m.recipient).collect();
    assert!(recipients.contains(&parent_rb_go));
    assert!(recipients.contains(&child_col_go));
    assert!(recipients.contains(&k_go));
    assert!(!recipients.contains(&s1_go));
    assert!(!recipients.contains(&s2_go));
}

#[test]
fn trigger_events_fire_enter_stay_exit_only_when_rigidbody_present() {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    let trig_go = EntityId::new(10);
    let mover_go = EntityId::new(11);

    let trig_col = world.add_collider(
        ColliderDesc2D::box_collider(trig_go, [1.0, 1.0]).with_trigger(true),
    );
    let mover_body = world.add_body(BodyDesc2D::dynamic(mover_go, [-2.0, 0.0]));
    let mover_col = world.add_collider(
        ColliderDesc2D::circle_collider(mover_go, 0.5).with_attached_body(mover_body),
    );

    if let Some(b) = world.body_mut(mover_body) {
        b.move_position([0.0, 0.0]);
    }
    world.step(0.02);
    let enter_msgs = world.drain_messages();
    assert_eq!(enter_msgs.len(), 2);
    assert!(enter_msgs
        .iter()
        .all(|m| m.kind == MessageKind2D::TriggerEnter2D));
    assert!(world.is_touching(trig_col, mover_col));

    world.step(0.02);
    let stay_msgs = world.drain_messages();
    assert_eq!(stay_msgs.len(), 2);
    assert!(stay_msgs
        .iter()
        .all(|m| m.kind == MessageKind2D::TriggerStay2D));

    if let Some(b) = world.body_mut(mover_body) {
        b.move_position([5.0, 0.0]);
    }
    world.step(0.02);
    let exit_msgs = world.drain_messages();
    assert_eq!(exit_msgs.len(), 2);
    assert!(exit_msgs
        .iter()
        .all(|m| m.kind == MessageKind2D::TriggerExit2D));
}

#[test]
fn shapes_box_circle_edge_polygon_and_spatial_queries_respect_contact_filter() {
    let mut world = World2D::with_gravity([0.0, 0.0]);

    let box_col = world.add_collider(
        ColliderDesc2D::box_collider(EntityId::new(1), [1.0, 1.0])
            .with_offset([2.0, 0.0])
            .with_layer(1)
            .with_depth(0.0, 2.0),
    );
    let circle_col = world.add_collider(
        ColliderDesc2D::circle_collider(EntityId::new(2), 0.75)
            .with_offset([5.0, 0.0])
            .with_layer(2)
            .with_trigger(true)
            .with_depth(3.0, 5.0),
    );
    let edge_col = world.add_collider(
        ColliderDesc2D::edge_collider(EntityId::new(3), [8.0, -2.0], [8.0, 2.0])
            .with_layer(1)
            .with_depth(0.0, 2.0),
    );
    let poly_col = world.add_collider(
        ColliderDesc2D::polygon_collider(
            EntityId::new(4),
            vec![[10.0, -1.0], [12.0, -1.0], [11.0, 1.0]],
        )
        .with_layer(1)
        .with_depth(0.0, 2.0),
    );

    let all_hits = world.raycast([-1.0, 0.0], [1.0, 0.0], 20.0, ContactFilter2D::no_filter());
    let hit_ids: Vec<_> = all_hits.iter().map(|h| h.collider).collect();
    assert_eq!(hit_ids, vec![box_col, circle_col, edge_col, poly_col]);

    let no_trigger_filter = ContactFilter2D::default().with_triggers(false);
    let solid_hits = world.raycast([-1.0, 0.0], [1.0, 0.0], 20.0, no_trigger_filter);
    let solid_ids: Vec<_> = solid_hits.iter().map(|h| h.collider).collect();
    assert_eq!(solid_ids, vec![box_col, edge_col, poly_col]);

    let layer2_filter = ContactFilter2D::default().with_layer_mask(1 << 2);
    let l2_hits = world.raycast([-1.0, 0.0], [1.0, 0.0], 20.0, layer2_filter);
    assert_eq!(l2_hits.len(), 1);
    assert_eq!(l2_hits[0].collider, circle_col);

    let depth_filter = ContactFilter2D::default().with_depth(2.5, 6.0);
    let depth_hits = world.raycast([-1.0, 0.0], [1.0, 0.0], 20.0, depth_filter);
    assert_eq!(depth_hits.len(), 1);
    assert_eq!(depth_hits[0].collider, circle_col);

    assert_eq!(
        world.overlap_point([2.0, 0.0], ContactFilter2D::no_filter()),
        vec![box_col]
    );
    assert_eq!(
        world.overlap_point([11.0, 0.0], ContactFilter2D::no_filter()),
        vec![poly_col]
    );
    assert_eq!(
        world.overlap_circle([5.0, 0.0], 0.5, ContactFilter2D::no_filter()),
        vec![circle_col]
    );
    assert_eq!(
        world.overlap_box([8.0, 0.0], [0.5, 0.5], 0.0, ContactFilter2D::no_filter()),
        vec![edge_col]
    );
}

#[test]
fn fixed_timestep_accumulator_sleeping_and_simulated_flag() {
    let mut world = World2D::with_gravity([0.0, -10.0]);
    world.set_fixed_timestep(0.02);

    let go = EntityId::new(50);
    let body_id = world.add_body(
        BodyDesc2D::dynamic(go, [0.0, 5.0])
            .with_gravity_scale(1.0)
            .with_mass(2.0),
    );
    let col_id = world.add_collider(
        ColliderDesc2D::circle_collider(go, 0.5)
            .with_attached_body(body_id)
            .with_material(PhysicsMaterial2D::new(0.5, 0.5)),
    );
    assert_eq!(
        world.collider(col_id).map(|c| c.material),
        Some(PhysicsMaterial2D::new(0.5, 0.5))
    );

    let steps = world.advance(0.05);
    assert_eq!(steps, 2);

    let y_after = world.body(body_id).map_or(5.0, |b| b.position[1]);
    assert!(y_after < 5.0);

    if let Some(b) = world.body_mut(body_id) {
        b.simulated = false;
    }
    let frozen_y = y_after;
    world.step(0.1);
    assert_eq!(
        world.body(body_id).map(|b| b.position[1]),
        Some(frozen_y)
    );
    assert!(world
        .overlap_point([0.0, frozen_y], ContactFilter2D::no_filter())
        .is_empty());

    if let Some(b) = world.body_mut(body_id) {
        b.simulated = true;
        b.gravity_scale = 0.0;
        b.linear_velocity = [0.0, 0.0];
        b.angular_velocity = 0.0;
    }
    for _ in 0..30 {
        world.step(0.02);
    }
    assert_eq!(world.body(body_id).map(|b| b.is_sleeping()), Some(true));

    if let Some(b) = world.body_mut(body_id) {
        b.add_force([10.0, 0.0]);
    }
    assert_eq!(world.body(body_id).map(|b| b.is_sleeping()), Some(false));
    assert_eq!(
        world.body(body_id).map(|b| b.body_type),
        Some(BodyType2D::Dynamic)
    );
    assert!(matches!(
        world.collider(col_id).map(|c| &c.shape),
        Some(Shape2D::Circle { .. })
    ));
}

#[test]
fn composite_collider_merges_child_colliders_and_manual_generation_gates_rebuild() {
    use unitas_physics::two_d::{
        CompositeCollider2D, CompositeGenerationType2D, CompositeGeometryType2D,
    };

    let mut world = World2D::with_gravity([0.0, 0.0]);
    let comp_go = EntityId::new(200);
    let comp_body = world.add_body(BodyDesc2D::static_body(comp_go, [0.0, 0.0]));
    let comp_col = world.add_collider(
        ColliderDesc2D::box_collider(comp_go, [0.0, 0.0]).with_attached_body(comp_body),
    );
    world.attach_composite(
        comp_col,
        CompositeCollider2D::new(
            CompositeGeometryType2D::Polygons,
            CompositeGenerationType2D::Synchronous,
        ),
    );

    let left_col = world.add_collider(
        ColliderDesc2D::box_collider(comp_go, [0.5, 0.5])
            .with_attached_body(comp_body)
            .with_offset([-0.5, 0.0])
            .with_used_by_composite(true),
    );
    let right_col = world.add_collider(
        ColliderDesc2D::box_collider(comp_go, [0.5, 0.5])
            .with_attached_body(comp_body)
            .with_offset([0.5, 0.0])
            .with_used_by_composite(true),
    );

    let hits = world.raycast([0.0, 2.0], [0.0, -1.0], 5.0, ContactFilter2D::no_filter());
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].collider, comp_col);
    assert!(!hits.iter().any(|h| h.collider == left_col || h.collider == right_col));

    world.set_composite_generation_type(comp_col, CompositeGenerationType2D::Manual);
    let extra_col = world.add_collider(
        ColliderDesc2D::box_collider(comp_go, [0.5, 0.5])
            .with_attached_body(comp_body)
            .with_offset([2.5, 0.0])
            .with_used_by_composite(true),
    );
    assert!(
        world
            .overlap_point([2.5, 0.0], ContactFilter2D::no_filter())
            .is_empty(),
        "manual generation must not include new child collider until regenerate_composite"
    );

    world.regenerate_composite(comp_col);
    assert_eq!(
        world.overlap_point([2.5, 0.0], ContactFilter2D::no_filter()),
        vec![comp_col]
    );
    let _ = extra_col;
}

#[test]
fn tilemap_collider_merged_into_composite_updates_when_tile_removed_with_one_way_platform() {
    use unitas_physics::two_d::{
        CompositeCollider2D, CompositeGenerationType2D, CompositeGeometryType2D,
        PlatformEffector2D, TileColliderType2D, TilemapCollider2D,
    };

    let mut world = World2D::with_gravity([0.0, -10.0]);
    let tilemap_go = EntityId::new(300);
    let tilemap_body = world.add_body(BodyDesc2D::static_body(tilemap_go, [0.0, 0.0]));
    let comp_col = world.add_collider(
        ColliderDesc2D::box_collider(tilemap_go, [0.0, 0.0])
            .with_attached_body(tilemap_body)
            .with_used_by_effector(true),
    );
    world.attach_composite(
        comp_col,
        CompositeCollider2D::new(
            CompositeGeometryType2D::Outlines,
            CompositeGenerationType2D::Synchronous,
        ),
    );
    world.attach_effector(comp_col, PlatformEffector2D::default());

    let mut tilemap = TilemapCollider2D::new([1.0, 0.5]).with_used_by_composite(true);
    tilemap.set_tile([-1, 0], TileColliderType2D::Grid);
    tilemap.set_tile([0, 0], TileColliderType2D::Grid);
    tilemap.set_tile([1, 0], TileColliderType2D::Grid);
    world.attach_tilemap(comp_col, tilemap);

    assert_eq!(
        world.overlap_point([0.5, 0.25], ContactFilter2D::no_filter()),
        vec![comp_col]
    );

    let player_go = EntityId::new(301);
    let player_body = world.add_body(
        BodyDesc2D::dynamic(player_go, [0.5, -1.2]).with_velocity([0.0, 12.0]),
    );
    let player_col = world.add_collider(
        ColliderDesc2D::box_collider(player_go, [0.3, 0.3]).with_attached_body(player_body),
    );

    let mut passed_up = false;
    for _ in 0..35 {
        world.step(0.02);
        let y = world.body(player_body).map_or(-1.2, |b| b.position[1]);
        let vy = world.body(player_body).map_or(0.0, |b| b.linear_velocity[1]);
        if vy > 0.0 {
            assert!(
                world.drain_messages().is_empty(),
                "jumping upward through composite one-way tilemap platform must fire no events"
            );
        }
        if y > 1.0 {
            passed_up = true;
            break;
        }
    }
    assert!(passed_up, "player must jump up through one-way composite tilemap");

    let mut landed_enter = false;
    for _ in 0..120 {
        world.step(0.02);
        for msg in world.drain_messages() {
            if msg.recipient == player_go && msg.kind == MessageKind2D::CollisionEnter2D {
                landed_enter = true;
            }
        }
    }
    assert!(landed_enter, "player landing on composite tilemap must receive OnCollisionEnter2D");
    assert!(world.is_touching(player_col, comp_col));

    world.remove_tilemap_tile(comp_col, [0, 0]);
    assert!(
        world
            .overlap_point([0.5, 0.25], ContactFilter2D::no_filter())
            .is_empty(),
        "removing tile [0, 0] must remove collision at that tile cell"
    );
    assert_eq!(
        world.overlap_point([-0.5, 0.25], ContactFilter2D::no_filter()),
        vec![comp_col],
        "neighboring tile [-1, 0] must remain solid in composite"
    );
    assert_eq!(
        world.overlap_point([1.5, 0.25], ContactFilter2D::no_filter()),
        vec![comp_col],
        "neighboring tile [1, 0] must remain solid in composite"
    );

    if let Some(b) = world.body_mut(player_body) {
        b.wake_up();
    }
    for _ in 0..40 {
        world.step(0.02);
    }
    let y_after_remove = world.body(player_body).map_or(1.0, |b| b.position[1]);
    assert!(
        y_after_remove < -0.5,
        "player standing over removed tile [0, 0] must fall through gap, got y={y_after_remove}"
    );
}

#[test]
fn separate_tilemap_collider_supports_grid_sprite_and_none_tile_types_and_feeds_composite() {
    use unitas_physics::two_d::{
        CompositeCollider2D, CompositeGenerationType2D, CompositeGeometryType2D, TileColliderType2D,
        TilemapCollider2D,
    };

    let mut world = World2D::with_gravity([0.0, 0.0]);
    let parent_go = EntityId::new(400);
    let child_tm_go = EntityId::new(401);
    let body = world.add_body(BodyDesc2D::static_body(parent_go, [0.0, 0.0]));

    let comp_col = world.add_collider(
        ColliderDesc2D::box_collider(parent_go, [0.0, 0.0]).with_attached_body(body),
    );
    world.attach_composite(
        comp_col,
        CompositeCollider2D::new(
            CompositeGeometryType2D::Polygons,
            CompositeGenerationType2D::Synchronous,
        ),
    );

    let tm_col = world.add_collider(
        ColliderDesc2D::box_collider(child_tm_go, [0.0, 0.0]).with_attached_body(body),
    );
    let mut tilemap = TilemapCollider2D::new([2.0, 2.0]).with_used_by_composite(true);
    tilemap.set_tile([0, 0], TileColliderType2D::Grid);
    tilemap.set_tile(
        [2, 0],
        TileColliderType2D::Sprite {
            vertices: vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]],
        },
    );
    world.attach_tilemap(tm_col, tilemap);

    assert_eq!(
        world.overlap_point([1.0, 1.0], ContactFilter2D::no_filter()),
        vec![comp_col]
    );
    assert_eq!(
        world.overlap_point([5.0, 0.5], ContactFilter2D::no_filter()),
        vec![comp_col]
    );

    world.set_tilemap_tile(tm_col, [0, 0], TileColliderType2D::None);
    assert!(
        world
            .overlap_point([1.0, 1.0], ContactFilter2D::no_filter())
            .is_empty(),
        "setting tile collider type to None must clear tile collision in composite"
    );
    assert_eq!(
        world.overlap_point([5.0, 0.5], ContactFilter2D::no_filter()),
        vec![comp_col],
        "sprite tile at [2, 0] must remain active in composite"
    );
}
