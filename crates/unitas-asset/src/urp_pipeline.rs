use std::fmt;
use std::path::{Path, PathBuf};

use unitas_core::compat::{URP_PIPELINE_ASSET_SCRIPT_GUID, URP_RENDERER_DATA_SCRIPT_GUID};

use crate::unity_yaml::{self, Node, Stream, YamlError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrpRenderingMode {
    Forward,
    Deferred,
    ForwardPlus,
    Unknown(i64),
}

impl UrpRenderingMode {
    pub fn from_code(code: i64) -> Self {
        match code {
            0 => UrpRenderingMode::Forward,
            1 => UrpRenderingMode::Deferred,
            2 => UrpRenderingMode::ForwardPlus,
            other => UrpRenderingMode::Unknown(other),
        }
    }

    pub fn code(self) -> i64 {
        match self {
            UrpRenderingMode::Forward => 0,
            UrpRenderingMode::Deferred => 1,
            UrpRenderingMode::ForwardPlus => 2,
            UrpRenderingMode::Unknown(code) => code,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            UrpRenderingMode::Forward => "Forward",
            UrpRenderingMode::Deferred => "Deferred",
            UrpRenderingMode::ForwardPlus => "Forward+",
            UrpRenderingMode::Unknown(_) => "Unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UrpPipelineAsset {
    pub script_guid: Option<String>,
    pub default_renderer_index: i64,
    pub renderer_data_guids: Vec<Option<String>>,
    pub supports_hdr: bool,
    pub msaa_sample_count: u32,
    pub render_scale: f64,
    pub require_depth_texture: bool,
    pub require_opaque_texture: bool,
    pub additional_lights_per_object_limit: u32,
    pub shadow_distance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UrpRendererData {
    pub script_guid: Option<String>,
    pub rendering_mode: UrpRenderingMode,
    pub renderer_feature_guids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PipelineAssetKind {
    UrpPipelineAsset(UrpPipelineAsset),
    UrpRendererData(UrpRendererData),
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedRenderPipelineAsset {
    pub guid: String,
    pub path: PathBuf,
    pub kind: PipelineAssetKind,
}

#[derive(Debug)]
pub enum UrpPipelineError {
    Yaml(YamlError),
    MissingDocument,
}

impl fmt::Display for UrpPipelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UrpPipelineError::Yaml(error) => write!(formatter, "yaml error: {error}"),
            UrpPipelineError::MissingDocument => {
                write!(formatter, "missing render pipeline asset document")
            }
        }
    }
}

impl std::error::Error for UrpPipelineError {}

pub fn parse_urp_pipeline_asset(path: &Path) -> Result<Option<UrpPipelineAsset>, UrpPipelineError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path).map_err(UrpPipelineError::Yaml)?;
    parse_urp_pipeline_asset_stream(&stream).map(Some)
}

pub fn parse_urp_renderer_data(path: &Path) -> Result<Option<UrpRendererData>, UrpPipelineError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path).map_err(UrpPipelineError::Yaml)?;
    parse_urp_renderer_data_stream(&stream).map(Some)
}

pub fn parse_urp_pipeline_asset_stream(
    stream: &Stream,
) -> Result<UrpPipelineAsset, UrpPipelineError> {
    let body = pipeline_document_body(stream).ok_or(UrpPipelineError::MissingDocument)?;
    let renderer_data_guids = body
        .get("m_RendererDataList")
        .and_then(Node::as_sequence)
        .map_or_else(Vec::new, |entries| {
            entries.iter().map(reference_guid).collect()
        });
    let default_renderer_index = optional_i64(body, "m_DefaultRendererIndex").unwrap_or_default();
    Ok(UrpPipelineAsset {
        script_guid: script_guid(body),
        default_renderer_index,
        renderer_data_guids,
        supports_hdr: optional_bool(body, "m_SupportsHDR").unwrap_or(true),
        msaa_sample_count: optional_u32(body, "m_MSAA").unwrap_or(1),
        render_scale: optional_f64(body, "m_RenderScale").unwrap_or(1.0),
        require_depth_texture: optional_bool(body, "m_RequireDepthTexture").unwrap_or(false),
        require_opaque_texture: optional_bool(body, "m_RequireOpaqueTexture").unwrap_or(false),
        additional_lights_per_object_limit: optional_u32(body, "m_AdditionalLightsPerObjectLimit")
            .unwrap_or(4),
        shadow_distance: optional_f64(body, "m_ShadowDistance").unwrap_or(50.0),
    })
}

