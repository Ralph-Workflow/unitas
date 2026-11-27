use std::error::Error;
use std::path::Path;

use unitas_asset::prefab::{Modification, PrefabInstance, PrefabSource};
use unitas_asset::prefab_variant::{
    OverrideLevel, PrefabComponentRecord, PrefabNodeRecord, PrefabRegistry,
    PrefabVariantDefinition, PrefabVariantError, ResolvedHierarchy, ScenePrefabInstance,
    SourceObjectId,
};

type TestResult = Result<(), Box<dyn Error>>;

const BASE_GUID: &str = "b0000000000000000000000000000001";
const VARIANT_GUID: &str = "b0000000000000000000000000000002";

const ROOT_GO_ID: i64 = 100;
const ROOT_TRANSFORM_ID: i64 = 101;
const ROOT_RIGIDBODY_ID: i64 = 102;
const FRONT_WHEEL_ID: i64 = 200;
const REAR_WHEEL_ID: i64 = 300;
const SHIELD_ID: i64 = 400;

const PICKUP_GUID: &str = "22222222222222222222222222220001";
const PICKUP_GOLD_GUID: &str = "22222222222222222222222222220002";
const PICKUP_BEHAVIOUR_ID: i64 = 11400000;
const PICKUP_PREFAB_FIXTURE: &str = "tests/fixtures/prefab_project/Assets/Prefabs/Pickup.prefab";
const PICKUP_GOLD_PREFAB_FIXTURE: &str =
    "tests/fixtures/prefab_project/Assets/Prefabs/PickupGold.prefab";

fn base_definition() -> PrefabVariantDefinition {
    let root = PrefabNodeRecord::new(ROOT_GO_ID, "BaseRig", None)
        .with_children(vec![FRONT_WHEEL_ID, REAR_WHEEL_ID])
        .with_property("m_Layer", "0")
        .with_component(
            PrefabComponentRecord::new(ROOT_TRANSFORM_ID, "Transform")
                .with_property("m_LocalPosition.x", "0"),
        )
        .with_component(
            PrefabComponentRecord::new(ROOT_RIGIDBODY_ID, "Rigidbody")
                .with_property("m_Mass", "10")
                .with_property("m_Drag", "0.5"),
        );
    let front = PrefabNodeRecord::new(FRONT_WHEEL_ID, "FrontWheel", Some(ROOT_GO_ID))
        .with_property("m_Radius", "0.3");
    let rear = PrefabNodeRecord::new(REAR_WHEEL_ID, "RearWheel", Some(ROOT_GO_ID))
        .with_property("m_Radius", "0.3");
    PrefabVariantDefinition::base_prefab(BASE_GUID, "BaseRig", ROOT_GO_ID, vec![root, front, rear])
}

fn variant_definition() -> Result<PrefabVariantDefinition, PrefabVariantError> {
    let mut variant = PrefabVariantDefinition::variant(
        VARIANT_GUID,
        "RigVariant",
        PrefabSource::new(BASE_GUID, ROOT_GO_ID),
    );
    variant.record_override(Modification::new(ROOT_RIGIDBODY_ID, "m_Mass", "20"))?;
    Ok(variant)
}

fn instance(instance_id: &str, modifications: Vec<Modification>) -> ScenePrefabInstance {
    ScenePrefabInstance::new(
        instance_id,
        PrefabInstance::new(
            PrefabSource::new(VARIANT_GUID, ROOT_GO_ID),
            0,
            modifications,
        ),
    )
}

fn build_registry() -> Result<(PrefabRegistry, Vec<ScenePrefabInstance>), PrefabVariantError> {
    let mut registry = PrefabRegistry::new();
    registry.register(base_definition());
    registry.register(variant_definition()?);
    let instances = vec![
        instance("rig_instance_1", Vec::new()),
        instance("rig_instance_2", Vec::new()),
        instance(
            "rig_instance_3",
            vec![Modification::new(ROOT_RIGIDBODY_ID, "m_Mass", "99")],
        ),
    ];
    Ok((registry, instances))
}

fn hierarchy(
    registry: &PrefabRegistry,
    instance: &ScenePrefabInstance,
) -> Result<ResolvedHierarchy, Box<dyn Error>> {
    let resolved = registry.resolve_instance(instance)?;
    let hierarchy = resolved
        .as_complete()
        .ok_or("expected a complete hierarchy")?;
    Ok(hierarchy.clone())
}

fn effective(
    registry: &PrefabRegistry,
    instance: &ScenePrefabInstance,
    target_file_id: i64,
    property_path: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let hierarchy = hierarchy(registry, instance)?;
    Ok(hierarchy
        .effective_property(target_file_id, property_path)
        .map(str::to_string))
}

#[test]
fn variant_property_override_edit_propagates_to_every_instance() -> TestResult {
    let (mut registry, instances) = build_registry()?;

    for instance in &instances {
        assert_eq!(
            effective(&registry, instance, ROOT_RIGIDBODY_ID, "m_Mass")?,
            if instance.instance_id == "rig_instance_3" {
                Some(String::from("99"))
            } else {
                Some(String::from("20"))
            }
        );
    }

    let variant = registry
        .get_mut(VARIANT_GUID)
        .ok_or("variant missing from registry")?;
    variant.record_override(Modification::new(ROOT_RIGIDBODY_ID, "m_Mass", "25"))?;

    for instance in &instances {
        assert_eq!(
            effective(&registry, instance, ROOT_RIGIDBODY_ID, "m_Mass")?,
            if instance.instance_id == "rig_instance_3" {
                Some(String::from("99"))
            } else {
                Some(String::from("25"))
            },
            "instance {} did not receive the variant override edit",
            instance.instance_id
        );
    }

    let hierarchy = hierarchy(&registry, &instances[0])?;
    assert_eq!(
        hierarchy.effective_owner(ROOT_RIGIDBODY_ID, "m_Mass"),
        Some(&OverrideLevel::Prefab(VARIANT_GUID.to_string()))
    );

    Ok(())
}

