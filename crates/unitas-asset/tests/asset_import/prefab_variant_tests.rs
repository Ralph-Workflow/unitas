use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::prefab::{Modification, PrefabInstance, PrefabSource};
use unitas_asset::prefab_variant::{
    NestedPrefabReference, OverrideLevel, PrefabComponentRecord, PrefabNodeRecord, PrefabRegistry,
    PrefabVariantDefinition, PrefabVariantError, ScenePrefabInstance, SourceObjectId,
    TargetedModification, ValueProvenanceEntry,
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
