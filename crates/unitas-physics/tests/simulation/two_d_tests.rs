use unitas_core::EntityId;
use unitas_physics::two_d::{
    BodyDesc2D, BodyId2D, BodyType2D, ColliderDesc2D, ConstantForce2D, ContactFilter2D,
    DistanceJoint2D, FixedJoint2D, FrictionJoint2D, HingeJoint2D, Joint2D, JointLimits2D,
    JointMotor2D, MessageKind2D, PhysicsMaterial2D, RelativeJoint2D, Shape2D, SliderJoint2D,
    SpringJoint2D, TargetJoint2D, WheelJoint2D, World2D,
};

#[test]
fn phy_002_story_14_dynamic_body_hitting_static_box_delivers_enter_touching_and_ignore_collision() {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    let floor_go = EntityId::new(1);
    let box_go = EntityId::new(2);

    let floor_body = world.add_body(BodyDesc2D::static_body(floor_go, [0.0, 0.0]));
    let floor_col = world.add_collider(
        ColliderDesc2D::box_collider(floor_go, [5.0, 0.5]).with_attached_body(floor_body),
    );

    let dyn_body =
        world.add_body(BodyDesc2D::dynamic(box_go, [0.0, 0.9]).with_velocity([0.0, -2.0]));
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
fn phy_002_story_2_2d_layer_matrix_gates_collision_callbacks_and_default_mask_queries() {
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
fn the_2d_layer_collision_matrix_decides_which_layer_pairs_collide() {
    let mut world = World2D::with_gravity([0.0, 0.0]);

    let one_go = EntityId::new(400);
    let two_go = EntityId::new(401);
    let three_go = EntityId::new(402);

    let one_body = world.add_body(BodyDesc2D::static_body(one_go, [0.0, 0.0]).with_layer(1));
    let one_col = world.add_collider(
        ColliderDesc2D::circle_collider(one_go, 1.0)
            .with_attached_body(one_body)
            .with_layer(1),
    );
    let two_body = world.add_body(BodyDesc2D::static_body(two_go, [0.0, 0.0]).with_layer(2));
    let two_col = world.add_collider(
        ColliderDesc2D::box_collider(two_go, [1.0, 1.0])
            .with_attached_body(two_body)
            .with_layer(2),
    );
    let three_body = world.add_body(BodyDesc2D::static_body(three_go, [0.0, 0.0]).with_layer(3));
    let three_col = world.add_collider(
        ColliderDesc2D::circle_collider(three_go, 0.75)
            .with_attached_body(three_body)
            .with_layer(3),
    );

    world.step(0.02);
    assert!(world.is_touching(one_col, two_col));
    assert!(world.is_touching(two_col, three_col));
    assert!(world.is_touching(one_col, three_col));

    world.ignore_layer_collision(1, 2, true);
    world.ignore_layer_collision(2, 3, true);
    world.step(0.02);
    assert!(!world.is_touching(one_col, two_col));
    assert!(!world.is_touching(two_col, three_col));
    assert!(world.is_touching(one_col, three_col));

    assert!(world.get_ignore_layer_collision(1, 2));
    assert!(world.get_ignore_layer_collision(2, 1));
    assert!(world.get_ignore_layer_collision(2, 3));
    assert!(world.get_ignore_layer_collision(3, 2));
    assert!(!world.get_ignore_layer_collision(1, 3));

    assert_eq!(world.layer_mask_for_layer(1) & (1 << 2), 0);
    assert_eq!(world.layer_mask_for_layer(1) & (1 << 3), 1 << 3);
    assert_eq!(world.layer_mask_for_layer(2) & (1 << 1), 0);
    assert_eq!(world.layer_mask_for_layer(2) & (1 << 3), 0);
    assert_eq!(world.layer_mask_for_layer(3) & (1 << 1), 1 << 1);

    world.ignore_layer_collision(1, 2, false);
    world.step(0.02);
    assert!(world.is_touching(one_col, two_col));
    assert!(!world.is_touching(two_col, three_col));
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
    world.add_collider(ColliderDesc2D::box_collider(k_go, [1.0, 1.0]).with_attached_body(k_body));
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

    let trig_col =
        world.add_collider(ColliderDesc2D::box_collider(trig_go, [1.0, 1.0]).with_trigger(true));
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
    assert_eq!(world.body(body_id).map(|b| b.position[1]), Some(frozen_y));
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
    assert!(!hits
        .iter()
        .any(|h| h.collider == left_col || h.collider == right_col));

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
fn phy_002_story_5_tilemap_collider_merged_into_composite_updates_when_tile_removed() {
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
    let player_body =
        world.add_body(BodyDesc2D::dynamic(player_go, [0.5, -1.2]).with_velocity([0.0, 12.0]));
    let player_col = world.add_collider(
        ColliderDesc2D::box_collider(player_go, [0.3, 0.3]).with_attached_body(player_body),
    );

    let mut passed_up = false;
    for _ in 0..35 {
        world.step(0.02);
        let y = world.body(player_body).map_or(-1.2, |b| b.position[1]);
        let ascent_messages = world.drain_messages();
        assert!(
            ascent_messages.is_empty(),
            "PHY-002 story 5: jumping upward through a composite one-way tilemap platform must fire no events, got {ascent_messages:?}"
        );
        if y > 1.0 {
            passed_up = true;
            break;
        }
    }
    assert!(
        passed_up,
        "player must jump up through one-way composite tilemap"
    );

    let mut landed_enter = false;
    for _ in 0..120 {
        world.step(0.02);
        for msg in world.drain_messages() {
            if msg.recipient == player_go && msg.kind == MessageKind2D::CollisionEnter2D {
                landed_enter = true;
            }
        }
    }
    assert!(
        landed_enter,
        "player landing on composite tilemap must receive OnCollisionEnter2D"
    );
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
        CompositeCollider2D, CompositeGenerationType2D, CompositeGeometryType2D,
        TileColliderType2D, TilemapCollider2D,
    };

    let mut world = World2D::with_gravity([0.0, 0.0]);
    let parent_go = EntityId::new(400);
    let child_tm_go = EntityId::new(401);
    let body = world.add_body(BodyDesc2D::static_body(parent_go, [0.0, 0.0]));

    let comp_col = world
        .add_collider(ColliderDesc2D::box_collider(parent_go, [0.0, 0.0]).with_attached_body(body));
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

#[test]
fn continuous_collision_detection_prevents_fast_body_tunneling_while_discrete_tunnels() {
    use unitas_physics::two_d::body::CollisionDetectionMode2D;

    let run_with_mode = |mode: CollisionDetectionMode2D| -> (f32, bool) {
        let mut world = World2D::with_gravity([0.0, 0.0]);
        let wall_go = EntityId::new(500);
        let bullet_go = EntityId::new(501);

        let wall_body = world.add_body(BodyDesc2D::static_body(wall_go, [2.0, 0.0]));
        world.add_collider(
            ColliderDesc2D::box_collider(wall_go, [0.05, 3.0]).with_attached_body(wall_body),
        );

        let bullet_body = world.add_body(
            BodyDesc2D::dynamic(bullet_go, [0.0, 0.0])
                .with_velocity([200.0, 0.0])
                .with_collision_detection_mode(mode),
        );
        world.add_collider(
            ColliderDesc2D::circle_collider(bullet_go, 0.2).with_attached_body(bullet_body),
        );

        world.step(0.05);
        let x = world.body(bullet_body).map_or(0.0, |b| b.position[0]);
        let hit_msg = world
            .drain_messages()
            .iter()
            .any(|m| m.recipient == bullet_go && m.kind == MessageKind2D::CollisionEnter2D);
        (x, hit_msg)
    };

    let (discrete_x, discrete_hit) = run_with_mode(CollisionDetectionMode2D::Discrete);
    assert!(
        discrete_x > 5.0 && !discrete_hit,
        "Discrete fast body must tunnel past thin wall at x=2.0, got x={discrete_x}, hit={discrete_hit}"
    );

    let (continuous_x, continuous_hit) = run_with_mode(CollisionDetectionMode2D::Continuous);
    assert!(
        continuous_x < 2.0 && continuous_hit,
        "Continuous fast body must be stopped by thin wall at x=2.0 and emit CollisionEnter2D, got x={continuous_x}, hit={continuous_hit}"
    );
}

#[test]
fn rigidbody2d_sleep_modes_and_interpolation_modes_behave_per_mode() {
    use unitas_physics::two_d::body::{RigidbodyInterpolation2D, RigidbodySleepMode2D};

    let mut world = World2D::with_gravity([0.0, -10.0]);

    let never_go = EntityId::new(600);
    let awake_go = EntityId::new(601);
    let asleep_go = EntityId::new(602);

    let never_body = world.add_body(
        BodyDesc2D::dynamic(never_go, [0.0, 0.0])
            .with_gravity_scale(0.0)
            .with_sleep_mode(RigidbodySleepMode2D::NeverSleep),
    );
    let awake_body = world.add_body(
        BodyDesc2D::dynamic(awake_go, [2.0, 0.0])
            .with_gravity_scale(0.0)
            .with_sleep_mode(RigidbodySleepMode2D::StartAwake),
    );
    let asleep_body = world.add_body(
        BodyDesc2D::dynamic(asleep_go, [4.0, 5.0])
            .with_gravity_scale(1.0)
            .with_sleep_mode(RigidbodySleepMode2D::StartAsleep),
    );

    assert_eq!(world.body(never_body).map(|b| b.is_sleeping()), Some(false));
    assert_eq!(world.body(awake_body).map(|b| b.is_sleeping()), Some(false));
    assert_eq!(world.body(asleep_body).map(|b| b.is_sleeping()), Some(true));

    for _ in 0..35 {
        world.step(0.02);
    }

    assert_eq!(
        world.body(never_body).map(|b| b.is_sleeping()),
        Some(false),
        "NeverSleep body must stay awake even after resting past sleep threshold"
    );
    assert_eq!(
        world.body(awake_body).map(|b| b.is_sleeping()),
        Some(true),
        "StartAwake stationary body must fall asleep after resting past sleep threshold"
    );
    assert_eq!(
        world.body(asleep_body).map(|b| b.position[1]),
        Some(5.0),
        "StartAsleep body must remain asleep and not fall under gravity until woken"
    );

    if let Some(b) = world.body_mut(asleep_body) {
        b.add_impulse([0.0, -2.0]);
    }
    world.step(0.05);
    let asleep_y_after_wake = world.body(asleep_body).map_or(5.0, |b| b.position[1]);
    assert!(
        asleep_y_after_wake < 4.95,
        "StartAsleep body woken by impulse must move on subsequent step, got y={asleep_y_after_wake}"
    );

    let mut interp_world = World2D::with_gravity([0.0, 0.0]);
    interp_world.set_fixed_timestep(0.1);

    let none_body = interp_world.add_body(
        BodyDesc2D::dynamic(EntityId::new(700), [0.0, 0.0])
            .with_velocity([10.0, 0.0])
            .with_interpolation(RigidbodyInterpolation2D::None),
    );
    let interp_body = interp_world.add_body(
        BodyDesc2D::dynamic(EntityId::new(701), [0.0, 0.0])
            .with_velocity([10.0, 0.0])
            .with_interpolation(RigidbodyInterpolation2D::Interpolate),
    );
    let extrap_body = interp_world.add_body(
        BodyDesc2D::dynamic(EntityId::new(702), [0.0, 0.0])
            .with_velocity([10.0, 0.0])
            .with_interpolation(RigidbodyInterpolation2D::Extrapolate),
    );

    let steps = interp_world.advance(0.15);
    assert_eq!(steps, 1);

    let (none_pos, _) = interp_world
        .presented_pose(none_body)
        .unwrap_or(([0.0, 0.0], 0.0));
    let (interp_pos, _) = interp_world
        .presented_pose(interp_body)
        .unwrap_or(([0.0, 0.0], 0.0));
    let (extrap_pos, _) = interp_world
        .presented_pose(extrap_body)
        .unwrap_or(([0.0, 0.0], 0.0));

    assert!(
        (none_pos[0] - 1.0).abs() < 1e-4,
        "Interpolation::None must present exact simulated position 1.0, got {}",
        none_pos[0]
    );
    assert!(
        (interp_pos[0] - 0.5).abs() < 1e-4,
        "Interpolation::Interpolate at alpha=0.5 between 0.0 and 1.0 must present 0.5, got {}",
        interp_pos[0]
    );
    assert!(
        (extrap_pos[0] - 1.5).abs() < 1e-4,
        "Interpolation::Extrapolate at alpha=0.5 (dt_rem=0.05) from 1.0 with v=10.0 must present 1.5, got {}",
        extrap_pos[0]
    );
}

#[test]
fn phy_002_story_2_2d_layer_matrix_gates_trigger_callbacks_and_default_mask_overlap_queries() {
    use unitas_physics::matrix::CollisionMatrix;

    let mut world = World2D::with_gravity([0.0, 0.0]);
    let mut matrix = CollisionMatrix::new();
    matrix.disable_pair(1, 2);
    world.set_collision_matrix(matrix);

    let trigger_go = EntityId::new(810);
    let mover_go = EntityId::new(811);

    let trigger_col = world.add_collider(
        ColliderDesc2D::box_collider(trigger_go, [1.0, 1.0])
            .with_trigger(true)
            .with_layer(1),
    );
    let mover_body = world.add_body(BodyDesc2D::dynamic(mover_go, [0.0, 0.0]).with_layer(2));
    let mover_col = world.add_collider(
        ColliderDesc2D::circle_collider(mover_go, 0.4)
            .with_attached_body(mover_body)
            .with_layer(2),
    );

    world.step(0.02);
    assert!(
        world.drain_messages().is_empty(),
        "PHY-002 story 2: a disabled layer pair must stop 2D trigger callbacks"
    );
    assert!(!world.is_touching(trigger_col, mover_col));

    let ray_hits = world.raycast_default_layer([-5.0, 0.0], [1.0, 0.0], 20.0, 1);
    assert!(
        ray_hits.iter().all(|hit| hit.collider != mover_col),
        "PHY-002 story 2: a disabled layer pair must drop that layer from default-mask 2D raycasts"
    );
    assert!(
        !world
            .overlap_circle_default_layer([0.0, 0.0], 0.1, 1)
            .contains(&mover_col),
        "PHY-002 story 2: a disabled layer pair must drop that layer from default-mask 2D overlap queries"
    );

    world.ignore_layer_collision(1, 2, false);
    world.step(0.02);
    let messages = world.drain_messages();
    assert!(
        messages
            .iter()
            .any(|message| message.kind == MessageKind2D::TriggerEnter2D),
        "PHY-002 story 2: re-enabling the layer pair must restore 2D trigger callbacks"
    );
    assert!(world.is_touching(trigger_col, mover_col));
    assert!(
        world
            .overlap_circle_default_layer([0.0, 0.0], 0.1, 1)
            .contains(&mover_col),
        "PHY-002 story 2: re-enabling the layer pair must restore that layer in default-mask 2D overlap queries"
    );
}

#[test]
fn phy_002_story_2_2d_collider_without_material_uses_the_default_physics_material() {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    world.set_default_material(PhysicsMaterial2D::new(0.8, 0.4));

    let default_go = EntityId::new(820);
    let explicit_go = EntityId::new(821);
    let default_col = world.add_collider(ColliderDesc2D::box_collider(default_go, [1.0, 1.0]));
    let explicit_col = world.add_collider(
        ColliderDesc2D::box_collider(explicit_go, [1.0, 1.0])
            .with_material(PhysicsMaterial2D::new(0.25, 0.9)),
    );

    assert_eq!(
        world
            .collider(default_col)
            .map(|collider| collider.material),
        Some(PhysicsMaterial2D::new(0.8, 0.4)),
        "PHY-002 story 2: a 2D collider with no material must use the world default material"
    );
    assert_eq!(
        world
            .collider(explicit_col)
            .map(|collider| collider.material),
        Some(PhysicsMaterial2D::new(0.25, 0.9))
    );

    world.set_default_material(PhysicsMaterial2D::new(0.1, 0.2));
    assert_eq!(
        world.collider(default_col).map(|collider| collider.material),
        Some(PhysicsMaterial2D::new(0.1, 0.2)),
        "PHY-002 story 2: a new world default material must reach 2D colliders that use the default"
    );
    assert_eq!(
        world.collider(explicit_col).map(|collider| collider.material),
        Some(PhysicsMaterial2D::new(0.25, 0.9)),
        "PHY-002 story 2: an explicit 2D collider material must survive a world default material change"
    );
}

#[test]
fn phy_002_story_2_auto_sync_transforms_off_hides_moved_2d_collider_from_queries_until_sync_or_step(
) {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    world.set_auto_sync_transforms(false);
    assert!(!world.auto_sync_transforms());

    let go = EntityId::new(830);
    let body = world.add_body(BodyDesc2D::kinematic(go, [0.0, 0.0]));
    let col =
        world.add_collider(ColliderDesc2D::box_collider(go, [0.5, 0.5]).with_attached_body(body));

    assert_eq!(
        world.overlap_point([0.0, 0.0], ContactFilter2D::no_filter()),
        vec![col],
        "PHY-002 story 2: initial 2D collider pose must be visible to queries"
    );

    world.set_body_position(body, [5.0, 0.0]);
    assert_eq!(
        world.overlap_point([0.0, 0.0], ContactFilter2D::no_filter()),
        vec![col],
        "PHY-002 story 2: with auto_sync_transforms off, queries must still see the old 2D position before SyncTransforms"
    );
    assert!(
        world
            .overlap_point([5.0, 0.0], ContactFilter2D::no_filter())
            .is_empty(),
        "PHY-002 story 2: with auto_sync_transforms off, moved 2D collider must not be visible at new position before SyncTransforms"
    );
    assert!(
        world
            .raycast([5.0, 3.0], [0.0, -1.0], 5.0, ContactFilter2D::no_filter())
            .is_empty(),
        "PHY-002 story 2: with auto_sync_transforms off, 2D raycast must not hit moved collider before SyncTransforms"
    );

    world.sync_transforms();
    assert!(
        world
            .overlap_point([0.0, 0.0], ContactFilter2D::no_filter())
            .is_empty(),
        "PHY-002 story 2: after SyncTransforms, old 2D position must no longer hit"
    );
    assert_eq!(
        world.overlap_point([5.0, 0.0], ContactFilter2D::no_filter()),
        vec![col],
        "PHY-002 story 2: after SyncTransforms, moved 2D collider must be visible at new position"
    );

    world.set_body_position(body, [10.0, 0.0]);
    assert!(
        world
            .overlap_point([10.0, 0.0], ContactFilter2D::no_filter())
            .is_empty(),
        "PHY-002 story 2: second 2D transform move with auto_sync_transforms off must stay hidden until step"
    );
    world.step(0.02);
    assert_eq!(
        world.overlap_point([10.0, 0.0], ContactFilter2D::no_filter()),
        vec![col],
        "PHY-002 story 2: simulation step must synchronize moved 2D collider for subsequent queries"
    );
}

#[test]
fn phy_002_story_5_constant_force_2d_and_2d_joints_constrain_and_break() {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    phy_002_story_5_constant_force(&mut world);
    let anchor_b = phy_002_story_5_joints_constrain(&mut world);
    phy_002_story_5_motor_joints(&mut world, anchor_b);
    phy_002_story_5_motorized_joints(&mut world, anchor_b);
    phy_002_story_5_joint_breaks(&mut world, anchor_b);
}

fn phy_002_story_5_constant_force(world: &mut World2D) {
    let cf_go = EntityId::new(900);
    let cf_body = world.add_body(
        BodyDesc2D::dynamic(cf_go, [0.0, 0.0]).with_rotation(std::f32::consts::FRAC_PI_2),
    );
    world.attach_constant_force(
        cf_body,
        ConstantForce2D {
            force: [4.0, 0.0],
            relative_force: [2.0, 0.0],
            torque: 3.0,
        },
    );
    world.step(0.5);
    let b_vel = world
        .body(cf_body)
        .map_or([0.0, 0.0], |b| b.linear_velocity);
    let b_ang = world.body(cf_body).map_or(0.0, |b| b.angular_velocity);
    assert!(
        (b_vel[0] - 2.0).abs() < 1e-3 && (b_vel[1] - 1.0).abs() < 1e-3,
        "PHY-002 story 5: ConstantForce2D must apply world force and rotated relative_force every step, got {b_vel:?}"
    );
    assert!(
        (b_ang - 1.4625).abs() < 1e-3,
        "PHY-002 story 5: ConstantForce2D must apply torque every step, got {b_ang}"
    );
}

fn phy_002_story_5_joints_constrain(world: &mut World2D) -> BodyId2D {
    let anchor_b = world.add_body(BodyDesc2D::static_body(EntityId::new(901), [0.0, 0.0]));
    let orb_b = world.add_body(BodyDesc2D::dynamic(EntityId::new(902), [5.0, 0.0]));
    let dist_j = world.add_joint(Joint2D::Distance(DistanceJoint2D {
        body_a: anchor_b,
        body_b: Some(orb_b),
        anchor_a: [0.0, 0.0],
        anchor_b: [0.0, 0.0],
        distance: 2.0,
        max_distance_only: false,
        break_force: f32::INFINITY,
    }));
    world.step(0.05);
    let orb_x = world.body(orb_b).map_or(5.0, |b| b.position[0]);
    assert!(
        (orb_x - 2.0).abs() < 0.1,
        "PHY-002 story 5: DistanceJoint2D must pull body to target distance 2.0, got {orb_x}"
    );
    assert!(world.joint(dist_j).is_some());

    let spr_b = world.add_body(BodyDesc2D::dynamic(EntityId::new(903), [4.0, 0.0]));
    world.add_joint(Joint2D::Spring(SpringJoint2D {
        body_a: anchor_b,
        body_b: Some(spr_b),
        anchor_a: [0.0, 0.0],
        anchor_b: [0.0, 0.0],
        distance: 1.0,
        damping_ratio: 0.5,
        frequency: 4.0,
        break_force: f32::INFINITY,
    }));
    world.step(0.05);
    assert!(
        world.body(spr_b).map_or(4.0, |b| b.position[0]) < 3.9,
        "PHY-002 story 5: SpringJoint2D must accelerate stretched body toward rest distance"
    );

    let tgt_b = world.add_body(BodyDesc2D::dynamic(EntityId::new(904), [0.0, 0.0]));
    world.add_joint(Joint2D::Target(TargetJoint2D {
        body: tgt_b,
        anchor: [0.0, 0.0],
        target: [3.0, 4.0],
        max_force: 100.0,
        frequency: 5.0,
        damping_ratio: 0.7,
        break_force: f32::INFINITY,
    }));
    world.step(0.05);
    let tgt_pos = world.body(tgt_b).map_or([0.0, 0.0], |b| b.position);
    assert!(
        tgt_pos[0] > 0.1 && tgt_pos[1] > 0.1,
        "PHY-002 story 5: TargetJoint2D must pull body toward world target"
    );

    let fix_b =
        world.add_body(BodyDesc2D::dynamic(EntityId::new(905), [3.0, -2.0]).with_rotation(1.2));
    world.add_joint(Joint2D::Fixed(FixedJoint2D {
        body_a: anchor_b,
        body_b: Some(fix_b),
        local_offset: [1.0, 2.0],
        reference_angle: 0.25,
        damping_ratio: 1.0,
        frequency: 0.0,
        break_force: f32::INFINITY,
        break_torque: f32::INFINITY,
    }));
    world.step(0.05);
    let fb_pos = world.body(fix_b).map_or([0.0, 0.0], |b| b.position);
    let fb_rot = world.body(fix_b).map_or(0.0, |b| b.rotation);
    assert!(
        (fb_pos[0] - 1.0).abs() < 1e-3
            && (fb_pos[1] - 2.0).abs() < 1e-3
            && (fb_rot - 0.25).abs() < 1e-3,
        "PHY-002 story 5: FixedJoint2D must lock relative offset and rotation"
    );
    anchor_b
}

fn phy_002_story_5_motor_joints(world: &mut World2D, anchor_b: BodyId2D) {
    let rel_b = world.add_body(BodyDesc2D::dynamic(EntityId::new(906), [5.0, 5.0]));
    world.add_joint(Joint2D::Relative(RelativeJoint2D {
        body_a: anchor_b,
        body_b: Some(rel_b),
        linear_offset: [-1.0, 0.5],
        angular_offset: -0.5,
        max_force: 500.0,
        max_torque: 500.0,
        correction_scale: 1.0,
        break_force: f32::INFINITY,
        break_torque: f32::INFINITY,
    }));
    world.step(0.05);
    let rb_x = world.body(rel_b).map_or(5.0, |b| b.position[0]);
    let rb_rot = world.body(rel_b).map_or(0.0, |b| b.rotation);
    assert!(
        rb_x < 4.5 && rb_rot < 0.0,
        "PHY-002 story 5: RelativeJoint2D must drive body toward linear and angular offsets"
    );
}

fn phy_002_story_5_motorized_joints(world: &mut World2D, anchor_b: BodyId2D) {
    let fric_b = world
        .add_body(BodyDesc2D::dynamic(EntityId::new(907), [0.0, 0.0]).with_velocity([10.0, 0.0]));
    if let Some(b) = world.body_mut(fric_b) {
        b.angular_velocity = 8.0;
    }
    world.add_joint(Joint2D::Friction(FrictionJoint2D {
        body_a: anchor_b,
        body_b: Some(fric_b),
        max_force: 10.0,
        max_torque: 6.0,
        break_force: f32::INFINITY,
        break_torque: f32::INFINITY,
    }));
    world.step(0.1);
    let frb_vx = world.body(fric_b).map_or(10.0, |b| b.linear_velocity[0]);
    let frb_w = world.body(fric_b).map_or(8.0, |b| b.angular_velocity);
    assert!(
        frb_vx < 9.5 && frb_w < 7.6,
        "PHY-002 story 5: FrictionJoint2D must decelerate linear and angular velocity"
    );

    let hinge_b = world.add_body(BodyDesc2D::dynamic(EntityId::new(908), [0.0, 0.0]));
    world.add_joint(Joint2D::Hinge(HingeJoint2D {
        body_a: anchor_b,
        body_b: Some(hinge_b),
        anchor_a: [0.0, 0.0],
        anchor_b: [0.0, 0.0],
        use_motor: true,
        motor: JointMotor2D {
            motor_speed: 10.0,
            max_motor_torque: 100.0,
        },
        use_limits: true,
        limits: JointLimits2D {
            min: -0.2,
            max: 0.3,
        },
        break_force: f32::INFINITY,
        break_torque: f32::INFINITY,
    }));
    world.step(0.1);
    let hb_rot = world.body(hinge_b).map_or(0.0, |b| b.rotation);
    assert!(
        (hb_rot - 0.3).abs() < 1e-3,
        "PHY-002 story 5: HingeJoint2D motor and upper angular limit must clamp rotation to 0.3, got {hb_rot}"
    );

    let slide_b = world.add_body(BodyDesc2D::dynamic(EntityId::new(909), [0.0, 3.0]));
    world.add_joint(Joint2D::Slider(SliderJoint2D {
        body_a: anchor_b,
        body_b: Some(slide_b),
        anchor_a: [0.0, 0.0],
        anchor_b: [0.0, 0.0],
        angle_degrees: 0.0,
        use_motor: true,
        motor: JointMotor2D {
            motor_speed: 20.0,
            max_motor_torque: 500.0,
        },
        use_limits: true,
        limits: JointLimits2D {
            min: -1.0,
            max: 1.5,
        },
        break_force: f32::INFINITY,
        break_torque: f32::INFINITY,
    }));
    world.step(0.1);
    let sb_pos = world.body(slide_b).map_or([0.0, 3.0], |b| b.position);
    assert!(
        sb_pos[1].abs() < 1e-3 && (sb_pos[0] - 1.5).abs() < 1e-2,
        "PHY-002 story 5: SliderJoint2D must project body onto slide axis and clamp translation to limit 1.5, got {sb_pos:?}"
    );

    let wheel_b = world.add_body(BodyDesc2D::dynamic(EntityId::new(910), [0.0, -2.0]));
    world.add_joint(Joint2D::Wheel(WheelJoint2D {
        body_a: anchor_b,
        body_b: Some(wheel_b),
        anchor_a: [0.0, 0.0],
        anchor_b: [0.0, 0.0],
        suspension_angle_degrees: 90.0,
        suspension_frequency: 6.0,
        suspension_damping_ratio: 0.7,
        use_motor: true,
        motor: JointMotor2D {
            motor_speed: 6.0,
            max_motor_torque: 50.0,
        },
        break_force: f32::INFINITY,
        break_torque: f32::INFINITY,
    }));
    world.step(0.1);
    let wb_y = world.body(wheel_b).map_or(-2.0, |b| b.position[1]);
    let wb_w = world.body(wheel_b).map_or(0.0, |b| b.angular_velocity);
    assert!(
        wb_y > -2.0 && wb_w > 1.0,
        "PHY-002 story 5: WheelJoint2D suspension and motor must pull wheel toward anchor and spin it"
    );
}

fn phy_002_story_5_joint_breaks(world: &mut World2D, anchor_b: BodyId2D) {
    let fragile_b = world.add_body(BodyDesc2D::dynamic(EntityId::new(911), [10.0, 0.0]));
    let fragile_j = world.add_joint(Joint2D::Distance(DistanceJoint2D {
        body_a: anchor_b,
        body_b: Some(fragile_b),
        anchor_a: [0.0, 0.0],
        anchor_b: [0.0, 0.0],
        distance: 1.0,
        max_distance_only: false,
        break_force: 5.0,
    }));
    world.step(0.02);
    assert!(
        world.joint(fragile_j).is_none(),
        "PHY-002 story 5: 2D joint exceeding break_force must break and be removed"
    );
    assert_eq!(world.drain_broken_joints(), vec![fragile_j]);
}

#[test]
fn matrix_2d_disabled_layer_pair_produces_no_contacts_and_is_ignored_by_box_cast_with_default_filter_and_honors_queries_hit_triggers(
) {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    assert_eq!(world.velocity_iterations(), 8);
    assert_eq!(world.position_iterations(), 3);
    assert!(world.queries_hit_triggers());

    world.set_velocity_iterations(12);
    world.set_position_iterations(4);
    assert_eq!(world.velocity_iterations(), 12);
    assert_eq!(world.position_iterations(), 4);

    let go_default = EntityId::new(1000);
    let go_disabled = EntityId::new(1001);
    let go_trigger = EntityId::new(1002);

    let body_default = world.add_body(BodyDesc2D::dynamic(go_default, [0.0, 0.0]).with_layer(0));
    let col_default = world.add_collider(
        ColliderDesc2D::box_collider(go_default, [0.5, 0.5])
            .with_attached_body(body_default)
            .with_layer(0),
    );

    let body_disabled = world.add_body(BodyDesc2D::dynamic(go_disabled, [0.2, 0.0]).with_layer(3));
    let col_disabled = world.add_collider(
        ColliderDesc2D::box_collider(go_disabled, [0.5, 0.5])
            .with_attached_body(body_disabled)
            .with_layer(3),
    );

    let col_trigger = world.add_collider(
        ColliderDesc2D::box_collider(go_trigger, [0.5, 0.5])
            .with_offset([3.0, 0.0])
            .with_trigger(true)
            .with_layer(0),
    );

    world.ignore_layer_collision(0, 3, true);
    world.step(0.02);

    assert!(
        !world.is_touching(col_default, col_disabled),
        "disabled layer pair (0, 3) must produce no contacts"
    );
    assert!(
        world.get_contacts(col_default).is_empty(),
        "disabled layer pair (0, 3) must record no contact points"
    );
    assert!(
        world.drain_messages().is_empty(),
        "disabled layer pair (0, 3) must emit no collision callbacks"
    );

    let default_box_hits = world.box_cast(
        [-3.0, 0.0],
        [0.25, 0.25],
        0.0,
        [1.0, 0.0],
        10.0,
        ContactFilter2D::default(),
    );
    let hit_colliders: Vec<_> = default_box_hits.iter().map(|h| h.collider).collect();
    assert!(
        !hit_colliders.contains(&col_disabled),
        "disabled matrix pair (0, 3) must be ignored by BoxCast with ContactFilter2D::default(), got {hit_colliders:?}"
    );
    assert!(
        hit_colliders.contains(&col_default),
        "enabled layer 0 collider must be hit by BoxCast with default filter"
    );
    assert!(
        hit_colliders.contains(&col_trigger),
        "with queries_hit_triggers=true, trigger collider must be hit by BoxCast with default filter"
    );

    world.set_queries_hit_triggers(false);
    assert!(!world.queries_hit_triggers());
    let no_trigger_box_hits = world.box_cast(
        [-3.0, 0.0],
        [0.25, 0.25],
        0.0,
        [1.0, 0.0],
        10.0,
        ContactFilter2D::default(),
    );
    assert!(
        no_trigger_box_hits
            .iter()
            .all(|h| h.collider != col_trigger),
        "with queries_hit_triggers=false, trigger collider must be ignored by BoxCast with default filter"
    );
    assert!(
        world
            .overlap_point([3.0, 0.0], ContactFilter2D::default())
            .is_empty(),
        "with queries_hit_triggers=false, OverlapPoint with default filter must ignore trigger collider"
    );
}
