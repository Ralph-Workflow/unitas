use unitas_core::EntityId;

use crate::unity_yaml::Node;

pub const TRAIL_RENDERER_CLASS: u32 = 96;

const DEFAULT_TRAIL_TIME: f32 = 1.0;
const DEFAULT_TRAIL_WIDTH: f32 = 1.0;
const DEFAULT_TRAIL_MIN_VERTEX_DISTANCE: f32 = 0.0;
const DEFAULT_TRAIL_COLOR: [f32; 4] = [1.0; 4];

#[derive(Debug, Clone, PartialEq)]
pub struct TrailRendererBinding {
    pub entity: EntityId,
    pub time: f32,
    pub width_start: f32,
    pub width_end: f32,
    pub min_vertex_distance: f32,
    pub start_color: [f32; 4],
    pub end_color: [f32; 4],
}

pub fn trail_renderer_component(entity: EntityId, body: &Node) -> TrailRendererBinding {
    let (width_start, width_end) = width_endpoints(body);
    TrailRendererBinding {
        entity,
        time: yaml_f32(body.get("m_Time"), DEFAULT_TRAIL_TIME, |value| value >= 0.0),
        width_start,
        width_end,
        min_vertex_distance: yaml_f32(
            body.get("m_MinVertexDistance"),
            DEFAULT_TRAIL_MIN_VERTEX_DISTANCE,
            |value| value >= 0.0,
        ),
        start_color: color4_or(body.get("m_StartColor"), DEFAULT_TRAIL_COLOR),
        end_color: color4_or(body.get("m_EndColor"), DEFAULT_TRAIL_COLOR),
    }
}

fn width_endpoints(body: &Node) -> (f32, f32) {
    if let Some(curve) = body.get("m_WidthCurve") {
        let start = curve
            .get("start")
            .and_then(Node::as_f64)
            .filter(|value| value.is_finite());
        let end = curve
            .get("end")
            .and_then(Node::as_f64)
            .filter(|value| value.is_finite());
        if let (Some(start), Some(end)) = (start, end) {
            return (start as f32, end as f32);
        }
        if let Some(keys) = curve.get("m_Curve").and_then(Node::as_sequence) {
            let start = keys
                .first()
                .and_then(|key| key.get("value"))
                .and_then(Node::as_f64)
                .filter(|value| value.is_finite());
            let end = keys
                .last()
                .and_then(|key| key.get("value"))
                .and_then(Node::as_f64)
                .filter(|value| value.is_finite());
            if let (Some(start), Some(end)) = (start, end) {
                return (start as f32, end as f32);
            }
        }
    }
    let width = yaml_f32(body.get("m_Width"), DEFAULT_TRAIL_WIDTH, |value| {
        value >= 0.0
    });
    (width, width)
}

fn color4_or(node: Option<&Node>, default: [f32; 4]) -> [f32; 4] {
    let node = match node {
        Some(node) => node,
        None => return default,
    };
    let red = node.get("r").and_then(Node::as_f64);
    let green = node.get("g").and_then(Node::as_f64);
    let blue = node.get("b").and_then(Node::as_f64);
    match (red, green, blue) {
        (Some(red), Some(green), Some(blue)) => {
            let alpha = node
                .get("a")
                .and_then(Node::as_f64)
                .filter(|value| value.is_finite())
                .unwrap_or(1.0);
            [red as f32, green as f32, blue as f32, alpha as f32]
        }
        _ => default,
    }
}

fn yaml_f32(node: Option<&Node>, default: f32, valid: impl Fn(f32) -> bool) -> f32 {
    match node.and_then(Node::as_f64) {
        Some(value) if value.is_finite() => {
            let value = value as f32;
            if value.is_finite() && valid(value) {
                value
            } else {
                default
            }
        }
        _ => default,
    }
}
