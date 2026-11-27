use super::build_report;
use super::{CompatibilityReport, CompatibilityState, ContentIdentity, Finding, Tier};
use std::cmp::Ordering;

fn compare_content_identity(left: &ContentIdentity, right: &ContentIdentity) -> Ordering {
    left.asset_path
        .as_deref()
        .unwrap_or("")
        .cmp(right.asset_path.as_deref().unwrap_or(""))
        .then_with(|| {
            left.guid
                .as_deref()
                .unwrap_or("")
                .cmp(right.guid.as_deref().unwrap_or(""))
        })
        .then_with(|| {
            left.sub_asset_id
                .as_deref()
                .unwrap_or("")
                .cmp(right.sub_asset_id.as_deref().unwrap_or(""))
        })
        .then_with(|| {
            left.object_name
                .as_deref()
                .unwrap_or("")
                .cmp(right.object_name.as_deref().unwrap_or(""))
        })
}

fn tier_rank(tier: Tier) -> u8 {
    match tier {
        Tier::Exact => 0,
        Tier::Semantic => 1,
        Tier::Familiar => 2,
        Tier::Native => 3,
    }
}

fn compare_findings(left: &Finding, right: &Finding) -> Ordering {
    left.feature_id
        .cmp(&right.feature_id)
        .then_with(|| compare_content_identity(&left.content, &right.content))
        .then_with(|| left.behavior.cmp(&right.behavior))
        .then_with(|| left.state.cmp(&right.state))
        .then_with(|| tier_rank(left.tier).cmp(&tier_rank(right.tier)))
        .then_with(|| {
            left.observable_consequence
                .cmp(&right.observable_consequence)
        })
        .then_with(|| {
            left.relied_upon_unity_differences
                .cmp(&right.relied_upon_unity_differences)
        })
        .then_with(|| left.conditions.cmp(&right.conditions))
        .then_with(|| left.related_findings.cmp(&right.related_findings))
}

pub(super) fn build_canonical_report(
    unitas_version: &str,
    mut findings: Vec<Finding>,
) -> CompatibilityReport {
    findings.sort_by(compare_findings);
    let mut report = build_report(unitas_version, findings);
    for package in &mut report.packages {
        package.feature_ids.sort();
    }
    report
        .packages
        .sort_by(|left, right| left.package_name.cmp(&right.package_name));
    report
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CanonicalReportParseError {
    MissingHeader,
    MissingVersion,
    MalformedLine { line_number: usize, line: String },
    InvalidTier(String),
    InvalidState(String),
    InvalidEscapeSequence(String),
    UnexpectedSectionLine { line_number: usize, line: String },
}

impl std::fmt::Display for CanonicalReportParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingHeader => write!(formatter, "missing UNITAS_COMPAT_REPORT v1 header"),
            Self::MissingVersion => write!(formatter, "missing unitas_version field"),
            Self::MalformedLine { line_number, line } => {
                write!(formatter, "malformed line {line_number}: '{line}'")
            }
            Self::InvalidTier(value) => write!(formatter, "invalid compatibility tier '{value}'"),
            Self::InvalidState(value) => write!(formatter, "invalid compatibility state '{value}'"),
            Self::InvalidEscapeSequence(value) => {
                write!(formatter, "invalid escape sequence in '{value}'")
            }
            Self::UnexpectedSectionLine { line_number, line } => {
                write!(
                    formatter,
                    "unexpected line {line_number} outside section: '{line}'"
                )
            }
        }
    }
}

impl std::error::Error for CanonicalReportParseError {}

fn format_tier(tier: Tier) -> &'static str {
    match tier {
        Tier::Exact => "Exact",
        Tier::Semantic => "Semantic",
        Tier::Familiar => "Familiar",
        Tier::Native => "Native",
    }
}

fn parse_tier(value: &str) -> Result<Tier, CanonicalReportParseError> {
    match value {
        "Exact" => Ok(Tier::Exact),
        "Semantic" => Ok(Tier::Semantic),
        "Familiar" => Ok(Tier::Familiar),
        "Native" => Ok(Tier::Native),
        other => Err(CanonicalReportParseError::InvalidTier(other.to_string())),
    }
}

