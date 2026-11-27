#[path = "simulation/matrix_tests.rs"]
mod matrix_tests;
#[path = "simulation/particles_tests.rs"]
mod particles_tests;
#[path = "simulation/sleep_tests.rs"]
mod sleep_tests;
#[path = "simulation/teleport_tests.rs"]
mod teleport_tests;
#[path = "simulation/trails_tests.rs"]
mod trails_tests;
#[path = "simulation/two_d_effector_tests.rs"]
mod two_d_effector_tests;
#[path = "simulation/two_d_game02_tests.rs"]
mod two_d_game02_tests;
#[path = "simulation/two_d_tests.rs"]
mod two_d_tests;

#[path = "simulation/articulation_tests.rs"]
mod articulation_tests;
#[path = "simulation/cloth_tests.rs"]
mod cloth_tests;
#[path = "simulation/force_mode_tests.rs"]
mod force_mode_tests;
#[path = "simulation/joint_3d_tests.rs"]
mod joint_3d_tests;
#[path = "simulation/particles_2d_tests.rs"]
mod particles_2d_tests;
#[path = "simulation/sim_control_tests.rs"]
mod sim_control_tests;

#[path = "simulation/perf_step.rs"]
mod perf_step;

#[path = "simulation/perf_particles.rs"]
mod perf_particles;

#[path = "physics/ccd_mode_tests.rs"]
mod ccd_mode_tests;
#[path = "physics/character_tests.rs"]
mod character_tests;
#[path = "physics/collision_tests.rs"]
mod collision_tests;
#[path = "physics/kinematic_rotation_tests.rs"]
mod kinematic_rotation_tests;
#[path = "physics/navigation_tests.rs"]
mod navigation_tests;
#[path = "physics/query_tests.rs"]
mod query_tests;
#[path = "physics/terrain_tests.rs"]
mod terrain_tests;
#[path = "physics/unity_semantics_tests.rs"]
mod unity_semantics_tests;
#[path = "physics/wheel_tests.rs"]
mod wheel_tests;
#[path = "physics/world_tests.rs"]
mod world_tests;

use unitas_core::EntityId;
use unitas_physics::{
    BodyDesc, BodyRole, ColliderDesc, ColliderShape, CollisionDetectionMode3D, PhysicsError, Shape,
    World,
};

const FRAME_VERTICES: [[f64; 3]; 8] = [
    [-2.0, 0.0, -2.0],
    [2.0, 0.0, -2.0],
    [2.0, 0.0, 2.0],
    [-2.0, 0.0, 2.0],
    [-1.0, 0.0, -1.0],
    [1.0, 0.0, -1.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
];

const FRAME_INDICES: [u32; 24] = [
    0, 5, 1, 0, 4, 5, 7, 2, 6, 7, 3, 2, 0, 7, 4, 0, 3, 7, 5, 2, 1, 5, 6, 2,
];

const STEP: f64 = 1.0 / 120.0;
const STEPS: usize = 120;

fn frame_mesh() -> ColliderShape {
    ColliderShape::Mesh {
        vertices: FRAME_VERTICES.to_vec(),
        indices: FRAME_INDICES.to_vec(),
        convex: false,
    }
}

fn body_desc(position: [f64; 3], shape: ColliderShape, role: BodyRole) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: if role == BodyRole::Dynamic { 1.0 } else { 0.0 },
        velocity: [0.0; 3],
        use_gravity: role == BodyRole::Dynamic,
        is_trigger: false,
        collider_shape: shape,
        layer: 0,
        role,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: CollisionDetectionMode3D::Discrete,
    }
}

fn falling_sphere(position: [f64; 3]) -> BodyDesc {
    body_desc(
        position,
        ColliderShape::Primitive(Shape::Sphere { radius: 0.25 }),
        BodyRole::Dynamic,
    )
}

fn settle(world: &mut World) -> Result<(), PhysicsError> {
    for _ in 0..STEPS {
        world.step(STEP)?;
    }
    Ok(())
}

fn height_of(world: &World, entity: EntityId) -> Result<f64, PhysicsError> {
    Ok(world
        .position(entity)
        .ok_or(PhysicsError::InvalidBody(entity))?[1])
}

