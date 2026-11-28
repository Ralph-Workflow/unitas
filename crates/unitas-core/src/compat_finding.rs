use super::compat_canonical::build_canonical_report;
use super::compat_catalog::lookup_catalog_feature;
use super::{CompatibilityReport, CompatibilityState, ContentIdentity, Finding, Tier};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FindingInput {
    pub feature_id: String,
    pub tier: Tier,
    pub state: CompatibilityState,
    pub content: ContentIdentity,
    pub behavior: String,
    pub relied_upon_unity_differences: Vec<String>,
    pub observable_consequence: String,
    pub conditions: Vec<String>,
    pub related_findings: Vec<String>,
}

impl FindingInput {
    pub fn new(
        feature_id: impl Into<String>,
        tier: Tier,
        state: CompatibilityState,
        content: ContentIdentity,
        behavior: impl Into<String>,
        observable_consequence: impl Into<String>,
    ) -> Self {
        Self {
            feature_id: feature_id.into(),
            tier,
            state,
            content,
            behavior: behavior.into(),
            relied_upon_unity_differences: Vec::new(),
            observable_consequence: observable_consequence.into(),
            conditions: Vec::new(),
            related_findings: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_unity_difference(mut self, difference: impl Into<String>) -> Self {
        self.relied_upon_unity_differences.push(difference.into());
        self
    }

    #[must_use]
    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.conditions.push(condition.into());
        self
    }

    #[must_use]
    pub fn with_related_finding(mut self, related: impl Into<String>) -> Self {
        self.related_findings.push(related.into());
        self
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FindingSubmissionError {
    EmptyFeatureId,
    UnknownCatalogFeatureId {
        feature_id: String,
    },
    CatalogTierMismatch {
        feature_id: String,
        expected: Tier,
        submitted: Tier,
    },
    MissingContentIdentity,
    EmptyBehavior,
    EmptyObservableConsequence,
    SupportedFindingClaimsUnityDifference {
        feature_id: String,
        tier: Tier,
    },
    MissingReliedUponUnityDifference {
        feature_id: String,
        tier: Tier,
        state: CompatibilityState,
    },
    UnityScreenshotPixelDifferenceIsNotCompatibilityFailure {
        feature_id: String,
    },
    DuplicateFindingIdentity {
        feature_id: String,
        content: Box<ContentIdentity>,
    },
}

impl std::fmt::Display for FindingSubmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyFeatureId => write!(formatter, "finding feature_id must not be empty"),
            Self::UnknownCatalogFeatureId { feature_id } => {
                write!(formatter, "unknown catalog feature_id '{feature_id}'")
            }
            Self::CatalogTierMismatch {
                feature_id,
                expected,
                submitted,
            } => write!(
                formatter,
                "catalog tier mismatch for '{feature_id}': expected {expected:?}, got {submitted:?}"
            ),
            Self::MissingContentIdentity => {
                write!(
                    formatter,
                    "finding content identity must name at least one non-empty field"
                )
            }
            Self::EmptyBehavior => write!(formatter, "finding behavior must not be empty"),
            Self::EmptyObservableConsequence => {
                write!(
                    formatter,
                    "finding observable_consequence must not be empty"
                )
            }
            Self::SupportedFindingClaimsUnityDifference { feature_id, tier } => write!(
                formatter,
                "finding for '{feature_id}' ({tier:?}) cannot claim Supported while listing a Unity difference"
            ),
            Self::MissingReliedUponUnityDifference {
                feature_id,
                tier,
                state,
            } => write!(
                formatter,
                "finding for '{feature_id}' ({tier:?}) with state {state:?} must list at least one relied-upon Unity difference"
            ),
            Self::UnityScreenshotPixelDifferenceIsNotCompatibilityFailure { feature_id } => write!(
                formatter,
                "finding for '{feature_id}' cannot report a pixel difference from a Unity screenshot as a compatibility failure"
            ),
            Self::DuplicateFindingIdentity {
                feature_id,
                content,
            } => write!(
                formatter,
                "duplicate finding identity for feature '{feature_id}' and content {content:?}"
            ),
        }
    }
}

impl std::error::Error for FindingSubmissionError {}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FindingRejection {
    pub index: usize,
    pub error: FindingSubmissionError,
}

impl std::fmt::Display for FindingRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "finding submission rejected at index {}: {}",
            self.index, self.error
        )
    }
}

impl std::error::Error for FindingRejection {}

