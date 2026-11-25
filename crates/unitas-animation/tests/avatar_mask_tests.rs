use unitas_animation::avatar_mask::AvatarMask;
use unitas_animation::{AnimatedValue, CurveBindingTarget, SampledBinding};

fn sampled(object_path: &str, value: f32) -> SampledBinding {
    SampledBinding {
        target: CurveBindingTarget::new(object_path, "Transform", "localPosition.x"),
        value: AnimatedValue::Float(value),
    }
}

#[test]
fn avatar_mask_includes_the_masked_path_and_its_children() {
    let mask = AvatarMask::new("UpperBody").with_path("Spine");
    assert!(mask.includes_path("Spine"));
    assert!(mask.includes_path("Spine/Head"));
    assert!(!mask.includes_path("Spine2"));
    assert!(!mask.includes_path("Leg"));
    assert!(!mask.includes_path(""));
    assert_eq!(mask.name(), "UpperBody");
    assert_eq!(mask.paths(), &["Spine".to_string()]);
}

#[test]
fn avatar_mask_root_entry_matches_only_the_root_path() {
    let mask = AvatarMask::new("Root").with_path("");
    assert!(mask.includes_path(""));
    assert!(!mask.includes_path("Spine"));
}

#[test]
fn avatar_mask_without_paths_matches_nothing() {
    let mask = AvatarMask::new("Empty");
    assert!(mask.is_empty());
    assert!(!mask.includes_path(""));
    assert!(!mask.includes_path("Spine"));
}

#[test]
fn avatar_mask_filters_bindings_to_the_masked_paths() {
    let mask = AvatarMask::new("Arm").with_path("Arm");
    let bindings = vec![
        sampled("", 1.0),
        sampled("Arm", 2.0),
        sampled("Arm/Hand", 3.0),
        sampled("Leg", 4.0),
    ];
    let filtered = mask.filter_bindings(&bindings);
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].target.object_path, "Arm");
    assert_eq!(filtered[1].target.object_path, "Arm/Hand");
}
