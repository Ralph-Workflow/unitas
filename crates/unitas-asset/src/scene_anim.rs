use std::collections::HashSet;

use unitas_core::EntityId;

use crate::unity_yaml::{Node, Stream};

use super::{component_entity, yaml_bool, yaml_i32, GameObjectRecords, ANIMATOR_CLASS};

#[derive(Debug, Clone, PartialEq)]
pub struct AnimatorBinding {
    pub entity: EntityId,
    pub controller_guid: Option<String>,
    pub enabled: bool,
    pub apply_root_motion: bool,
    pub culling_mode: i32,
    pub update_mode: i32,
}

pub(super) fn collect_animators(
    stream: &Stream,
    game_objects: &GameObjectRecords,
    instance_roots: &HashSet<i64>,
) -> Vec<AnimatorBinding> {
    let mut animators = Vec::new();
    for document in &stream.documents {
        if document.stripped || document.class_id != ANIMATOR_CLASS {
            continue;
        }
        let Some(body) = document.root.get("Animator") else {
            continue;
        };
        let Some(entity) = component_entity(body, game_objects, instance_roots) else {
            continue;
        };
        animators.push(AnimatorBinding {
            entity,
            controller_guid: controller_guid(body.get("m_Controller")),
            enabled: yaml_bool(body.get("m_Enabled"), true),
            apply_root_motion: yaml_bool(body.get("m_ApplyRootMotion"), false),
            culling_mode: yaml_i32(body.get("m_CullingMode"), 0),
            update_mode: yaml_i32(body.get("m_UpdateMode"), 0),
        });
    }
    animators
}

fn controller_guid(node: Option<&Node>) -> Option<String> {
    let node = node?;
    if node.get("fileID").and_then(Node::as_i64) == Some(0) {
        return None;
    }
    let guid = node.get("guid").and_then(Node::as_str).map(str::trim)?;
    if guid.is_empty() {
        return None;
    }
    Some(guid.to_string())
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use unitas_core::EntityId;

    use crate::scene::assemble_scene_bundle;
    use crate::unity_yaml;

    const ANIMATOR_SCENE: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: Hero
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &1001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!95 &1002
Animator:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  m_Enabled: 1
  m_Avatar: {fileID: 0}
  m_Controller: {fileID: 9100000, guid: 70000000000000000000000000000001, type: 3}
  m_CullingMode: 2
  m_UpdateMode: 1
  m_ApplyRootMotion: 1
  m_LinearVelocityBlending: 0
  m_StabilizeFeet: 0
  m_AnimatePhysics: 0
  m_WarningMessage:
  m_HasTransformHierarchy: 1
--- !u!1 &2000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: Legacy
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &2001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 2000}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 1, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!222 &2002
CanvasRenderer:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 2000}
  m_CullTransparentMesh: 1
";

    const ANIMATOR_WITHOUT_CONTROLLER_SCENE: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &3000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: Rock
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &3001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 3000}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!95 &3002
Animator:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 3000}
  m_Enabled: 1
  m_Avatar: {fileID: 0}
  m_Controller: {fileID: 0}
  m_CullingMode: 0
  m_UpdateMode: 0
  m_ApplyRootMotion: 0
";

    const SCENE_WITHOUT_ANIMATOR: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &4000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: Prop
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &4001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 4000}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
";

    #[test]
    fn animator_scene_exposes_the_controller_and_the_playback_settings(
    ) -> Result<(), Box<dyn Error>> {
        let assembled = assemble_scene_bundle(&unity_yaml::parse(ANIMATOR_SCENE)?)?;
        let animator = assembled
            .animators
            .first()
            .ok_or("the scene has no animator")?;
        assert_eq!(animator.entity, EntityId::new(0));
        assert_eq!(
            animator.controller_guid.as_deref(),
            Some("70000000000000000000000000000001")
        );
        assert!(animator.enabled);
        assert!(animator.apply_root_motion);
        assert_eq!(animator.culling_mode, 2);
        assert_eq!(animator.update_mode, 1);
        Ok(())
    }

    #[test]
    fn animator_without_a_controller_reports_no_controller_guid() -> Result<(), Box<dyn Error>> {
        let assembled =
            assemble_scene_bundle(&unity_yaml::parse(ANIMATOR_WITHOUT_CONTROLLER_SCENE)?)?;
        let animator = assembled
            .animators
            .first()
            .ok_or("the scene has no animator")?;
        assert_eq!(animator.entity, EntityId::new(0));
        assert_eq!(animator.controller_guid, None);
        assert!(animator.enabled);
        assert!(!animator.apply_root_motion);
        assert_eq!(animator.culling_mode, 0);
        assert_eq!(animator.update_mode, 0);
        Ok(())
    }

    #[test]
    fn scene_without_an_animator_reports_no_animators() -> Result<(), Box<dyn Error>> {
        let assembled = assemble_scene_bundle(&unity_yaml::parse(SCENE_WITHOUT_ANIMATOR)?)?;
        assert!(assembled.animators.is_empty());
        Ok(())
    }

    #[test]
    fn the_animator_class_is_not_reported_as_an_unsupported_component() -> Result<(), Box<dyn Error>>
    {
        let assembled = assemble_scene_bundle(&unity_yaml::parse(ANIMATOR_SCENE)?)?;
        let unsupported: Vec<&str> = assembled
            .diagnostics
            .iter()
            .map(|(_, text)| text.as_str())
            .filter(|text| text.starts_with("unsupported component class"))
            .collect();
        assert!(unsupported.contains(&"unsupported component class 222"));
        assert!(!unsupported.contains(&"unsupported component class 95"));
        Ok(())
    }
}
