use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::BuildMode;
use crate::licensing::{LicensingRecord, ThirdPartyNotice};
use crate::targets::{BuildTarget, TargetArch, TargetOs};
use crate::toolchain::{TargetAvailability, ToolchainCause, ToolchainProbe, UnavailableToolchain};

use super::{
    parse_state, parse_tier, state_name, tier_name, BuildDiagnostic, CompatibilityFinding,
    DesktopPackageOutcome, ReportParseError, SizeCategoryEntry, SizeContributor,
    StartupMeasurement, StartupStatus, DESKTOP_PACKAGE_STATE_EXTERNALLY_UNAVAILABLE,
    DESKTOP_PACKAGE_STATE_PACKAGED, STARTUP_BASIS,
};

pub(super) fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push(' ');
    }
}

pub(super) fn push_escaped_json_str(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            control if (control as u32) < 0x20 => {
                let code = control as u32;
                out.push_str(&format!("\\u{code:04x}"));
            }
            normal => out.push(normal),
        }
    }
    out.push('"');
}

pub(super) fn push_trailing(out: &mut String, trailing_comma: bool) {
    if trailing_comma {
        out.push_str(",\n");
    } else {
        out.push('\n');
    }
}

pub(super) fn push_kv_str(
    out: &mut String,
    indent: usize,
    key: &str,
    value: &str,
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    out.push_str(": ");
    push_escaped_json_str(out, value);
    push_trailing(out, trailing_comma);
}

pub(super) fn push_kv_opt_str(
    out: &mut String,
    indent: usize,
    key: &str,
    value: Option<&str>,
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    out.push_str(": ");
    match value {
        Some(text) => push_escaped_json_str(out, text),
        None => out.push_str("null"),
    }
    push_trailing(out, trailing_comma);
}

pub(super) fn push_kv_opt_path(
    out: &mut String,
    indent: usize,
    key: &str,
    value: Option<&Path>,
    trailing_comma: bool,
) {
    let text = value.map(|path| path.to_string_lossy().into_owned());
    push_kv_opt_str(out, indent, key, text.as_deref(), trailing_comma);
}

pub(super) fn push_kv_u64(
    out: &mut String,
    indent: usize,
    key: &str,
    value: u64,
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    out.push_str(": ");
    out.push_str(&value.to_string());
    push_trailing(out, trailing_comma);
}

pub(super) fn push_kv_bool(
    out: &mut String,
    indent: usize,
    key: &str,
    value: bool,
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    out.push_str(": ");
    out.push_str(if value { "true" } else { "false" });
    push_trailing(out, trailing_comma);
}