fn sanitize_optional_field(field: Option<&str>) -> Option<String> {
    field
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn sanitize_content_identity(content: &ContentIdentity) -> ContentIdentity {
    ContentIdentity {
        guid: sanitize_optional_field(content.guid.as_deref()),
        asset_path: sanitize_optional_field(content.asset_path.as_deref()),
        sub_asset_id: sanitize_optional_field(content.sub_asset_id.as_deref()),
        object_name: sanitize_optional_field(content.object_name.as_deref()),
    }
}

fn validate_finding_header(input: &FindingInput) -> Result<String, FindingSubmissionError> {
    let feature_id = input.feature_id.trim().to_string();
    if feature_id.is_empty() {
        return Err(FindingSubmissionError::EmptyFeatureId);
    }
    let Some(catalog_entry) = lookup_catalog_feature(&feature_id) else {
        return Err(FindingSubmissionError::UnknownCatalogFeatureId { feature_id });
    };
    if catalog_entry.tier != input.tier {
        return Err(FindingSubmissionError::CatalogTierMismatch {
            feature_id,
            expected: catalog_entry.tier,
            submitted: input.tier,
        });
    }
    Ok(feature_id)
}

fn validate_finding_content(
    input: &FindingInput,
) -> Result<ContentIdentity, FindingSubmissionError> {
    let content = sanitize_content_identity(&input.content);
    if !content.names_content() {
        return Err(FindingSubmissionError::MissingContentIdentity);
    }
    Ok(content)
}

fn validate_finding_text(input: &FindingInput) -> Result<(String, String), FindingSubmissionError> {
    let behavior = input.behavior.trim().to_string();
    if behavior.is_empty() {
        return Err(FindingSubmissionError::EmptyBehavior);
    }
    let observable_consequence = input.observable_consequence.trim().to_string();
    if observable_consequence.is_empty() {
        return Err(FindingSubmissionError::EmptyObservableConsequence);
    }
    Ok((behavior, observable_consequence))
}

fn validate_finding_consistency(
    tier: Tier,
    state: CompatibilityState,
    feature_id: &str,
    behavior: &str,
    observable_consequence: &str,
    relied_upon_unity_differences: &[String],
) -> Result<(), FindingSubmissionError> {
    if matches!(tier, Tier::Exact | Tier::Semantic)
        && state == CompatibilityState::Supported
        && !relied_upon_unity_differences.is_empty()
    {
        return Err(
            FindingSubmissionError::SupportedFindingClaimsUnityDifference {
                feature_id: feature_id.to_string(),
                tier,
            },
        );
    }
    if tier == Tier::Familiar
        && matches!(
            state,
            CompatibilityState::PartiallySupported | CompatibilityState::Unsupported
        )
        && relied_upon_unity_differences.is_empty()
    {
        return Err(FindingSubmissionError::MissingReliedUponUnityDifference {
            feature_id: feature_id.to_string(),
            tier,
            state,
        });
    }
    let lower_consequence = observable_consequence.to_ascii_lowercase();
    let lower_behavior = behavior.to_ascii_lowercase();
    if tier == Tier::Native
        && matches!(
            state,
            CompatibilityState::PartiallySupported | CompatibilityState::Unsupported
        )
        && ((lower_consequence.contains("pixel") && lower_consequence.contains("unity screenshot"))
            || (lower_behavior.contains("pixel") && lower_behavior.contains("unity screenshot")))
    {
        return Err(
            FindingSubmissionError::UnityScreenshotPixelDifferenceIsNotCompatibilityFailure {
                feature_id: feature_id.to_string(),
            },
        );
    }
    Ok(())
}

pub(super) fn validate_finding_input(
    input: FindingInput,
    existing: &[Finding],
) -> Result<Finding, FindingSubmissionError> {
    let feature_id = validate_finding_header(&input)?;
    let content = validate_finding_content(&input)?;
    let (behavior, observable_consequence) = validate_finding_text(&input)?;
    let tier = input.tier;
    let state = input.state;
    let relied_upon_unity_differences: Vec<String> = input
        .relied_upon_unity_differences
        .into_iter()
        .map(|diff| diff.trim().to_string())
        .filter(|diff| !diff.is_empty())
        .collect();
    validate_finding_consistency(
        tier,
        state,
        &feature_id,
        &behavior,
        &observable_consequence,
        &relied_upon_unity_differences,
    )?;
    let is_duplicate = existing.iter().any(|item| {
        item.feature_id == feature_id && sanitize_content_identity(&item.content) == content
    });
    if is_duplicate {
        return Err(FindingSubmissionError::DuplicateFindingIdentity {
            feature_id,
            content: Box::new(content),
        });
    }
    Ok(Finding {
        feature_id,
        tier,
        state,
        content,
        behavior,
        relied_upon_unity_differences,
        observable_consequence,
        conditions: input.conditions,
        related_findings: input.related_findings,
    })
}

pub fn submit_finding(
    report: &CompatibilityReport,
    input: FindingInput,
) -> Result<CompatibilityReport, FindingSubmissionError> {
    let validated = validate_finding_input(input, &report.findings)?;
    let mut findings = report.findings.clone();
    findings.push(validated);
    Ok(build_canonical_report(
        &report.produced_by_unitas_version,
        findings,
    ))
}

pub fn submit_findings<I>(
    report: &CompatibilityReport,
    inputs: I,
) -> Result<CompatibilityReport, FindingRejection>
where
    I: IntoIterator<Item = FindingInput>,
{
    let mut findings = report.findings.clone();
    for (index, input) in inputs.into_iter().enumerate() {
        match validate_finding_input(input, &findings) {
            Ok(validated) => findings.push(validated),
            Err(error) => return Err(FindingRejection { index, error }),
        }
    }
    Ok(build_canonical_report(
        &report.produced_by_unitas_version,
        findings,
    ))
}

pub fn merge_findings(
    report: &CompatibilityReport,
    external_findings: &[Finding],
) -> Result<CompatibilityReport, FindingRejection> {
    let inputs = external_findings
        .iter()
        .cloned()
        .map(|finding| FindingInput {
            feature_id: finding.feature_id,
            tier: finding.tier,
            state: finding.state,
            content: finding.content,
            behavior: finding.behavior,
            relied_upon_unity_differences: finding.relied_upon_unity_differences,
            observable_consequence: finding.observable_consequence,
            conditions: finding.conditions,
            related_findings: finding.related_findings,
        });
    submit_findings(report, inputs)
}
