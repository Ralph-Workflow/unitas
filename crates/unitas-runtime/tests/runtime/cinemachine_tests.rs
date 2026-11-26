use std::error::Error;

use unitas_core::EntityId;
use unitas_runtime::cinemachine::{
    BlendStyle, BrainUpdateMethod, CameraBlend, CameraNotLiveReason, CinemachineWorld,
    VirtualCamera,
};

fn live_name(cameras: &CinemachineWorld) -> Option<String> {
    cameras.live_camera().map(|camera| camera.name.clone())
}

fn cross_product(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ]
}

fn rotate_vector(rotation: [f64; 4], vector: [f64; 3]) -> [f64; 3] {
    let axis = [rotation[0], rotation[1], rotation[2]];
    let twice_cross = cross_product(axis, vector);
    let scaled = [
        twice_cross[0] * 2.0,
        twice_cross[1] * 2.0,
        twice_cross[2] * 2.0,
    ];
    let second = cross_product(axis, scaled);
    [
        vector[0] + rotation[3] * scaled[0] + second[0],
        vector[1] + rotation[3] * scaled[1] + second[1],
        vector[2] + rotation[3] * scaled[2] + second[2],
    ]
}

fn dot_product(first: [f64; 3], second: [f64; 3]) -> f64 {
    first[0] * second[0] + first[1] * second[1] + first[2] * second[2]
}

fn rotation_length(rotation: [f64; 4]) -> f64 {
    (rotation[0] * rotation[0]
        + rotation[1] * rotation[1]
        + rotation[2] * rotation[2]
        + rotation[3] * rotation[3])
        .sqrt()
}

#[test]
fn cinemachine_highest_priority_active_camera_is_live() -> Result<(), Box<dyn Error>> {
    let mut cameras = CinemachineWorld::new();
    let low = cameras.add_camera(VirtualCamera::new("Low", 1));
    let high = cameras.add_camera(VirtualCamera::new("High", 10));
    cameras.set_active(low, true);
    cameras.set_active(high, true);
    cameras.update(0.0);
    assert_eq!(live_name(&cameras).as_deref(), Some("High"));
    assert_eq!(
        cameras.not_live_reason(low),
        Some(CameraNotLiveReason::LowerPriority)
    );
    assert_eq!(cameras.not_live_reason(high), None);
    Ok(())
}

#[test]
fn cinemachine_most_recently_activated_camera_wins_among_equal_priorities()
-> Result<(), Box<dyn Error>> {
    let mut cameras = CinemachineWorld::new();
    let first = cameras.add_camera(VirtualCamera::new("First", 5));
    let second = cameras.add_camera(VirtualCamera::new("Second", 5));
    cameras.set_active(first, true);
    cameras.set_active(second, true);
    cameras.update(0.0);
    assert_eq!(live_name(&cameras).as_deref(), Some("Second"));
    cameras.activate(first);
    cameras.update(0.0);
    assert_eq!(live_name(&cameras).as_deref(), Some("First"));
    assert_eq!(
        cameras.not_live_reason(second),
        Some(CameraNotLiveReason::EqualPriority)
    );
    Ok(())
}

#[test]
fn cin_001_outcome_33_camera_swap_keeps_the_live_camera_and_active_blend_inspectable_during_play()
-> Result<(), Box<dyn Error>> {
    let mut cameras = CinemachineWorld::new();
    let first = cameras.add_camera(VirtualCamera::new("First", 10));
    let second = cameras.add_camera(VirtualCamera::new("Second", 10));
    cameras.set_active(first, true);
    cameras.update(0.0);
    assert_eq!(live_name(&cameras).as_deref(), Some("First"));
    assert!(cameras.active_blend().is_none());
    cameras.set_active(second, true);
    let blend = cameras
        .active_blend()
        .ok_or("a camera swap starts a blend")?;
    assert_eq!(blend.from, "First");
    assert_eq!(blend.to, "Second");
    assert_eq!(blend.style, BlendStyle::EaseInOut);
    assert_eq!(blend.progress(), 0.0);
    cameras.update(1.0);
    assert_eq!(
        cameras.active_blend().map(|blend| blend.progress()),
        Some(0.5)
    );
    cameras.update(1.0);
    assert!(cameras.active_blend().is_none());
    assert_eq!(live_name(&cameras).as_deref(), Some("Second"));
    Ok(())
}

