use unitas_core::EntityId;
use unitas_physics::{
    BodyDesc, BodyRole, CollisionEvent, Phase, PhysicsError, QueryHit, Shape, World,
};

const LAYER_ONE: u32 = 1;
const LAYER_TWO: u32 = 2;

const ROTATED_45_DEGREES_ABOUT_Z: [f64; 4] = [0.0, 0.0, 0.38268343, 0.92387953];

fn sphere(position: [f64; 3], radius: f64, layer: u32) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        shape: Shape::Sphere { radius },
        layer,
        role: BodyRole::Dynamic,
        friction: 0.6,
        restitution: 0.0,
    }
}

fn box_body(position: [f64; 3], half_extents: [f64; 3], layer: u32) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 0.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        shape: Shape::Box { half_extents },
        layer,
        role: BodyRole::Static,
        friction: 0.6,
        restitution: 0.0,
    }
}

fn trigger_sphere(position: [f64; 3], radius: f64, layer: u32) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 0.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: true,
        shape: Shape::Sphere { radius },
        layer,
        role: BodyRole::Static,
        friction: 0.6,
        restitution: 0.0,
    }
}

fn oriented_box(
    position: [f64; 3],
    rotation: [f64; 4],
    half_extents: [f64; 3],
    layer: u32,
) -> BodyDesc {
    BodyDesc {
        position,
        rotation,
        mass: 0.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        shape: Shape::Box { half_extents },
        layer,
        role: BodyRole::Static,
        friction: 0.6,
        restitution: 0.0,
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
        vec![QueryHit {
            entity,
            point: [-1.0, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 4.0
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
        vec![QueryHit {
            entity,
            point: [-1.0, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 4.0
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
        vec![QueryHit {
            entity,
            point: [-1.0, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 4.0
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
        vec![QueryHit {
            entity,
            point: [0.0, 0.0, -1.0],
            normal: [0.0, 0.0, -1.0],
            distance: 4.0
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
        vec![QueryHit {
            entity,
            point: [0.0, -1.0, 0.0],
            normal: [0.0, -1.0, 0.0],
            distance: 4.0
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
fn a_ray_starting_inside_a_sphere_hits_the_exit_surface_with_inward_normal() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.raycast([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert_eq!(
        hits,
        vec![QueryHit {
            entity,
            point: [1.0, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 1.0,
        }]
    );
    Ok(())
}

#[test]
fn a_ray_starting_inside_a_box_hits_the_exit_face_with_inward_normal() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        box_body([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], LAYER_ONE),
    )?;

    let hits = world.raycast([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert_eq!(
        hits,
        vec![QueryHit {
            entity,
            point: [1.0, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 1.0,
        }]
    );
    Ok(())
}

#[test]
fn a_ray_starting_on_the_surface_of_a_sphere_does_not_hit() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.raycast([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert!(hits.is_empty());
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
fn a_diagonal_ray_hit_on_a_sphere_reports_the_radial_normal() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.raycast([-5.0, -5.0, 0.0], [1.0, 1.0, 0.0], 100.0, LAYER_ONE);

    assert_eq!(hits.len(), 1);
    let diagonal = 1.0 / 2.0f64.sqrt();
    assert!((hits[0].normal[0] + diagonal).abs() < 1e-9);
    assert!((hits[0].normal[1] + diagonal).abs() < 1e-9);
    assert!(hits[0].normal[2].abs() < 1e-9);
    assert!((hits[0].point[0] + diagonal).abs() < 1e-9);
    assert!((hits[0].point[1] + diagonal).abs() < 1e-9);
    assert!(hits[0].point[2].abs() < 1e-9);
    Ok(())
}

#[test]
fn a_ray_hit_on_the_far_face_of_a_box_reports_the_outward_normal() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        box_body([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], LAYER_ONE),
    )?;

    let hits = world.raycast([5.0, 0.0, 0.0], [-1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert_eq!(
        hits,
        vec![QueryHit {
            entity,
            point: [1.0, 0.0, 0.0],
            normal: [1.0, 0.0, 0.0],
            distance: 4.0
        }]
    );
    Ok(())
}

#[test]
fn triggers_are_excluded_from_raycast_by_default() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        trigger_sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE),
    )?;

    let hits = world.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn triggers_are_excluded_from_overlap_sphere_by_default() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        trigger_sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE),
    )?;

    let overlapping = world.overlap_sphere([0.0, 0.0, 0.0], 0.5, LAYER_ONE);

    assert!(overlapping.is_empty());
    Ok(())
}

#[test]
fn hits_at_the_same_distance_are_ordered_by_entity() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let second = EntityId::new(2);
    let first = EntityId::new(1);
    world.add_body(second, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;
    world.add_body(first, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    let entities: Vec<EntityId> = hits.iter().map(|hit| hit.entity).collect();
    assert_eq!(entities, vec![first, second]);
    Ok(())
}

#[test]
fn triggers_are_included_in_raycast_when_requested() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, trigger_sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits =
        world.raycast_with_triggers([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE, true);

    assert_eq!(
        hits,
        vec![QueryHit {
            entity,
            point: [-1.0, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 4.0
        }]
    );
    Ok(())
}

#[test]
fn triggers_are_included_in_overlap_sphere_when_requested() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, trigger_sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let overlapping = world.overlap_sphere_with_triggers([0.0, 0.0, 0.0], 0.5, LAYER_ONE, true);

    assert_eq!(overlapping, vec![entity]);
    Ok(())
}

#[test]
fn a_linecast_reports_the_hits_between_its_endpoints() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.linecast([-5.0, 0.0, 0.0], [5.0, 0.0, 0.0], LAYER_ONE, false);

    assert_eq!(
        hits,
        vec![QueryHit {
            entity,
            point: [-1.0, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 4.0
        }]
    );
    Ok(())
}

#[test]
fn a_linecast_stops_at_the_end_of_its_segment() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.linecast([-5.0, 0.0, 0.0], [-2.0, 0.0, 0.0], LAYER_ONE, false);

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn a_linecast_with_a_zero_length_segment_hits_nothing() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.linecast([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], LAYER_ONE, false);

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn a_linecast_starting_inside_a_collider_hits_the_exit_surface() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.linecast([0.0, 0.0, 0.0], [5.0, 0.0, 0.0], LAYER_ONE, false);

    assert_eq!(
        hits,
        vec![QueryHit {
            entity,
            point: [1.0, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 1.0,
        }]
    );
    Ok(())
}

#[test]
fn triggers_are_excluded_from_linecast_by_default() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        trigger_sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE),
    )?;

    let hits = world.linecast([-5.0, 0.0, 0.0], [5.0, 0.0, 0.0], LAYER_ONE, false);

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn triggers_are_included_in_linecast_when_requested() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, trigger_sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.linecast([-5.0, 0.0, 0.0], [5.0, 0.0, 0.0], LAYER_ONE, true);

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].entity, entity);
    Ok(())
}

#[test]
fn a_sphere_cast_reports_the_contact_point_and_normal_of_a_sphere() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.sphere_cast(
        [-5.0, 0.0, 0.0],
        0.5,
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );

    assert_eq!(
        hits,
        vec![QueryHit {
            entity,
            point: [-1.0, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 3.5
        }]
    );
    Ok(())
}

#[test]
fn a_sphere_cast_reports_the_contact_point_and_normal_of_a_box() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        box_body([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], LAYER_ONE),
    )?;

    let hits = world.sphere_cast(
        [-5.0, 0.0, 0.0],
        0.5,
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );

    assert_eq!(
        hits,
        vec![QueryHit {
            entity,
            point: [-1.0, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 3.5
        }]
    );
    Ok(())
}

#[test]
fn a_sphere_cast_starting_inside_a_collider_hits_at_distance_zero() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.sphere_cast(
        [-1.2, 0.0, 0.0],
        0.5,
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );

    assert_eq!(
        hits,
        vec![QueryHit {
            entity,
            point: [-1.0, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 0.0,
        }]
    );
    Ok(())
}

#[test]
fn a_sphere_cast_respects_the_layer_mask() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_TWO))?;

    let hits = world.sphere_cast(
        [-5.0, 0.0, 0.0],
        0.5,
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn a_sphere_cast_beyond_the_max_distance_is_not_reported() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.sphere_cast(
        [-5.0, 0.0, 0.0],
        0.5,
        [1.0, 0.0, 0.0],
        3.0,
        LAYER_ONE,
        false,
    );

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn triggers_are_excluded_from_sphere_cast_by_default() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        trigger_sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE),
    )?;

    let hits = world.sphere_cast(
        [-5.0, 0.0, 0.0],
        0.5,
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn triggers_are_included_in_sphere_cast_when_requested() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, trigger_sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.sphere_cast(
        [-5.0, 0.0, 0.0],
        0.5,
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        true,
    );

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].entity, entity);
    Ok(())
}

#[test]
fn a_box_cast_reports_the_contact_point_and_normal_of_a_box() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        box_body([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], LAYER_ONE),
    )?;

    let hits = world.box_cast(
        [-5.0, 0.0, 0.0],
        [0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );

    assert_eq!(
        hits,
        vec![QueryHit {
            entity,
            point: [-1.0, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 3.5
        }]
    );
    Ok(())
}

#[test]
fn a_box_cast_reports_the_contact_point_and_normal_of_a_sphere() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.box_cast(
        [-5.0, 0.0, 0.0],
        [0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );

    assert_eq!(
        hits,
        vec![QueryHit {
            entity,
            point: [-1.0, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 3.5
        }]
    );
    Ok(())
}

#[test]
fn a_box_cast_starting_inside_a_collider_hits_at_distance_zero() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        box_body([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], LAYER_ONE),
    )?;

    let hits = world.box_cast(
        [-1.2, 0.0, 0.0],
        [0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );

    assert_eq!(
        hits,
        vec![QueryHit {
            entity,
            point: [-1.0, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 0.0,
        }]
    );
    Ok(())
}

#[test]
fn a_box_cast_respects_the_layer_mask() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_TWO))?;

    let hits = world.box_cast(
        [-5.0, 0.0, 0.0],
        [0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn a_box_cast_beyond_the_max_distance_is_not_reported() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.box_cast(
        [-5.0, 0.0, 0.0],
        [0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        3.0,
        LAYER_ONE,
        false,
    );

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn triggers_are_excluded_from_box_cast_by_default() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        trigger_sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE),
    )?;

    let hits = world.box_cast(
        [-5.0, 0.0, 0.0],
        [0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn triggers_are_included_in_box_cast_when_requested() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, trigger_sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.box_cast(
        [-5.0, 0.0, 0.0],
        [0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        true,
    );

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].entity, entity);
    Ok(())
}

#[test]
fn a_sphere_cast_with_a_non_positive_radius_hits_nothing() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.sphere_cast(
        [-5.0, 0.0, 0.0],
        0.0,
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn a_box_cast_with_a_non_positive_half_extent_hits_nothing() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let hits = world.box_cast(
        [-5.0, 0.0, 0.0],
        [0.5, 0.0, 0.5],
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn a_body_that_was_never_added_is_never_returned_by_any_query() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let present = EntityId::new(1);
    let absent = EntityId::new(2);
    world.add_body(present, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let raycast_hits = world.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);
    let trigger_raycast_hits =
        world.raycast_with_triggers([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE, true);
    let linecast_hits = world.linecast([-5.0, 0.0, 0.0], [5.0, 0.0, 0.0], LAYER_ONE, true);
    let sphere_cast_hits = world.sphere_cast(
        [-5.0, 0.0, 0.0],
        0.5,
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        true,
    );
    let box_cast_hits = world.box_cast(
        [-5.0, 0.0, 0.0],
        [0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        true,
    );
    let overlapping = world.overlap_sphere([0.0, 0.0, 0.0], 0.5, LAYER_ONE);
    let trigger_overlapping =
        world.overlap_sphere_with_triggers([0.0, 0.0, 0.0], 0.5, LAYER_ONE, true);

    assert!(raycast_hits.iter().all(|hit| hit.entity != absent));
    assert!(trigger_raycast_hits.iter().all(|hit| hit.entity != absent));
    assert!(linecast_hits.iter().all(|hit| hit.entity != absent));
    assert!(sphere_cast_hits.iter().all(|hit| hit.entity != absent));
    assert!(box_cast_hits.iter().all(|hit| hit.entity != absent));
    assert!(!overlapping.contains(&absent));
    assert!(!trigger_overlapping.contains(&absent));
    assert!(raycast_hits.iter().any(|hit| hit.entity == present));
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

#[test]
fn ray_misses_a_rotated_box_where_the_axis_aligned_test_would_hit() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        oriented_box(
            [0.0, 0.0, 0.0],
            ROTATED_45_DEGREES_ABOUT_Z,
            [1.0, 1.0, 1.0],
            LAYER_ONE,
        ),
    )?;

    let hits = world.raycast([1.0, 1.0, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn ray_still_hits_a_rotated_box_through_its_flat_face() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        oriented_box(
            [0.0, 0.0, 0.0],
            ROTATED_45_DEGREES_ABOUT_Z,
            [1.0, 1.0, 1.0],
            LAYER_ONE,
        ),
    )?;

    let hits = world.raycast([0.0, 0.0, -5.0], [0.0, 0.0, 1.0], 100.0, LAYER_ONE);

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].entity, entity);
    assert!((hits[0].distance - 4.0).abs() < 1e-4);
    assert!(hits[0].point[0].abs() < 1e-4);
    assert!(hits[0].point[1].abs() < 1e-4);
    assert!((hits[0].point[2] + 1.0).abs() < 1e-4);
    assert!(hits[0].normal[0].abs() < 1e-4);
    assert!(hits[0].normal[1].abs() < 1e-4);
    assert!((hits[0].normal[2] + 1.0).abs() < 1e-4);
    Ok(())
}

#[test]
fn ray_hits_a_rotated_box_side_face_with_a_rotated_normal() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        oriented_box(
            [0.0, 0.0, 0.0],
            ROTATED_45_DEGREES_ABOUT_Z,
            [1.0, 1.0, 1.0],
            LAYER_ONE,
        ),
    )?;

    let hits = world.raycast([-5.0, 0.5, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].entity, entity);
    assert!((hits[0].distance - 4.0857864).abs() < 1e-4);
    let diagonal = 1.0 / 2.0f64.sqrt();
    assert!((hits[0].normal[0] + diagonal).abs() < 1e-4);
    assert!((hits[0].normal[1] - diagonal).abs() < 1e-4);
    assert!(hits[0].normal[2].abs() < 1e-4);
    Ok(())
}

#[test]
fn linecast_against_a_rotated_box_respects_orientation() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        oriented_box(
            [0.0, 0.0, 0.0],
            ROTATED_45_DEGREES_ABOUT_Z,
            [1.0, 1.0, 1.0],
            LAYER_ONE,
        ),
    )?;

    let hits = world.linecast([1.0, 1.0, 0.0], [5.0, 1.0, 0.0], LAYER_ONE, false);

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn sphere_cast_against_a_rotated_box_respects_orientation() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        oriented_box(
            [0.0, 0.0, 0.0],
            ROTATED_45_DEGREES_ABOUT_Z,
            [1.0, 1.0, 1.0],
            LAYER_ONE,
        ),
    )?;

    let hits = world.sphere_cast(
        [1.0, 1.0, 0.0],
        0.2,
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn overlap_sphere_against_a_rotated_box_respects_orientation() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        oriented_box(
            [0.0, 0.0, 0.0],
            ROTATED_45_DEGREES_ABOUT_Z,
            [1.0, 1.0, 1.0],
            LAYER_ONE,
        ),
    )?;

    let hits = world.overlap_sphere([1.0, 1.0, 0.0], 0.1, LAYER_ONE);

    assert!(hits.is_empty());
    Ok(())
}