fn format_state(state: CompatibilityState) -> &'static str {
    match state {
        CompatibilityState::Supported => "Supported",
        CompatibilityState::PartiallySupported => "PartiallySupported",
        CompatibilityState::PlatformLimited => "PlatformLimited",
        CompatibilityState::Unknown => "Unknown",
        CompatibilityState::Unsupported => "Unsupported",
        CompatibilityState::ExternallyUnavailable => "ExternallyUnavailable",
    }
}

fn parse_state(value: &str) -> Result<CompatibilityState, CanonicalReportParseError> {
    match value {
        "Supported" => Ok(CompatibilityState::Supported),
        "PartiallySupported" => Ok(CompatibilityState::PartiallySupported),
        "PlatformLimited" => Ok(CompatibilityState::PlatformLimited),
        "Unknown" => Ok(CompatibilityState::Unknown),
        "Unsupported" => Ok(CompatibilityState::Unsupported),
        "ExternallyUnavailable" => Ok(CompatibilityState::ExternallyUnavailable),
        other => Err(CanonicalReportParseError::InvalidState(other.to_string())),
    }
}

fn escape_canonical_str(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

fn unescape_canonical_str(value: &str) -> Result<String, CanonicalReportParseError> {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                _ => {
                    return Err(CanonicalReportParseError::InvalidEscapeSequence(
                        value.to_string(),
                    ));
                }
            }
        } else {
            out.push(ch);
        }
    }
    Ok(out)
}

fn format_optional_str(value: Option<&str>) -> String {
    match value {
        Some(inner) => format!("Some({})", escape_canonical_str(inner)),
        None => "None".to_string(),
    }
}

fn parse_optional_str(value: &str) -> Result<Option<String>, CanonicalReportParseError> {
    if value == "None" {
        return Ok(None);
    }
    if let Some(rest) = value.strip_prefix("Some(") {
        if let Some(inner) = rest.strip_suffix(')') {
            return Ok(Some(unescape_canonical_str(inner)?));
        }
    }
    Err(CanonicalReportParseError::InvalidEscapeSequence(
        value.to_string(),
    ))
}

pub fn render_canonical(report: &CompatibilityReport) -> String {
    let canonical =
        build_canonical_report(&report.produced_by_unitas_version, report.findings.clone());
    let summary = canonical.summary();
    let mut out = String::new();
    out.push_str("UNITAS_COMPAT_REPORT v1\n");
    out.push_str(&format!(
        "unitas_version: {}\n",
        escape_canonical_str(&canonical.produced_by_unitas_version)
    ));
    out.push_str(&format!(
        "overall_state: {}\n",
        format_state(summary.overall_state)
    ));
    out.push_str(&format!("total_findings: {}\n", summary.total_findings));
    out.push_str(&format!("present_findings: {}\n", summary.present_findings));
    out.push_str(&format!("absent_findings: {}\n", summary.absent_findings));
    out.push_str(&format!("supported_count: {}\n", summary.supported_count));
    out.push_str(&format!(
        "partially_supported_count: {}\n",
        summary.partially_supported_count
    ));
    out.push_str(&format!(
        "platform_limited_count: {}\n",
        summary.platform_limited_count
    ));
    out.push_str(&format!("unknown_count: {}\n", summary.unknown_count));
    out.push_str(&format!(
        "unsupported_count: {}\n",
        summary.unsupported_count
    ));
    out.push_str(&format!(
        "externally_unavailable_count: {}\n",
        summary.externally_unavailable_count
    ));
    out.push_str(&format!("packages_count: {}\n", canonical.packages.len()));
    for package in &canonical.packages {
        out.push_str("[package]\n");
        out.push_str(&format!(
            "package_name: {}\n",
            escape_canonical_str(&package.package_name)
        ));
        out.push_str(&format!("state: {}\n", format_state(package.state)));
        for feature_id in &package.feature_ids {
            out.push_str(&format!(
                "feature_id: {}\n",
                escape_canonical_str(feature_id)
            ));
        }
    }
    out.push_str(&format!("findings_count: {}\n", canonical.findings.len()));
    for finding in &canonical.findings {
        out.push_str("[finding]\n");
        out.push_str(&format!(
            "feature_id: {}\n",
            escape_canonical_str(&finding.feature_id)
        ));
        out.push_str(&format!("tier: {}\n", format_tier(finding.tier)));
        out.push_str(&format!("state: {}\n", format_state(finding.state)));
        out.push_str(&format!(
            "content.guid: {}\n",
            format_optional_str(finding.content.guid.as_deref())
        ));
        out.push_str(&format!(
            "content.asset_path: {}\n",
            format_optional_str(finding.content.asset_path.as_deref())
        ));
        out.push_str(&format!(
            "content.sub_asset_id: {}\n",
            format_optional_str(finding.content.sub_asset_id.as_deref())
        ));
        out.push_str(&format!(
            "content.object_name: {}\n",
            format_optional_str(finding.content.object_name.as_deref())
        ));
        out.push_str(&format!(
            "behavior: {}\n",
            escape_canonical_str(&finding.behavior)
        ));
        out.push_str(&format!(
            "observable_consequence: {}\n",
            escape_canonical_str(&finding.observable_consequence)
        ));
        for difference in &finding.relied_upon_unity_differences {
            out.push_str(&format!(
                "unity_difference: {}\n",
                escape_canonical_str(difference)
            ));
        }
        for condition in &finding.conditions {
            out.push_str(&format!("condition: {}\n", escape_canonical_str(condition)));
        }
        for related in &finding.related_findings {
            out.push_str(&format!(
                "related_finding: {}\n",
                escape_canonical_str(related)
            ));
        }
    }
    out
}

