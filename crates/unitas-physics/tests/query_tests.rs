use unitas_core::EntityId;
use unitas_physics::{BodyDesc, CollisionEvent, Phase, PhysicsError, RaycastHit, Shape, World};

const LAYER_ONE: u32 = 1;
const LAYER_TWO: u32 = 2;

fn sphere(position: [f64; 3], radius: f64, layer: u32) -> BodyDesc {
    BodyDesc {
        position,
        mass: 1.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        shape: Shape::Sphere { radius },
        layer,
    }
}

fn box_body(position: [f64; 3], half_extents: [f64; 3], layer: u32) -> BodyDesc {
    BodyDesc {
        position,
        mass: 0.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        shape: Shape::Box { half_extents },
        layer,
    }
}

#[test]
fn ray_hits_a_sphere_on_its_surface() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert_eq!(
        hits,
        vec![RaycastHit {
            entity,
            distance: 4.0,
            point: [-1.0, 0.0, 0.0]
        }]
    );
    Ok(())
}

#[test]
fn ray_direction_length_does_not_change_the_hit() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.raycast([-5.0, 0.0, 0.0], [7.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert_eq!(
        hits,
        vec![RaycastHit {
            entity,
            distance: 4.0,
            point: [-1.0, 0.0, 0.0]
        }]
    );
    Ok(())
}

#[test]
fn ray_hits_a_box_on_the_entry_face() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        box_body([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], LAYER_ONE),
    )?;

    let hits = world.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert_eq!(
        hits,
        vec![RaycastHit {
            entity,
            distance: 4.0,
            point: [-1.0, 0.0, 0.0]
        }]
    );
    Ok(())
}

#[test]
fn ray_hits_a_box_along_the_z_axis() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        box_body([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], LAYER_ONE),
    )?;

    let hits = world.raycast([0.0, 0.0, -5.0], [0.0, 0.0, 1.0], 100.0, LAYER_ONE);

    assert_eq!(
        hits,
        vec![RaycastHit {
            entity,
            distance: 4.0,
            point: [0.0, 0.0, -1.0]
        }]
    );
    Ok(())
}

#[test]
fn ray_hits_a_box_along_the_y_axis() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        box_body([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], LAYER_ONE),
    )?;

    let hits = world.raycast([0.0, -5.0, 0.0], [0.0, 1.0, 0.0], 100.0, LAYER_ONE);

    assert_eq!(
        hits,
        vec![RaycastHit {
            entity,
            distance: 4.0,
            point: [0.0, -1.0, 0.0]
        }]
    );
    Ok(())
}

#[test]
fn ray_misses_a_box_when_another_axis_leaves_the_slabs() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        box_body([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], LAYER_ONE),
    )?;

    let hits = world.raycast([-5.0, 2.5, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn hit_beyond_the_max_distance_is_not_reported() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 3.0, LAYER_ONE);

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn bodies_outside_the_layer_mask_are_not_hit() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_TWO))?;

    let hits = world.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn bodies_inside_the_layer_mask_are_hit() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].entity, entity);
    Ok(())
}

#[test]
fn a_body_sharing_a_layer_bit_with_the_mask_is_hit() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, sphere([0.0, 0.0, 0.0], 1.0, 3))?;

    let hits = world.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].entity, entity);
    Ok(())
}