pub fn parse_urp_renderer_data_stream(
    stream: &Stream,
) -> Result<UrpRendererData, UrpPipelineError> {
    let body = pipeline_document_body(stream).ok_or(UrpPipelineError::MissingDocument)?;
    let renderer_feature_guids = body
        .get("m_RendererFeatures")
        .and_then(Node::as_sequence)
        .map_or_else(Vec::new, |entries| {
            entries.iter().filter_map(reference_guid).collect()
        });
    Ok(UrpRendererData {
        script_guid: script_guid(body),
        rendering_mode: optional_i64(body, "m_RenderingMode")
            .map(UrpRenderingMode::from_code)
            .unwrap_or(UrpRenderingMode::Forward),
        renderer_feature_guids,
    })
}

pub fn classify_pipeline_asset_stream(stream: &Stream) -> PipelineAssetKind {
    let Some(body) = pipeline_document_body(stream) else {
        return PipelineAssetKind::Other;
    };
    match script_guid(body).as_deref() {
        Some(URP_PIPELINE_ASSET_SCRIPT_GUID) => {
            return parse_urp_pipeline_asset_stream(stream).map_or(
                PipelineAssetKind::Other,
                PipelineAssetKind::UrpPipelineAsset,
            );
        }
        Some(URP_RENDERER_DATA_SCRIPT_GUID) => {
            return parse_urp_renderer_data_stream(stream)
                .map_or(PipelineAssetKind::Other, PipelineAssetKind::UrpRendererData);
        }
        _ => {}
    }
    if body.get("m_RendererDataList").is_some() {
        return parse_urp_pipeline_asset_stream(stream).map_or(
            PipelineAssetKind::Other,
            PipelineAssetKind::UrpPipelineAsset,
        );
    }
    if body.get("m_RenderingMode").is_some() && body.get("m_RendererFeatures").is_some() {
        return parse_urp_renderer_data_stream(stream)
            .map_or(PipelineAssetKind::Other, PipelineAssetKind::UrpRendererData);
    }
    PipelineAssetKind::Other
}

pub fn import_render_pipeline_asset(
    guid: &str,
    path: &Path,
) -> Result<ImportedRenderPipelineAsset, UrpPipelineError> {
    let stream = unity_yaml::parse_file(path).map_err(UrpPipelineError::Yaml)?;
    let kind = classify_pipeline_asset_stream(&stream);
    Ok(ImportedRenderPipelineAsset {
        guid: guid.to_string(),
        path: path.to_path_buf(),
        kind,
    })
}

fn pipeline_document_body(stream: &Stream) -> Option<&Node> {
    stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| document.root.get("MonoBehaviour"))
}

fn script_guid(body: &Node) -> Option<String> {
    body.get("m_Script")
        .and_then(|node| node.get("guid"))
        .and_then(Node::as_str)
        .map(str::trim)
        .filter(|guid| !guid.is_empty())
        .map(str::to_string)
}

fn reference_guid(node: &Node) -> Option<String> {
    node.get("guid")
        .and_then(Node::as_str)
        .map(str::trim)
        .filter(|guid| !guid.is_empty())
        .map(str::to_string)
}

fn optional_i64(body: &Node, key: &str) -> Option<i64> {
    body.get(key).and_then(Node::as_i64)
}

fn optional_u32(body: &Node, key: &str) -> Option<u32> {
    optional_i64(body, key).and_then(|value| u32::try_from(value).ok())
}

fn optional_f64(body: &Node, key: &str) -> Option<f64> {
    body.get(key).and_then(Node::as_f64)
}

fn optional_bool(body: &Node, key: &str) -> Option<bool> {
    optional_i64(body, key).map(|value| value != 0)
}
