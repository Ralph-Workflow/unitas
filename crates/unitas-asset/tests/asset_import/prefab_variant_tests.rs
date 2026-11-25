use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::prefab::{Modification, PrefabInstance, PrefabSource, PrefabSourceReference};
use unitas_asset::prefab_variant::{
    NestedPrefabReference, OrphanedOverrideResolution, OverrideLevel, PrefabComponentRecord,
    PrefabNodeRecord, PrefabRegistry, PrefabVariantDefinition, PrefabVariantError,
    ScenePrefabInstance, SourceObjectId, TargetedModification, UnpackMode, ValueProvenanceEntry,
};
use unitas_asset::scene_save::{self, SceneSaveError};
use unitas_asset::unity_yaml;

type TestResult = Result<(), Box<dyn Error>>;

const BASE_GUID: &str = "10000000000000000000000000000001";
const VARIANT_A_GUID: &str = "10000000000000000000000000000002";
const VARIANT_B_GUID: &str = "10000000000000000000000000000003";
const NESTED_GUID: &str = "10000000000000000000000000000004";

const ROOT_GO_ID: i64 = 100;
const ROOT_TRANSFORM_ID: i64 = 101;
const ROOT_RIGIDBODY_ID: i64 = 102;
const CHILD_A_GO_ID: i64 = 200;
const CHILD_B_GO_ID: i64 = 300;

fn build_vehicle_registry() -> Result<(PrefabRegistry, ScenePrefabInstance), PrefabVariantError> {
    let mut registry = PrefabRegistry::new();

    let root_node = PrefabNodeRecord::new(ROOT_GO_ID, "BaseVehicle", None)
        .with_children(vec![CHILD_A_GO_ID, CHILD_B_GO_ID])
        .with_property("m_Layer", "0")
        .with_property("m_TagString", "Untagged")
        .with_component(
            PrefabComponentRecord::new(ROOT_TRANSFORM_ID, "Transform")
                .with_property("m_LocalPosition.x", "0")
                .with_property("m_LocalPosition.y", "0")
                .with_property("m_LocalPosition.z", "0")
                .with_property("m_LocalScale.x", "1"),
        )
        .with_component(
            PrefabComponentRecord::new(ROOT_RIGIDBODY_ID, "Rigidbody")
                .with_property("m_Mass", "1000")
                .with_property("m_Drag", "0.05")
                .with_property("m_UseGravity", "1"),
        );

    let child_a = PrefabNodeRecord::new(CHILD_A_GO_ID, "FrontWheel", Some(ROOT_GO_ID))
        .with_property("m_Radius", "0.35")
        .with_property("m_Suspension", "0.2");

    let child_b = PrefabNodeRecord::new(CHILD_B_GO_ID, "RearWheel", Some(ROOT_GO_ID))
        .with_property("m_Radius", "0.35")
        .with_property("m_Suspension", "0.2");

    let base = PrefabVariantDefinition::base_prefab(
        BASE_GUID,
        "BaseVehicle",
        ROOT_GO_ID,
        vec![root_node, child_a, child_b],
    );
    registry.register(base);

    let mut variant_a = PrefabVariantDefinition::variant(
        VARIANT_A_GUID,
        "ArmoredVehicleVariant",
        PrefabSource::new(BASE_GUID, ROOT_GO_ID),
    );
    variant_a.record_override(Modification::new(ROOT_GO_ID, "m_Name", "ArmoredVehicle"))?;
    variant_a.record_override(Modification::new(ROOT_RIGIDBODY_ID, "m_Mass", "2500"))?;
    variant_a.record_override(Modification::new(CHILD_A_GO_ID, "m_Radius", "0.45"))?;
    registry.register(variant_a);

    let mut variant_b = PrefabVariantDefinition::variant(
        VARIANT_B_GUID,
        "DesertArmoredVariant",
        PrefabSource::new(VARIANT_A_GUID, ROOT_GO_ID),
    );
    variant_b.record_override(Modification::new(ROOT_GO_ID, "m_TagString", "DesertCamo"))?;
    variant_b.record_override(Modification::new(ROOT_RIGIDBODY_ID, "m_Drag", "0.12"))?;
    variant_b.record_override(Modification::new(CHILD_A_GO_ID, "m_Radius", "0.55"))?;
    registry.register(variant_b);

    let instance = ScenePrefabInstance::new(
        "scene_vehicle_1",
        PrefabInstance::new(
            PrefabSource::new(VARIANT_B_GUID, ROOT_GO_ID),
            0,
            vec![
                Modification::new(ROOT_TRANSFORM_ID, "m_LocalPosition.x", "42"),
                Modification::new(ROOT_RIGIDBODY_ID, "m_Drag", "0.25"),
            ],
        ),
    );

    Ok((registry, instance))
}

#[test]
fn multi_level_variant_inheritance_resolves_closer_level_precedence_and_provenance() -> TestResult {
    let (registry, instance) = build_vehicle_registry()?;
    let resolved = registry.resolve_instance(&instance)?;
    let hierarchy = resolved
        .as_complete()
        .ok_or("expected complete hierarchy")?;

    assert_eq!(
        hierarchy.chain,
        vec![
            OverrideLevel::Prefab(BASE_GUID.to_string()),
            OverrideLevel::Prefab(VARIANT_A_GUID.to_string()),
            OverrideLevel::Prefab(VARIANT_B_GUID.to_string()),
            OverrideLevel::SceneInstance(String::from("scene_vehicle_1")),
        ]
    );

    assert_eq!(
        hierarchy.effective_property(ROOT_GO_ID, "m_Layer"),
        Some("0")
    );
    assert_eq!(
        hierarchy.effective_owner(ROOT_GO_ID, "m_Layer"),
        Some(&OverrideLevel::Prefab(BASE_GUID.to_string()))
    );

    assert_eq!(
        hierarchy.effective_property(ROOT_GO_ID, "m_Name"),
        Some("ArmoredVehicle")
    );
    assert_eq!(
        hierarchy.effective_owner(ROOT_GO_ID, "m_Name"),
        Some(&OverrideLevel::Prefab(VARIANT_A_GUID.to_string()))
    );

    assert_eq!(
        hierarchy.effective_property(ROOT_RIGIDBODY_ID, "m_Mass"),
        Some("2500")
    );
    assert_eq!(
        hierarchy.effective_owner(ROOT_RIGIDBODY_ID, "m_Mass"),
        Some(&OverrideLevel::Prefab(VARIANT_A_GUID.to_string()))
    );

    assert_eq!(
        hierarchy.effective_property(CHILD_A_GO_ID, "m_Radius"),
        Some("0.55")
    );
    assert_eq!(
        hierarchy.effective_owner(CHILD_A_GO_ID, "m_Radius"),
        Some(&OverrideLevel::Prefab(VARIANT_B_GUID.to_string()))
    );

    assert_eq!(
        hierarchy.effective_property(ROOT_RIGIDBODY_ID, "m_Drag"),
        Some("0.25")
    );
    assert_eq!(
        hierarchy.effective_owner(ROOT_RIGIDBODY_ID, "m_Drag"),
        Some(&OverrideLevel::SceneInstance(String::from(
            "scene_vehicle_1"
        )))
    );

    let drag_path = hierarchy
        .property_inheritance_path(ROOT_RIGIDBODY_ID, "m_Drag")
        .ok_or("missing provenance for m_Drag")?;
    assert_eq!(
        drag_path,
        &[
            ValueProvenanceEntry {
                level: OverrideLevel::Prefab(BASE_GUID.to_string()),
                value: String::from("0.05"),
                is_effective: false,
            },
            ValueProvenanceEntry {
                level: OverrideLevel::Prefab(VARIANT_B_GUID.to_string()),
                value: String::from("0.12"),
                is_effective: false,
            },
            ValueProvenanceEntry {
                level: OverrideLevel::SceneInstance(String::from("scene_vehicle_1")),
                value: String::from("0.25"),
                is_effective: true,
            },
        ]
    );

    Ok(())
}

#[test]
fn base_prefab_changes_propagate_through_variants_and_instances_without_erasing_overrides(
) -> TestResult {
    let (mut registry, instance) = build_vehicle_registry()?;

    let base_mut = registry
        .get_mut(BASE_GUID)
        .ok_or("missing base prefab in registry")?;
    base_mut.set_base_property(CHILD_B_GO_ID, "m_Suspension", "0.45")?;
    base_mut.set_base_property(ROOT_RIGIDBODY_ID, "m_Mass", "1500")?;
    base_mut.set_base_property(ROOT_RIGIDBODY_ID, "m_UseGravity", "0")?;

    let resolved = registry.resolve_instance(&instance)?;
    let hierarchy = resolved
        .as_complete()
        .ok_or("expected complete hierarchy")?;

    assert_eq!(
        hierarchy.effective_property(CHILD_B_GO_ID, "m_Suspension"),
        Some("0.45")
    );
    assert_eq!(
        hierarchy.effective_property(ROOT_RIGIDBODY_ID, "m_UseGravity"),
        Some("0")
    );

    assert_eq!(
        hierarchy.effective_property(ROOT_RIGIDBODY_ID, "m_Mass"),
        Some("2500")
    );
    assert_eq!(
        hierarchy.effective_property(CHILD_A_GO_ID, "m_Radius"),
        Some("0.55")
    );
    assert_eq!(
        hierarchy.effective_property(ROOT_RIGIDBODY_ID, "m_Drag"),
        Some("0.25")
    );

    Ok(())
}

#[test]
fn apply_override_targets_intermediate_variant_or_base_and_cleans_lower_redundant_overrides(
) -> TestResult {
    let (mut registry, mut instance) = build_vehicle_registry()?;

    let resolved_before = registry.resolve_instance(&instance)?;
    let hierarchy_before = resolved_before
        .as_complete()
        .ok_or("expected complete hierarchy")?;
    let instance_level = OverrideLevel::SceneInstance(String::from("scene_vehicle_1"));
    let eligible = hierarchy_before.eligible_apply_targets(&instance_level);
    assert_eq!(
        eligible,
        vec![
            OverrideLevel::Prefab(BASE_GUID.to_string()),
            OverrideLevel::Prefab(VARIANT_A_GUID.to_string()),
            OverrideLevel::Prefab(VARIANT_B_GUID.to_string()),
        ]
    );

    let variant_a_level = OverrideLevel::Prefab(VARIANT_A_GUID.to_string());
    registry.apply_override_to_level(
        Some(&mut instance),
        &instance_level,
        &variant_a_level,
        ROOT_RIGIDBODY_ID,
        "m_Drag",
    )?;

    assert!(!instance
        .instance
        .modifications
        .iter()
        .any(|m| m.target_file_id == ROOT_RIGIDBODY_ID && m.property_path == "m_Drag"));

    let variant_b_def = registry.get(VARIANT_B_GUID).ok_or("missing variant B")?;
    assert!(!variant_b_def
        .modifications
        .iter()
        .any(|m| m.target_file_id == ROOT_RIGIDBODY_ID && m.property_path == "m_Drag"));

    let resolved_mid = registry.resolve_instance(&instance)?;
    let hierarchy_mid = resolved_mid
        .as_complete()
        .ok_or("expected complete hierarchy")?;
    assert_eq!(
        hierarchy_mid.effective_property(ROOT_RIGIDBODY_ID, "m_Drag"),
        Some("0.25")
    );
    assert_eq!(
        hierarchy_mid.effective_owner(ROOT_RIGIDBODY_ID, "m_Drag"),
        Some(&variant_a_level)
    );

    let base_level = OverrideLevel::Prefab(BASE_GUID.to_string());
    registry.apply_override_to_level(
        Some(&mut instance),
        &instance_level,
        &base_level,
        ROOT_TRANSFORM_ID,
        "m_LocalPosition.x",
    )?;

    let resolved_after = registry.resolve_instance(&instance)?;
    let hierarchy_after = resolved_after
        .as_complete()
        .ok_or("expected complete hierarchy")?;
    assert_eq!(
        hierarchy_after.effective_property(ROOT_TRANSFORM_ID, "m_LocalPosition.x"),
        Some("42")
    );
    assert_eq!(
        hierarchy_after.effective_owner(ROOT_TRANSFORM_ID, "m_LocalPosition.x"),
        Some(&base_level)
    );

    Ok(())
}