#[test]
fn sphere_overlap_against_a_rotated_box_is_rotation_invariant() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        oriented_box(
            [0.0, 0.0, 0.0],
            ROTATED_45_DEGREES_ABOUT_Z,
            [1.0, 1.0, 1.0],
            LAYER_ONE,
        ),
    )?;

    let hits = world.overlap_sphere([0.0, 0.0, 0.0], 0.5, LAYER_ONE);

    assert_eq!(hits, vec![entity]);
    Ok(())
}

#[test]
fn degenerate_rotation_falls_back_to_identity() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        oriented_box(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            LAYER_ONE,
        ),
    )?;

    let hits = world.raycast([0.0, 0.0, -5.0], [0.0, 0.0, 1.0], 100.0, LAYER_ONE);

    assert_eq!(
        hits,
        vec![QueryHit {
            entity,
            point: [0.0, 0.0, -1.0],
            normal: [0.0, 0.0, -1.0],
            distance: 4.0
        }]
    );
    Ok(())
}

#[test]
fn sphere_overlap_against_a_rotated_non_cubic_box_uses_the_inverse_rotation(
) -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        oriented_box(
            [0.0, 0.0, 0.0],
            ROTATED_45_DEGREES_ABOUT_Z,
            [2.0, 1.0, 1.0],
            LAYER_ONE,
        ),
    )?;

    let hits = world.overlap_sphere([2.0, 1.0, 0.0], 0.2, LAYER_ONE);

    assert_eq!(hits, vec![entity]);
    Ok(())
}

