use std::fmt;
use std::path::Path;

use crate::dependencies::{ImportDiagnostic, ImportOutcome};
use crate::unity_yaml::{parse, YamlError, YamlNode};

#[path = "controller_import.rs"]
mod controller_import;

pub use controller_import::import_animator_controller;

pub const ANIMATION_CLIP_CLASS_ID: u32 = 74;

#[derive(Debug, Clone, PartialEq)]
pub struct AnimKeyframeVec3 {
    pub time: f64,
    pub value: [f64; 3],
    pub in_slope: [f64; 3],
    pub out_slope: [f64; 3],
    pub weighted_mode: i32,
    pub in_weight: [f64; 3],
    pub out_weight: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimKeyframeQuat {
    pub time: f64,
    pub value: [f64; 4],
    pub in_slope: [f64; 4],
    pub out_slope: [f64; 4],
    pub weighted_mode: i32,
    pub in_weight: [f64; 4],
    pub out_weight: [f64; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimKeyframeFloat {
    pub time: f64,
    pub value: f64,
    pub in_slope: f64,
    pub out_slope: f64,
    pub weighted_mode: i32,
    pub in_weight: f64,
    pub out_weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimVector3Curve {
    pub path: String,
    pub keyframes: Vec<AnimKeyframeVec3>,
    pub pre_infinity: i32,
    pub post_infinity: i32,
    pub rotation_order: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimQuaternionCurve {
    pub path: String,
    pub keyframes: Vec<AnimKeyframeQuat>,
    pub pre_infinity: i32,
    pub post_infinity: i32,
    pub rotation_order: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimFloatCurve {
    pub path: String,
    pub attribute: String,
    pub class_id: i32,
    pub script_guid: Option<String>,
    pub keyframes: Vec<AnimKeyframeFloat>,
    pub pre_infinity: i32,
    pub post_infinity: i32,
    pub rotation_order: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimPPtrKeyframe {
    pub time: f64,
    pub file_id: i64,
    pub guid: Option<String>,
    pub reference_type: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimPPtrCurve {
    pub path: String,
    pub attribute: String,
    pub class_id: i32,
    pub script_guid: Option<String>,
    pub keyframes: Vec<AnimPPtrKeyframe>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimClipSettings {
    pub start_time: f64,
    pub stop_time: f64,
    pub orientation_offset_y: f64,
    pub level: f64,
    pub cycle_offset: f64,
    pub has_additive_reference_pose: bool,
    pub loop_time: bool,
    pub loop_blend: bool,
    pub loop_blend_orientation: bool,
    pub loop_blend_position_y: bool,
    pub loop_blend_position_xz: bool,
    pub keep_original_orientation: bool,
    pub keep_original_position_y: bool,
    pub keep_original_position_xz: bool,
    pub height_from_feet: bool,
    pub mirror: bool,
}

impl Default for AnimClipSettings {
    fn default() -> Self {
        AnimClipSettings {
            start_time: 0.0,
            stop_time: 1.0,
            orientation_offset_y: 0.0,
            level: 0.0,
            cycle_offset: 0.0,
            has_additive_reference_pose: false,
            loop_time: false,
            loop_blend: false,
            loop_blend_orientation: false,
            loop_blend_position_y: false,
            loop_blend_position_xz: false,
            keep_original_orientation: true,
            keep_original_position_y: true,
            keep_original_position_xz: false,
            height_from_feet: false,
            mirror: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimEvent {
    pub time: f64,
    pub function_name: String,
    pub data: String,
    pub float_parameter: f64,
    pub int_parameter: i32,
    pub message_options: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedAnimationClip {
    pub name: String,
    pub legacy: bool,
    pub compressed: bool,
    pub use_high_quality_curve: bool,
    pub sample_rate: f64,
    pub wrap_mode: i32,
    pub position_curves: Vec<AnimVector3Curve>,
    pub rotation_curves: Vec<AnimQuaternionCurve>,
    pub euler_curves: Vec<AnimVector3Curve>,
    pub scale_curves: Vec<AnimVector3Curve>,
    pub float_curves: Vec<AnimFloatCurve>,
    pub pptr_curves: Vec<AnimPPtrCurve>,
    pub settings: AnimClipSettings,
    pub events: Vec<AnimEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimImportError {
    InvalidUtf8,
    Yaml(YamlError),
    MissingAnimationClipDocument,
    InvalidSampleRate(String),
    MalformedCurve(String),
    Read {
        asset: String,
        message: String,
    },
    MissingAnimatorControllerDocument {
        asset: String,
    },
    UnresolvedReference {
        asset: String,
        member: String,
        reference: String,
    },
    UnsupportedConstruct {
        asset: String,
        member: String,
        reason: String,
    },
    MalformedController {
        asset: String,
        member: String,
        reason: String,
    },
}

impl fmt::Display for AnimImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnimImportError::InvalidUtf8 => {
                write!(formatter, "animation clip file is not valid UTF-8")
            }
            AnimImportError::Yaml(error) => {
                write!(formatter, "invalid AnimationClip YAML: {error}")
            }
            AnimImportError::MissingAnimationClipDocument => {
                write!(
                    formatter,
                    "YAML stream does not contain an AnimationClip (class 74) document"
                )
            }
            AnimImportError::InvalidSampleRate(detail) => {
                write!(formatter, "invalid AnimationClip m_SampleRate: {detail}")
            }
            AnimImportError::MalformedCurve(detail) => {
                write!(formatter, "malformed AnimationClip curve: {detail}")
            }
            AnimImportError::Read { asset, message } => {
                write!(
                    formatter,
                    "cannot read animation asset '{asset}': {message}"
                )
            }
            AnimImportError::MissingAnimatorControllerDocument { asset } => {
                write!(
                    formatter,
                    "AnimatorController '{asset}' does not contain a class 91 AnimatorController document"
                )
            }
            AnimImportError::UnresolvedReference {
                asset,
                member,
                reference,
            } => {
                write!(
                    formatter,
                    "AnimatorController '{asset}' member '{member}' references unresolvable asset '{reference}'"
                )
            }
            AnimImportError::UnsupportedConstruct {
                asset,
                member,
                reason,
            } => {
                write!(
                    formatter,
                    "AnimatorController '{asset}' member '{member}' uses unsupported construct: {reason}"
                )
            }
            AnimImportError::MalformedController {
                asset,
                member,
                reason,
            } => {
                write!(
                    formatter,
                    "AnimatorController '{asset}' member '{member}' is malformed: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for AnimImportError {}

impl From<YamlError> for AnimImportError {
    fn from(error: YamlError) -> Self {
        AnimImportError::Yaml(error)
    }
}

fn hermite_interpolate(t0: f64, v0: f64, m0: f64, t1: f64, v1: f64, m1: f64, time: f64) -> f64 {
    let dt = t1 - t0;
    if dt.abs() <= 1e-12 || !dt.is_finite() {
        return v0;
    }
    if !m0.is_finite() || !m1.is_finite() {
        return v0;
    }
    let u = ((time - t0) / dt).clamp(0.0, 1.0);
    let u2 = u * u;
    let u3 = u2 * u;
    let h00 = 2.0 * u3 - 3.0 * u2 + 1.0;
    let h10 = u3 - 2.0 * u2 + u;
    let h01 = -2.0 * u3 + 3.0 * u2;
    let h11 = u3 - u2;
    h00 * v0 + h10 * dt * m0 + h01 * v1 + h11 * dt * m1
}

impl AnimFloatCurve {
    pub fn sample(&self, time: f64) -> f64 {
        let first = match self.keyframes.first() {
            Some(first) => first,
            None => return 0.0,
        };
        if time <= first.time || self.keyframes.len() == 1 {
            return first.value;
        }
        let last = match self.keyframes.last() {
            Some(last) => last,
            None => return first.value,
        };
        if time >= last.time {
            return last.value;
        }
        for pair in self.keyframes.windows(2) {
            let k0 = &pair[0];
            let k1 = &pair[1];
            if time >= k0.time && time <= k1.time {
                return hermite_interpolate(
                    k0.time,
                    k0.value,
                    k0.out_slope,
                    k1.time,
                    k1.value,
                    k1.in_slope,
                    time,
                );
            }
        }
        last.value
    }
}

impl AnimVector3Curve {
    pub fn sample(&self, time: f64) -> [f64; 3] {
        let first = match self.keyframes.first() {
            Some(first) => first,
            None => return [0.0, 0.0, 0.0],
        };
        if time <= first.time || self.keyframes.len() == 1 {
            return first.value;
        }
        let last = match self.keyframes.last() {
            Some(last) => last,
            None => return first.value,
        };
        if time >= last.time {
            return last.value;
        }
        for pair in self.keyframes.windows(2) {
            let k0 = &pair[0];
            let k1 = &pair[1];
            if time >= k0.time && time <= k1.time {
                let mut out = [0.0f64; 3];
                for (axis, slot) in out.iter_mut().enumerate() {
                    *slot = hermite_interpolate(
                        k0.time,
                        k0.value[axis],
                        k0.out_slope[axis],
                        k1.time,
                        k1.value[axis],
                        k1.in_slope[axis],
                        time,
                    );
                }
                return out;
            }
        }
        last.value
    }
}

impl AnimQuaternionCurve {
    pub fn sample(&self, time: f64) -> [f64; 4] {
        let first = match self.keyframes.first() {
            Some(first) => first,
            None => return [0.0, 0.0, 0.0, 1.0],
        };
        if time <= first.time || self.keyframes.len() == 1 {
            return normalize_quat(first.value);
        }
        let last = match self.keyframes.last() {
            Some(last) => last,
            None => return normalize_quat(first.value),
        };
        if time >= last.time {
            return normalize_quat(last.value);
        }
        for pair in self.keyframes.windows(2) {
            let k0 = &pair[0];
            let k1 = &pair[1];
            if time >= k0.time && time <= k1.time {
                let mut out = [0.0f64; 4];
                for (axis, slot) in out.iter_mut().enumerate() {
                    *slot = hermite_interpolate(
                        k0.time,
                        k0.value[axis],
                        k0.out_slope[axis],
                        k1.time,
                        k1.value[axis],
                        k1.in_slope[axis],
                        time,
                    );
                }
                return normalize_quat(out);
            }
        }
        normalize_quat(last.value)
    }
}

fn normalize_quat(q: [f64; 4]) -> [f64; 4] {
    let len_sq = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
    if len_sq > 1e-18 && len_sq.is_finite() {
        let inv = 1.0 / len_sq.sqrt();
        [q[0] * inv, q[1] * inv, q[2] * inv, q[3] * inv]
    } else {
        q
    }
}

pub fn import_animation_clip(
    bytes: &[u8],
    file_name: &str,
) -> Result<ImportedAnimationClip, AnimImportError> {
    let without_bom = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let raw_text = std::str::from_utf8(without_bom).map_err(|_| AnimImportError::InvalidUtf8)?;
    let normalized = normalize_multiline_flow_braces(raw_text);
    let stream = parse(&normalized)?;
    let clip_node = stream
        .documents
        .iter()
        .find_map(|doc| {
            if let Some(node) = doc.root.get("AnimationClip") {
                Some(node)
            } else if doc.class_id == ANIMATION_CLIP_CLASS_ID && doc.root.as_mapping().is_some() {
                Some(&doc.root)
            } else {
                None
            }
        })
        .ok_or(AnimImportError::MissingAnimationClipDocument)?;

    if clip_node.as_mapping().is_none() {
        return Err(AnimImportError::MissingAnimationClipDocument);
    }

    let fallback_name = Path::new(file_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(file_name)
        .to_string();
    let name = clip_node
        .get("m_Name")
        .and_then(YamlNode::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or(fallback_name);

    let legacy = node_bool(clip_node, "m_Legacy", false);
    let compressed = node_bool(clip_node, "m_Compressed", false);
    let use_high_quality_curve = node_bool(clip_node, "m_UseHighQualityCurve", true);
    let sample_rate = match clip_node.get("m_SampleRate") {
        Some(node) => {
            let raw = node.as_str().unwrap_or_default().trim();
            let parsed = raw
                .parse::<f64>()
                .map_err(|_| AnimImportError::InvalidSampleRate(raw.to_string()))?;
            if !parsed.is_finite() || parsed <= 0.0 {
                return Err(AnimImportError::InvalidSampleRate(raw.to_string()));
            }
            parsed
        }
        None => 60.0,
    };
    let wrap_mode = node_i32(clip_node, "m_WrapMode", 0);

    let position_curves = parse_vec3_curves(clip_node.get("m_PositionCurves"))?;
    let rotation_curves = parse_quat_curves(clip_node.get("m_RotationCurves"))?;
    let euler_curves = parse_vec3_curves(clip_node.get("m_EulerCurves"))?;
    let scale_curves = parse_vec3_curves(clip_node.get("m_ScaleCurves"))?;
    let float_curves = parse_float_curves(clip_node.get("m_FloatCurves"))?;
    let pptr_curves = parse_pptr_curves(clip_node.get("m_PPtrCurves"))?;
    let settings = parse_clip_settings(clip_node.get("m_AnimationClipSettings"));
    let events = parse_events(clip_node.get("m_Events"))?;

    Ok(ImportedAnimationClip {
        name,
        legacy,
        compressed,
        use_high_quality_curve,
        sample_rate,
        wrap_mode,
        position_curves,
        rotation_curves,
        euler_curves,
        scale_curves,
        float_curves,
        pptr_curves,
        settings,
        events,
    })
}

pub fn anim_import_outcome(
    asset_label: &str,
    result: Result<&ImportedAnimationClip, &AnimImportError>,
) -> ImportOutcome {
    match result {
        Ok(_) => ImportOutcome::ready(),
        Err(error) => ImportOutcome::failed(ImportDiagnostic::new(
            asset_label,
            error.to_string(),
            "the AnimationClip cannot be imported; the asset stays failed and its curves are unavailable",
        )),
    }
}

fn normalize_multiline_flow_braces(text: &str) -> String {
    let raw_lines: Vec<&str> = text.lines().collect();
    let mut joined_lines: Vec<String> = Vec::with_capacity(raw_lines.len());
    let mut open_braces = 0i32;
    for &line in &raw_lines {
        let delta = brace_delta(line);
        if open_braces > 0 {
            if let Some(last) = joined_lines.last_mut() {
                last.push(' ');
                last.push_str(line.trim());
            }
        } else {
            joined_lines.push(line.to_string());
        }
        open_braces = (open_braces + delta).max(0);
    }

    let mut out_lines: Vec<String> = Vec::with_capacity(joined_lines.len());
    let mut key_map: Vec<(usize, usize)> = Vec::new();
    let mut seq_map: Vec<(usize, usize)> = Vec::new();
    let mut last_new_key_indent = 0usize;

    for line in &joined_lines {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with("---") || trimmed.starts_with('%') {
            key_map.clear();
            seq_map.clear();
            last_new_key_indent = 0;
            out_lines.push(line.clone());
            continue;
        }
        let orig_indent = line.len() - trimmed.len();
        let is_seq_item = trimmed == "-" || trimmed.starts_with("- ");
        if is_seq_item {
            key_map.retain(|&(orig, _)| orig <= orig_indent);
            seq_map.retain(|&(orig, _)| orig <= orig_indent);
            let new_dash = match seq_map.iter().rev().find(|&&(orig, _)| orig == orig_indent) {
                Some(&(_, mapped)) => mapped,
                None => {
                    let mapped = last_new_key_indent + 1;
                    seq_map.push((orig_indent, mapped));
                    mapped
                }
            };
            let mut dash_line = String::with_capacity(new_dash + 1);
            for _ in 0..new_dash {
                dash_line.push(' ');
            }
            dash_line.push('-');
            out_lines.push(dash_line);

            let after_dash = trimmed[1..].trim_start();
            if !after_dash.is_empty() {
                let orig_content = orig_indent + (trimmed.len() - after_dash.len());
                let new_content = new_dash + 1;
                let mut content_line = String::with_capacity(new_content + after_dash.len());
                for _ in 0..new_content {
                    content_line.push(' ');
                }
                content_line.push_str(after_dash);
                out_lines.push(content_line);
                key_map.retain(|&(orig, _)| orig < orig_content);
                key_map.push((orig_content, new_content));
                if after_dash.ends_with(':') {
                    last_new_key_indent = new_content;
                }
            }
        } else {
            key_map.retain(|&(orig, _)| orig <= orig_indent);
            let new_key = match key_map.iter().rev().find(|&&(orig, _)| orig == orig_indent) {
                Some(&(_, mapped)) => mapped,
                None => {
                    let mapped = match seq_map.iter().rev().find(|&&(orig, _)| orig == orig_indent)
                    {
                        Some(&(_, seq_mapped)) => seq_mapped,
                        None => {
                            if orig_indent == 0 {
                                0
                            } else {
                                last_new_key_indent + 2
                            }
                        }
                    };
                    key_map.push((orig_indent, mapped));
                    mapped
                }
            };
            seq_map.retain(|&(_, mapped)| mapped < new_key);
            let mut new_line = String::with_capacity(new_key + trimmed.len());
            for _ in 0..new_key {
                new_line.push(' ');
            }
            new_line.push_str(trimmed);
            out_lines.push(new_line);
            if trimmed.ends_with(':') {
                last_new_key_indent = new_key;
            }
        }
    }

    let mut out = out_lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn brace_delta(line: &str) -> i32 {
    let mut delta = 0i32;
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match quote {
            Some('"') => {
                if ch == '\\' {
                    let _ = chars.next();
                    continue;
                }
                if ch == '"' {
                    quote = None;
                }
            }
            Some('\'') => {
                if ch == '\'' {
                    if chars.peek() == Some(&'\'') {
                        let _ = chars.next();
                    } else {
                        quote = None;
                    }
                }
            }
            _ => match ch {
                '"' | '\'' => quote = Some(ch),
                '{' => delta += 1,
                '}' => delta -= 1,
                _ => {}
            },
        }
    }
    delta
}

fn node_bool(parent: &YamlNode, key: &str, default: bool) -> bool {
    match parent.get(key).and_then(YamlNode::as_str).map(str::trim) {
        Some("1" | "true" | "True") => true,
        Some("0" | "false" | "False") => false,
        _ => default,
    }
}

fn node_i32(parent: &YamlNode, key: &str, default: i32) -> i32 {
    parent
        .get(key)
        .and_then(YamlNode::as_i64)
        .and_then(|v| i32::try_from(v).ok())
        .unwrap_or(default)
}

fn node_f64(parent: &YamlNode, key: &str, default: f64) -> f64 {
    parent
        .get(key)
        .and_then(YamlNode::as_f64)
        .unwrap_or(default)
}

fn parse_vec3_node(
    node: Option<&YamlNode>,
    default: [f64; 3],
) -> Result<[f64; 3], AnimImportError> {
    let Some(node) = node else {
        return Ok(default);
    };
    if let Some(scalar) = node.as_str() {
        let trimmed = scalar.trim();
        if trimmed.is_empty() {
            return Ok(default);
        }
        let parsed = trimmed.parse::<f64>().map_err(|_| {
            AnimImportError::MalformedCurve(format!("invalid vec3 scalar '{trimmed}'"))
        })?;
        return Ok([parsed, parsed, parsed]);
    }
    if node.as_mapping().is_none() {
        return Err(AnimImportError::MalformedCurve(
            "expected mapping for Vector3 value".to_string(),
        ));
    }
    let x = required_f64(node, "x", default[0])?;
    let y = required_f64(node, "y", default[1])?;
    let z = required_f64(node, "z", default[2])?;
    Ok([x, y, z])
}

fn parse_quat_node(
    node: Option<&YamlNode>,
    default: [f64; 4],
) -> Result<[f64; 4], AnimImportError> {
    let Some(node) = node else {
        return Ok(default);
    };
    if let Some(scalar) = node.as_str() {
        let trimmed = scalar.trim();
        if trimmed.is_empty() {
            return Ok(default);
        }
        let parsed = trimmed.parse::<f64>().map_err(|_| {
            AnimImportError::MalformedCurve(format!("invalid quat scalar '{trimmed}'"))
        })?;
        return Ok([parsed, parsed, parsed, parsed]);
    }
    if node.as_mapping().is_none() {
        return Err(AnimImportError::MalformedCurve(
            "expected mapping for Quaternion value".to_string(),
        ));
    }
    let x = required_f64(node, "x", default[0])?;
    let y = required_f64(node, "y", default[1])?;
    let z = required_f64(node, "z", default[2])?;
    let w = required_f64(node, "w", default[3])?;
    Ok([x, y, z, w])
}

fn required_f64(parent: &YamlNode, key: &str, default: f64) -> Result<f64, AnimImportError> {
    let Some(child) = parent.get(key) else {
        return Ok(default);
    };
    let raw = child
        .as_str()
        .ok_or_else(|| AnimImportError::MalformedCurve(format!("field '{key}' is not a scalar")))?
        .trim();
    if raw.eq_ignore_ascii_case("Infinity")
        || raw.eq_ignore_ascii_case("+Infinity")
        || raw.eq_ignore_ascii_case("inf")
    {
        return Ok(f64::INFINITY);
    }
    if raw.eq_ignore_ascii_case("-Infinity") || raw.eq_ignore_ascii_case("-inf") {
        return Ok(f64::NEG_INFINITY);
    }
    raw.parse::<f64>()
        .map_err(|_| AnimImportError::MalformedCurve(format!("invalid float '{raw}' for '{key}'")))
}

fn parse_vec3_curves(node: Option<&YamlNode>) -> Result<Vec<AnimVector3Curve>, AnimImportError> {
    let Some(items) = node_sequence(node)? else {
        return Ok(Vec::new());
    };
    let mut curves = Vec::with_capacity(items.len());
    for entry in items {
        if entry.as_mapping().is_none() {
            return Err(AnimImportError::MalformedCurve(
                "Vector3 curve entry is not a mapping".to_string(),
            ));
        }
        let path = entry
            .get("path")
            .and_then(YamlNode::as_str)
            .unwrap_or_default()
            .to_string();
        let curve_node = entry.get("curve").unwrap_or(entry);
        let pre_infinity = node_i32(curve_node, "m_PreInfinity", 2);
        let post_infinity = node_i32(curve_node, "m_PostInfinity", 2);
        let rotation_order = node_i32(curve_node, "m_RotationOrder", 4);
        let mut keyframes = Vec::new();
        if let Some(kf_items) = node_sequence(curve_node.get("m_Curve"))? {
            for kf in kf_items {
                if kf.as_mapping().is_none() {
                    return Err(AnimImportError::MalformedCurve(
                        "Vector3 keyframe is not a mapping".to_string(),
                    ));
                }
                let time = required_f64(kf, "time", 0.0)?;
                let value = parse_vec3_node(kf.get("value"), [0.0, 0.0, 0.0])?;
                let in_slope = parse_vec3_node(kf.get("inSlope"), [0.0, 0.0, 0.0])?;
                let out_slope = parse_vec3_node(kf.get("outSlope"), [0.0, 0.0, 0.0])?;
                let weighted_mode = node_i32(kf, "weightedMode", 0);
                let in_weight =
                    parse_vec3_node(kf.get("inWeight"), [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0])?;
                let out_weight =
                    parse_vec3_node(kf.get("outWeight"), [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0])?;
                keyframes.push(AnimKeyframeVec3 {
                    time,
                    value,
                    in_slope,
                    out_slope,
                    weighted_mode,
                    in_weight,
                    out_weight,
                });
            }
        }
        curves.push(AnimVector3Curve {
            path,
            keyframes,
            pre_infinity,
            post_infinity,
            rotation_order,
        });
    }
    Ok(curves)
}

fn parse_quat_curves(node: Option<&YamlNode>) -> Result<Vec<AnimQuaternionCurve>, AnimImportError> {
    let Some(items) = node_sequence(node)? else {
        return Ok(Vec::new());
    };
    let mut curves = Vec::with_capacity(items.len());
    for entry in items {
        if entry.as_mapping().is_none() {
            return Err(AnimImportError::MalformedCurve(
                "Quaternion curve entry is not a mapping".to_string(),
            ));
        }
        let path = entry
            .get("path")
            .and_then(YamlNode::as_str)
            .unwrap_or_default()
            .to_string();
        let curve_node = entry.get("curve").unwrap_or(entry);
        let pre_infinity = node_i32(curve_node, "m_PreInfinity", 2);
        let post_infinity = node_i32(curve_node, "m_PostInfinity", 2);
        let rotation_order = node_i32(curve_node, "m_RotationOrder", 4);
        let mut keyframes = Vec::new();
        if let Some(kf_items) = node_sequence(curve_node.get("m_Curve"))? {
            for kf in kf_items {
                if kf.as_mapping().is_none() {
                    return Err(AnimImportError::MalformedCurve(
                        "Quaternion keyframe is not a mapping".to_string(),
                    ));
                }
                let time = required_f64(kf, "time", 0.0)?;
                let value = parse_quat_node(kf.get("value"), [0.0, 0.0, 0.0, 1.0])?;
                let in_slope = parse_quat_node(kf.get("inSlope"), [0.0, 0.0, 0.0, 0.0])?;
                let out_slope = parse_quat_node(kf.get("outSlope"), [0.0, 0.0, 0.0, 0.0])?;
                let weighted_mode = node_i32(kf, "weightedMode", 0);
                let in_weight = parse_quat_node(
                    kf.get("inWeight"),
                    [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0],
                )?;
                let out_weight = parse_quat_node(
                    kf.get("outWeight"),
                    [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0],
                )?;
                keyframes.push(AnimKeyframeQuat {
                    time,
                    value,
                    in_slope,
                    out_slope,
                    weighted_mode,
                    in_weight,
                    out_weight,
                });
            }
        }
        curves.push(AnimQuaternionCurve {
            path,
            keyframes,
            pre_infinity,
            post_infinity,
            rotation_order,
        });
    }
    Ok(curves)
}

fn parse_float_curves(node: Option<&YamlNode>) -> Result<Vec<AnimFloatCurve>, AnimImportError> {
    let Some(items) = node_sequence(node)? else {
        return Ok(Vec::new());
    };
    let mut curves = Vec::with_capacity(items.len());
    for entry in items {
        if entry.as_mapping().is_none() {
            return Err(AnimImportError::MalformedCurve(
                "Float curve entry is not a mapping".to_string(),
            ));
        }
        let path = entry
            .get("path")
            .and_then(YamlNode::as_str)
            .unwrap_or_default()
            .to_string();
        let attribute = entry
            .get("attribute")
            .and_then(YamlNode::as_str)
            .unwrap_or_default()
            .to_string();
        let class_id = node_i32(entry, "classID", 0);
        let script_guid = entry
            .get("script")
            .and_then(|script| script.get("guid"))
            .and_then(YamlNode::as_str)
            .map(str::trim)
            .filter(|guid| !guid.is_empty())
            .map(str::to_string);
        let curve_node = entry.get("curve").unwrap_or(entry);
        let pre_infinity = node_i32(curve_node, "m_PreInfinity", 2);
        let post_infinity = node_i32(curve_node, "m_PostInfinity", 2);
        let rotation_order = node_i32(curve_node, "m_RotationOrder", 4);
        let mut keyframes = Vec::new();
        if let Some(kf_items) = node_sequence(curve_node.get("m_Curve"))? {
            for kf in kf_items {
                if kf.as_mapping().is_none() {
                    return Err(AnimImportError::MalformedCurve(
                        "Float keyframe is not a mapping".to_string(),
                    ));
                }
                let time = required_f64(kf, "time", 0.0)?;
                let value = required_f64(kf, "value", 0.0)?;
                let in_slope = required_f64(kf, "inSlope", 0.0)?;
                let out_slope = required_f64(kf, "outSlope", 0.0)?;
                let weighted_mode = node_i32(kf, "weightedMode", 0);
                let in_weight = required_f64(kf, "inWeight", 1.0 / 3.0)?;
                let out_weight = required_f64(kf, "outWeight", 1.0 / 3.0)?;
                keyframes.push(AnimKeyframeFloat {
                    time,
                    value,
                    in_slope,
                    out_slope,
                    weighted_mode,
                    in_weight,
                    out_weight,
                });
            }
        }
        curves.push(AnimFloatCurve {
            path,
            attribute,
            class_id,
            script_guid,
            keyframes,
            pre_infinity,
            post_infinity,
            rotation_order,
        });
    }
    Ok(curves)
}

fn parse_pptr_curves(node: Option<&YamlNode>) -> Result<Vec<AnimPPtrCurve>, AnimImportError> {
    let Some(items) = node_sequence(node)? else {
        return Ok(Vec::new());
    };
    let mut curves = Vec::with_capacity(items.len());
    for entry in items {
        if entry.as_mapping().is_none() {
            return Err(AnimImportError::MalformedCurve(
                "PPtr curve entry is not a mapping".to_string(),
            ));
        }
        let path = entry
            .get("path")
            .and_then(YamlNode::as_str)
            .unwrap_or_default()
            .to_string();
        let attribute = entry
            .get("attribute")
            .and_then(YamlNode::as_str)
            .unwrap_or_default()
            .to_string();
        let class_id = node_i32(entry, "classID", 0);
        let script_guid = entry
            .get("script")
            .and_then(|script| script.get("guid"))
            .and_then(YamlNode::as_str)
            .map(str::trim)
            .filter(|guid| !guid.is_empty())
            .map(str::to_string);
        let mut keyframes = Vec::new();
        if let Some(kf_items) = node_sequence(entry.get("curve"))? {
            for kf in kf_items {
                if kf.as_mapping().is_none() {
                    return Err(AnimImportError::MalformedCurve(
                        "PPtr keyframe is not a mapping".to_string(),
                    ));
                }
                let time = required_f64(kf, "time", 0.0)?;
                let value_node = kf.get("value").unwrap_or(kf);
                let file_id = value_node
                    .get("fileID")
                    .and_then(YamlNode::as_i64)
                    .unwrap_or(0);
                let guid = value_node
                    .get("guid")
                    .and_then(YamlNode::as_str)
                    .map(str::trim)
                    .filter(|g| !g.is_empty())
                    .map(str::to_string);
                let reference_type = value_node.get("type").and_then(YamlNode::as_i64);
                keyframes.push(AnimPPtrKeyframe {
                    time,
                    file_id,
                    guid,
                    reference_type,
                });
            }
        }
        curves.push(AnimPPtrCurve {
            path,
            attribute,
            class_id,
            script_guid,
            keyframes,
        });
    }
    Ok(curves)
}

fn parse_clip_settings(node: Option<&YamlNode>) -> AnimClipSettings {
    let default = AnimClipSettings::default();
    let Some(node) = node else {
        return default;
    };
    AnimClipSettings {
        start_time: node_f64(node, "m_StartTime", default.start_time),
        stop_time: node_f64(node, "m_StopTime", default.stop_time),
        orientation_offset_y: node_f64(node, "m_OrientationOffsetY", default.orientation_offset_y),
        level: node_f64(node, "m_Level", default.level),
        cycle_offset: node_f64(node, "m_CycleOffset", default.cycle_offset),
        has_additive_reference_pose: node_bool(
            node,
            "m_HasAdditiveReferencePose",
            default.has_additive_reference_pose,
        ),
        loop_time: node_bool(node, "m_LoopTime", default.loop_time),
        loop_blend: node_bool(node, "m_LoopBlend", default.loop_blend),
        loop_blend_orientation: node_bool(
            node,
            "m_LoopBlendOrientation",
            default.loop_blend_orientation,
        ),
        loop_blend_position_y: node_bool(
            node,
            "m_LoopBlendPositionY",
            default.loop_blend_position_y,
        ),
        loop_blend_position_xz: node_bool(
            node,
            "m_LoopBlendPositionXZ",
            default.loop_blend_position_xz,
        ),
        keep_original_orientation: node_bool(
            node,
            "m_KeepOriginalOrientation",
            default.keep_original_orientation,
        ),
        keep_original_position_y: node_bool(
            node,
            "m_KeepOriginalPositionY",
            default.keep_original_position_y,
        ),
        keep_original_position_xz: node_bool(
            node,
            "m_KeepOriginalPositionXZ",
            default.keep_original_position_xz,
        ),
        height_from_feet: node_bool(node, "m_HeightFromFeet", default.height_from_feet),
        mirror: node_bool(node, "m_Mirror", default.mirror),
    }
}

fn parse_events(node: Option<&YamlNode>) -> Result<Vec<AnimEvent>, AnimImportError> {
    let Some(items) = node_sequence(node)? else {
        return Ok(Vec::new());
    };
    let mut events = Vec::with_capacity(items.len());
    for entry in items {
        if entry.as_mapping().is_none() {
            return Err(AnimImportError::MalformedCurve(
                "AnimationEvent entry is not a mapping".to_string(),
            ));
        }
        let time = required_f64(entry, "time", 0.0)?;
        let function_name = entry
            .get("functionName")
            .and_then(YamlNode::as_str)
            .unwrap_or_default()
            .to_string();
        let data = entry
            .get("data")
            .and_then(YamlNode::as_str)
            .unwrap_or_default()
            .to_string();
        let float_parameter = required_f64(entry, "floatParameter", 0.0)?;
        let int_parameter = node_i32(entry, "intParameter", 0);
        let message_options = node_i32(entry, "messageOptions", 0);
        events.push(AnimEvent {
            time,
            function_name,
            data,
            float_parameter,
            int_parameter,
            message_options,
        });
    }
    Ok(events)
}

fn node_sequence(node: Option<&YamlNode>) -> Result<Option<&[YamlNode]>, AnimImportError> {
    let Some(node) = node else {
        return Ok(None);
    };
    if let Some(items) = node.as_sequence() {
        return Ok(Some(items));
    }
    if let Some(scalar) = node.as_str() {
        if scalar.trim().is_empty() {
            return Ok(None);
        }
    }
    Err(AnimImportError::MalformedCurve(
        "expected sequence in AnimationClip curve list".to_string(),
    ))
}