#[test]
fn apply_all_and_revert_at_specific_level_restores_nearer_ancestor_values() -> TestResult {
    let (mut registry, mut instance) = build_vehicle_registry()?;

    let instance_level = OverrideLevel::SceneInstance(String::from("scene_vehicle_1"));
    let variant_b_level = OverrideLevel::Prefab(VARIANT_B_GUID.to_string());
    let variant_a_level = OverrideLevel::Prefab(VARIANT_A_GUID.to_string());

    registry.revert_override_at_level(
        Some(&mut instance),
        &instance_level,
        ROOT_RIGIDBODY_ID,
        "m_Drag",
    )?;

    let resolved_1 = registry.resolve_instance(&instance)?;
    let hierarchy_1 = resolved_1.as_complete().ok_or("expected complete")?;
    assert_eq!(
        hierarchy_1.effective_property(ROOT_RIGIDBODY_ID, "m_Drag"),
        Some("0.12")
    );
    assert_eq!(
        hierarchy_1.effective_owner(ROOT_RIGIDBODY_ID, "m_Drag"),
        Some(&variant_b_level)
    );

    registry.revert_override_at_level(None, &variant_b_level, CHILD_A_GO_ID, "m_Radius")?;

    let resolved_2 = registry.resolve_instance(&instance)?;
    let hierarchy_2 = resolved_2.as_complete().ok_or("expected complete")?;
    assert_eq!(
        hierarchy_2.effective_property(CHILD_A_GO_ID, "m_Radius"),
        Some("0.45")
    );
    assert_eq!(
        hierarchy_2.effective_owner(CHILD_A_GO_ID, "m_Radius"),
        Some(&variant_a_level)
    );

    registry.apply_all_to_level(Some(&mut instance), &instance_level, &variant_b_level)?;
    assert!(instance.instance.modifications.is_empty());

    let resolved_3 = registry.resolve_instance(&instance)?;
    let hierarchy_3 = resolved_3.as_complete().ok_or("expected complete")?;
    assert_eq!(
        hierarchy_3.effective_property(ROOT_TRANSFORM_ID, "m_LocalPosition.x"),
        Some("42")
    );
    assert_eq!(
        hierarchy_3.effective_owner(ROOT_TRANSFORM_ID, "m_LocalPosition.x"),
        Some(&variant_b_level)
    );

    registry.revert_all_at_level(None, &variant_b_level)?;
    let resolved_4 = registry.resolve_instance(&instance)?;
    let hierarchy_4 = resolved_4.as_complete().ok_or("expected complete")?;
    assert_eq!(
        hierarchy_4.effective_property(ROOT_RIGIDBODY_ID, "m_Drag"),
        Some("0.05")
    );
    assert_eq!(
        hierarchy_4.effective_property(ROOT_TRANSFORM_ID, "m_LocalPosition.x"),
        Some("0")
    );

    Ok(())
}

#[test]
fn structural_overrides_add_and_remove_children_and_components_and_order_added_siblings(
) -> TestResult {
    let (mut registry, mut instance) = build_vehicle_registry()?;

    let root_id = SourceObjectId::new(BASE_GUID, ROOT_GO_ID);
    let child_b_id = SourceObjectId::new(BASE_GUID, CHILD_B_GO_ID);

    let turret_child = PrefabNodeRecord::new(500, "RoofTurret", Some(ROOT_GO_ID))
        .with_property("m_Caliber", "20mm");
    let antenna_child = PrefabNodeRecord::new(501, "RadioAntenna", Some(ROOT_GO_ID))
        .with_property("m_Range", "5000");

    let variant_a_mut = registry
        .get_mut(VARIANT_A_GUID)
        .ok_or("missing variant A")?;
    variant_a_mut.add_child_override(root_id.clone(), turret_child, None)?;
    variant_a_mut.add_child_override(root_id.clone(), antenna_child, None)?;
    variant_a_mut.reorder_added_siblings(root_id.clone(), vec![501, 500])?;
    variant_a_mut.add_component_override(
        root_id.clone(),
        PrefabComponentRecord::new(600, "ArmorPlate").with_property("m_Thickness", "50"),
    );

    let variant_b_mut = registry
        .get_mut(VARIANT_B_GUID)
        .ok_or("missing variant B")?;
    variant_b_mut.remove_child_override(child_b_id);
    variant_b_mut.record_targeted_override(TargetedModification::new(
        SourceObjectId::new(VARIANT_A_GUID, 500),
        "m_Caliber",
        "30mm",
    ))?;

    instance.remove_component_override(SourceObjectId::new(BASE_GUID, ROOT_RIGIDBODY_ID));
    instance.add_child_override(
        root_id.clone(),
        PrefabNodeRecord::new(700, "Spotlight", Some(ROOT_GO_ID)),
        Some(0),
    );

    let resolved = registry.resolve_instance(&instance)?;
    let hierarchy = resolved.as_complete().ok_or("expected complete")?;

    assert!(hierarchy.node_by_file_id(CHILD_B_GO_ID).is_none());
    assert!(hierarchy.node_by_file_id(500).is_some());
    assert!(hierarchy.node_by_file_id(501).is_some());
    assert!(hierarchy.node_by_file_id(700).is_some());

    assert_eq!(hierarchy.effective_property(500, "m_Caliber"), Some("30mm"));
    assert_eq!(hierarchy.effective_property(600, "m_Thickness"), Some("50"));

    let root_node = hierarchy
        .node_by_file_id(ROOT_GO_ID)
        .ok_or("missing root node")?;
    assert!(!root_node
        .components
        .iter()
        .any(|c| c.identity.target_file_id == ROOT_RIGIDBODY_ID));
    assert!(root_node
        .components
        .iter()
        .any(|c| c.identity.target_file_id == 600));

    let child_file_ids: Vec<i64> = root_node
        .children
        .iter()
        .map(|cid| cid.target_file_id)
        .collect();
    assert_eq!(child_file_ids, vec![700, CHILD_A_GO_ID, 501, 500]);

    Ok(())
}

#[test]
fn identity_based_correspondence_survives_source_child_reordering_and_duplicate_display_names(
) -> TestResult {
    let mut registry = PrefabRegistry::new();

    let slot_left = PrefabNodeRecord::new(201, "MountSlot", Some(ROOT_GO_ID))
        .with_property("m_Loadout", "Laser");
    let slot_right = PrefabNodeRecord::new(202, "MountSlot", Some(ROOT_GO_ID))
        .with_property("m_Loadout", "Missile");
    let root = PrefabNodeRecord::new(ROOT_GO_ID, "Mech", None).with_children(vec![201, 202]);

    registry.register(PrefabVariantDefinition::base_prefab(
        BASE_GUID,
        "Mech",
        ROOT_GO_ID,
        vec![root, slot_left, slot_right],
    ));

    let mut variant = PrefabVariantDefinition::variant(
        VARIANT_A_GUID,
        "HeavyMech",
        PrefabSource::new(BASE_GUID, ROOT_GO_ID),
    );
    variant.record_override(Modification::new(201, "m_Loadout", "PlasmaCannon"))?;
    variant.record_override(Modification::new(202, "m_Loadout", "Railgun"))?;
    registry.register(variant);

    let base_mut = registry.get_mut(BASE_GUID).ok_or("missing base")?;
    base_mut.reorder_source_children(ROOT_GO_ID, vec![202, 201])?;

    let resolved = registry.resolve_prefab(VARIANT_A_GUID)?;
    let hierarchy = resolved.as_complete().ok_or("expected complete")?;

    let root_node = hierarchy
        .node_by_file_id(ROOT_GO_ID)
        .ok_or("missing root")?;
    let ordered_children: Vec<i64> = root_node
        .children
        .iter()
        .map(|c| c.target_file_id)
        .collect();
    assert_eq!(ordered_children, vec![202, 201]);

    assert_eq!(
        hierarchy.effective_property(201, "m_Loadout"),
        Some("PlasmaCannon")
    );
    assert_eq!(
        hierarchy.effective_property(202, "m_Loadout"),
        Some("Railgun")
    );

    Ok(())
}

#[test]
fn inheritance_and_nesting_cycles_are_detected_and_rejected_with_named_diagnostics() -> TestResult {
    let mut registry = PrefabRegistry::new();

    let cyc_a = PrefabVariantDefinition::variant(
        VARIANT_A_GUID,
        "CycleA",
        PrefabSource::new(VARIANT_B_GUID, ROOT_GO_ID),
    );
    let cyc_b = PrefabVariantDefinition::variant(
        VARIANT_B_GUID,
        "CycleB",
        PrefabSource::new(VARIANT_A_GUID, ROOT_GO_ID),
    );
    registry.register(cyc_a);
    registry.register(cyc_b);

    let err = registry
        .resolve_prefab(VARIANT_A_GUID)
        .err()
        .ok_or("expected inheritance cycle error")?;
    assert_eq!(
        err,
        PrefabVariantError::InheritanceCycle {
            chain: vec![
                VARIANT_A_GUID.to_string(),
                VARIANT_B_GUID.to_string(),
                VARIANT_A_GUID.to_string(),
            ],
        }
    );

    let mut nest_registry = PrefabRegistry::new();
    let mut base_outer = PrefabVariantDefinition::base_prefab(
        BASE_GUID,
        "Outer",
        ROOT_GO_ID,
        vec![PrefabNodeRecord::new(ROOT_GO_ID, "Outer", None)],
    );
    base_outer.nested_prefabs.push(NestedPrefabReference {
        instance_file_id: 900,
        parent_target_file_id: ROOT_GO_ID,
        source: PrefabSource::new(NESTED_GUID, ROOT_GO_ID),
        modifications: Vec::new(),
    });

    let mut nested_inner = PrefabVariantDefinition::base_prefab(
        NESTED_GUID,
        "Inner",
        ROOT_GO_ID,
        vec![PrefabNodeRecord::new(ROOT_GO_ID, "Inner", None)],
    );
    nested_inner.nested_prefabs.push(NestedPrefabReference {
        instance_file_id: 901,
        parent_target_file_id: ROOT_GO_ID,
        source: PrefabSource::new(BASE_GUID, ROOT_GO_ID),
        modifications: Vec::new(),
    });

    nest_registry.register(base_outer);
    nest_registry.register(nested_inner);

    let nest_err = nest_registry
        .resolve_prefab(BASE_GUID)
        .err()
        .ok_or("expected nesting cycle error")?;
    assert_eq!(
        nest_err,
        PrefabVariantError::NestingCycle {
            chain: vec![
                BASE_GUID.to_string(),
                NESTED_GUID.to_string(),
                BASE_GUID.to_string(),
            ],
        }
    );

    Ok(())
}