#[test]
fn an_empty_layer_mask_hits_nothing() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;
    world.add_body(EntityId::new(2), sphere([0.0, 0.0, 0.0], 1.0, LAYER_TWO))?;

    let hits = world.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, 0);

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn a_ray_tangent_to_a_sphere_does_not_hit() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.raycast([-5.0, 1.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn a_ray_traveling_along_a_box_face_does_not_hit() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        box_body([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], LAYER_ONE),
    )?;

    let hits = world.raycast([-5.0, 1.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn a_ray_grazing_a_box_edge_does_not_hit() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        box_body([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], LAYER_ONE),
    )?;

    let hits = world.raycast([-2.0, 0.0, 5.0], [3.0, 0.0, -4.0], 100.0, LAYER_ONE);

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn a_ray_starting_inside_a_sphere_hits_at_the_origin() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.raycast([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert_eq!(
        hits,
        vec![RaycastHit {
            entity,
            distance: 0.0,
            point: [0.0, 0.0, 0.0]
        }]
    );
    Ok(())
}

#[test]
fn a_ray_starting_inside_a_box_hits_at_the_origin() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        box_body([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], LAYER_ONE),
    )?;

    let hits = world.raycast([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert_eq!(
        hits,
        vec![RaycastHit {
            entity,
            distance: 0.0,
            point: [0.0, 0.0, 0.0]
        }]
    );
    Ok(())
}

#[test]
fn a_zero_length_direction_hits_nothing() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.raycast([-5.0, 0.0, 0.0], [0.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn a_non_positive_max_distance_hits_nothing() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    assert!(world
        .raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.0, LAYER_ONE)
        .is_empty());
    assert!(world
        .raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], -1.0, LAYER_ONE)
        .is_empty());
    assert!(world
        .raycast([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.0, LAYER_ONE)
        .is_empty());
    Ok(())
}

#[test]
fn hits_are_sorted_by_ascending_distance() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let far = EntityId::new(1);
    let middle = EntityId::new(2);
    let near = EntityId::new(3);
    world.add_body(far, sphere([6.0, 0.0, 0.0], 0.5, LAYER_ONE))?;
    world.add_body(middle, sphere([3.0, 0.0, 0.0], 0.5, LAYER_ONE))?;
    world.add_body(near, sphere([0.0, 0.0, 0.0], 0.5, LAYER_ONE))?;

    let hits = world.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    let entities: Vec<EntityId> = hits.iter().map(|hit| hit.entity).collect();
    let distances: Vec<f64> = hits.iter().map(|hit| hit.distance).collect();
    assert_eq!(entities, vec![near, middle, far]);
    assert_eq!(distances, vec![4.5, 7.5, 10.5]);
    Ok(())
}

#[test]
fn overlap_sphere_returns_the_overlapping_bodies_of_the_masked_layers() -> Result<(), PhysicsError>
{
    let mut world = World::new();
    let overlapping = EntityId::new(1);
    let touching = EntityId::new(2);
    let apart = EntityId::new(3);
    let boxed = EntityId::new(4);
    world.add_body(overlapping, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;
    world.add_body(touching, sphere([2.25, 0.0, 0.0], 1.0, LAYER_ONE))?;
    world.add_body(apart, sphere([10.0, 0.0, 0.0], 1.0, LAYER_ONE))?;
    world.add_body(boxed, box_body([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], LAYER_TWO))?;

    let on_first_layer = world.overlap_sphere([0.75, 0.0, 0.0], 0.5, LAYER_ONE);
    let on_second_layer = world.overlap_sphere([0.75, 0.0, 0.0], 0.5, LAYER_TWO);
    let on_no_layer = world.overlap_sphere([0.75, 0.0, 0.0], 0.5, 0);

    assert_eq!(on_first_layer, vec![overlapping]);
    assert_eq!(on_second_layer, vec![boxed]);
    assert!(on_no_layer.is_empty());
    Ok(())
}

#[test]
fn layers_do_not_filter_collision_events() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let left = EntityId::new(1);
    let right = EntityId::new(2);
    world.add_body(left, sphere([-0.4, 0.0, 0.0], 0.5, LAYER_ONE))?;
    world.add_body(right, sphere([0.4, 0.0, 0.0], 0.5, LAYER_TWO))?;

    world.step(0.1)?;

    assert_eq!(
        world.drain_events(),
        vec![CollisionEvent {
            a: left,
            b: right,
            phase: Phase::Enter
        }]
    );
    Ok(())
}