#[test]
fn a_mesh_collider_collides_with_the_mesh_and_not_with_its_bounding_box() -> Result<(), PhysicsError>
{
    let mut world = World::new();
    let frame = EntityId::new(1);
    world.add_body(
        frame,
        body_desc([0.0, 0.0, 0.0], frame_mesh(), BodyRole::Static),
    )?;

    let through_opening = EntityId::new(2);
    world.add_body(through_opening, falling_sphere([0.0, 2.0, 0.0]))?;
    settle(&mut world)?;
    let height = height_of(&world, through_opening)?;
    assert!(
        height < -0.5,
        "the body must pass through the opening of the mesh: got {height}"
    );

    let mut boxed = World::new();
    let box_frame = EntityId::new(1);
    boxed.add_body(
        box_frame,
        body_desc(
            [0.0, 0.0, 0.0],
            ColliderShape::Primitive(Shape::Box {
                half_extents: [2.0, 0.1, 2.0],
            }),
            BodyRole::Static,
        ),
    )?;
    let blocked = EntityId::new(2);
    boxed.add_body(blocked, falling_sphere([0.0, 2.0, 0.0]))?;
    settle(&mut boxed)?;
    let height = height_of(&boxed, blocked)?;
    assert!(
        height > 0.0,
        "the bounding box of the mesh must block the body: got {height}"
    );

    let on_the_rim = EntityId::new(3);
    world.add_body(on_the_rim, falling_sphere([0.0, 2.0, 1.5]))?;
    settle(&mut world)?;
    let height = height_of(&world, on_the_rim)?;
    assert!(
        height > 0.0,
        "the solid part of the mesh must block the body: got {height}"
    );
    Ok(())
}

#[test]
fn a_body_carries_every_collider_of_its_game_object() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let platform = EntityId::new(1);
    world.add_body(
        platform,
        body_desc(
            [0.0, 0.0, 0.0],
            ColliderShape::Primitive(Shape::Box {
                half_extents: [0.5, 0.5, 0.5],
            }),
            BodyRole::Static,
        ),
    )?;
    world.add_collider(
        platform,
        ColliderDesc {
            shape: ColliderShape::Primitive(Shape::Box {
                half_extents: [0.5, 0.5, 0.5],
            }),
            offset: [2.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            is_trigger: false,
            material: None,
            layer: 0,
        },
    )?;

    let faller = EntityId::new(2);
    world.add_body(faller, falling_sphere([2.0, 2.0, 0.0]))?;
    settle(&mut world)?;
    let height = height_of(&world, faller)?;
    assert!(
        height > 0.0,
        "the second collider of the body must block the falling body: got {height}"
    );

    let events = world.drain_events();
    assert!(
        events
            .iter()
            .any(|event| event.a == platform && event.b == faller),
        "the extra collider must report contacts under the entity of its body: got {events:?}"
    );
    Ok(())
}

#[test]
fn a_non_convex_mesh_collider_on_a_dynamic_body_is_refused() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    let error = world.add_body(
        entity,
        body_desc([0.0, 0.0, 0.0], frame_mesh(), BodyRole::Dynamic),
    );
    assert!(
        matches!(
            error,
            Err(PhysicsError::NonConvexMeshColliderOnDynamicBody { entity: refused, .. })
                if refused == entity
        ),
        "a non-convex mesh collider on a dynamic body must be refused: got {error:?}"
    );
    assert_eq!(world.position(entity), None);
    let diagnostics = world.entity_diagnostics(entity);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "PHY-MESH-NONCONVEX-DYNAMIC");
    assert!(
        diagnostics[0].message.contains("Non-convex MeshCollider"),
        "the diagnostic must state the non-convex MeshCollider cause: {}",
        diagnostics[0].message
    );

    let mut world = World::new();
    let dynamic = EntityId::new(1);
    world.add_body(
        dynamic,
        body_desc(
            [0.0, 0.0, 0.0],
            ColliderShape::Primitive(Shape::Box {
                half_extents: [0.5, 0.5, 0.5],
            }),
            BodyRole::Dynamic,
        ),
    )?;
    let error = world.add_collider(
        dynamic,
        ColliderDesc {
            shape: frame_mesh(),
            offset: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            is_trigger: false,
            material: None,
            layer: 0,
        },
    );
    assert!(
        matches!(
            error,
            Err(PhysicsError::NonConvexMeshColliderOnDynamicBody { entity: refused, .. })
                if refused == dynamic
        ),
        "a non-convex mesh collider on a dynamic body must be refused: got {error:?}"
    );
    Ok(())
}

#[test]
fn a_non_convex_trigger_mesh_collider_is_refused() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    let error = world.add_body(
        entity,
        BodyDesc {
            is_trigger: true,
            ..body_desc([0.0, 0.0, 0.0], frame_mesh(), BodyRole::Static)
        },
    );
    assert!(
        matches!(error, Err(PhysicsError::InvalidBody(refused)) if refused == entity),
        "a non-convex trigger mesh collider must be refused: got {error:?}"
    );
    let diagnostics = world.entity_diagnostics(entity);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "PHY-MESH-NONCONVEX-TRIGGER");
    Ok(())
}