#[test]
fn variant_structural_override_edit_propagates_to_every_instance() -> TestResult {
    let (mut registry, instances) = build_registry()?;

    for instance in &instances {
        let hierarchy = hierarchy(&registry, instance)?;
        assert!(hierarchy.node_by_file_id(SHIELD_ID).is_none());
        assert!(hierarchy.node_by_file_id(REAR_WHEEL_ID).is_some());
    }

    let variant = registry
        .get_mut(VARIANT_GUID)
        .ok_or("variant missing from registry")?;
    variant.add_child_override(
        SourceObjectId::new(BASE_GUID, ROOT_GO_ID),
        PrefabNodeRecord::new(SHIELD_ID, "Shield", Some(ROOT_GO_ID)),
        None,
    )?;
    variant.remove_child_override(SourceObjectId::new(BASE_GUID, REAR_WHEEL_ID));

    for instance in &instances {
        let hierarchy = hierarchy(&registry, instance)?;
        let shield = hierarchy
            .node_by_file_id(SHIELD_ID)
            .ok_or("variant added child missing from instance")?;
        assert_eq!(shield.name, "Shield");
        assert_eq!(
            shield.parent,
            Some(SourceObjectId::new(BASE_GUID, ROOT_GO_ID))
        );
        assert!(hierarchy.node_by_file_id(REAR_WHEEL_ID).is_none());
    }

    Ok(())
}

#[test]
fn base_prefab_edits_reach_instances_unless_the_variant_overrides_them() -> TestResult {
    let (mut registry, instances) = build_registry()?;

    let base = registry
        .get_mut(BASE_GUID)
        .ok_or("base prefab missing from registry")?;
    base.set_base_property(FRONT_WHEEL_ID, "m_Radius", "0.45")?;
    base.set_base_property(ROOT_RIGIDBODY_ID, "m_Mass", "15")?;
    base.reorder_source_children(ROOT_GO_ID, vec![REAR_WHEEL_ID, FRONT_WHEEL_ID])?;

    for instance in &instances {
        assert_eq!(
            effective(&registry, instance, FRONT_WHEEL_ID, "m_Radius")?,
            Some(String::from("0.45"))
        );
        assert_eq!(
            effective(&registry, instance, ROOT_RIGIDBODY_ID, "m_Mass")?,
            if instance.instance_id == "rig_instance_3" {
                Some(String::from("99"))
            } else {
                Some(String::from("20"))
            },
            "instance {} lost the variant override of a base edited property",
            instance.instance_id
        );

        let hierarchy = hierarchy(&registry, instance)?;
        let root = hierarchy
            .node(&SourceObjectId::new(BASE_GUID, ROOT_GO_ID))
            .ok_or("root node missing from instance")?;
        let child_ids: Vec<i64> = root
            .children
            .iter()
            .map(|child| child.target_file_id)
            .collect();
        assert_eq!(child_ids, vec![REAR_WHEEL_ID, FRONT_WHEEL_ID]);
    }

    Ok(())
}

#[test]
fn variant_override_edit_on_a_loaded_prefab_reaches_every_instance() -> TestResult {
    let mut registry = PrefabRegistry::new();
    registry.register(PrefabVariantDefinition::load_from_file(
        Path::new(PICKUP_PREFAB_FIXTURE),
        PICKUP_GUID,
        "Pickup",
    )?);
    registry.register(PrefabVariantDefinition::load_from_file(
        Path::new(PICKUP_GOLD_PREFAB_FIXTURE),
        PICKUP_GOLD_GUID,
        "PickupGold",
    )?);

    let instances = vec![
        ScenePrefabInstance::new(
            "gold_instance_1",
            PrefabInstance::new(
                PrefabSource::new(PICKUP_GOLD_GUID, 100100000),
                0,
                Vec::new(),
            ),
        ),
        ScenePrefabInstance::new(
            "gold_instance_2",
            PrefabInstance::new(
                PrefabSource::new(PICKUP_GOLD_GUID, 100100000),
                0,
                Vec::new(),
            ),
        ),
    ];

    for instance in &instances {
        assert_eq!(
            effective(&registry, instance, PICKUP_BEHAVIOUR_ID, "score")?,
            Some(String::from("5"))
        );
    }

    let variant = registry
        .get_mut(PICKUP_GOLD_GUID)
        .ok_or("loaded variant missing from registry")?;
    variant.record_override(Modification::new(PICKUP_BEHAVIOUR_ID, "score", "9"))?;

    for instance in &instances {
        assert_eq!(
            effective(&registry, instance, PICKUP_BEHAVIOUR_ID, "score")?,
            Some(String::from("9")),
            "instance {} did not receive the loaded variant override edit",
            instance.instance_id
        );
    }

    let base = registry
        .get_mut(PICKUP_GUID)
        .ok_or("loaded base prefab missing from registry")?;
    base.set_base_property(PICKUP_BEHAVIOUR_ID, "m_Enabled", "0")?;

    for instance in &instances {
        assert_eq!(
            effective(&registry, instance, PICKUP_BEHAVIOUR_ID, "m_Enabled")?,
            Some(String::from("0")),
            "instance {} did not receive the base prefab edit",
            instance.instance_id
        );
    }

    Ok(())
}
