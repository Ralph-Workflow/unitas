use unitas_core::EntityId;
use unitas_physics::cloth::{Cloth, ClothCollider, ClothDesc, ClothError};
use unitas_physics::{BodyDesc, BodyRole, ColliderShape, Shape, World};

const TIMESTEP: f64 = 1.0 / 60.0;
const CAPSULE_RADIUS: f64 = 0.4;
const CAPSULE_HEIGHT: f64 = 2.0;
const CONTACT_DISTANCE: f64 = CAPSULE_RADIUS + 0.05;

fn static_capsule(position: [f64; 3]) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 0.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        collider_shape: ColliderShape::Primitive(Shape::Capsule {
            radius: CAPSULE_RADIUS,
            height: CAPSULE_HEIGHT,
            direction: 1,
        }),
        layer: 0,
        role: BodyRole::Static,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
    }
}

#[test]
fn cloth_grid_has_a_particle_and_a_constraint_for_every_cell(
) -> Result<(), Box<dyn std::error::Error>> {
    let cloth = Cloth::new(ClothDesc::new([-1.5, 0.0, 0.0], 3.0, 1.0, 5, 2))?;

    assert_eq!(cloth.desc().resolution_x, 5);
    assert_eq!(cloth.desc().resolution_y, 2);
    assert_eq!(cloth.particle_count(), 10);
    assert_eq!(cloth.constraints().len(), 13);

    let particles = cloth.particles();
    assert_eq!(particles[0].position, [-1.5, 0.0, 0.0]);
    assert_eq!(particles[9].position, [1.5, 1.0, 0.0]);

    Ok(())
}

#[test]
fn cloth_collides_with_assigned_capsules_only() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let assigned = EntityId::new(1);
    let unassigned = EntityId::new(2);
    world.add_body(assigned, static_capsule([-1.0, 0.5, 0.0]))?;
    world.add_body(unassigned, static_capsule([1.0, 0.5, 0.0]))?;

    let mut cloth = Cloth::new(ClothDesc::new([-1.5, 0.0, 0.0], 3.0, 1.0, 5, 2))?;
    cloth.assign_collider(ClothCollider::capsule(
        assigned,
        CAPSULE_RADIUS,
        CAPSULE_HEIGHT,
    ))?;
    assert_eq!(
        cloth.colliders().len(),
        1,
        "the cloth holds one assigned collider"
    );
    assert_eq!(cloth.colliders()[0].entity, assigned);

    cloth.simulate(&world, TIMESTEP)?;

    let contacts = cloth.contacts();
    assert!(
        contacts.iter().all(|contact| contact.entity == assigned),
        "cloth reported a contact with a collider it was not assigned"
    );
    assert_eq!(
        contacts.len(),
        2,
        "expected a contact for every penetrating particle"
    );

    let pushed = cloth.particle(6).ok_or("missing particle")?;
    assert!(
        (pushed.position[0] - (-1.0 + CONTACT_DISTANCE)).abs() < 1e-6,
        "particle stayed at {} instead of resting on the assigned capsule",
        pushed.position[0]
    );

    let untouched = cloth.particle(8).ok_or("missing particle")?;
    assert_eq!(
        untouched.position,
        [0.75, 1.0, 0.0],
        "cloth pushed a particle out of a collider it was not assigned"
    );

    Ok(())
}

#[test]
fn cloth_grid_keeps_its_spacing_while_falling() -> Result<(), Box<dyn std::error::Error>> {
    let world = World::new();
    let mut cloth = Cloth::new(ClothDesc::new([0.0, 1.0, 0.0], 1.0, 1.0, 4, 4))?;

    for _ in 0..120 {
        cloth.simulate(&world, TIMESTEP)?;
    }

    for constraint in cloth.constraints() {
        let first = cloth.particle(constraint.first).ok_or("missing particle")?;
        let second = cloth
            .particle(constraint.second)
            .ok_or("missing particle")?;
        let delta = [
            second.position[0] - first.position[0],
            second.position[1] - first.position[1],
            second.position[2] - first.position[2],
        ];
        let distance = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
        assert!(
            (distance - constraint.rest_length).abs() < constraint.rest_length * 0.1,
            "cloth link stretched from {} to {distance}",
            constraint.rest_length
        );
    }

    let corner = cloth.particle(0).ok_or("missing particle")?;
    assert!(
        corner.position[1] < 1.0,
        "cloth did not fall: the first particle is still at {}",
        corner.position[1]
    );

    Ok(())
}

#[test]
fn cloth_rejects_a_description_without_a_grid() {
    assert!(matches!(
        Cloth::new(ClothDesc::new([0.0, 0.0, 0.0], 1.0, 1.0, 1, 4)),
        Err(ClothError::InvalidDescription)
    ));
}

#[test]
fn phy_004_story_7_cloth_on_character_collides_with_assigned_capsule_colliders_only_and_clears_transform_motion(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let character_capsule = EntityId::new(1);
    let scene_capsule = EntityId::new(2);
    world.add_body(character_capsule, static_capsule([-1.0, 0.5, 0.0]))?;
    world.add_body(scene_capsule, static_capsule([1.0, 0.5, 0.0]))?;

    let mut cloth = Cloth::new(ClothDesc::new([-1.5, 0.0, 0.0], 3.0, 1.0, 5, 2))?;
    cloth.assign_collider(ClothCollider::capsule(
        character_capsule,
        CAPSULE_RADIUS,
        CAPSULE_HEIGHT,
    ))?;
    cloth.simulate(&world, TIMESTEP)?;

    assert!(
        cloth
            .contacts()
            .iter()
            .all(|contact| contact.entity == character_capsule),
        "PHY-004 story 7: cloth on character must collide with its assigned capsule colliders only"
    );
    assert_eq!(cloth.contacts().len(), 2);

    cloth.clear_transform_motion();
    for particle in cloth.particles() {
        assert_eq!(
            particle.position, particle.previous_position,
            "PHY-004 story 7: ClearTransformMotion must discard accumulated velocity"
        );
    }
    Ok(())
}