fn capsule_body(
    position: [f64; 3],
    rotation: [f64; 4],
    radius: f64,
    height: f64,
    direction: u8,
    layer: u32,
) -> BodyDesc {
    BodyDesc {
        position,
        rotation,
        mass: 0.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        shape: Shape::Capsule {
            radius,
            height,
            direction,
        },
        layer,
        role: BodyRole::Static,
        friction: 0.6,
        restitution: 0.0,
    }
}

#[test]
fn overlap_sphere_returns_entities_sorted_by_ascending_entity_id() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let third = EntityId::new(30);
    let first = EntityId::new(10);
    let second = EntityId::new(20);
    world.add_body(third, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;
    world.add_body(first, sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;
    world.add_body(second, trigger_sphere([0.0, 0.0, 0.0], 1.0, LAYER_ONE))?;

    let non_trigger = world.overlap_sphere([0.0, 0.0, 0.0], 0.5, LAYER_ONE);
    let with_triggers = world.overlap_sphere_with_triggers([0.0, 0.0, 0.0], 0.5, LAYER_ONE, true);

    assert_eq!(non_trigger, vec![first, third]);
    assert_eq!(with_triggers, vec![first, second, third]);
    Ok(())
}

#[test]
fn raycast_and_linecast_hit_capsule_cylinder_and_cap_surfaces() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        capsule_body(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            1.0,
            4.0,
            1,
            LAYER_ONE,
        ),
    )?;

    let cylinder_hits = world.raycast([-5.0, 0.5, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);
    assert_eq!(
        cylinder_hits,
        vec![QueryHit {
            entity,
            point: [-1.0, 0.5, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 4.0,
        }]
    );

    let cap_hits = world.linecast([0.0, 5.0, 0.0], [0.0, -5.0, 0.0], LAYER_ONE, false);
    assert_eq!(
        cap_hits,
        vec![QueryHit {
            entity,
            point: [0.0, 2.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            distance: 3.0,
        }]
    );

    let miss_above_cap = world.raycast([-5.0, 2.5, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);
    assert!(miss_above_cap.is_empty());
    Ok(())
}

#[test]
fn raycast_starting_inside_capsule_hits_exit_surface() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        capsule_body(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            1.0,
            4.0,
            1,
            LAYER_ONE,
        ),
    )?;

    let hits = world.raycast([0.0, 0.5, 0.0], [1.0, 0.0, 0.0], 100.0, LAYER_ONE);

    assert_eq!(
        hits,
        vec![QueryHit {
            entity,
            point: [1.0, 0.5, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 1.0,
        }]
    );
    Ok(())
}

#[test]
fn sphere_cast_and_box_cast_hit_capsule_target_and_handle_initial_overlap(
) -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        capsule_body(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            1.0,
            4.0,
            1,
            LAYER_ONE,
        ),
    )?;

    let sphere_hits = world.sphere_cast(
        [-5.0, 0.5, 0.0],
        0.5,
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );
    assert_eq!(
        sphere_hits,
        vec![QueryHit {
            entity,
            point: [-1.0, 0.5, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 3.5,
        }]
    );

    let overlapped_sphere = world.sphere_cast(
        [-1.2, 0.5, 0.0],
        0.5,
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );
    assert_eq!(
        overlapped_sphere,
        vec![QueryHit {
            entity,
            point: [-1.0, 0.5, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 0.0,
        }]
    );

    let box_hits = world.box_cast(
        [-5.0, 0.5, 0.0],
        [0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );
    assert_eq!(
        box_hits,
        vec![QueryHit {
            entity,
            point: [-1.0, 0.5, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 3.5,
        }]
    );

    let overlapped_box = world.box_cast(
        [-1.2, 0.5, 0.0],
        [0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        100.0,
        LAYER_ONE,
        false,
    );
    assert_eq!(
        overlapped_box,
        vec![QueryHit {
            entity,
            point: [-1.0, 0.5, 0.0],
            normal: [-1.0, 0.0, 0.0],
            distance: 0.0,
        }]
    );
    Ok(())
}

#[test]
fn capsule_direction_and_rotation_are_honored_in_queries() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        capsule_body(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            0.5,
            4.0,
            0,
            LAYER_ONE,
        ),
    )?;

    let hit_along_x_extent = world.raycast([1.2, -3.0, 0.0], [0.0, 1.0, 0.0], 100.0, LAYER_ONE);
    assert_eq!(
        hit_along_x_extent,
        vec![QueryHit {
            entity,
            point: [1.2, -0.5, 0.0],
            normal: [0.0, -1.0, 0.0],
            distance: 2.5,
        }]
    );
    assert_eq!(
        world.overlap_sphere([1.6, 0.0, 0.0], 0.6, LAYER_ONE),
        vec![entity]
    );
    Ok(())
}