#[test]
fn cinemachine_custom_blend_is_used_for_a_matching_camera_pair() -> Result<(), Box<dyn Error>> {
    let mut cameras = CinemachineWorld::new();
    cameras.set_custom_blend(
        "First",
        "Second",
        CameraBlend {
            style: BlendStyle::Linear,
            duration: 0.5,
        },
    );
    let first = cameras.add_camera(VirtualCamera::new("First", 10));
    let second = cameras.add_camera(VirtualCamera::new("Second", 10));
    cameras.set_active(first, true);
    cameras.set_active(second, true);
    let blend = cameras
        .active_blend()
        .ok_or("a camera swap starts a blend")?;
    assert_eq!(blend.style, BlendStyle::Linear);
    assert_eq!(blend.duration, 0.5);
    assert_eq!(
        cameras
            .custom_blend("First", "Second")
            .map(|blend| blend.duration),
        Some(0.5)
    );
    Ok(())
}

#[test]
fn cinemachine_cut_blend_swaps_without_an_active_blend() -> Result<(), Box<dyn Error>> {
    let mut cameras = CinemachineWorld::new();
    cameras.set_default_blend(CameraBlend::cut());
    let first = cameras.add_camera(VirtualCamera::new("First", 10));
    let second = cameras.add_camera(VirtualCamera::new("Second", 10));
    cameras.set_active(first, true);
    cameras.set_active(second, true);
    assert!(cameras.active_blend().is_none());
    assert_eq!(live_name(&cameras).as_deref(), Some("Second"));
    Ok(())
}

#[test]
fn cinemachine_not_live_reason_explains_why_a_camera_is_not_live() -> Result<(), Box<dyn Error>> {
    let mut cameras = CinemachineWorld::new();
    let main = cameras.add_camera(VirtualCamera::new("Main", 10));
    let backup = cameras.add_camera(VirtualCamera::new("Backup", 1));
    cameras.set_active(main, true);
    cameras.update(0.0);
    assert_eq!(
        cameras.not_live_reason(backup),
        Some(CameraNotLiveReason::Inactive)
    );
    cameras.set_active(backup, true);
    assert_eq!(
        cameras.not_live_reason(backup),
        Some(CameraNotLiveReason::LowerPriority)
    );
    assert_eq!(cameras.not_live_reason(main), None);
    Ok(())
}

#[test]
fn cinemachine_shot_override_makes_the_shot_camera_live_and_releases_it()
-> Result<(), Box<dyn Error>> {
    let mut cameras = CinemachineWorld::new();
    let main = cameras.add_camera(VirtualCamera::new("Main", 10));
    cameras.add_camera(VirtualCamera::new("Shot", 1));
    cameras.set_active(main, true);
    cameras.update(0.0);
    assert!(cameras.set_shot(Some("Shot"), None));
    assert_eq!(live_name(&cameras).as_deref(), Some("Shot"));
    assert!(cameras.shot_index().is_some());
    cameras.set_shot(None, None);
    assert_eq!(live_name(&cameras).as_deref(), Some("Main"));
    assert!(cameras.shot_index().is_none());
    assert!(!cameras.set_shot(Some("Ghost"), None));
    Ok(())
}

#[test]
fn cinemachine_brain_update_method_is_inspectable() -> Result<(), Box<dyn Error>> {
    let mut cameras = CinemachineWorld::new();
    cameras.set_update_method(BrainUpdateMethod::Late);
    assert_eq!(cameras.update_method(), BrainUpdateMethod::Late);
    cameras.set_update_method(BrainUpdateMethod::Manual);
    assert_eq!(cameras.update_method(), BrainUpdateMethod::Manual);
    Ok(())
}

