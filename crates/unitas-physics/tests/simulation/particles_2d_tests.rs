use unitas_core::EntityId;
use unitas_physics::particles::{EmitParams, ParticleSystem2D, StopAction};
use unitas_physics::two_d::{ColliderDesc2D, World2D};

fn add_circle(
    world: &mut World2D,
    game_object: EntityId,
    position: [f32; 2],
    radius: f32,
    layer: u8,
    is_trigger: bool,
) {
    world.add_collider(
        ColliderDesc2D::circle_collider(game_object, radius)
            .with_offset(position)
            .with_layer(layer)
            .with_trigger(is_trigger),
    );
}

#[test]
fn a_particle_overlapping_a_solid_2d_collider_reports_a_collision_event() {
    let mut world = World2D::new();
    let solid = EntityId::new(1);
    add_circle(&mut world, solid, [0.0, 0.0], 1.0, 1, false);

    let mut system = ParticleSystem2D::new(16);
    system.system_mut().emit(1);
    system.update_collisions(&world);

    let events = system.get_collision_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].entity, solid);
    assert_eq!(events[0].particle_index, 0);
    assert_eq!(events[0].position, [0.0, 0.0]);
    assert!(system.get_trigger_events().is_empty());
}

#[test]
fn a_particle_overlapping_a_2d_trigger_collider_reports_a_trigger_event() {
    let mut world = World2D::new();
    let trigger = EntityId::new(2);
    add_circle(&mut world, trigger, [0.0, 0.0], 1.0, 1, true);

    let mut system = ParticleSystem2D::new(17);
    system.system_mut().emit(1);
    system.update_collisions(&world);

    let events = system.get_trigger_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].entity, trigger);
    assert_eq!(events[0].particle_index, 0);
    assert_eq!(events[0].position, [0.0, 0.0]);
    assert!(system.get_collision_events().is_empty());
}

#[test]
fn a_2d_particle_collision_reports_the_normal_from_the_collider_center() {
    let mut world = World2D::new();
    add_circle(&mut world, EntityId::new(3), [0.0, 0.0], 1.0, 1, false);

    let mut system = ParticleSystem2D::new(18);
    system.system_mut().set_emit_position([0.75, 0.0, 0.0]);
    system.system_mut().emit(1);
    system.update_collisions(&world);

    let events = system.get_collision_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].position, [0.75, 0.0]);
    assert!((events[0].normal[0] - 1.0).abs() < 1e-5);
    assert!(events[0].normal[1].abs() < 1e-5);
}

#[test]
fn the_2d_collision_layer_mask_selects_the_colliders_a_particle_hits() {
    let mut world = World2D::new();
    let on_layer_one = EntityId::new(4);
    let on_layer_two = EntityId::new(5);
    add_circle(&mut world, on_layer_one, [0.0, 0.0], 1.0, 1, false);
    add_circle(&mut world, on_layer_two, [0.0, 0.0], 1.0, 2, false);

    let mut system = ParticleSystem2D::new(19);
    system.system_mut().emit(1);
    system.update_collisions(&world);

    let entities: Vec<EntityId> = system
        .get_collision_events()
        .iter()
        .map(|event| event.entity)
        .collect();
    assert_eq!(entities.len(), 2);
    assert!(entities.contains(&on_layer_one));
    assert!(entities.contains(&on_layer_two));

    system.set_collision_layer_mask(1 << 2);
    assert_eq!(system.collision_layer_mask(), 1 << 2);
    system.update_collisions(&world);

    let entities: Vec<EntityId> = system
        .get_collision_events()
        .iter()
        .map(|event| event.entity)
        .collect();
    assert_eq!(entities, vec![on_layer_two]);
}

#[test]
fn a_particle_away_from_the_2d_world_reports_no_events() {
    let mut world = World2D::new();
    add_circle(&mut world, EntityId::new(6), [0.0, 0.0], 1.0, 1, false);

    let mut system = ParticleSystem2D::new(20);
    system.system_mut().set_emit_position([100.0, 0.0, 0.0]);
    system.system_mut().emit(1);
    system.update_collisions(&world);

    assert!(system.get_collision_events().is_empty());
    assert!(system.get_trigger_events().is_empty());
}

#[test]
fn a_2d_particle_without_a_usable_size_reports_no_events() {
    let mut world = World2D::new();
    add_circle(&mut world, EntityId::new(7), [0.0, 0.0], 1.0, 1, false);

    let mut zero_sized = ParticleSystem2D::new(21);
    zero_sized.system_mut().set_emit_params(EmitParams {
        start_size: 0.0,
        ..EmitParams::default()
    });
    zero_sized.system_mut().emit(1);
    zero_sized.update_collisions(&world);
    assert!(zero_sized.get_collision_events().is_empty());

    let mut nan_sized = ParticleSystem2D::new(22);
    nan_sized.system_mut().set_emit_params(EmitParams {
        start_size: f32::NAN,
        ..EmitParams::default()
    });
    nan_sized.system_mut().emit(1);
    nan_sized.update_collisions(&world);
    assert!(nan_sized.get_collision_events().is_empty());
}

#[test]
fn a_disabled_2d_particle_system_reports_no_events() {
    let mut world = World2D::new();
    add_circle(&mut world, EntityId::new(8), [0.0, 0.0], 1.0, 1, false);

    let mut system = ParticleSystem2D::new(23);
    system.system_mut().stop(StopAction::Disable);
    system.system_mut().emit(1);
    system.update_collisions(&world);

    assert!(system.is_disabled());
    assert!(system.get_collision_events().is_empty());
    assert!(system.get_trigger_events().is_empty());
}