#[test]
fn missing_base_prefab_remains_visible_as_broken_relationship_with_overrides_preserved(
) -> TestResult {
    let (mut registry, instance) = build_vehicle_registry()?;
    let _ = registry.remove(BASE_GUID);

    let resolved = registry.resolve_instance(&instance)?;
    assert!(resolved.as_complete().is_none());
    let broken = resolved
        .as_broken()
        .ok_or("expected broken base state when base is missing")?;

    assert_eq!(broken.missing_guid, BASE_GUID);
    assert_eq!(
        broken.broken_at_level,
        OverrideLevel::Prefab(VARIANT_A_GUID.to_string())
    );
    assert_eq!(
        broken.chain_before_break,
        vec![
            OverrideLevel::Prefab(VARIANT_A_GUID.to_string()),
            OverrideLevel::Prefab(VARIANT_B_GUID.to_string()),
            OverrideLevel::SceneInstance(String::from("scene_vehicle_1")),
        ]
    );

    assert!(broken.retained_modifications.iter().any(|(lvl, m)| {
        *lvl == OverrideLevel::Prefab(VARIANT_A_GUID.to_string())
            && m.target_file_id == ROOT_RIGIDBODY_ID
            && m.property_path == "m_Mass"
            && m.value == "2500"
    }));
    assert!(broken.retained_modifications.iter().any(|(lvl, m)| {
        *lvl == OverrideLevel::Prefab(VARIANT_B_GUID.to_string())
            && m.target_file_id == CHILD_A_GO_ID
            && m.property_path == "m_Radius"
            && m.value == "0.55"
    }));
    assert!(broken.retained_modifications.iter().any(|(lvl, m)| {
        *lvl == OverrideLevel::SceneInstance(String::from("scene_vehicle_1"))
            && m.target_file_id == ROOT_TRANSFORM_ID
            && m.property_path == "m_LocalPosition.x"
            && m.value == "42"
    }));

    Ok(())
}

#[test]
fn unrecordable_overrides_are_rejected_with_clear_named_reasons() -> TestResult {
    let (mut registry, mut instance) = build_vehicle_registry()?;
    let variant_a = registry
        .get_mut(VARIANT_A_GUID)
        .ok_or("missing variant A")?;

    let err_hide_flags = variant_a
        .record_override(Modification::new(ROOT_GO_ID, "m_ObjectHideFlags", "1"))
        .err()
        .ok_or("expected rejection for m_ObjectHideFlags")?;
    assert!(matches!(
        err_hide_flags,
        PrefabVariantError::UnrecordableOverride {
            target_file_id: ROOT_GO_ID,
            ref property_path,
            ref reason,
        } if property_path == "m_ObjectHideFlags" && reason.contains("hide flags")
    ));

    let err_file_id = variant_a
        .record_override(Modification::new(ROOT_GO_ID, "fileID", "999"))
        .err()
        .ok_or("expected rejection for fileID")?;
    assert!(matches!(
        err_file_id,
        PrefabVariantError::UnrecordableOverride {
            target_file_id: ROOT_GO_ID,
            ref property_path,
            ref reason,
        } if property_path == "fileID" && reason.contains("file identifier")
    ));

    let err_script = variant_a
        .record_override(Modification::new(ROOT_GO_ID, "m_Script", "new_script"))
        .err()
        .ok_or("expected rejection for m_Script")?;
    assert!(matches!(
        err_script,
        PrefabVariantError::UnrecordableOverride {
            target_file_id: ROOT_GO_ID,
            ref property_path,
            ref reason,
        } if property_path == "m_Script" && reason.contains("script")
    ));

    let err_reorder = variant_a
        .reject_source_child_reorder(ROOT_GO_ID)
        .err()
        .ok_or("expected rejection for reordering source children")?;
    assert!(matches!(
        err_reorder,
        PrefabVariantError::UnrecordableOverride {
            target_file_id: ROOT_GO_ID,
            ref property_path,
            ref reason,
        } if property_path == "m_Children" && reason.contains("reordering source children")
    ));

    let err_cross_prefab = instance
        .record_override(Modification::new(
            ROOT_GO_ID,
            "m_Target",
            "cross_prefab_ref:transient_object",
        ))
        .err()
        .ok_or("expected rejection for cross-prefab non-serializable reference")?;
    assert!(matches!(
        err_cross_prefab,
        PrefabVariantError::UnrecordableOverride {
            target_file_id: ROOT_GO_ID,
            ref property_path,
            ref reason,
        } if property_path == "m_Target" && reason.contains("cross-prefab non-serializable")
    ));

    let err_cross_scene = instance
        .record_override(Modification::new(
            ROOT_GO_ID,
            "m_SceneTarget",
            "cross_scene:OtherScene:100",
        ))
        .err()
        .ok_or("expected rejection for cross-scene reference")?;
    assert!(matches!(
        err_cross_scene,
        PrefabVariantError::UnrecordableOverride {
            target_file_id: ROOT_GO_ID,
            ref property_path,
            ref reason,
        } if property_path == "m_SceneTarget" && reason.contains("cross-scene")
    ));

    Ok(())
}

#[test]
fn removed_source_element_with_lower_overrides_identifies_orphaned_overrides() -> TestResult {
    let (mut registry, instance) = build_vehicle_registry()?;

    let variant_a_mut = registry
        .get_mut(VARIANT_A_GUID)
        .ok_or("missing variant A")?;
    variant_a_mut.remove_child_override(SourceObjectId::new(BASE_GUID, CHILD_A_GO_ID));

    let resolved = registry.resolve_instance(&instance)?;
    let hierarchy = resolved.as_complete().ok_or("expected complete")?;

    assert!(hierarchy.node_by_file_id(CHILD_A_GO_ID).is_none());
    assert!(hierarchy.orphaned_overrides.iter().any(|orphan| {
        orphan.level == OverrideLevel::Prefab(VARIANT_B_GUID.to_string())
            && orphan.target_file_id == CHILD_A_GO_ID
            && orphan.property_path == "m_Radius"
            && orphan.value == "0.55"
    }));

    Ok(())
}

