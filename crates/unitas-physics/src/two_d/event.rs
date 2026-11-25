use unitas_core::EntityId;

use crate::event::Phase;
use crate::matrix::CollisionMatrix;
use crate::two_d::body::{BodyId2D, BodyType2D};
use crate::two_d::collider::ColliderId2D;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactPoint2D {
    pub point: [f32; 2],
    pub normal: [f32; 2],
    pub separation: f32,
    pub normal_impulse: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind2D {
    CollisionEnter2D,
    CollisionStay2D,
    CollisionExit2D,
    TriggerEnter2D,
    TriggerStay2D,
    TriggerExit2D,
}

impl MessageKind2D {
    pub fn is_trigger(self) -> bool {
        matches!(
            self,
            Self::TriggerEnter2D | Self::TriggerStay2D | Self::TriggerExit2D
        )
    }

    pub fn phase(self) -> Phase {
        match self {
            Self::CollisionEnter2D | Self::TriggerEnter2D => Phase::Enter,
            Self::CollisionStay2D | Self::TriggerStay2D => Phase::Stay,
            Self::CollisionExit2D | Self::TriggerExit2D => Phase::Exit,
        }
    }

    pub fn from_phase(phase: Phase, is_trigger: bool) -> Self {
        match (phase, is_trigger) {
            (Phase::Enter, false) => Self::CollisionEnter2D,
            (Phase::Stay, false) => Self::CollisionStay2D,
            (Phase::Exit, false) => Self::CollisionExit2D,
            (Phase::Enter, true) => Self::TriggerEnter2D,
            (Phase::Stay, true) => Self::TriggerStay2D,
            (Phase::Exit, true) => Self::TriggerExit2D,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoutedMessage2D {
    pub recipient: EntityId,
    pub kind: MessageKind2D,
    pub self_collider: ColliderId2D,
    pub other_collider: ColliderId2D,
    pub other_body: Option<BodyId2D>,
    pub contacts: Vec<ContactPoint2D>,
    pub relative_velocity: [f32; 2],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Endpoint2D {
    pub(crate) collider_id: ColliderId2D,
    pub(crate) collider_game_object: EntityId,
    pub(crate) body_id: Option<BodyId2D>,
    pub(crate) body_game_object: Option<EntityId>,
    pub(crate) body_type: Option<BodyType2D>,
    pub(crate) use_full_kinematic_contacts: bool,
    pub(crate) is_trigger: bool,
    pub(crate) layer: u8,
    pub(crate) active: bool,
}

impl Endpoint2D {
    fn has_rigidbody(self) -> bool {
        self.body_type.is_some()
    }

    fn is_static(self) -> bool {
        matches!(self.body_type, None | Some(BodyType2D::Static))
    }

    fn is_kinematic(self) -> bool {
        self.body_type == Some(BodyType2D::Kinematic)
    }

    fn is_dynamic(self) -> bool {
        self.body_type == Some(BodyType2D::Dynamic)
    }

    fn recipients(self) -> Vec<EntityId> {
        let mut list = vec![self.collider_game_object];
        if let Some(body_go) = self.body_game_object {
            if !list.contains(&body_go) {
                list.push(body_go);
            }
        }
        list
    }
}

pub(crate) fn pair_messages_allowed_2d(
    a: Endpoint2D,
    b: Endpoint2D,
    matrix: &CollisionMatrix,
) -> bool {
    if !a.active || !b.active {
        return false;
    }
    if !matrix.is_enabled(a.layer as usize, b.layer as usize) {
        return false;
    }
    if !(a.has_rigidbody() || b.has_rigidbody()) {
        return false;
    }
    if a.is_static() && b.is_static() {
        return false;
    }
    let full_kinematic = a.use_full_kinematic_contacts || b.use_full_kinematic_contacts;
    if !full_kinematic {
        if a.is_kinematic() && !b.is_dynamic() {
            return false;
        }
        if b.is_kinematic() && !a.is_dynamic() {
            return false;
        }
    }
    true
}

fn endpoint_receives_2d(receiver: Endpoint2D, other: Endpoint2D, is_trigger: bool) -> bool {
    let full_kinematic = receiver.use_full_kinematic_contacts || other.use_full_kinematic_contacts;
    if !full_kinematic {
        if receiver.is_kinematic() && !other.is_dynamic() {
            return false;
        }
        if other.is_kinematic() && !receiver.is_dynamic() {
            return false;
        }
    }
    if is_trigger && receiver.is_trigger {
        return true;
    }
    if !receiver.has_rigidbody() || receiver.is_static() {
        return false;
    }
    true
}

pub(crate) fn route_pair_messages_2d(
    phase: Phase,
    is_trigger: bool,
    a: Endpoint2D,
    b: Endpoint2D,
    contacts_for_a: &[ContactPoint2D],
    rel_vel_for_a: [f32; 2],
    matrix: &CollisionMatrix,
) -> Vec<RoutedMessage2D> {
    if !pair_messages_allowed_2d(a, b, matrix) {
        return Vec::new();
    }
    let kind = MessageKind2D::from_phase(phase, is_trigger);
    let mut routed = Vec::new();
    if endpoint_receives_2d(a, b, is_trigger) {
        for recipient in a.recipients() {
            routed.push(RoutedMessage2D {
                recipient,
                kind,
                self_collider: a.collider_id,
                other_collider: b.collider_id,
                other_body: b.body_id,
                contacts: if is_trigger {
                    Vec::new()
                } else {
                    contacts_for_a.to_vec()
                },
                relative_velocity: if is_trigger {
                    [0.0, 0.0]
                } else {
                    rel_vel_for_a
                },
            });
        }
    }
    if endpoint_receives_2d(b, a, is_trigger) {
        let contacts_for_b: Vec<ContactPoint2D> = if is_trigger {
            Vec::new()
        } else {
            contacts_for_a
                .iter()
                .map(|c| ContactPoint2D {
                    point: c.point,
                    normal: [-c.normal[0], -c.normal[1]],
                    separation: c.separation,
                    normal_impulse: c.normal_impulse,
                })
                .collect()
        };
        let rel_vel_for_b = [-rel_vel_for_a[0], -rel_vel_for_a[1]];
        for recipient in b.recipients() {
            routed.push(RoutedMessage2D {
                recipient,
                kind,
                self_collider: b.collider_id,
                other_collider: a.collider_id,
                other_body: a.body_id,
                contacts: contacts_for_b.clone(),
                relative_velocity: if is_trigger {
                    [0.0, 0.0]
                } else {
                    rel_vel_for_b
                },
            });
        }
    }
    routed
}