pub(super) fn push_kv_string_array(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[String],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_indent(out, indent + 2);
        push_escaped_json_str(out, item);
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

pub(super) fn push_kv_diagnostics(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[BuildDiagnostic],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_indent(out, indent + 2);
        out.push_str("{\n");
        push_kv_str(out, indent + 4, "code", &item.code, true);
        push_kv_str(out, indent + 4, "message", &item.message, true);
        push_kv_opt_str(out, indent + 4, "source", item.source.as_deref(), true);
        push_kv_opt_str(out, indent + 4, "remedy", item.remedy.as_deref(), false);
        push_indent(out, indent + 2);
        out.push('}');
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

pub(super) fn push_kv_compatibility_findings(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[CompatibilityFinding],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_compatibility_finding(out, indent, item);
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

fn push_compatibility_finding(out: &mut String, indent: usize, item: &CompatibilityFinding) {
    push_indent(out, indent + 2);
    out.push_str("{\n");
    push_compatibility_finding_identity(out, indent, item);
    push_compatibility_finding_details(out, indent, item);
    push_indent(out, indent + 2);
    out.push('}');
}

fn push_compatibility_finding_identity(
    out: &mut String,
    indent: usize,
    item: &CompatibilityFinding,
) {
    push_kv_str(out, indent + 4, "feature_id", &item.feature_id, true);
    push_kv_str(out, indent + 4, "tier", tier_name(item.tier), true);
    push_kv_str(out, indent + 4, "state", state_name(item.state), true);
    push_kv_opt_str(out, indent + 4, "guid", item.guid.as_deref(), true);
    push_kv_opt_str(
        out,
        indent + 4,
        "asset_path",
        item.asset_path.as_deref(),
        true,
    );
    push_kv_opt_str(
        out,
        indent + 4,
        "sub_asset_id",
        item.sub_asset_id.as_deref(),
        true,
    );
    push_kv_opt_str(
        out,
        indent + 4,
        "object_name",
        item.object_name.as_deref(),
        true,
    );
}

fn push_compatibility_finding_details(
    out: &mut String,
    indent: usize,
    item: &CompatibilityFinding,
) {
    push_kv_str(out, indent + 4, "behavior", &item.behavior, true);
    push_kv_string_array(
        out,
        indent + 4,
        "relied_upon_unity_differences",
        &item.relied_upon_unity_differences,
        true,
    );
    push_kv_str(
        out,
        indent + 4,
        "observable_consequence",
        &item.observable_consequence,
        true,
    );
    push_kv_string_array(out, indent + 4, "conditions", &item.conditions, true);
    push_kv_string_array(
        out,
        indent + 4,
        "related_findings",
        &item.related_findings,
        false,
    );
}

pub(super) fn push_kv_category_sizes(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[SizeCategoryEntry],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_indent(out, indent + 2);
        out.push_str("{\n");
        push_kv_str(out, indent + 4, "category", &item.category, true);
        push_kv_u64(out, indent + 4, "size_bytes", item.size_bytes, false);
        push_indent(out, indent + 2);
        out.push('}');
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

pub(super) fn push_kv_contributors(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[SizeContributor],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_indent(out, indent + 2);
        out.push_str("{\n");
        push_kv_str(out, indent + 4, "path", &item.path, true);
        push_kv_str(out, indent + 4, "category", &item.category, true);
        push_kv_u64(out, indent + 4, "size_bytes", item.size_bytes, true);
        push_kv_str(out, indent + 4, "content_hash", &item.content_hash, false);
        push_indent(out, indent + 2);
        out.push('}');
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

fn push_kv_opt_u64(
    out: &mut String,
    indent: usize,
    key: &str,
    value: Option<u64>,
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    out.push_str(": ");
    match value {
        Some(number) => out.push_str(&number.to_string()),
        None => out.push_str("null"),
    }
    push_trailing(out, trailing_comma);
}

pub(super) fn push_kv_startup_time(
    out: &mut String,
    indent: usize,
    key: &str,
    value: &StartupMeasurement,
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    out.push_str(": {\n");
    push_kv_str(out, indent + 2, "status", value.status.as_str(), true);
    push_kv_opt_u64(out, indent + 2, "duration_ms", value.duration_ms, true);
    push_kv_str(out, indent + 2, "basis", &value.basis, true);
    push_kv_opt_str(out, indent + 2, "reason", value.reason.as_deref(), false);
    push_indent(out, indent);
    out.push('}');
    push_trailing(out, trailing_comma);
}

pub(super) fn push_kv_target_availability(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[TargetAvailability],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_target_availability(out, indent, item);
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

fn push_target_availability(out: &mut String, indent: usize, item: &TargetAvailability) {
    push_indent(out, indent + 2);
    out.push_str("{\n");
    push_kv_str(out, indent + 4, "target", &item.target.identifier(), true);
    push_kv_str(out, indent + 4, "state", item.state(), true);
    push_unavailable_toolchain(out, indent + 4, item.unavailable.as_ref(), true);
    push_kv_tool_probes(out, indent + 4, "tools", &item.tools, false);
    push_indent(out, indent + 2);
    out.push('}');
}

pub(super) fn push_kv_desktop_packaging(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[DesktopPackageOutcome],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_desktop_packaging(out, indent, item);
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

fn push_desktop_packaging(out: &mut String, indent: usize, item: &DesktopPackageOutcome) {
    push_indent(out, indent + 2);
    out.push_str("{\n");
    push_kv_str(out, indent + 4, "target", &item.target.identifier(), true);
    push_kv_str(out, indent + 4, "state", &item.state, true);
    push_kv_opt_path(out, indent + 4, "app_path", item.app_path.as_deref(), true);
    push_kv_opt_path(
        out,
        indent + 4,
        "launch_artifact",
        item.launch_artifact.as_deref(),
        true,
    );
    push_kv_str(
        out,
        indent + 4,
        "build_report_path",
        &item.build_report_path.to_string_lossy(),
        true,
    );
    push_kv_bool(out, indent + 4, "launched", item.launched, true);
    push_unavailable_toolchain(out, indent + 4, item.unavailable.as_ref(), false);
    push_indent(out, indent + 2);
    out.push('}');
}

fn push_unavailable_toolchain(
    out: &mut String,
    indent: usize,
    unavailable: Option<&UnavailableToolchain>,
    trailing_comma: bool,
) {
    match unavailable {
        None => {
            push_kv_opt_str(out, indent, "cause", None, true);
            push_kv_opt_str(out, indent, "tool", None, true);
            push_kv_opt_str(out, indent, "version_found", None, true);
            push_kv_opt_str(out, indent, "required_range", None, true);
            push_kv_opt_str(out, indent, "resolving_action", None, trailing_comma);
        }
        Some(unavailable) => {
            push_kv_str(out, indent, "cause", unavailable.cause.as_str(), true);
            push_kv_str(out, indent, "tool", &unavailable.tool, true);
            push_kv_opt_str(
                out,
                indent,
                "version_found",
                unavailable.version_found.as_deref(),
                true,
            );
            push_kv_str(
                out,
                indent,
                "required_range",
                &unavailable.required_range,
                true,
            );
            push_kv_str(
                out,
                indent,
                "resolving_action",
                &unavailable.resolving_action,
                trailing_comma,
            );
        }
    }
}

fn push_kv_tool_probes(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[ToolchainProbe],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_indent(out, indent + 2);
        out.push_str("{\n");
        push_kv_str(out, indent + 4, "tool", &item.tool, true);
        push_kv_str(
            out,
            indent + 4,
            "required_range",
            &item.required_range,
            true,
        );
        push_kv_str(out, indent + 4, "purpose", &item.purpose, true);
        push_kv_bool(out, indent + 4, "present", item.present, true);
        push_kv_opt_str(
            out,
            indent + 4,
            "version_found",
            item.version_found.as_deref(),
            false,
        );
        push_indent(out, indent + 2);
        out.push('}');
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

pub(super) fn push_kv_licensing(
    out: &mut String,
    indent: usize,
    key: &str,
    licensing: Option<&LicensingRecord>,
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    let Some(record) = licensing else {
        out.push_str(": null");
        push_trailing(out, trailing_comma);
        return;
    };
    out.push_str(": {\n");
    push_licensing_identity(out, indent, record);
    push_licensing_statements(out, indent, record);
    push_indent(out, indent);
    out.push('}');
    push_trailing(out, trailing_comma);
}

fn push_licensing_identity(out: &mut String, indent: usize, record: &LicensingRecord) {
    push_kv_str(
        out,
        indent + 2,
        "application_identifier",
        &record.application_identifier,
        true,
    );
    push_kv_str(
        out,
        indent + 2,
        "governing_license",
        &record.governing_license,
        true,
    );
    push_kv_str(
        out,
        indent + 2,
        "governing_license_name",
        &record.governing_license_name,
        true,
    );
    push_kv_str(
        out,
        indent + 2,
        "conveyance_statement",
        &record.conveyance_statement,
        true,
    );
    push_kv_str(
        out,
        indent + 2,
        "corresponding_source_obligation",
        &record.corresponding_source_obligation,
        true,
    );
}

fn push_licensing_statements(out: &mut String, indent: usize, record: &LicensingRecord) {
    push_kv_str(
        out,
        indent + 2,
        "corresponding_source_availability",
        &record.corresponding_source_availability,
        true,
    );
    push_kv_str(
        out,
        indent + 2,
        "linking_exception_statement",
        &record.linking_exception_statement,
        true,
    );
    push_kv_str(
        out,
        indent + 2,
        "commercial_license_statement",
        &record.commercial_license_statement,
        true,
    );
    push_kv_str(
        out,
        indent + 2,
        "not_legal_advice_statement",
        &record.not_legal_advice_statement,
        true,
    );
    push_kv_string_array(
        out,
        indent + 2,
        "license_texts",
        &record.license_texts,
        true,
    );
    push_kv_notices(
        out,
        indent + 2,
        "third_party_notices",
        &record.third_party_notices,
        false,
    );
}

fn push_kv_notices(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[ThirdPartyNotice],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_indent(out, indent + 2);
        out.push_str("{\n");
        push_kv_str(out, indent + 4, "ecosystem", &item.ecosystem, true);
        push_kv_str(out, indent + 4, "component", &item.component, true);
        push_kv_str(out, indent + 4, "version", &item.version, true);
        push_kv_str(out, indent + 4, "license", &item.license, false);
        push_indent(out, indent + 2);
        out.push('}');
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JsonValue {
    Null,
    Bool(bool),
    Number(u64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    pub(crate) fn as_object(
        &self,
        context: &str,
    ) -> Result<&BTreeMap<String, JsonValue>, ReportParseError> {
        match self {
            JsonValue::Object(map) => Ok(map),
            _ => Err(ReportParseError::new(format!(
                "expected JSON object for {context}"
            ))),
        }
    }

    pub(crate) fn as_array(&self, context: &str) -> Result<&[JsonValue], ReportParseError> {
        match self {
            JsonValue::Array(items) => Ok(items.as_slice()),
            _ => Err(ReportParseError::new(format!(
                "expected JSON array for {context}"
            ))),
        }
    }

    pub(crate) fn as_str(&self, context: &str) -> Result<&str, ReportParseError> {
        match self {
            JsonValue::String(text) => Ok(text.as_str()),
            _ => Err(ReportParseError::new(format!(
                "expected JSON string for {context}"
            ))),
        }
    }

    fn as_u64(&self, context: &str) -> Result<u64, ReportParseError> {
        match self {
            JsonValue::Number(num) => Ok(*num),
            _ => Err(ReportParseError::new(format!(
                "expected JSON unsigned integer for {context}"
            ))),
        }
    }

    fn as_bool(&self, context: &str) -> Result<bool, ReportParseError> {
        match self {
            JsonValue::Bool(flag) => Ok(*flag),
            _ => Err(ReportParseError::new(format!(
                "expected JSON boolean for {context}"
            ))),
        }
    }
}

pub(super) struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

/// Parses a JSON document into the report's JSON value model.
pub(crate) fn parse_json_document(input: &str) -> Result<JsonValue, ReportParseError> {
    JsonParser::parse(input)
}

impl<'a> JsonParser<'a> {
    pub(super) fn parse(input: &'a str) -> Result<JsonValue, ReportParseError> {
        let mut parser = Self {
            bytes: input.as_bytes(),
            pos: 0,
        };
        let value = parser.parse_value()?;
        parser.skip_whitespace();
        if parser.pos != parser.bytes.len() {
            return Err(ReportParseError::new(format!(
                "unexpected trailing JSON content at byte {}",
                parser.pos
            )));
        }
        Ok(value)
    }

    fn skip_whitespace(&mut self) {
        while let Some(&byte) = self.bytes.get(self.pos) {
            if matches!(byte, b' ' | b'\n' | b'\r' | b'\t') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn parse_value(&mut self) -> Result<JsonValue, ReportParseError> {
        self.skip_whitespace();
        let byte = self.peek().ok_or_else(|| {
            ReportParseError::new(format!("unexpected end of JSON at byte {}", self.pos))
        })?;
        match byte {
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b'"' => self.parse_string().map(JsonValue::String),
            b't' | b'f' => self.parse_bool().map(JsonValue::Bool),
            b'n' => self.parse_null(),
            b'0'..=b'9' => self.parse_number().map(JsonValue::Number),
            other => Err(ReportParseError::new(format!(
                "unexpected JSON token '{}' at byte {}",
                char::from(other),
                self.pos
            ))),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, ReportParseError> {
        self.pos += 1;
        let mut map = BTreeMap::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsonValue::Object(map));
        }

        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(ReportParseError::new(format!(
                    "expected object key string at byte {}",
                    self.pos
                )));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.peek() != Some(b':') {
                return Err(ReportParseError::new(format!(
                    "expected ':' after key '{key}' at byte {}",
                    self.pos
                )));
            }
            self.pos += 1;
            let value = self.parse_value()?;
            map.insert(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(JsonValue::Object(map));
                }
                _ => {
                    return Err(ReportParseError::new(format!(
                        "expected ',' or '}}' in object at byte {}",
                        self.pos
                    )));
                }
            }
        }
    }

    fn parse_array(&mut self) -> Result<JsonValue, ReportParseError> {
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JsonValue::Array(items));
        }

        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(JsonValue::Array(items));
                }
                _ => {
                    return Err(ReportParseError::new(format!(
                        "expected ',' or ']' in array at byte {}",
                        self.pos
                    )));
                }
            }
        }
    }

    fn parse_string(&mut self) -> Result<String, ReportParseError> {
        self.pos += 1;
        let mut raw = Vec::new();
        while let Some(&byte) = self.bytes.get(self.pos) {
            self.pos += 1;
            match byte {
                b'"' => {
                    return String::from_utf8(raw).map_err(|err| {
                        ReportParseError::new(format!("invalid UTF-8 in JSON string: {err}"))
                    });
                }
                b'\\' => {
                    let escaped = self.bytes.get(self.pos).copied().ok_or_else(|| {
                        ReportParseError::new("unexpected end of JSON escape sequence")
                    })?;
                    self.pos += 1;
                    self.parse_escape(escaped, &mut raw)?;
                }
                normal => raw.push(normal),
            }
        }
        Err(ReportParseError::new("unterminated JSON string"))
    }

    fn parse_escape(&mut self, escaped: u8, raw: &mut Vec<u8>) -> Result<(), ReportParseError> {
        match escaped {
            b'"' => raw.push(b'"'),
            b'\\' => raw.push(b'\\'),
            b'/' => raw.push(b'/'),
            b'n' => raw.push(b'\n'),
            b'r' => raw.push(b'\r'),
            b't' => raw.push(b'\t'),
            b'b' => raw.push(0x08),
            b'f' => raw.push(0x0C),
            b'u' => {
                let hex_slice = self
                    .bytes
                    .get(self.pos..self.pos + 4)
                    .ok_or_else(|| ReportParseError::new("incomplete \\uXXXX escape sequence"))?;
                let hex_str = std::str::from_utf8(hex_slice)
                    .map_err(|_| ReportParseError::new("invalid hex digits in \\uXXXX escape"))?;
                let code = u32::from_str_radix(hex_str, 16)
                    .map_err(|_| ReportParseError::new("invalid hex value in \\uXXXX escape"))?;
                let ch = char::from_u32(code).ok_or_else(|| {
                    ReportParseError::new("invalid Unicode scalar in \\uXXXX escape")
                })?;
                let mut buf = [0u8; 4];
                raw.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                self.pos += 4;
            }
            other => {
                return Err(ReportParseError::new(format!(
                    "unsupported JSON escape '\\{}'",
                    char::from(other)
                )));
            }
        }
        Ok(())
    }

    fn parse_bool(&mut self) -> Result<bool, ReportParseError> {
        if self.bytes.get(self.pos..self.pos + 4) == Some(b"true") {
            self.pos += 4;
            Ok(true)
        } else if self.bytes.get(self.pos..self.pos + 5) == Some(b"false") {
            self.pos += 5;
            Ok(false)
        } else {
            Err(ReportParseError::new(format!(
                "invalid boolean literal at byte {}",
                self.pos
            )))
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, ReportParseError> {
        if self.bytes.get(self.pos..self.pos + 4) == Some(b"null") {
            self.pos += 4;
            Ok(JsonValue::Null)
        } else {
            Err(ReportParseError::new(format!(
                "invalid null literal at byte {}",
                self.pos
            )))
        }
    }

    fn parse_number(&mut self) -> Result<u64, ReportParseError> {
        let start = self.pos;
        while let Some(&byte) = self.bytes.get(self.pos) {
            if byte.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        let slice = self
            .bytes
            .get(start..self.pos)
            .ok_or_else(|| ReportParseError::new("invalid numeric slice"))?;
        let text = std::str::from_utf8(slice)
            .map_err(|_| ReportParseError::new("invalid UTF-8 in number"))?;
        text.parse::<u64>()
            .map_err(|err| ReportParseError::new(format!("invalid u64 number '{text}': {err}")))
    }
}

pub(super) fn get_field<'a>(
    obj: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a JsonValue, ReportParseError> {
    obj.get(key)
        .ok_or_else(|| ReportParseError::new(format!("missing required field '{key}'")))
}

pub(super) fn get_required_str<'a>(
    obj: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, ReportParseError> {
    get_field(obj, key)?.as_str(key)
}

pub(super) fn get_optional_string(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Option<String>, ReportParseError> {
    match get_field(obj, key)? {
        JsonValue::Null => Ok(None),
        value => Ok(Some(value.as_str(key)?.to_string())),
    }
}

pub(super) fn get_optional_string_or_default(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<String, ReportParseError> {
    match obj.get(key) {
        None | Some(JsonValue::Null) => Ok(String::new()),
        Some(value) => Ok(value.as_str(key)?.to_string()),
    }
}

pub(super) fn get_optional_string_list(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<String>, ReportParseError> {
    match obj.get(key) {
        None | Some(JsonValue::Null) => Ok(Vec::new()),
        Some(value) => value
            .as_array(key)?
            .iter()
            .map(|item| item.as_str(key).map(str::to_string))
            .collect::<Result<Vec<String>, ReportParseError>>(),
    }
}

pub(super) fn get_optional_path(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Option<PathBuf>, ReportParseError> {
    Ok(get_optional_string(obj, key)?.map(PathBuf::from))
}

pub(super) fn get_required_u64(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<u64, ReportParseError> {
    get_field(obj, key)?.as_u64(key)
}

pub(super) fn get_required_bool(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<bool, ReportParseError> {
    get_field(obj, key)?.as_bool(key)
}

pub(super) fn get_required_string_list(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<String>, ReportParseError> {
    let arr = get_field(obj, key)?.as_array(key)?;
    let mut list = Vec::with_capacity(arr.len());
    for item in arr {
        list.push(item.as_str(key)?.to_string());
    }
    Ok(list)
}

pub(super) fn get_required_diagnostics(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<BuildDiagnostic>, ReportParseError> {
    let arr = get_field(obj, key)?.as_array(key)?;
    let mut list = Vec::with_capacity(arr.len());
    for item in arr {
        let diag_obj = item.as_object(key)?;
        let code = get_required_str(diag_obj, "code")?.to_string();
        let message = get_required_str(diag_obj, "message")?.to_string();
        let source = get_optional_string(diag_obj, "source")?;
        let remedy = get_optional_string(diag_obj, "remedy")?;
        list.push(BuildDiagnostic {
            code,
            message,
            source,
            remedy,
        });
    }
    Ok(list)
}

pub(super) fn get_required_compatibility_findings(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<CompatibilityFinding>, ReportParseError> {
    let arr = get_field(obj, key)?.as_array(key)?;
    let mut list = Vec::with_capacity(arr.len());
    for item in arr {
        let entry_obj = item.as_object(key)?;
        let feature_id = get_required_str(entry_obj, "feature_id")?.to_string();
        let tier = parse_tier(get_required_str(entry_obj, "tier")?)?;
        let state = parse_state(get_required_str(entry_obj, "state")?)?;
        let guid = get_optional_string(entry_obj, "guid")?;
        let asset_path = get_optional_string(entry_obj, "asset_path")?;
        let sub_asset_id = get_optional_string(entry_obj, "sub_asset_id")?;
        let object_name = get_optional_string(entry_obj, "object_name")?;
        let behavior = get_required_str(entry_obj, "behavior")?.to_string();
        let relied_upon_unity_differences =
            get_required_string_list(entry_obj, "relied_upon_unity_differences")?;
        let observable_consequence =
            get_required_str(entry_obj, "observable_consequence")?.to_string();
        let conditions = get_required_string_list(entry_obj, "conditions")?;
        let related_findings = get_required_string_list(entry_obj, "related_findings")?;
        list.push(CompatibilityFinding {
            feature_id,
            tier,
            state,
            guid,
            asset_path,
            sub_asset_id,
            object_name,
            behavior,
            relied_upon_unity_differences,
            observable_consequence,
            conditions,
            related_findings,
        });
    }
    Ok(list)
}

pub(super) fn get_required_category_sizes(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<SizeCategoryEntry>, ReportParseError> {
    let arr = get_field(obj, key)?.as_array(key)?;
    let mut list = Vec::with_capacity(arr.len());
    for item in arr {
        let entry_obj = item.as_object(key)?;
        let category = get_required_str(entry_obj, "category")?.to_string();
        let size_bytes = get_required_u64(entry_obj, "size_bytes")?;
        list.push(SizeCategoryEntry {
            category,
            size_bytes,
        });
    }
    Ok(list)
}

fn parse_contributors(
    arr: &[JsonValue],
    key: &str,
) -> Result<Vec<SizeContributor>, ReportParseError> {
    let mut list = Vec::with_capacity(arr.len());
    for item in arr {
        let entry_obj = item.as_object(key)?;
        let path = get_required_str(entry_obj, "path")?.to_string();
        let category = get_required_str(entry_obj, "category")?.to_string();
        let size_bytes = get_required_u64(entry_obj, "size_bytes")?;
        let content_hash = get_required_str(entry_obj, "content_hash")?.to_string();
        list.push(SizeContributor {
            path,
            category,
            size_bytes,
            content_hash,
        });
    }
    Ok(list)
}

pub(super) fn get_required_contributors(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<SizeContributor>, ReportParseError> {
    let arr = get_field(obj, key)?.as_array(key)?;
    parse_contributors(arr, key)
}

pub(super) fn get_optional_contributors(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<SizeContributor>, ReportParseError> {
    match obj.get(key) {
        None | Some(JsonValue::Null) => Ok(Vec::new()),
        Some(value) => parse_contributors(value.as_array(key)?, key),
    }
}

pub(super) fn get_optional_licensing(
    obj: &BTreeMap<String, JsonValue>,
) -> Result<Option<LicensingRecord>, ReportParseError> {
    match obj.get("licensing") {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => {
            let record = value.as_object("licensing")?;
            Ok(Some(LicensingRecord {
                application_identifier: get_required_str(record, "application_identifier")?
                    .to_string(),
                governing_license: get_required_str(record, "governing_license")?.to_string(),
                governing_license_name: get_required_str(record, "governing_license_name")?
                    .to_string(),
                conveyance_statement: get_required_str(record, "conveyance_statement")?.to_string(),
                corresponding_source_obligation: get_required_str(
                    record,
                    "corresponding_source_obligation",
                )?
                .to_string(),
                corresponding_source_availability: get_required_str(
                    record,
                    "corresponding_source_availability",
                )?
                .to_string(),
                linking_exception_statement: get_required_str(
                    record,
                    "linking_exception_statement",
                )?
                .to_string(),
                commercial_license_statement: get_required_str(
                    record,
                    "commercial_license_statement",
                )?
                .to_string(),
                not_legal_advice_statement: get_required_str(record, "not_legal_advice_statement")?
                    .to_string(),
                license_texts: get_required_string_list(record, "license_texts")?,
                third_party_notices: get_required_notices(record, "third_party_notices")?,
            }))
        }
    }
}

pub(super) fn get_optional_startup_time(
    obj: &BTreeMap<String, JsonValue>,
) -> Result<StartupMeasurement, ReportParseError> {
    let Some(value) = obj.get("startup_time") else {
        return Ok(StartupMeasurement::unavailable(
            "the report carries no startup measurement",
            STARTUP_BASIS,
        ));
    };
    if matches!(value, JsonValue::Null) {
        return Ok(StartupMeasurement::unavailable(
            "the report carries no startup measurement",
            STARTUP_BASIS,
        ));
    }
    let entry = value.as_object("startup_time")?;
    let status_name = get_required_str(entry, "status")?;
    let status = StartupStatus::from_name(status_name).ok_or_else(|| {
        ReportParseError::new(format!("unrecognized startup status '{status_name}'"))
    })?;
    let duration_ms = match entry.get("duration_ms") {
        None | Some(JsonValue::Null) => None,
        Some(number) => Some(number.as_u64("duration_ms")?),
    };
    let reason = match entry.get("reason") {
        None | Some(JsonValue::Null) => None,
        Some(text) => Some(text.as_str("reason")?.to_string()),
    };
    Ok(StartupMeasurement {
        status,
        duration_ms,
        basis: get_required_str(entry, "basis")?.to_string(),
        reason,
    })
}

pub(super) fn get_optional_target_availability(
    obj: &BTreeMap<String, JsonValue>,
) -> Result<Vec<TargetAvailability>, ReportParseError> {
    let Some(value) = obj.get("target_availability") else {
        return Ok(Vec::new());
    };
    if matches!(value, JsonValue::Null) {
        return Ok(Vec::new());
    }
    let mut availability = Vec::new();
    for entry in value.as_array("target_availability")? {
        availability.push(parse_target_availability_entry(entry)?);
    }
    Ok(availability)
}

fn parse_target_availability_entry(
    entry: &JsonValue,
) -> Result<TargetAvailability, ReportParseError> {
    let record = entry.as_object("target_availability")?;
    let target_name = get_required_str(record, "target")?;
    let target = parse_build_target(target_name, TargetArch::X86_64)?;
    let state = get_required_str(record, "state")?;
    if state != crate::toolchain::TOOLCHAIN_AVAILABLE
        && state != crate::toolchain::TOOLCHAIN_EXTERNALLY_UNAVAILABLE
    {
        return Err(ReportParseError::new(format!(
            "unrecognized target availability state '{state}'"
        )));
    }
    let cause = parse_optional_toolchain_cause(record)?;
    let tool = parse_optional_string_field(record, "tool")?;
    let version_found = parse_optional_string_field(record, "version_found")?;
    let required_range = parse_optional_string_field(record, "required_range")?;
    let resolving_action = parse_optional_string_field(record, "resolving_action")?;
    let tools = parse_toolchain_probes(record)?;
    let unavailable = build_unavailable_toolchain(
        cause,
        tool,
        version_found,
        required_range,
        resolving_action,
        "target availability",
    )?;
    Ok(TargetAvailability {
        target,
        tools,
        unavailable,
    })
}

fn parse_optional_toolchain_cause(
    record: &BTreeMap<String, JsonValue>,
) -> Result<Option<ToolchainCause>, ReportParseError> {
    match record.get("cause") {
        None | Some(JsonValue::Null) => Ok(None),
        Some(text) => {
            let name = text.as_str("cause")?;
            Ok(Some(ToolchainCause::from_name(name).ok_or_else(|| {
                ReportParseError::new(format!("unrecognized toolchain cause '{name}'"))
            })?))
        }
    }
}

fn parse_optional_string_field(
    record: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Option<String>, ReportParseError> {
    match record.get(key) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(text) => Ok(Some(text.as_str(key)?.to_string())),
    }
}

fn parse_toolchain_probes(
    record: &BTreeMap<String, JsonValue>,
) -> Result<Vec<ToolchainProbe>, ReportParseError> {
    let mut tools = Vec::new();
    if let Some(probes) = record.get("tools") {
        for probe in probes.as_array("tools")? {
            let fields = probe.as_object("tools")?;
            let version = parse_optional_string_field(fields, "version_found")?;
            tools.push(ToolchainProbe {
                tool: get_required_str(fields, "tool")?.to_string(),
                required_range: get_required_str(fields, "required_range")?.to_string(),
                purpose: get_required_str(fields, "purpose")?.to_string(),
                present: get_required_bool(fields, "present")?,
                version_found: version,
            });
        }
    }
    Ok(tools)
}

fn build_unavailable_toolchain(
    cause: Option<ToolchainCause>,
    tool: Option<String>,
    version_found: Option<String>,
    required_range: Option<String>,
    resolving_action: Option<String>,
    context: &str,
) -> Result<Option<UnavailableToolchain>, ReportParseError> {
    match (cause, tool, required_range, resolving_action) {
        (Some(cause), Some(tool), Some(required_range), Some(resolving_action)) => {
            Ok(Some(UnavailableToolchain {
                cause,
                tool,
                version_found,
                required_range,
                resolving_action,
            }))
        }
        (None, None, None, None) => Ok(None),
        _ => Err(ReportParseError::new(format!(
            "a {context} record carries an incomplete unavailable state"
        ))),
    }
}

pub(super) fn get_optional_desktop_packaging(
    obj: &BTreeMap<String, JsonValue>,
) -> Result<Vec<DesktopPackageOutcome>, ReportParseError> {
    let Some(value) = obj.get("desktop_packaging") else {
        return Ok(Vec::new());
    };
    if matches!(value, JsonValue::Null) {
        return Ok(Vec::new());
    }
    let mut outcomes = Vec::new();
    for entry in value.as_array("desktop_packaging")? {
        outcomes.push(parse_desktop_packaging_entry(entry)?);
    }
    Ok(outcomes)
}

fn parse_desktop_packaging_entry(
    entry: &JsonValue,
) -> Result<DesktopPackageOutcome, ReportParseError> {
    let record = entry.as_object("desktop_packaging")?;
    let target_name = get_required_str(record, "target")?;
    let target = parse_build_target(target_name, TargetArch::X86_64)?;
    let state = get_required_str(record, "state")?.to_string();
    if state != DESKTOP_PACKAGE_STATE_PACKAGED
        && state != DESKTOP_PACKAGE_STATE_EXTERNALLY_UNAVAILABLE
    {
        return Err(ReportParseError::new(format!(
            "unrecognized desktop packaging state '{state}'"
        )));
    }
    let app_path = get_optional_path(record, "app_path")?;
    let launch_artifact = get_optional_path(record, "launch_artifact")?;
    let build_report_path = PathBuf::from(get_required_str(record, "build_report_path")?);
    let launched = get_required_bool(record, "launched")?;
    let cause = parse_optional_toolchain_cause(record)?;
    let tool = parse_optional_string_field(record, "tool")?;
    let version_found = parse_optional_string_field(record, "version_found")?;
    let required_range = parse_optional_string_field(record, "required_range")?;
    let resolving_action = parse_optional_string_field(record, "resolving_action")?;
    let unavailable = build_unavailable_toolchain(
        cause,
        tool,
        version_found,
        required_range,
        resolving_action,
        "desktop packaging",
    )?;
    Ok(DesktopPackageOutcome {
        target,
        state,
        app_path,
        launch_artifact,
        build_report_path,
        launched,
        unavailable,
    })
}

fn get_required_notices(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<ThirdPartyNotice>, ReportParseError> {
    let array = get_field(obj, key)?.as_array(key)?;
    let mut notices = Vec::with_capacity(array.len());
    for item in array {
        let entry = item.as_object(key)?;
        notices.push(ThirdPartyNotice {
            ecosystem: get_required_str(entry, "ecosystem")?.to_string(),
            component: get_required_str(entry, "component")?.to_string(),
            version: get_required_str(entry, "version")?.to_string(),
            license: get_required_str(entry, "license")?.to_string(),
        });
    }
    Ok(notices)
}

pub(super) fn parse_target_arch(raw: &str) -> Result<TargetArch, ReportParseError> {
    match raw.to_ascii_lowercase().as_str() {
        "x86_64" | "x86-64" => Ok(TargetArch::X86_64),
        "aarch64" | "arm64" => Ok(TargetArch::Arm64),
        _ => Err(ReportParseError::new(format!(
            "unrecognized target architecture '{raw}'"
        ))),
    }
}

pub(super) fn parse_build_target(
    raw: &str,
    fallback_arch: TargetArch,
) -> Result<BuildTarget, ReportParseError> {
    for candidate in BuildTarget::all() {
        if candidate.identifier().eq_ignore_ascii_case(raw)
            || candidate.to_string().eq_ignore_ascii_case(raw)
        {
            return Ok(*candidate);
        }
    }
    let os = match raw.to_ascii_lowercase().as_str() {
        "windows" => Some(TargetOs::Windows),
        "macos" => Some(TargetOs::MacOs),
        "linux" => Some(TargetOs::Linux),
        _ => None,
    };
    if let Some(target_os) = os {
        return Ok(BuildTarget::new(target_os, fallback_arch));
    }
    Err(ReportParseError::new(format!(
        "unrecognized build target '{raw}'"
    )))
}

pub(super) fn parse_build_mode(raw: &str) -> Result<BuildMode, ReportParseError> {
    match raw.to_ascii_lowercase().as_str() {
        "development" => Ok(BuildMode::Development),
        "release" => Ok(BuildMode::Release),
        _ => Err(ReportParseError::new(format!(
            "unrecognized build configuration mode '{raw}'"
        ))),
    }
}