fn temp_prefab_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!(
        "unitas_asset_prefab_variant_{name}_{}_{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

#[test]
fn prefab_variant_and_scene_instance_disk_round_trip_preserves_overrides_identity_and_unedited_documents(
) -> TestResult {
    let root = temp_prefab_dir("round_trip")?;
    let base_path = root.join("BaseVehicle.prefab");
    let variant_a_path = root.join("ArmoredVehicleVariant.prefab");
    let variant_b_path = root.join("DesertArmoredVariant.prefab");
    let scene_path = root.join("DesertLevel.unity");

    let (mut registry, mut instance) = build_vehicle_registry()?;
    let root_id = SourceObjectId::new(BASE_GUID, ROOT_GO_ID);
    let child_b_id = SourceObjectId::new(BASE_GUID, CHILD_B_GO_ID);

    let turret_child = PrefabNodeRecord::new(500, "RoofTurret", Some(ROOT_GO_ID))
        .with_property("m_Caliber", "20mm");
    let antenna_child = PrefabNodeRecord::new(501, "RadioAntenna", Some(ROOT_GO_ID))
        .with_property("m_Range", "5000");

    let variant_a_mut = registry
        .get_mut(VARIANT_A_GUID)
        .ok_or("missing variant A")?;
    variant_a_mut.add_child_override(root_id.clone(), turret_child, None)?;
    variant_a_mut.add_child_override(root_id.clone(), antenna_child, None)?;
    variant_a_mut.reorder_added_siblings(root_id.clone(), vec![501, 500])?;
    variant_a_mut.add_component_override(
        root_id.clone(),
        PrefabComponentRecord::new(600, "ArmorPlate").with_property("m_Thickness", "50"),
    );

    let variant_b_mut = registry
        .get_mut(VARIANT_B_GUID)
        .ok_or("missing variant B")?;
    variant_b_mut.remove_child_override(child_b_id);
    variant_b_mut.record_targeted_override(TargetedModification::new(
        SourceObjectId::new(VARIANT_A_GUID, 500),
        "m_Caliber",
        "30mm",
    ))?;

    instance.remove_component_override(SourceObjectId::new(BASE_GUID, ROOT_RIGIDBODY_ID));
    instance.add_child_override(
        root_id.clone(),
        PrefabNodeRecord::new(700, "Spotlight", Some(ROOT_GO_ID))
            .with_property("m_Intensity", "800"),
        Some(0),
    );
    instance.add_component_override(
        root_id,
        PrefabComponentRecord::new(710, "Beacon").with_property("m_Frequency", "2.5"),
    );

    let base_def = registry.get(BASE_GUID).ok_or("missing base")?.clone();
    let variant_a_def = registry
        .get(VARIANT_A_GUID)
        .ok_or("missing variant A")?
        .clone();
    let variant_b_def = registry
        .get(VARIANT_B_GUID)
        .ok_or("missing variant B")?
        .clone();

    scene_save::save_prefab_variant(&base_path, &base_def)?;
    scene_save::save_prefab_variant(&variant_a_path, &variant_a_def)?;
    scene_save::save_prefab_variant(&variant_b_path, &variant_b_def)?;

    let initial_scene_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!29 &1\nOcclusionCullingSettings:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_SceneGUID: 00000000000000000000000000000000\n--- !u!1001 &900100\nPrefabInstance:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_SourcePrefab: {{fileID: {ROOT_GO_ID}, guid: {VARIANT_B_GUID}, type: 3}}\n--- !u!104 &2\nRenderSettings:\n  m_ObjectHideFlags: 0\n  m_Fog: 1\n"
    );
    let mut scene_stream = unity_yaml::parse(&initial_scene_yaml)?;
    let unedited_doc_0 = scene_stream.documents[0].clone();
    let unedited_doc_2 = scene_stream.documents[2].clone();

    scene_save::save_scene_prefab_instance(&scene_path, &mut scene_stream, &instance, 900100)?;

    let saved_scene_text_1 = fs::read_to_string(&scene_path)?;
    assert!(saved_scene_text_1.contains("--- !u!1001 &900100\nPrefabInstance:\n"));
    assert!(saved_scene_text_1.contains("m_Modification:\n"));
    assert!(saved_scene_text_1.contains("m_Modifications:\n"));
    assert!(saved_scene_text_1.contains("m_RemovedComponents:\n"));
    assert!(saved_scene_text_1.contains("m_AddedGameObjects:\n"));
    assert!(saved_scene_text_1.contains("m_AddedComponents:\n"));

    instance.record_override(Modification::new(
        ROOT_TRANSFORM_ID,
        "m_LocalPosition.x",
        "99",
    ))?;
    let mut edited_variant_b = PrefabVariantDefinition::load_from_file(
        &variant_b_path,
        VARIANT_B_GUID,
        "DesertArmoredVariant",
    )?;
    edited_variant_b.record_override(Modification::new(CHILD_A_GO_ID, "m_Radius", "0.65"))?;
    scene_save::save_prefab_variant(&variant_b_path, &edited_variant_b)?;

    let mut reloaded_scene_stream = unity_yaml::parse_file(&scene_path)?;
    scene_save::save_scene_prefab_instance(
        &scene_path,
        &mut reloaded_scene_stream,
        &instance,
        900100,
    )?;

    let final_scene_stream = unity_yaml::parse_file(&scene_path)?;
    assert_eq!(final_scene_stream.documents[0], unedited_doc_0);
    assert_eq!(final_scene_stream.documents[2], unedited_doc_2);

    let loaded_base =
        PrefabVariantDefinition::load_from_file(&base_path, BASE_GUID, "BaseVehicle")?;
    let loaded_variant_a = PrefabVariantDefinition::load_from_file(
        &variant_a_path,
        VARIANT_A_GUID,
        "ArmoredVehicleVariant",
    )?;
    let loaded_variant_b = PrefabVariantDefinition::load_from_file(
        &variant_b_path,
        VARIANT_B_GUID,
        "DesertArmoredVariant",
    )?;
    let loaded_instance =
        ScenePrefabInstance::from_stream(&final_scene_stream, "scene_vehicle_1", 900100)
            .ok_or("failed to load ScenePrefabInstance from stream")?;

    assert_eq!(loaded_base, base_def);
    assert_eq!(loaded_variant_a, variant_a_def);
    assert_eq!(loaded_variant_b, edited_variant_b);
    assert_eq!(loaded_instance, instance);

    registry.register(edited_variant_b);
    let mut loaded_registry = PrefabRegistry::new();
    loaded_registry.register(loaded_base);
    loaded_registry.register(loaded_variant_a);
    loaded_registry.register(loaded_variant_b);

    let expected_resolved = registry.resolve_instance(&instance)?;
    let actual_resolved = loaded_registry.resolve_instance(&loaded_instance)?;
    assert_eq!(actual_resolved, expected_resolved);

    let hierarchy = actual_resolved
        .as_complete()
        .ok_or("expected complete resolved hierarchy after reload")?;
    assert_eq!(
        hierarchy.effective_property(ROOT_TRANSFORM_ID, "m_LocalPosition.x"),
        Some("99")
    );
    assert_eq!(
        hierarchy.effective_property(CHILD_A_GO_ID, "m_Radius"),
        Some("0.65")
    );
    assert_eq!(hierarchy.effective_property(500, "m_Caliber"), Some("30mm"));
    assert_eq!(
        hierarchy.effective_property(700, "m_Intensity"),
        Some("800")
    );
    assert_eq!(
        hierarchy.effective_property(710, "m_Frequency"),
        Some("2.5")
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn failed_prefab_variant_save_leaves_prior_on_disk_file_intact() -> TestResult {
    let root = temp_prefab_dir("failed_save")?;
    let variant_path = root.join("ArmoredVehicleVariant.prefab");

    let (mut registry, _) = build_vehicle_registry()?;
    let variant_a = registry
        .get_mut(VARIANT_A_GUID)
        .ok_or("missing variant A")?;
    variant_a.save_to_file(&variant_path)?;
    let original_on_disk = fs::read_to_string(&variant_path)?;

    fs::create_dir(root.join("ArmoredVehicleVariant.prefab.tmp"))?;
    variant_a.record_override(Modification::new(ROOT_RIGIDBODY_ID, "m_Mass", "9999"))?;

    let err = match variant_a.save_to_file(&variant_path) {
        Ok(()) => return Err("expected prefab save to fail when temp path is a directory".into()),
        Err(err) => err,
    };
    assert!(matches!(err, SceneSaveError::Io { .. }));
    assert_eq!(fs::read_to_string(&variant_path)?, original_on_disk);

    let reloaded = PrefabVariantDefinition::load_from_file(
        &variant_path,
        VARIANT_A_GUID,
        "ArmoredVehicleVariant",
    )?;
    assert!(reloaded
        .modifications
        .iter()
        .any(|m| m.target_file_id == ROOT_RIGIDBODY_ID
            && m.property_path == "m_Mass"
            && m.value == "2500"));

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn unchanged_prefab_save_byte_identity_and_nested_prefab_plus_broken_base_disk_round_trip(
) -> TestResult {
    let root = temp_prefab_dir("unchanged_nested_broken")?;
    let base_with_nested_path = root.join("OuterWithNested.prefab");
    let broken_variant_path = root.join("BrokenBaseVariant.prefab");
    let scene_path = root.join("SceneWithInstances.unity");

    let mut base_with_nested = PrefabVariantDefinition::base_prefab(
        BASE_GUID,
        "OuterWithNested",
        ROOT_GO_ID,
        vec![PrefabNodeRecord::new(ROOT_GO_ID, "OuterWithNested", None)
            .with_property("m_Layer", "2")
            .with_component(
                PrefabComponentRecord::new(ROOT_TRANSFORM_ID, "Transform")
                    .with_property("m_LocalPosition.x", "0"),
            )],
    );
    base_with_nested.nested_prefabs.push(NestedPrefabReference {
        instance_file_id: 900500,
        parent_target_file_id: ROOT_TRANSFORM_ID,
        source: PrefabSource::new(NESTED_GUID, 400),
        modifications: vec![Modification::new(400, "m_Name", "NestedTurretSub")],
    });

    scene_save::save_prefab_variant(&base_with_nested_path, &base_with_nested)?;
    let base_bytes_1 = fs::read(&base_with_nested_path)?;

    let loaded_base_unchanged = PrefabVariantDefinition::load_from_file(
        &base_with_nested_path,
        BASE_GUID,
        "OuterWithNested",
    )?;
    assert_eq!(loaded_base_unchanged, base_with_nested);
    scene_save::save_prefab_variant(&base_with_nested_path, &loaded_base_unchanged)?;
    let base_bytes_2 = fs::read(&base_with_nested_path)?;
    assert_eq!(base_bytes_2, base_bytes_1);

    let missing_base_guid = "99999999999999999999999999999999";
    let mut broken_variant = PrefabVariantDefinition::variant(
        VARIANT_A_GUID,
        "BrokenBaseVariant",
        PrefabSource::new(missing_base_guid, ROOT_GO_ID),
    );
    broken_variant.record_override(Modification::new(ROOT_GO_ID, "m_Name", "OrphanedVariant"))?;
    broken_variant.record_override(Modification::new(ROOT_RIGIDBODY_ID, "m_Mass", "3200"))?;
    broken_variant.nested_prefabs.push(NestedPrefabReference {
        instance_file_id: 900600,
        parent_target_file_id: ROOT_TRANSFORM_ID,
        source: PrefabSource::new(NESTED_GUID, 400),
        modifications: vec![Modification::new(400, "m_TagString", "NestedInVariant")],
    });

    scene_save::save_prefab_variant(&broken_variant_path, &broken_variant)?;
    let broken_bytes_1 = fs::read(&broken_variant_path)?;

    let loaded_broken_variant = PrefabVariantDefinition::load_from_file(
        &broken_variant_path,
        VARIANT_A_GUID,
        "BrokenBaseVariant",
    )?;
    assert_eq!(loaded_broken_variant, broken_variant);
    scene_save::save_prefab_variant(&broken_variant_path, &loaded_broken_variant)?;
    assert_eq!(fs::read(&broken_variant_path)?, broken_bytes_1);

    let scene_instance = ScenePrefabInstance::new(
        "broken_scene_inst",
        PrefabInstance::new(
            PrefabSource::new(VARIANT_A_GUID, ROOT_GO_ID),
            0,
            vec![Modification::new(
                ROOT_TRANSFORM_ID,
                "m_LocalPosition.y",
                "15",
            )],
        ),
    );
    let mut scene_stream = scene_instance.to_stream(800100);
    scene_save::save_scene_prefab_instance(
        &scene_path,
        &mut scene_stream,
        &scene_instance,
        800100,
    )?;
    let scene_bytes_1 = fs::read(&scene_path)?;

    let loaded_scene_instance =
        ScenePrefabInstance::load_from_file(&scene_path, "broken_scene_inst", 800100)?;
    assert_eq!(loaded_scene_instance, scene_instance);
    let mut reloaded_scene_stream = unity_yaml::parse_file(&scene_path)?;
    scene_save::save_scene_prefab_instance(
        &scene_path,
        &mut reloaded_scene_stream,
        &loaded_scene_instance,
        800100,
    )?;
    assert_eq!(fs::read(&scene_path)?, scene_bytes_1);

    let mut registry = PrefabRegistry::new();
    registry.register(loaded_broken_variant);

    let resolved = registry.resolve_instance(&loaded_scene_instance)?;
    let broken_state = resolved
        .as_broken()
        .ok_or("expected BrokenBase after disk reload")?;
    assert_eq!(broken_state.missing_guid, missing_base_guid);
    assert_eq!(
        broken_state.broken_at_level,
        OverrideLevel::Prefab(VARIANT_A_GUID.to_string())
    );
    assert!(broken_state.retained_modifications.iter().any(|(lvl, m)| {
        *lvl == OverrideLevel::Prefab(VARIANT_A_GUID.to_string())
            && m.target_file_id == ROOT_RIGIDBODY_ID
            && m.property_path == "m_Mass"
            && m.value == "3200"
    }));
    assert!(broken_state.retained_modifications.iter().any(|(lvl, m)| {
        *lvl == OverrideLevel::SceneInstance(String::from("broken_scene_inst"))
            && m.target_file_id == ROOT_TRANSFORM_ID
            && m.property_path == "m_LocalPosition.y"
            && m.value == "15"
    }));

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

const NESTED_ROOT_GO_ID: i64 = 400;
const NESTED_BARREL_ID: i64 = 401;
const NESTED_INSTANCE_FILE_ID: i64 = 900500;
const MISSING_GUID: &str = "99999999999999999999999999999999";

fn scene_with_prefab_instance_yaml(guid: &str, file_id: i64) -> String {
    format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_ObjectHideFlags: 0\n  m_Name: HealthySibling\n--- !u!29 &1\nOcclusionCullingSettings:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_SceneGUID: 00000000000000000000000000000000\n--- !u!1001 &900100\nPrefabInstance:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_SourcePrefab: {{fileID: {file_id}, guid: {guid}, type: 3}}\n--- !u!104 &2\nRenderSettings:\n  m_ObjectHideFlags: 0\n  m_Fog: 1\n"
    )
}

fn rename_game_object(root: &mut unity_yaml::Node, name: &str) -> bool {
    let entries = match root {
        unity_yaml::Node::Mapping(entries) => entries,
        _ => return false,
    };
    for (key, body) in entries.iter_mut() {
        if key != "GameObject" {
            continue;
        }
        let fields = match body {
            unity_yaml::Node::Mapping(fields) => fields,
            _ => return false,
        };
        for (field, value) in fields.iter_mut() {
            if field == "m_Name" {
                *value = unity_yaml::Node::Scalar(String::from(name));
                return true;
            }
        }
    }
    false
}

fn build_nested_registry() -> Result<PrefabRegistry, PrefabVariantError> {
    let mut registry = PrefabRegistry::new();

    let nested_root = PrefabNodeRecord::new(NESTED_ROOT_GO_ID, "TurretRoot", None)
        .with_property("m_Caliber", "20mm")
        .with_component(
            PrefabComponentRecord::new(NESTED_BARREL_ID, "Barrel")
                .with_property("m_Length", "2")
                .with_property("m_Muzzle", "MuzzleA"),
        );
    registry.register(PrefabVariantDefinition::base_prefab(
        NESTED_GUID,
        "NestedTurret",
        NESTED_ROOT_GO_ID,
        vec![nested_root],
    ));

    let mut outer = PrefabVariantDefinition::base_prefab(
        BASE_GUID,
        "OuterVehicle",
        ROOT_GO_ID,
        vec![PrefabNodeRecord::new(ROOT_GO_ID, "OuterVehicle", None)
            .with_property("m_Tint", "silver")
            .with_component(
                PrefabComponentRecord::new(ROOT_TRANSFORM_ID, "Transform")
                    .with_property("m_LocalPosition.x", "0"),
            )],
    );
    outer.nested_prefabs.push(NestedPrefabReference {
        instance_file_id: NESTED_INSTANCE_FILE_ID,
        parent_target_file_id: ROOT_TRANSFORM_ID,
        source: PrefabSource::new(NESTED_GUID, NESTED_ROOT_GO_ID),
        modifications: vec![Modification::new(NESTED_ROOT_GO_ID, "m_Caliber", "25mm")],
    });
    registry.register(outer);

    let mut variant = PrefabVariantDefinition::variant(
        VARIANT_A_GUID,
        "OuterVehicleVariant",
        PrefabSource::new(BASE_GUID, ROOT_GO_ID),
    );
    variant.record_override(Modification::new(ROOT_GO_ID, "m_Tint", "gold"))?;
    registry.register(variant);

    Ok(registry)
}

#[test]
fn ser003_outcome7_source_prefab_change_reaches_variants_and_instances_without_erasing_overrides_across_load_edit_save(
) -> TestResult {
    let root = temp_prefab_dir("ser003_outc7_source_change")?;
    let base_path = root.join("BaseVehicle.prefab");
    let variant_a_path = root.join("ArmoredVehicleVariant.prefab");
    let variant_b_path = root.join("DesertArmoredVariant.prefab");
    let scene_path = root.join("DesertLevel.unity");

    let (registry, instance) = build_vehicle_registry()?;
    let base_def = registry
        .get(BASE_GUID)
        .ok_or("missing base prefab")?
        .clone();
    let variant_a_def = registry
        .get(VARIANT_A_GUID)
        .ok_or("missing variant A")?
        .clone();
    let variant_b_def = registry
        .get(VARIANT_B_GUID)
        .ok_or("missing variant B")?
        .clone();

    scene_save::save_prefab_variant(&base_path, &base_def)?;
    scene_save::save_prefab_variant(&variant_a_path, &variant_a_def)?;
    scene_save::save_prefab_variant(&variant_b_path, &variant_b_def)?;

    let mut scene_stream =
        unity_yaml::parse(&scene_with_prefab_instance_yaml(VARIANT_B_GUID, ROOT_GO_ID))?;
    scene_save::save_scene_prefab_instance(&scene_path, &mut scene_stream, &instance, 900100)?;

    let mut edited_base =
        PrefabVariantDefinition::load_from_file(&base_path, BASE_GUID, "BaseVehicle")?;
    edited_base.set_base_property(CHILD_B_GO_ID, "m_Suspension", "0.45")?;
    edited_base.set_base_property(ROOT_RIGIDBODY_ID, "m_Mass", "1500")?;
    scene_save::save_prefab_variant(&base_path, &edited_base)?;

    let loaded_base =
        PrefabVariantDefinition::load_from_file(&base_path, BASE_GUID, "BaseVehicle")?;
    let loaded_variant_a = PrefabVariantDefinition::load_from_file(
        &variant_a_path,
        VARIANT_A_GUID,
        "ArmoredVehicleVariant",
    )?;
    let loaded_variant_b = PrefabVariantDefinition::load_from_file(
        &variant_b_path,
        VARIANT_B_GUID,
        "DesertArmoredVariant",
    )?;
    let loaded_instance =
        ScenePrefabInstance::load_from_file(&scene_path, "scene_vehicle_1", 900100)?;

    let mut loaded_registry = PrefabRegistry::new();
    loaded_registry.register(loaded_base);
    loaded_registry.register(loaded_variant_a);
    loaded_registry.register(loaded_variant_b);

    let variant_resolved = loaded_registry.resolve_prefab(VARIANT_A_GUID)?;
    let variant_hierarchy = variant_resolved
        .as_complete()
        .ok_or("expected complete variant hierarchy after reload")?;
    assert_eq!(
        variant_hierarchy.effective_property(CHILD_B_GO_ID, "m_Suspension"),
        Some("0.45")
    );
    assert_eq!(
        variant_hierarchy.effective_owner(CHILD_B_GO_ID, "m_Suspension"),
        Some(&OverrideLevel::Prefab(BASE_GUID.to_string()))
    );

    let resolved = loaded_registry.resolve_instance(&loaded_instance)?;
    let hierarchy = resolved
        .as_complete()
        .ok_or("expected complete hierarchy after reload")?;

    assert_eq!(
        hierarchy.effective_property(CHILD_B_GO_ID, "m_Suspension"),
        Some("0.45")
    );
    assert_eq!(
        hierarchy.effective_owner(CHILD_B_GO_ID, "m_Suspension"),
        Some(&OverrideLevel::Prefab(BASE_GUID.to_string()))
    );

    assert_eq!(
        hierarchy.effective_property(ROOT_RIGIDBODY_ID, "m_Mass"),
        Some("2500")
    );
    assert_eq!(
        hierarchy.effective_owner(ROOT_RIGIDBODY_ID, "m_Mass"),
        Some(&OverrideLevel::Prefab(VARIANT_A_GUID.to_string()))
    );
    assert_eq!(
        hierarchy.effective_property(CHILD_A_GO_ID, "m_Radius"),
        Some("0.55")
    );
    assert_eq!(
        hierarchy.effective_property(ROOT_RIGIDBODY_ID, "m_Drag"),
        Some("0.25")
    );
    assert_eq!(
        hierarchy.effective_owner(ROOT_RIGIDBODY_ID, "m_Drag"),
        Some(&OverrideLevel::SceneInstance(String::from(
            "scene_vehicle_1"
        )))
    );
    assert_eq!(
        hierarchy.effective_property(ROOT_TRANSFORM_ID, "m_LocalPosition.x"),
        Some("42")
    );
    assert_eq!(
        hierarchy.effective_property(ROOT_GO_ID, "m_Name"),
        Some("ArmoredVehicle")
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn ser003_outcome7_apply_and_revert_touch_only_the_selected_ownership_level_across_load_edit_save(
) -> TestResult {
    let root = temp_prefab_dir("ser003_outc7_apply_revert")?;
    let base_path = root.join("BaseVehicle.prefab");
    let variant_a_path = root.join("ArmoredVehicleVariant.prefab");
    let variant_b_path = root.join("DesertArmoredVariant.prefab");
    let scene_path = root.join("DesertLevel.unity");

    let (registry, instance) = build_vehicle_registry()?;
    let base_def = registry
        .get(BASE_GUID)
        .ok_or("missing base prefab")?
        .clone();
    let variant_a_def = registry
        .get(VARIANT_A_GUID)
        .ok_or("missing variant A")?
        .clone();
    let variant_b_def = registry
        .get(VARIANT_B_GUID)
        .ok_or("missing variant B")?
        .clone();

    scene_save::save_prefab_variant(&base_path, &base_def)?;
    scene_save::save_prefab_variant(&variant_a_path, &variant_a_def)?;
    scene_save::save_prefab_variant(&variant_b_path, &variant_b_def)?;

    let mut scene_stream =
        unity_yaml::parse(&scene_with_prefab_instance_yaml(VARIANT_B_GUID, ROOT_GO_ID))?;
    scene_save::save_scene_prefab_instance(&scene_path, &mut scene_stream, &instance, 900100)?;

    let mut loaded_registry = PrefabRegistry::new();
    loaded_registry.register(PrefabVariantDefinition::load_from_file(
        &base_path,
        BASE_GUID,
        "BaseVehicle",
    )?);
    loaded_registry.register(PrefabVariantDefinition::load_from_file(
        &variant_a_path,
        VARIANT_A_GUID,
        "ArmoredVehicleVariant",
    )?);
    loaded_registry.register(PrefabVariantDefinition::load_from_file(
        &variant_b_path,
        VARIANT_B_GUID,
        "DesertArmoredVariant",
    )?);
    let mut loaded_instance =
        ScenePrefabInstance::load_from_file(&scene_path, "scene_vehicle_1", 900100)?;

    let instance_level = OverrideLevel::SceneInstance(String::from("scene_vehicle_1"));
    let variant_a_level = OverrideLevel::Prefab(VARIANT_A_GUID.to_string());

    loaded_registry.apply_override_to_level(
        Some(&mut loaded_instance),
        &instance_level,
        &variant_a_level,
        ROOT_RIGIDBODY_ID,
        "m_Drag",
    )?;

    assert!(!loaded_instance
        .instance
        .modifications
        .iter()
        .any(|m| m.target_file_id == ROOT_RIGIDBODY_ID && m.property_path == "m_Drag"));
    let applied_variant_a = loaded_registry
        .get(VARIANT_A_GUID)
        .ok_or("missing variant A")?
        .clone();
    assert!(applied_variant_a
        .modifications
        .iter()
        .any(|m| m.target_file_id == ROOT_RIGIDBODY_ID
            && m.property_path == "m_Drag"
            && m.value == "0.25"));
    let cleaned_variant_b = loaded_registry
        .get(VARIANT_B_GUID)
        .ok_or("missing variant B")?
        .clone();
    assert!(!cleaned_variant_b
        .modifications
        .iter()
        .any(|m| m.target_file_id == ROOT_RIGIDBODY_ID && m.property_path == "m_Drag"));

    scene_save::save_prefab_variant(&variant_a_path, &applied_variant_a)?;
    scene_save::save_prefab_variant(&variant_b_path, &cleaned_variant_b)?;
    let mut scene_stream = unity_yaml::parse_file(&scene_path)?;
    scene_save::save_scene_prefab_instance(
        &scene_path,
        &mut scene_stream,
        &loaded_instance,
        900100,
    )?;

    let reloaded_variant_a = PrefabVariantDefinition::load_from_file(
        &variant_a_path,
        VARIANT_A_GUID,
        "ArmoredVehicleVariant",
    )?;
    let mut reloaded_instance =
        ScenePrefabInstance::load_from_file(&scene_path, "scene_vehicle_1", 900100)?;

    assert_eq!(reloaded_variant_a, applied_variant_a);
    assert!(!reloaded_instance
        .instance
        .modifications
        .iter()
        .any(|m| m.property_path == "m_Drag"));
    assert!(reloaded_instance
        .instance
        .modifications
        .iter()
        .any(|m| m.property_path == "m_LocalPosition.x" && m.value == "42"));

    let mut reloaded_registry = PrefabRegistry::new();
    reloaded_registry.register(PrefabVariantDefinition::load_from_file(
        &base_path,
        BASE_GUID,
        "BaseVehicle",
    )?);
    reloaded_registry.register(reloaded_variant_a);
    reloaded_registry.register(PrefabVariantDefinition::load_from_file(
        &variant_b_path,
        VARIANT_B_GUID,
        "DesertArmoredVariant",
    )?);

    let resolved = reloaded_registry.resolve_instance(&reloaded_instance)?;
    let hierarchy = resolved
        .as_complete()
        .ok_or("expected complete hierarchy")?;
    assert_eq!(
        hierarchy.effective_property(ROOT_RIGIDBODY_ID, "m_Drag"),
        Some("0.25")
    );
    assert_eq!(
        hierarchy.effective_owner(ROOT_RIGIDBODY_ID, "m_Drag"),
        Some(&variant_a_level)
    );
    assert_eq!(
        hierarchy.effective_property(ROOT_RIGIDBODY_ID, "m_Mass"),
        Some("2500")
    );
    assert_eq!(
        hierarchy.effective_property(ROOT_TRANSFORM_ID, "m_LocalPosition.x"),
        Some("42")
    );

    reloaded_registry.revert_override_at_level(
        Some(&mut reloaded_instance),
        &instance_level,
        ROOT_TRANSFORM_ID,
        "m_LocalPosition.x",
    )?;
    assert!(reloaded_instance.instance.modifications.is_empty());

    let mut scene_stream = unity_yaml::parse_file(&scene_path)?;
    scene_save::save_scene_prefab_instance(
        &scene_path,
        &mut scene_stream,
        &reloaded_instance,
        900100,
    )?;

    let reverted_instance =
        ScenePrefabInstance::load_from_file(&scene_path, "scene_vehicle_1", 900100)?;
    assert!(reverted_instance.instance.modifications.is_empty());
    assert!(reverted_instance.targeted_modifications.is_empty());

    let resolved = reloaded_registry.resolve_instance(&reverted_instance)?;
    let hierarchy = resolved
        .as_complete()
        .ok_or("expected complete hierarchy")?;
    assert_eq!(
        hierarchy.effective_property(ROOT_TRANSFORM_ID, "m_LocalPosition.x"),
        Some("0")
    );
    assert_eq!(
        hierarchy.effective_owner(ROOT_TRANSFORM_ID, "m_LocalPosition.x"),
        Some(&OverrideLevel::Prefab(BASE_GUID.to_string()))
    );
    assert_eq!(
        hierarchy.effective_property(ROOT_RIGIDBODY_ID, "m_Drag"),
        Some("0.25")
    );
    assert_eq!(
        hierarchy.effective_owner(ROOT_RIGIDBODY_ID, "m_Drag"),
        Some(&variant_a_level)
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn ser003_outcome8_nested_and_variant_values_expose_one_unambiguous_inheritance_path_across_load_edit_save(
) -> TestResult {
    let root = temp_prefab_dir("ser003_outc8_inheritance_path")?;
    let nested_path = root.join("NestedTurret.prefab");
    let base_path = root.join("OuterVehicle.prefab");
    let variant_path = root.join("OuterVehicleVariant.prefab");

    let registry = build_nested_registry()?;
    let nested_def = registry
        .get(NESTED_GUID)
        .ok_or("missing nested prefab")?
        .clone();
    let base_def = registry
        .get(BASE_GUID)
        .ok_or("missing outer prefab")?
        .clone();
    let variant_def = registry
        .get(VARIANT_A_GUID)
        .ok_or("missing variant")?
        .clone();

    scene_save::save_prefab_variant(&nested_path, &nested_def)?;
    scene_save::save_prefab_variant(&base_path, &base_def)?;
    scene_save::save_prefab_variant(&variant_path, &variant_def)?;

    let mut edited_base =
        PrefabVariantDefinition::load_from_file(&base_path, BASE_GUID, "OuterVehicle")?;
    edited_base.set_base_property(ROOT_TRANSFORM_ID, "m_LocalPosition.y", "3")?;
    scene_save::save_prefab_variant(&base_path, &edited_base)?;

    let loaded_nested =
        PrefabVariantDefinition::load_from_file(&nested_path, NESTED_GUID, "NestedTurret")?;
    let loaded_base =
        PrefabVariantDefinition::load_from_file(&base_path, BASE_GUID, "OuterVehicle")?;
    let loaded_variant = PrefabVariantDefinition::load_from_file(
        &variant_path,
        VARIANT_A_GUID,
        "OuterVehicleVariant",
    )?;

    let mut loaded_registry = PrefabRegistry::new();
    loaded_registry.register(loaded_nested);
    loaded_registry.register(loaded_base);
    loaded_registry.register(loaded_variant);

    let resolved = loaded_registry.resolve_prefab(VARIANT_A_GUID)?;
    let hierarchy = resolved
        .as_complete()
        .ok_or("expected complete hierarchy")?;

    let variant_entries = hierarchy
        .property_inheritance_path(ROOT_GO_ID, "m_Tint")
        .ok_or("missing variant value inheritance path")?;
    assert_eq!(
        variant_entries,
        &[
            ValueProvenanceEntry {
                level: OverrideLevel::Prefab(BASE_GUID.to_string()),
                value: String::from("silver"),
                is_effective: false,
            },
            ValueProvenanceEntry {
                level: OverrideLevel::Prefab(VARIANT_A_GUID.to_string()),
                value: String::from("gold"),
                is_effective: true,
            },
        ]
    );
    assert_eq!(
        variant_entries
            .iter()
            .filter(|entry| entry.is_effective)
            .count(),
        1
    );

    let nested = hierarchy
        .nested_by_instance_file_id(NESTED_INSTANCE_FILE_ID)
        .ok_or("missing nested prefab resolution")?;
    assert_eq!(nested.source.guid, NESTED_GUID);
    assert_eq!(
        nested.declared_at,
        OverrideLevel::Prefab(BASE_GUID.to_string())
    );
    let nested_hierarchy = nested
        .resolved
        .as_complete()
        .ok_or("expected complete nested hierarchy")?;
    let nested_entries = nested_hierarchy
        .property_inheritance_path(NESTED_ROOT_GO_ID, "m_Caliber")
        .ok_or("missing nested value inheritance path")?;
    assert_eq!(
        nested_entries,
        &[
            ValueProvenanceEntry {
                level: OverrideLevel::Prefab(NESTED_GUID.to_string()),
                value: String::from("20mm"),
                is_effective: false,
            },
            ValueProvenanceEntry {
                level: OverrideLevel::NestedPrefab {
                    owner: BASE_GUID.to_string(),
                    instance_file_id: NESTED_INSTANCE_FILE_ID,
                },
                value: String::from("25mm"),
                is_effective: true,
            },
        ]
    );
    assert_eq!(
        nested_entries
            .iter()
            .filter(|entry| entry.is_effective)
            .count(),
        1
    );
    assert_eq!(
        nested_hierarchy.effective_property(NESTED_ROOT_GO_ID, "m_Caliber"),
        Some("25mm")
    );

    assert!(hierarchy
        .effective_property(NESTED_ROOT_GO_ID, "m_Caliber")
        .is_none());

    let mut edited_nested =
        PrefabVariantDefinition::load_from_file(&nested_path, NESTED_GUID, "NestedTurret")?;
    edited_nested.set_base_property(NESTED_ROOT_GO_ID, "m_Caliber", "22mm")?;
    scene_save::save_prefab_variant(&nested_path, &edited_nested)?;

    let mut changed_registry = PrefabRegistry::new();
    changed_registry.register(PrefabVariantDefinition::load_from_file(
        &nested_path,
        NESTED_GUID,
        "NestedTurret",
    )?);
    changed_registry.register(PrefabVariantDefinition::load_from_file(
        &base_path,
        BASE_GUID,
        "OuterVehicle",
    )?);
    changed_registry.register(PrefabVariantDefinition::load_from_file(
        &variant_path,
        VARIANT_A_GUID,
        "OuterVehicleVariant",
    )?);

    let changed = changed_registry.resolve_prefab(VARIANT_A_GUID)?;
    let changed_hierarchy = changed.as_complete().ok_or("expected complete hierarchy")?;
    let changed_nested = changed_hierarchy
        .nested_by_instance_file_id(NESTED_INSTANCE_FILE_ID)
        .ok_or("missing nested prefab resolution")?;
    let changed_nested_hierarchy = changed_nested
        .resolved
        .as_complete()
        .ok_or("expected complete nested hierarchy")?;
    assert_eq!(
        changed_nested_hierarchy.effective_property(NESTED_ROOT_GO_ID, "m_Caliber"),
        Some("25mm")
    );
    let changed_entries = changed_nested_hierarchy
        .property_inheritance_path(NESTED_ROOT_GO_ID, "m_Caliber")
        .ok_or("missing nested value inheritance path")?;
    assert_eq!(
        changed_entries,
        &[
            ValueProvenanceEntry {
                level: OverrideLevel::Prefab(NESTED_GUID.to_string()),
                value: String::from("22mm"),
                is_effective: false,
            },
            ValueProvenanceEntry {
                level: OverrideLevel::NestedPrefab {
                    owner: BASE_GUID.to_string(),
                    instance_file_id: NESTED_INSTANCE_FILE_ID,
                },
                value: String::from("25mm"),
                is_effective: true,
            },
        ]
    );
    assert_eq!(
        changed_entries.iter().filter(|entry| entry.is_effective).count(),
        1
    );
    assert_eq!(
        changed_hierarchy.effective_property(ROOT_GO_ID, "m_Tint"),
        Some("gold")
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn ser003_outcome8_missing_base_stays_a_broken_relationship_across_load_edit_save() -> TestResult {
    let root = temp_prefab_dir("ser003_outc8_broken_base")?;
    let variant_path = root.join("BrokenBaseVariant.prefab");

    let mut broken_variant = PrefabVariantDefinition::variant(
        VARIANT_A_GUID,
        "BrokenBaseVariant",
        PrefabSource::new(MISSING_GUID, ROOT_GO_ID),
    );
    broken_variant.record_override(Modification::new(ROOT_GO_ID, "m_Name", "OrphanedVariant"))?;
    broken_variant.record_override(Modification::new(ROOT_RIGIDBODY_ID, "m_Mass", "3200"))?;

    scene_save::save_prefab_variant(&variant_path, &broken_variant)?;

    let mut loaded = PrefabVariantDefinition::load_from_file(
        &variant_path,
        VARIANT_A_GUID,
        "BrokenBaseVariant",
    )?;
    let mut registry = PrefabRegistry::new();
    registry.register(loaded.clone());
    let resolved = registry.resolve_prefab(VARIANT_A_GUID)?;
    let broken = resolved
        .as_broken()
        .ok_or("expected broken base relationship")?;
    assert_eq!(broken.missing_guid, MISSING_GUID);
    assert_eq!(
        broken.broken_at_level,
        OverrideLevel::Prefab(VARIANT_A_GUID.to_string())
    );

    loaded.record_override(Modification::new(ROOT_RIGIDBODY_ID, "m_Mass", "3300"))?;
    scene_save::save_prefab_variant(&variant_path, &loaded)?;

    let reloaded = PrefabVariantDefinition::load_from_file(
        &variant_path,
        VARIANT_A_GUID,
        "BrokenBaseVariant",
    )?;
    let mut registry = PrefabRegistry::new();
    registry.register(reloaded);
    let resolved = registry.resolve_prefab(VARIANT_A_GUID)?;
    let broken = resolved
        .as_broken()
        .ok_or("expected broken base relationship after reload")?;
    assert_eq!(broken.missing_guid, MISSING_GUID);
    assert_eq!(
        broken.broken_at_level,
        OverrideLevel::Prefab(VARIANT_A_GUID.to_string())
    );
    assert_eq!(
        broken.chain_before_break,
        vec![OverrideLevel::Prefab(VARIANT_A_GUID.to_string())]
    );
    assert!(broken.retained_modifications.iter().any(|(level, m)| {
        *level == OverrideLevel::Prefab(VARIANT_A_GUID.to_string())
            && m.target_file_id == ROOT_RIGIDBODY_ID
            && m.property_path == "m_Mass"
            && m.value == "3300"
    }));
    assert!(broken.retained_modifications.iter().any(|(level, m)| {
        *level == OverrideLevel::Prefab(VARIANT_A_GUID.to_string())
            && m.target_file_id == ROOT_GO_ID
            && m.property_path == "m_Name"
            && m.value == "OrphanedVariant"
    }));

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn ser004_outcome10_missing_prefab_source_stays_identifiable_with_data_preserved_across_unrelated_edit_and_save(
) -> TestResult {
    let root = temp_prefab_dir("ser004_outc10_missing_source")?;
    let scene_path = root.join("BrokenSourceScene.unity");

    let instance = ScenePrefabInstance::new(
        "broken_source_instance",
        PrefabInstance::new(
            PrefabSource::new(MISSING_GUID, ROOT_GO_ID),
            0,
            vec![
                Modification::new(400, "m_Mass", "3200"),
                Modification::new(401, "m_Name", "PreservedBrokenInstance"),
            ],
        ),
    );

    let mut scene_stream =
        unity_yaml::parse(&scene_with_prefab_instance_yaml(MISSING_GUID, ROOT_GO_ID))?;
    scene_save::save_scene_prefab_instance(&scene_path, &mut scene_stream, &instance, 900100)?;

    let mut scene_stream = unity_yaml::parse_file(&scene_path)?;
    assert!(rename_game_object(
        &mut scene_stream.documents[0].root,
        "HealthySiblingEdited"
    ));
    scene_save::save_scene_prefab_instance(&scene_path, &mut scene_stream, &instance, 900100)?;

    let saved_text = fs::read_to_string(&scene_path)?;
    assert!(saved_text.contains(&format!("guid: {MISSING_GUID}")));
    assert!(saved_text.contains("HealthySiblingEdited"));
    assert!(saved_text.contains("PreservedBrokenInstance"));

    let reloaded =
        ScenePrefabInstance::load_from_file(&scene_path, "broken_source_instance", 900100)?;
    assert_eq!(reloaded.instance.source.guid, MISSING_GUID);
    assert_eq!(
        reloaded.instance.modifications,
        instance.instance.modifications
    );

    let registry = PrefabRegistry::new();
    let resolved = registry.resolve_instance(&reloaded)?;
    let broken = resolved
        .as_broken()
        .ok_or("expected broken prefab instance for missing source")?;
    assert_eq!(broken.missing_guid, MISSING_GUID);
    assert_eq!(
        broken.broken_at_level,
        OverrideLevel::SceneInstance(String::from("broken_source_instance"))
    );
    assert!(broken.retained_modifications.iter().any(|(level, m)| {
        *level == OverrideLevel::SceneInstance(String::from("broken_source_instance"))
            && m.target_file_id == 400
            && m.property_path == "m_Mass"
            && m.value == "3200"
    }));
    assert!(broken.retained_modifications.iter().any(|(level, m)| {
        *level == OverrideLevel::SceneInstance(String::from("broken_source_instance"))
            && m.target_file_id == 401
            && m.property_path == "m_Name"
            && m.value == "PreservedBrokenInstance"
    }));

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn ser003_outcome8_nested_instance_revert_touches_only_the_nested_ownership_level_across_load_edit_save(
) -> TestResult {
    let root = temp_prefab_dir("ser003_outc8_nested_revert")?;
    let nested_path = root.join("NestedTurret.prefab");
    let base_path = root.join("OuterVehicle.prefab");

    let registry = build_nested_registry()?;
    let nested_def = registry
        .get(NESTED_GUID)
        .ok_or("missing nested prefab")?
        .clone();
    let base_def = registry
        .get(BASE_GUID)
        .ok_or("missing outer prefab")?
        .clone();

    scene_save::save_prefab_variant(&nested_path, &nested_def)?;
    scene_save::save_prefab_variant(&base_path, &base_def)?;

    let mut loaded_registry = PrefabRegistry::new();
    loaded_registry.register(PrefabVariantDefinition::load_from_file(
        &nested_path,
        NESTED_GUID,
        "NestedTurret",
    )?);
    loaded_registry.register(PrefabVariantDefinition::load_from_file(
        &base_path,
        BASE_GUID,
        "OuterVehicle",
    )?);

    let nested_level = OverrideLevel::NestedPrefab {
        owner: BASE_GUID.to_string(),
        instance_file_id: NESTED_INSTANCE_FILE_ID,
    };
    loaded_registry.revert_override_at_level(
        None,
        &nested_level,
        NESTED_ROOT_GO_ID,
        "m_Caliber",
    )?;

    let reverted_base = loaded_registry
        .get(BASE_GUID)
        .ok_or("missing outer prefab")?
        .clone();
    assert!(reverted_base.nested_prefabs[0].modifications.is_empty());

    scene_save::save_prefab_variant(&base_path, &reverted_base)?;

    let mut reloaded_registry = PrefabRegistry::new();
    reloaded_registry.register(PrefabVariantDefinition::load_from_file(
        &nested_path,
        NESTED_GUID,
        "NestedTurret",
    )?);
    reloaded_registry.register(PrefabVariantDefinition::load_from_file(
        &base_path,
        BASE_GUID,
        "OuterVehicle",
    )?);

    let resolved = reloaded_registry.resolve_prefab(BASE_GUID)?;
    let hierarchy = resolved
        .as_complete()
        .ok_or("expected complete hierarchy")?;
    let nested = hierarchy
        .nested_by_instance_file_id(NESTED_INSTANCE_FILE_ID)
        .ok_or("missing nested prefab resolution")?;
    let nested_hierarchy = nested
        .resolved
        .as_complete()
        .ok_or("expected complete nested hierarchy")?;
    assert_eq!(
        nested_hierarchy.effective_property(NESTED_ROOT_GO_ID, "m_Caliber"),
        Some("20mm")
    );
    assert_eq!(
        nested_hierarchy.effective_owner(NESTED_ROOT_GO_ID, "m_Caliber"),
        Some(&OverrideLevel::Prefab(NESTED_GUID.to_string()))
    );
    assert_eq!(
        hierarchy.effective_property(ROOT_GO_ID, "m_Tint"),
        Some("silver")
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn ser003_outcome7_object_reference_override_round_trips_and_propagates_across_variants_and_instances(
) -> TestResult {
    let root = temp_prefab_dir("ser003_outc7_object_ref")?;
    let base_path = root.join("BaseVehicle.prefab");
    let variant_a_path = root.join("ArmoredVehicleVariant.prefab");
    let variant_b_path = root.join("DesertArmoredVariant.prefab");
    let scene_path = root.join("DesertLevel.unity");

    let (mut registry, mut instance) = build_vehicle_registry()?;
    let mat_guid_1 = "a1000000000000000000000000000001";
    let mat_guid_2 = "a1000000000000000000000000000002";

    let variant_a_mut = registry
        .get_mut(VARIANT_A_GUID)
        .ok_or("missing variant A")?;
    variant_a_mut.record_override(Modification::with_object_reference(
        ROOT_RIGIDBODY_ID,
        "m_Material",
        PrefabSourceReference::external(2100000, mat_guid_1, 2),
    ))?;
    variant_a_mut.record_override(Modification::with_object_reference(
        CHILD_B_GO_ID,
        "m_SharedMesh",
        PrefabSourceReference::local(4300000),
    ))?;

    instance.record_override(Modification::with_object_reference(
        ROOT_RIGIDBODY_ID,
        "m_Material",
        PrefabSourceReference::external(2100000, mat_guid_2, 2),
    ))?;

    scene_save::save_prefab_variant(
        &base_path,
        registry.get(BASE_GUID).ok_or("missing base")?,
    )?;
    scene_save::save_prefab_variant(
        &variant_a_path,
        registry.get(VARIANT_A_GUID).ok_or("missing variant A")?,
    )?;
    scene_save::save_prefab_variant(
        &variant_b_path,
        registry.get(VARIANT_B_GUID).ok_or("missing variant B")?,
    )?;

    let mut scene_stream =
        unity_yaml::parse(&scene_with_prefab_instance_yaml(VARIANT_B_GUID, ROOT_GO_ID))?;
    scene_save::save_scene_prefab_instance(&scene_path, &mut scene_stream, &instance, 900100)?;

    let saved_variant_a_yaml = fs::read_to_string(&variant_a_path)?;
    assert!(saved_variant_a_yaml.contains(&format!(
        "objectReference: {{fileID: 2100000, guid: {mat_guid_1}, type: 2}}"
    )));
    assert!(saved_variant_a_yaml.contains("objectReference: {fileID: 4300000}"));

    let saved_scene_yaml = fs::read_to_string(&scene_path)?;
    assert!(saved_scene_yaml.contains(&format!(
        "objectReference: {{fileID: 2100000, guid: {mat_guid_2}, type: 2}}"
    )));

    let mut loaded_registry = PrefabRegistry::new();
    loaded_registry.register(PrefabVariantDefinition::load_from_file(
        &base_path,
        BASE_GUID,
        "BaseVehicle",
    )?);
    loaded_registry.register(PrefabVariantDefinition::load_from_file(
        &variant_a_path,
        VARIANT_A_GUID,
        "ArmoredVehicleVariant",
    )?);
    loaded_registry.register(PrefabVariantDefinition::load_from_file(
        &variant_b_path,
        VARIANT_B_GUID,
        "DesertArmoredVariant",
    )?);
    let mut loaded_instance =
        ScenePrefabInstance::load_from_file(&scene_path, "scene_vehicle_1", 900100)?;

    let resolved = loaded_registry.resolve_instance(&loaded_instance)?;
    let hierarchy = resolved.as_complete().ok_or("expected complete")?;

    assert_eq!(
        hierarchy.effective_object_reference(ROOT_RIGIDBODY_ID, "m_Material"),
        Some(&PrefabSourceReference::external(2100000, mat_guid_2, 2))
    );
    assert_eq!(
        hierarchy.effective_owner(ROOT_RIGIDBODY_ID, "m_Material"),
        Some(&OverrideLevel::SceneInstance(String::from(
            "scene_vehicle_1"
        )))
    );
    assert_eq!(
        hierarchy.effective_object_reference(CHILD_B_GO_ID, "m_SharedMesh"),
        Some(&PrefabSourceReference::local(4300000))
    );
    assert_eq!(
        hierarchy.effective_owner(CHILD_B_GO_ID, "m_SharedMesh"),
        Some(&OverrideLevel::Prefab(VARIANT_A_GUID.to_string()))
    );

    let instance_level = OverrideLevel::SceneInstance(String::from("scene_vehicle_1"));
    let variant_b_level = OverrideLevel::Prefab(VARIANT_B_GUID.to_string());
    loaded_registry.apply_override_to_level(
        Some(&mut loaded_instance),
        &instance_level,
        &variant_b_level,
        ROOT_RIGIDBODY_ID,
        "m_Material",
    )?;

    let variant_b_after = loaded_registry
        .get(VARIANT_B_GUID)
        .ok_or("missing variant B")?
        .clone();
    scene_save::save_prefab_variant(&variant_b_path, &variant_b_after)?;
    let mut scene_stream_2 = unity_yaml::parse_file(&scene_path)?;
    scene_save::save_scene_prefab_instance(
        &scene_path,
        &mut scene_stream_2,
        &loaded_instance,
        900100,
    )?;

    let reloaded_variant_b = PrefabVariantDefinition::load_from_file(
        &variant_b_path,
        VARIANT_B_GUID,
        "DesertArmoredVariant",
    )?;
    let reloaded_instance =
        ScenePrefabInstance::load_from_file(&scene_path, "scene_vehicle_1", 900100)?;
    loaded_registry.register(reloaded_variant_b);

    let resolved_after = loaded_registry.resolve_instance(&reloaded_instance)?;
    let hierarchy_after = resolved_after.as_complete().ok_or("expected complete")?;
    assert_eq!(
        hierarchy_after.effective_object_reference(ROOT_RIGIDBODY_ID, "m_Material"),
        Some(&PrefabSourceReference::external(2100000, mat_guid_2, 2))
    );
    assert_eq!(
        hierarchy_after.effective_owner(ROOT_RIGIDBODY_ID, "m_Material"),
        Some(&variant_b_level)
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn ser003_outcome7_nested_prefab_apply_to_source_and_orphaned_override_resolution_round_trip(
) -> TestResult {
    let root = temp_prefab_dir("ser003_outc7_nested_apply_orphan")?;
    let nested_path = root.join("NestedTurret.prefab");
    let outer_path = root.join("OuterVehicle.prefab");
    let variant_path = root.join("OuterVehicleVariant.prefab");

    let mut registry = build_nested_registry()?;
    let nested_level = OverrideLevel::NestedPrefab {
        owner: BASE_GUID.to_string(),
        instance_file_id: NESTED_INSTANCE_FILE_ID,
    };
    let nested_source_level = OverrideLevel::Prefab(NESTED_GUID.to_string());

    registry.apply_override_to_level(
        None,
        &nested_level,
        &nested_source_level,
        NESTED_ROOT_GO_ID,
        "m_Caliber",
    )?;

    let nested_def = registry
        .get(NESTED_GUID)
        .ok_or("missing nested")?
        .clone();
    let outer_def = registry.get(BASE_GUID).ok_or("missing outer")?.clone();
    assert!(outer_def.nested_prefabs[0].modifications.is_empty());

    scene_save::save_prefab_variant(&nested_path, &nested_def)?;
    scene_save::save_prefab_variant(&outer_path, &outer_def)?;

    let mut reloaded_registry = PrefabRegistry::new();
    reloaded_registry.register(PrefabVariantDefinition::load_from_file(
        &nested_path,
        NESTED_GUID,
        "NestedTurret",
    )?);
    reloaded_registry.register(PrefabVariantDefinition::load_from_file(
        &outer_path,
        BASE_GUID,
        "OuterVehicle",
    )?);

    let resolved = reloaded_registry.resolve_prefab(BASE_GUID)?;
    let hierarchy = resolved.as_complete().ok_or("expected complete")?;
    let nested = hierarchy
        .nested_by_instance_file_id(NESTED_INSTANCE_FILE_ID)
        .ok_or("missing nested")?;
    let nested_hierarchy = nested
        .resolved
        .as_complete()
        .ok_or("expected complete nested")?;
    assert_eq!(
        nested_hierarchy.effective_property(NESTED_ROOT_GO_ID, "m_Caliber"),
        Some("25mm")
    );
    assert_eq!(
        nested_hierarchy.effective_owner(NESTED_ROOT_GO_ID, "m_Caliber"),
        Some(&nested_source_level)
    );

    let (mut veh_registry, mut veh_instance) = build_vehicle_registry()?;
    let variant_a_mut = veh_registry
        .get_mut(VARIANT_A_GUID)
        .ok_or("missing variant A")?;
    variant_a_mut.remove_child_override(SourceObjectId::new(BASE_GUID, CHILD_A_GO_ID));

    let resolved_with_orphan = veh_registry.resolve_instance(&veh_instance)?;
    let hierarchy_with_orphan = resolved_with_orphan
        .as_complete()
        .ok_or("expected complete")?;
    assert_eq!(hierarchy_with_orphan.orphaned_overrides.len(), 2);
    for orphan in &hierarchy_with_orphan.orphaned_overrides {
        assert_eq!(
            orphan.resolution_options(),
            &[
                OrphanedOverrideResolution::Discard,
                OrphanedOverrideResolution::Retain,
            ]
        );
    }

    let variant_a_level = OverrideLevel::Prefab(VARIANT_A_GUID.to_string());
    let variant_b_level = OverrideLevel::Prefab(VARIANT_B_GUID.to_string());
    veh_registry.resolve_orphaned_override(
        Some(&mut veh_instance),
        &variant_a_level,
        CHILD_A_GO_ID,
        "m_Radius",
        OrphanedOverrideResolution::Retain,
    )?;
    veh_registry.resolve_orphaned_override(
        Some(&mut veh_instance),
        &variant_b_level,
        CHILD_A_GO_ID,
        "m_Radius",
        OrphanedOverrideResolution::Discard,
    )?;

    let variant_b_saved = veh_registry
        .get(VARIANT_B_GUID)
        .ok_or("missing variant B")?
        .clone();
    scene_save::save_prefab_variant(&variant_path, &variant_b_saved)?;
    let reloaded_variant_b = PrefabVariantDefinition::load_from_file(
        &variant_path,
        VARIANT_B_GUID,
        "DesertArmoredVariant",
    )?;
    veh_registry.register(reloaded_variant_b);

    let resolved_cleaned = veh_registry.resolve_instance(&veh_instance)?;
    let hierarchy_cleaned = resolved_cleaned.as_complete().ok_or("expected complete")?;
    assert_eq!(hierarchy_cleaned.orphaned_overrides.len(), 1);
    assert_eq!(
        hierarchy_cleaned.orphaned_overrides[0].level,
        variant_a_level
    );

    veh_registry.resolve_orphaned_override(
        Some(&mut veh_instance),
        &variant_a_level,
        CHILD_A_GO_ID,
        "m_Radius",
        OrphanedOverrideResolution::Discard,
    )?;
    let resolved_all_cleaned = veh_registry.resolve_instance(&veh_instance)?;
    let hierarchy_all_cleaned = resolved_all_cleaned
        .as_complete()
        .ok_or("expected complete")?;
    assert!(hierarchy_all_cleaned.orphaned_overrides.is_empty());

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn ser003_outcome7_outcome8_unpack_one_level_and_completely_preserves_values_and_round_trips_to_scene_yaml(
) -> TestResult {
    let root = temp_prefab_dir("ser003_unpack_round_trip")?;
    let one_level_scene_path = root.join("UnpackedOneLevel.unity");
    let complete_scene_path = root.join("UnpackedCompletely.unity");

    let mut registry = build_nested_registry()?;
    let variant_mut = registry
        .get_mut(VARIANT_A_GUID)
        .ok_or("missing variant A")?;
    variant_mut.add_child_override(
        SourceObjectId::new(BASE_GUID, ROOT_GO_ID),
        PrefabNodeRecord::new(600, "AddedBanner", Some(ROOT_GO_ID))
            .with_property("m_Color", "red"),
        None,
    )?;

    let instance = ScenePrefabInstance::new(
        "scene_outer_1",
        PrefabInstance::new(
            PrefabSource::new(VARIANT_A_GUID, ROOT_GO_ID),
            0,
            vec![
                Modification::new(ROOT_GO_ID, "m_Name", "UnpackedOuterVehicle"),
                Modification::new(ROOT_TRANSFORM_ID, "m_LocalPosition.x", "77"),
            ],
        ),
    );

    let unpacked_one = registry.unpack_instance(&instance, UnpackMode::OneLevel)?;
    assert_eq!(unpacked_one.nested_prefabs.len(), 1);
    assert_eq!(
        unpacked_one.nested_prefabs[0].source.guid,
        NESTED_GUID
    );
    assert_eq!(
        unpacked_one.effective_property(ROOT_GO_ID, "m_Name"),
        Some("UnpackedOuterVehicle")
    );
    assert_eq!(
        unpacked_one.effective_property(ROOT_GO_ID, "m_Tint"),
        Some("gold")
    );
    assert_eq!(
        unpacked_one.effective_property(ROOT_TRANSFORM_ID, "m_LocalPosition.x"),
        Some("77")
    );
    assert_eq!(
        unpacked_one.effective_property(600, "m_Color"),
        Some("red")
    );

    let mut one_level_stream =
        unity_yaml::parse(&scene_with_prefab_instance_yaml(VARIANT_A_GUID, ROOT_GO_ID))?;
    unpacked_one.apply_to_scene_stream(&mut one_level_stream, 900100);
    scene_save::save_stream(&one_level_scene_path, &one_level_stream)?;

    let reloaded_one_stream = unity_yaml::parse_file(&one_level_scene_path)?;
    assert!(!reloaded_one_stream
        .documents
        .iter()
        .any(|d| d.class_id == 1001 && d.file_id == 900100));
    assert!(reloaded_one_stream
        .documents
        .iter()
        .any(|d| d.class_id == 1001 && d.file_id == NESTED_INSTANCE_FILE_ID));
    let reloaded_nested_inst = ScenePrefabInstance::from_stream(
        &reloaded_one_stream,
        "nested_after_one_level_unpack",
        NESTED_INSTANCE_FILE_ID,
    )
    .ok_or("expected nested prefab boundary to survive one-level unpack")?;
    assert_eq!(reloaded_nested_inst.instance.source.guid, NESTED_GUID);
    assert!(reloaded_nested_inst
        .instance
        .modifications
        .iter()
        .any(|m| m.target_file_id == NESTED_ROOT_GO_ID
            && m.property_path == "m_Caliber"
            && m.value == "25mm"));

    let unpacked_complete = registry.unpack_instance(&instance, UnpackMode::Completely)?;
    assert!(unpacked_complete.nested_prefabs.is_empty());
    assert_eq!(
        unpacked_complete.effective_property(ROOT_GO_ID, "m_Name"),
        Some("UnpackedOuterVehicle")
    );
    assert_eq!(
        unpacked_complete.effective_property(ROOT_GO_ID, "m_Tint"),
        Some("gold")
    );
    assert_eq!(
        unpacked_complete.effective_property(NESTED_ROOT_GO_ID, "m_Caliber"),
        Some("25mm")
    );
    assert_eq!(
        unpacked_complete.effective_property(NESTED_BARREL_ID, "m_Length"),
        Some("2")
    );

    let mut complete_stream =
        unity_yaml::parse(&scene_with_prefab_instance_yaml(VARIANT_A_GUID, ROOT_GO_ID))?;
    unpacked_complete.apply_to_scene_stream(&mut complete_stream, 900100);
    scene_save::save_stream(&complete_scene_path, &complete_stream)?;

    let reloaded_complete_stream = unity_yaml::parse_file(&complete_scene_path)?;
    assert!(!reloaded_complete_stream
        .documents
        .iter()
        .any(|d| d.class_id == 1001));
    let complete_yaml_text = fs::read_to_string(&complete_scene_path)?;
    assert!(complete_yaml_text.contains("m_Name: UnpackedOuterVehicle"));
    assert!(complete_yaml_text.contains("m_Tint: gold"));
    assert!(complete_yaml_text.contains("m_Caliber: 25mm"));
    assert!(complete_yaml_text.contains("m_Color: red"));

    let _ = fs::remove_dir_all(&root);
    Ok(())
}