#[test]
fn cinemachine_virtual_camera_follows_target_and_aims_at_look_at_target()
-> Result<(), Box<dyn Error>> {
    let follow_target = EntityId::new(41);
    let look_target = EntityId::new(42);
    let mut damped = CinemachineWorld::new();
    let camera = damped.add_camera(
        VirtualCamera::new("Damped", 10)
            .with_follow(follow_target)
            .with_look_at(look_target)
            .with_follow_offset([0.0, 4.0, 0.0])
            .with_damping(4.0, 4.0),
    );
    damped.set_target_position(follow_target, [10.0, 0.0, 0.0]);
    damped.set_target_position(look_target, [10.0, 0.0, 10.0]);
    assert_eq!(
        damped.target_position(follow_target),
        Some([10.0, 0.0, 0.0])
    );
    damped.set_active(camera, true);
    damped.update(0.1);
    let live = damped.live_camera().ok_or("the active camera is live")?;
    assert!(live.position[0] > 0.0 && live.position[0] < 10.0);
    assert!(live.position[1] > 0.0 && live.position[1] < 4.0);
    assert_eq!(live.position[2], 0.0);
    assert!(live.rotation.iter().all(|value| value.is_finite()));
    assert!((rotation_length(live.rotation) - 1.0).abs() < 1e-9);

    let mut snapped = CinemachineWorld::new();
    let camera = snapped.add_camera(
        VirtualCamera::new("Snapped", 10)
            .with_follow(follow_target)
            .with_look_at(look_target)
            .with_follow_offset([0.0, 4.0, 0.0])
            .with_damping(0.0, 0.0),
    );
    snapped.set_target_position(follow_target, [10.0, 0.0, 0.0]);
    snapped.set_target_position(look_target, [10.0, 0.0, 10.0]);
    snapped.set_active(camera, true);
    snapped.update(0.1);
    let live = snapped.live_camera().ok_or("the snapped camera is live")?;
    assert_eq!(live.position, [10.0, 4.0, 0.0]);
    assert!((rotation_length(live.rotation) - 1.0).abs() < 1e-9);
    let forward = rotate_vector(live.rotation, [0.0, 0.0, 1.0]);
    let to_target = [
        10.0 - live.position[0],
        0.0 - live.position[1],
        10.0 - live.position[2],
    ];
    let length = (dot_product(to_target, to_target)).sqrt();
    let expected = [
        to_target[0] / length,
        to_target[1] / length,
        to_target[2] / length,
    ];
    assert!(forward.iter().all(|value| value.is_finite()));
    assert!(dot_product(forward, expected) > 0.9999);
    Ok(())
}

#[test]
fn cinemachine_presented_pose_blends_between_following_cameras() -> Result<(), Box<dyn Error>> {
    let first_target = EntityId::new(51);
    let second_target = EntityId::new(52);
    let mut cameras = CinemachineWorld::new();
    cameras.set_target_position(first_target, [0.0, 0.0, 0.0]);
    cameras.set_target_position(second_target, [10.0, 0.0, 0.0]);
    let first = cameras.add_camera(VirtualCamera::new("First", 10).with_follow(first_target));
    let second = cameras.add_camera(
        VirtualCamera::new("Second", 10)
            .with_follow(second_target)
            .with_damping(4.0, 0.0),
    );
    cameras.set_active(first, true);
    cameras.update(0.0);
    assert!(cameras.active_blend().is_none());
    let (position, rotation) = cameras
        .presented_pose()
        .ok_or("the live camera presents a pose")?;
    assert_eq!(position, [0.0, 0.0, 0.0]);
    assert_eq!(rotation, [0.0, 0.0, 0.0, 1.0]);
    cameras.set_active(second, true);
    cameras.update(1.0);
    assert!(cameras.active_blend().is_some());
    let first_position = cameras
        .camera(first)
        .ok_or("the first camera exists")?
        .position;
    let second_position = cameras
        .camera(second)
        .ok_or("the second camera exists")?
        .position;
    let damped_step = 10.0 * (1.0 - (-4.0f64).exp());
    assert!((second_position[0] - damped_step).abs() < 1e-9);
    let (position, rotation) = cameras
        .presented_pose()
        .ok_or("the blend presents a pose")?;
    assert!(position[0] > first_position[0] && position[0] < second_position[0]);
    let midpoint = 0.5 * (first_position[0] + second_position[0]);
    assert!((position[0] - midpoint).abs() < 1e-9);
    assert_eq!(position[1], 0.0);
    assert_eq!(position[2], 0.0);
    assert!((rotation_length(rotation) - 1.0).abs() < 1e-9);
    cameras.update(1.0);
    assert!(cameras.active_blend().is_none());
    let live_position = cameras
        .camera(second)
        .ok_or("the second camera exists")?
        .position;
    let (position, rotation) = cameras
        .presented_pose()
        .ok_or("the live camera presents a pose")?;
    assert_eq!(position, live_position);
    assert_eq!(rotation, [0.0, 0.0, 0.0, 1.0]);
    Ok(())
}