struct PendingFinding {
    feature_id: Option<String>,
    tier: Option<Tier>,
    state: Option<CompatibilityState>,
    guid: Option<String>,
    asset_path: Option<String>,
    sub_asset_id: Option<String>,
    object_name: Option<String>,
    behavior: Option<String>,
    observable_consequence: Option<String>,
    relied_upon_unity_differences: Vec<String>,
    conditions: Vec<String>,
    related_findings: Vec<String>,
}

impl PendingFinding {
    fn new() -> Self {
        Self {
            feature_id: None,
            tier: None,
            state: None,
            guid: None,
            asset_path: None,
            sub_asset_id: None,
            object_name: None,
            behavior: None,
            observable_consequence: None,
            relied_upon_unity_differences: Vec::new(),
            conditions: Vec::new(),
            related_findings: Vec::new(),
        }
    }

    fn finish(self, line_number: usize) -> Result<Finding, CanonicalReportParseError> {
        let Some(feature_id) = self.feature_id else {
            return Err(CanonicalReportParseError::MalformedLine {
                line_number,
                line: "missing feature_id in [finding]".to_string(),
            });
        };
        let Some(tier) = self.tier else {
            return Err(CanonicalReportParseError::MalformedLine {
                line_number,
                line: "missing tier in [finding]".to_string(),
            });
        };
        let Some(state) = self.state else {
            return Err(CanonicalReportParseError::MalformedLine {
                line_number,
                line: "missing state in [finding]".to_string(),
            });
        };
        let Some(behavior) = self.behavior else {
            return Err(CanonicalReportParseError::MalformedLine {
                line_number,
                line: "missing behavior in [finding]".to_string(),
            });
        };
        let Some(observable_consequence) = self.observable_consequence else {
            return Err(CanonicalReportParseError::MalformedLine {
                line_number,
                line: "missing observable_consequence in [finding]".to_string(),
            });
        };
        Ok(Finding {
            feature_id,
            tier,
            state,
            content: ContentIdentity {
                guid: self.guid,
                asset_path: self.asset_path,
                sub_asset_id: self.sub_asset_id,
                object_name: self.object_name,
            },
            behavior,
            relied_upon_unity_differences: self.relied_upon_unity_differences,
            observable_consequence,
            conditions: self.conditions,
            related_findings: self.related_findings,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParseSection {
    Header,
    Package,
    Finding,
}

pub fn parse_canonical(text: &str) -> Result<CompatibilityReport, CanonicalReportParseError> {
    let mut lines = text.lines().enumerate();
    let Some((_, first_line)) = lines.next() else {
        return Err(CanonicalReportParseError::MissingHeader);
    };
    if first_line.trim() != "UNITAS_COMPAT_REPORT v1" {
        return Err(CanonicalReportParseError::MissingHeader);
    }

    let mut unitas_version: Option<String> = None;
    let mut section = ParseSection::Header;
    let mut findings: Vec<Finding> = Vec::new();
    let mut current_finding: Option<PendingFinding> = None;

    for (zero_index, raw_line) in lines {
        let line_number = zero_index + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "[package]" {
            if let Some(pending) = current_finding.take() {
                findings.push(pending.finish(line_number)?);
            }
            section = ParseSection::Package;
            continue;
        }
        if trimmed == "[finding]" {
            if let Some(pending) = current_finding.take() {
                findings.push(pending.finish(line_number)?);
            }
            section = ParseSection::Finding;
            current_finding = Some(PendingFinding::new());
            continue;
        }
        let Some((key, raw_value)) = raw_line.split_once(": ") else {
            return Err(CanonicalReportParseError::MalformedLine {
                line_number,
                line: raw_line.to_string(),
            });
        };
        let key = key.trim();
        if key == "findings_count" || key == "packages_count" {
            continue;
        }
        match section {
            ParseSection::Header => match key {
                "unitas_version" => {
                    unitas_version = Some(unescape_canonical_str(raw_value)?);
                }
                "overall_state"
                | "total_findings"
                | "present_findings"
                | "absent_findings"
                | "supported_count"
                | "partially_supported_count"
                | "platform_limited_count"
                | "unknown_count"
                | "unsupported_count"
                | "externally_unavailable_count" => {}
                _ => {
                    return Err(CanonicalReportParseError::UnexpectedSectionLine {
                        line_number,
                        line: raw_line.to_string(),
                    });
                }
            },
            ParseSection::Package => match key {
                "package_name" | "state" | "feature_id" => {}
                _ => {
                    return Err(CanonicalReportParseError::UnexpectedSectionLine {
                        line_number,
                        line: raw_line.to_string(),
                    });
                }
            },
            ParseSection::Finding => {
                let Some(pending) = current_finding.as_mut() else {
                    return Err(CanonicalReportParseError::UnexpectedSectionLine {
                        line_number,
                        line: raw_line.to_string(),
                    });
                };
                match key {
                    "feature_id" => pending.feature_id = Some(unescape_canonical_str(raw_value)?),
                    "tier" => pending.tier = Some(parse_tier(raw_value)?),
                    "state" => pending.state = Some(parse_state(raw_value)?),
                    "content.guid" => pending.guid = parse_optional_str(raw_value)?,
                    "content.asset_path" => pending.asset_path = parse_optional_str(raw_value)?,
                    "content.sub_asset_id" => pending.sub_asset_id = parse_optional_str(raw_value)?,
                    "content.object_name" => pending.object_name = parse_optional_str(raw_value)?,
                    "behavior" => pending.behavior = Some(unescape_canonical_str(raw_value)?),
                    "observable_consequence" => {
                        pending.observable_consequence = Some(unescape_canonical_str(raw_value)?);
                    }
                    "unity_difference" => pending
                        .relied_upon_unity_differences
                        .push(unescape_canonical_str(raw_value)?),
                    "condition" => pending.conditions.push(unescape_canonical_str(raw_value)?),
                    "related_finding" => pending
                        .related_findings
                        .push(unescape_canonical_str(raw_value)?),
                    _ => {
                        return Err(CanonicalReportParseError::UnexpectedSectionLine {
                            line_number,
                            line: raw_line.to_string(),
                        });
                    }
                }
            }
        }
    }

    if let Some(pending) = current_finding.take() {
        findings.push(pending.finish(text.lines().count())?);
    }

    let Some(version) = unitas_version else {
        return Err(CanonicalReportParseError::MissingVersion);
    };

    Ok(build_canonical_report(&version, findings))
}
