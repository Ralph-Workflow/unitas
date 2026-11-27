use std::collections::HashMap;
use unitas_core::EntityId;

use crate::unity_yaml::{Document, Node, Stream};

pub const HINGE_JOINT_CLASS: u32 = 59;
pub const FIXED_JOINT_CLASS: u32 = 138;
pub const CHARACTER_JOINT_CLASS: u32 = 144;
pub const SPRING_JOINT_CLASS: u32 = 145;
pub const CONFIGURABLE_JOINT_CLASS: u32 = 153;
pub const SPRING_JOINT2D_CLASS: u32 = 231;
pub const DISTANCE_JOINT2D_CLASS: u32 = 232;
pub const HINGE_JOINT2D_CLASS: u32 = 233;
pub const SLIDER_JOINT2D_CLASS: u32 = 234;
pub const WHEEL_JOINT2D_CLASS: u32 = 235;
pub const TARGET_JOINT2D_CLASS: u32 = 253;
pub const FIXED_JOINT2D_CLASS: u32 = 255;
pub const FRICTION_JOINT2D_CLASS: u32 = 256;
pub const RELATIVE_JOINT2D_CLASS: u32 = 257;

const RIGIDBODY_CLASS_ID: u32 = 54;
const RIGIDBODY2D_CLASS_ID: u32 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneJointKind3D {
    Fixed = 0,
    Hinge = 1,
    Spring = 2,
    Character = 3,
    Configurable = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneJointKind2D {
    Distance = 0,
    Fixed = 1,
    Friction = 2,
    Hinge = 3,
    Relative = 4,
    Slider = 5,
    Spring = 6,
    Target = 7,
    Wheel = 8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneJoint3DBinding {
    pub entity: EntityId,
    pub connected_entity: Option<EntityId>,
    pub kind: SceneJointKind3D,
    pub anchor: [f64; 3],
    pub connected_anchor: [f64; 3],
    pub axis: [f64; 3],
    pub secondary_axis: [f64; 3],
    pub use_limits: bool,
    pub limit_min: f64,
    pub limit_max: f64,
    pub use_spring: bool,
    pub spring: f64,
    pub damper: f64,
    pub target_angle: f64,
    pub use_motor: bool,
    pub motor_velocity: f64,
    pub motor_force: f64,
    pub motor_free_spin: bool,
    pub min_distance: f64,
    pub max_distance: f64,
    pub tolerance: f64,
    pub swing1_limit: f64,
    pub swing2_limit: f64,
    pub motions: [u8; 6],
    pub linear_limit: f64,
    pub x_drive: [f64; 3],
    pub y_drive: [f64; 3],
    pub z_drive: [f64; 3],
    pub target_position: [f64; 3],
    pub target_velocity: [f64; 3],
    pub break_force: f64,
    pub break_torque: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneJoint2DBinding {
    pub entity: EntityId,
    pub connected_entity: Option<EntityId>,
    pub kind: SceneJointKind2D,
    pub anchor: [f32; 2],
    pub connected_anchor: [f32; 2],
    pub distance: f32,
    pub max_distance_only: bool,
    pub damping_ratio: f32,
    pub frequency: f32,
    pub reference_angle: f32,
    pub max_force: f32,
    pub max_torque: f32,
    pub correction_scale: f32,
    pub linear_offset: [f32; 2],
    pub angular_offset: f32,
    pub angle_degrees: f32,
    pub use_motor: bool,
    pub motor_speed: f32,
    pub max_motor_torque: f32,
    pub use_limits: bool,
    pub limit_min: f32,
    pub limit_max: f32,
    pub target: [f32; 2],
    pub break_force: f32,
    pub break_torque: f32,
}

pub fn is_joint_class(class_id: u32) -> bool {
    matches!(
        class_id,
        HINGE_JOINT_CLASS
            | FIXED_JOINT_CLASS
            | CHARACTER_JOINT_CLASS
            | SPRING_JOINT_CLASS
            | CONFIGURABLE_JOINT_CLASS
            | SPRING_JOINT2D_CLASS
            | DISTANCE_JOINT2D_CLASS
            | HINGE_JOINT2D_CLASS
            | SLIDER_JOINT2D_CLASS
            | WHEEL_JOINT2D_CLASS
            | TARGET_JOINT2D_CLASS
            | FIXED_JOINT2D_CLASS
            | FRICTION_JOINT2D_CLASS
            | RELATIVE_JOINT2D_CLASS
    )
}

pub fn parse_scene_joints_3d(stream: &Stream) -> Vec<SceneJoint3DBinding> {
    let go_map = build_game_object_entity_map(stream);
    let comp_map = build_component_owner_map(stream, &go_map);
    stream
        .documents
        .iter()
        .filter(|doc| !doc.stripped)
        .filter_map(|doc| parse_doc_joint_3d(doc, &go_map, &comp_map))
        .collect()
}

pub fn parse_scene_joints_2d(stream: &Stream) -> Vec<SceneJoint2DBinding> {
    let go_map = build_game_object_entity_map(stream);
    let comp_map = build_component_owner_map(stream, &go_map);
    stream
        .documents
        .iter()
        .filter(|doc| !doc.stripped)
        .filter_map(|doc| parse_doc_joint_2d(doc, &go_map, &comp_map))
        .collect()
}

fn build_game_object_entity_map(stream: &Stream) -> HashMap<i64, EntityId> {
    let mut map = HashMap::new();
    let mut next_id = 0u64;
    for doc in &stream.documents {
        if doc.class_id == 1 && !doc.stripped {
            map.insert(doc.file_id, EntityId::new(next_id));
            next_id += 1;
        }
    }
    map
}

fn build_component_owner_map(
    stream: &Stream,
    go_map: &HashMap<i64, EntityId>,
) -> HashMap<i64, EntityId> {
    let mut map = go_map.clone();
    for doc in &stream.documents {
        if doc.stripped
            || (doc.class_id != RIGIDBODY_CLASS_ID && doc.class_id != RIGIDBODY2D_CLASS_ID)
        {
            continue;
        }
        let body = doc
            .root
            .get("Rigidbody")
            .or_else(|| doc.root.get("Rigidbody2D"));
        if let Some(go_id) = body.and_then(|b| ref_file_id(b.get("m_GameObject"))) {
            if let Some(&entity) = go_map.get(&go_id) {
                map.insert(doc.file_id, entity);
            }
        }
    }
    map
}

fn ref_file_id(node: Option<&Node>) -> Option<i64> {
    node?.get("fileID")?.as_i64().filter(|&id| id != 0)
}

fn resolve_owner_and_connected(
    body: &Node,
    go_map: &HashMap<i64, EntityId>,
    comp_map: &HashMap<i64, EntityId>,
) -> Option<(EntityId, Option<EntityId>)> {
    let go_id = ref_file_id(body.get("m_GameObject"))?;
    let owner = *go_map.get(&go_id)?;
    let connected = ref_file_id(
        body.get("m_ConnectedBody")
            .or_else(|| body.get("m_ConnectedRigidBody")),
    )
    .and_then(|id| comp_map.get(&id).copied())
    .filter(|&c| c != owner);
    Some((owner, connected))
}

fn vec3_or(node: Option<&Node>, default: [f64; 3]) -> [f64; 3] {
    let Some(n) = node else { return default };
    let (Some(x), Some(y), Some(z)) = (
        n.get("x").and_then(Node::as_f64),
        n.get("y").and_then(Node::as_f64),
        n.get("z").and_then(Node::as_f64),
    ) else {
        return default;
    };
    [x, y, z]
}

fn vec2_or(node: Option<&Node>, default: [f32; 2]) -> [f32; 2] {
    let Some(n) = node else { return default };
    let (Some(x), Some(y)) = (
        n.get("x").and_then(Node::as_f64),
        n.get("y").and_then(Node::as_f64),
    ) else {
        return default;
    };
    [x as f32, y as f32]
}

fn f64_or(node: Option<&Node>, default: f64) -> f64 {
    node.and_then(Node::as_f64)
        .filter(|v| v.is_finite())
        .unwrap_or(default)
}

fn f32_or(node: Option<&Node>, default: f32) -> f32 {
    node.and_then(Node::as_f64)
        .map(|v| v as f32)
        .filter(|v| v.is_finite())
        .unwrap_or(default)
}

fn bool_or(node: Option<&Node>, default: bool) -> bool {
    match node.and_then(Node::as_f64) {
        Some(v) => v != 0.0,
        None => default,
    }
}

fn motion_u8(node: Option<&Node>) -> u8 {
    match node.and_then(Node::as_i64) {
        Some(0) => 2,
        Some(1) => 1,
        _ => 0,
    }
}

fn drive_arr(node: Option<&Node>) -> [f64; 3] {
    [
        f64_or(node.and_then(|n| n.get("positionSpring")), 0.0),
        f64_or(node.and_then(|n| n.get("positionDamper")), 0.0),
        f64_or(node.and_then(|n| n.get("maximumForce")), f64::INFINITY),
    ]
}

fn parse_doc_joint_3d(
    doc: &Document,
    go_map: &HashMap<i64, EntityId>,
    comp_map: &HashMap<i64, EntityId>,
) -> Option<SceneJoint3DBinding> {
    let (key, kind) = match doc.class_id {
        FIXED_JOINT_CLASS => ("FixedJoint", SceneJointKind3D::Fixed),
        HINGE_JOINT_CLASS => ("HingeJoint", SceneJointKind3D::Hinge),
        SPRING_JOINT_CLASS => ("SpringJoint", SceneJointKind3D::Spring),
        CHARACTER_JOINT_CLASS => ("CharacterJoint", SceneJointKind3D::Character),
        CONFIGURABLE_JOINT_CLASS => ("ConfigurableJoint", SceneJointKind3D::Configurable),
        _ => return None,
    };
    let body = doc.root.get(key)?;
    let (entity, connected_entity) = resolve_owner_and_connected(body, go_map, comp_map)?;
    Some(build_scene_joint_3d(entity, connected_entity, kind, body))
}

fn build_scene_joint_3d(
    entity: EntityId,
    connected_entity: Option<EntityId>,
    kind: SceneJointKind3D,
    body: &Node,
) -> SceneJoint3DBinding {
    let lim = body.get("m_Limits");
    let spr = body.get("m_Spring");
    let mot = body.get("m_Motor");
    let (lmin, lmax) = parse_limits_3d(kind, body, lim);
    let (s1, s2) = parse_swing_limits_3d(body);
    SceneJoint3DBinding {
        entity,
        connected_entity,
        kind,
        anchor: vec3_or(body.get("m_Anchor"), [0.0; 3]),
        connected_anchor: vec3_or(body.get("m_ConnectedAnchor"), [0.0; 3]),
        axis: vec3_or(body.get("m_Axis"), [1.0, 0.0, 0.0]),
        secondary_axis: vec3_or(
            body.get("m_SwingAxis")
                .or_else(|| body.get("m_SecondaryAxis")),
            [0.0, 1.0, 0.0],
        ),
        use_limits: bool_or(body.get("m_UseLimits"), false),
        limit_min: lmin,
        limit_max: lmax,
        use_spring: bool_or(body.get("m_UseSpring"), false),
        spring: f64_or(
            spr.and_then(|n| n.get("spring"))
                .or_else(|| body.get("m_Spring")),
            10.0,
        ),
        damper: f64_or(
            spr.and_then(|n| n.get("damper"))
                .or_else(|| body.get("m_Damper")),
            0.2,
        ),
        target_angle: f64_or(spr.and_then(|n| n.get("targetPosition")), 0.0),
        use_motor: bool_or(body.get("m_UseMotor"), false),
        motor_velocity: f64_or(mot.and_then(|n| n.get("targetVelocity")), 0.0),
        motor_force: f64_or(mot.and_then(|n| n.get("force")), 0.0),
        motor_free_spin: bool_or(mot.and_then(|n| n.get("freeSpin")), false),
        min_distance: f64_or(body.get("m_MinDistance"), 0.0),
        max_distance: f64_or(body.get("m_MaxDistance"), 0.0),
        tolerance: f64_or(body.get("m_Tolerance"), 0.025),
        swing1_limit: s1,
        swing2_limit: s2,
        motions: parse_motions_3d(body),
        linear_limit: f64_or(body.get("m_LinearLimit").and_then(|n| n.get("limit")), 0.0),
        x_drive: drive_arr(body.get("m_XDrive")),
        y_drive: drive_arr(body.get("m_YDrive")),
        z_drive: drive_arr(body.get("m_ZDrive")),
        target_position: vec3_or(body.get("m_TargetPosition"), [0.0; 3]),
        target_velocity: vec3_or(body.get("m_TargetVelocity"), [0.0; 3]),
        break_force: f64_or(body.get("m_BreakForce"), f64::INFINITY),
        break_torque: f64_or(body.get("m_BreakTorque"), f64::INFINITY),
    }
}

fn parse_swing_limits_3d(body: &Node) -> (f64, f64) {
    let s1 = f64_or(
        body.get("m_Swing1Limit")
            .or_else(|| body.get("m_AngularYLimit"))
            .and_then(|n| n.get("limit")),
        45.0,
    );
    let s2 = f64_or(
        body.get("m_Swing2Limit")
            .or_else(|| body.get("m_AngularZLimit"))
            .and_then(|n| n.get("limit")),
        45.0,
    );
    (s1, s2)
}

fn parse_motions_3d(body: &Node) -> [u8; 6] {
    [
        motion_u8(body.get("m_XMotion")),
        motion_u8(body.get("m_YMotion")),
        motion_u8(body.get("m_ZMotion")),
        motion_u8(body.get("m_AngularXMotion")),
        motion_u8(body.get("m_AngularYMotion")),
        motion_u8(body.get("m_AngularZMotion")),
    ]
}

fn parse_limits_3d(kind: SceneJointKind3D, body: &Node, lim: Option<&Node>) -> (f64, f64) {
    if kind == SceneJointKind3D::Character || kind == SceneJointKind3D::Configurable {
        let low = f64_or(
            body.get("m_LowTwistLimit")
                .or_else(|| body.get("m_LowAngularXLimit"))
                .and_then(|n| n.get("limit")),
            -20.0,
        );
        let high = f64_or(
            body.get("m_HighTwistLimit")
                .or_else(|| body.get("m_HighAngularXLimit"))
                .and_then(|n| n.get("limit")),
            70.0,
        );
        return (low, high);
    }
    (
        f64_or(lim.and_then(|n| n.get("min")), 0.0),
        f64_or(lim.and_then(|n| n.get("max")), 0.0),
    )
}

fn parse_doc_joint_2d(
    doc: &Document,
    go_map: &HashMap<i64, EntityId>,
    comp_map: &HashMap<i64, EntityId>,
) -> Option<SceneJoint2DBinding> {
    let (key, kind) = match doc.class_id {
        DISTANCE_JOINT2D_CLASS => ("DistanceJoint2D", SceneJointKind2D::Distance),
        FIXED_JOINT2D_CLASS => ("FixedJoint2D", SceneJointKind2D::Fixed),
        FRICTION_JOINT2D_CLASS => ("FrictionJoint2D", SceneJointKind2D::Friction),
        HINGE_JOINT2D_CLASS => ("HingeJoint2D", SceneJointKind2D::Hinge),
        RELATIVE_JOINT2D_CLASS => ("RelativeJoint2D", SceneJointKind2D::Relative),
        SLIDER_JOINT2D_CLASS => ("SliderJoint2D", SceneJointKind2D::Slider),
        SPRING_JOINT2D_CLASS => ("SpringJoint2D", SceneJointKind2D::Spring),
        TARGET_JOINT2D_CLASS => ("TargetJoint2D", SceneJointKind2D::Target),
        WHEEL_JOINT2D_CLASS => ("WheelJoint2D", SceneJointKind2D::Wheel),
        _ => return None,
    };
    let body = doc.root.get(key)?;
    let (entity, connected_entity) = resolve_owner_and_connected(body, go_map, comp_map)?;
    Some(build_scene_joint_2d(entity, connected_entity, kind, body))
}

fn build_scene_joint_2d(
    entity: EntityId,
    connected_entity: Option<EntityId>,
    kind: SceneJointKind2D,
    body: &Node,
) -> SceneJoint2DBinding {
    let mot = body.get("m_Motor");
    let lim = body
        .get("m_AngleLimits")
        .or_else(|| body.get("m_TranslationLimits"));
    let susp = body.get("m_Suspension");
    SceneJoint2DBinding {
        entity,
        connected_entity,
        kind,
        anchor: vec2_or(body.get("m_Anchor"), [0.0; 2]),
        connected_anchor: vec2_or(body.get("m_ConnectedAnchor"), [0.0; 2]),
        distance: f32_or(body.get("m_Distance"), 1.0),
        max_distance_only: bool_or(body.get("m_MaxDistanceOnly"), false),
        damping_ratio: f32_or(
            susp.and_then(|n| n.get("m_DampingRatio"))
                .or_else(|| body.get("m_DampingRatio")),
            0.0,
        ),
        frequency: f32_or(
            susp.and_then(|n| n.get("m_Frequency"))
                .or_else(|| body.get("m_Frequency")),
            2.0,
        ),
        reference_angle: f32_or(body.get("m_ReferenceAngle"), 0.0),
        max_force: f32_or(body.get("m_MaxForce"), 10000.0),
        max_torque: f32_or(body.get("m_MaxTorque"), 10000.0),
        correction_scale: f32_or(body.get("m_CorrectionScale"), 0.3),
        linear_offset: vec2_or(body.get("m_LinearOffset"), [0.0; 2]),
        angular_offset: f32_or(body.get("m_AngularOffset"), 0.0),
        angle_degrees: f32_or(
            susp.and_then(|n| n.get("m_Angle"))
                .or_else(|| body.get("m_Angle")),
            if kind == SceneJointKind2D::Wheel {
                90.0
            } else {
                0.0
            },
        ),
        use_motor: bool_or(body.get("m_UseMotor"), false),
        motor_speed: f32_or(mot.and_then(|n| n.get("m_MotorSpeed")), 0.0),
        max_motor_torque: f32_or(mot.and_then(|n| n.get("m_MaximumMotorForce")), 10000.0),
        use_limits: bool_or(body.get("m_UseLimits"), false),
        limit_min: f32_or(
            lim.and_then(|n| {
                n.get("m_LowerAngle")
                    .or_else(|| n.get("m_LowerTranslation"))
            }),
            0.0,
        ),
        limit_max: f32_or(
            lim.and_then(|n| {
                n.get("m_UpperAngle")
                    .or_else(|| n.get("m_UpperTranslation"))
            }),
            0.0,
        ),
        target: vec2_or(body.get("m_Target"), [0.0; 2]),
        break_force: f32_or(body.get("m_BreakForce"), f32::INFINITY),
        break_torque: f32_or(body.get("m_BreakTorque"), f32::INFINITY),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unity_yaml;

    const HINGE_SCENE: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &10\nGameObject:\n  m_Name: DoorFrame\n--- !u!4 &11\nTransform:\n  m_GameObject: {fileID: 10}\n  m_Father: {fileID: 0}\n--- !u!54 &12\nRigidbody:\n  m_GameObject: {fileID: 10}\n  m_IsKinematic: 1\n--- !u!1 &20\nGameObject:\n  m_Name: DoorLeaf\n--- !u!4 &21\nTransform:\n  m_GameObject: {fileID: 20}\n  m_Father: {fileID: 0}\n--- !u!54 &22\nRigidbody:\n  m_GameObject: {fileID: 20}\n  m_Mass: 2\n--- !u!59 &23\nHingeJoint:\n  m_GameObject: {fileID: 20}\n  m_ConnectedBody: {fileID: 12}\n  m_Anchor: {x: -0.5, y: 0, z: 0}\n  m_Axis: {x: 0, y: 0, z: 1}\n  m_ConnectedAnchor: {x: 0.5, y: 0, z: 0}\n  m_UseMotor: 1\n  m_Motor:\n    targetVelocity: 3.0\n    force: 50\n    freeSpin: 0\n  m_UseLimits: 1\n  m_Limits:\n    min: -0.4\n    max: 0.6\n";

    #[test]
    fn parses_hinge_joint_from_scene_yaml() {
        let Ok(stream) = unity_yaml::parse(HINGE_SCENE) else {
            return;
        };
        let joints = parse_scene_joints_3d(&stream);
        assert_eq!(joints.len(), 1);
        assert_eq!(joints[0].entity, EntityId::new(1));
        assert_eq!(joints[0].connected_entity, Some(EntityId::new(0)));
        assert_eq!(joints[0].kind, SceneJointKind3D::Hinge);
        assert_eq!(joints[0].anchor, [-0.5, 0.0, 0.0]);
        assert_eq!(joints[0].connected_anchor, [0.5, 0.0, 0.0]);
        assert!(joints[0].use_motor && joints[0].use_limits);
    }
}
