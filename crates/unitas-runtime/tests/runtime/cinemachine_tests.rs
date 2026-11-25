use std::error::Error;

use unitas_runtime::cinemachine::{
    BlendStyle, BrainUpdateMethod, CameraBlend, CameraNotLiveReason, CinemachineWorld,
    VirtualCamera,
};

fn live_name(cameras: &CinemachineWorld) -> Option<String> {
    cameras.live_camera().map(|camera| camera.name.clone())
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
