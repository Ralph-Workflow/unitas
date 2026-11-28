use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::project_settings::{
    parse_graphics_settings_stream, parse_quality_settings_stream,
};
use unitas_asset::unity_yaml;
use unitas_asset::urp_pipeline::{
    classify_pipeline_asset_stream, import_render_pipeline_asset, parse_urp_pipeline_asset,
    parse_urp_pipeline_asset_stream, parse_urp_renderer_data, parse_urp_renderer_data_stream,
    PipelineAssetKind, UrpRenderingMode,
};

const URP_ASSET_YAML: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!114 &11400000
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_CorrespondingSourceObject: {fileID: 0}
  m_PrefabInstance: {fileID: 0}
  m_PrefabAsset: {fileID: 0}
  m_GameObject: {fileID: 0}
  m_Enabled: 1
  m_EditorHideFlags: 0
  m_Script: {fileID: 11500000, guid: bf2edee5c58d82540a51f03df9d42094, type: 3}
  m_Name: UniversalRP
  m_EditorClassIdentifier:
  k_AssetVersion: 11
  k_AssetPreviousVersion: 11
  m_RendererType: 1
  m_RendererData: {fileID: 0}
  m_RendererDataList:
  - {fileID: 11400000, guid: 1234567890abcdef1234567890abcdef, type: 2}
  - {fileID: 11400000, guid: 00000000000000000000000000000000, type: 2}
  m_DefaultRendererIndex: 0
  m_RequireDepthTexture: 1
  m_RequireOpaqueTexture: 0
  m_OpaqueDownsampling: 1
  m_SupportsTerrainHoles: 1
  m_SupportsHDR: 1
  m_HDRColorBufferPrecision: 0
  m_MSAA: 4
  m_RenderScale: 1.25
  m_UpscalingFilter: 0
  m_MainLightRenderingMode: 1
  m_MainLightShadowsSupported: 1
  m_MainLightShadowmapResolution: 4096
  m_AdditionalLightsRenderingMode: 1
  m_AdditionalLightsPerObjectLimit: 4
  m_AdditionalLightShadowsSupported: 1
  m_AdditionalLightShadowmapResolution: 2048
  m_ShadowDistance: 50
  m_ShadowCascadeCount: 4
  m_SoftShadowsSupported: 1
";

const URP_RENDERER_DATA_YAML: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!114 &11400000
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_CorrespondingSourceObject: {fileID: 0}
  m_PrefabInstance: {fileID: 0}
  m_PrefabAsset: {fileID: 0}
  m_GameObject: {fileID: 0}
  m_Enabled: 1
  m_EditorHideFlags: 0
  m_Script: {fileID: 11500000, guid: de640fe3d0db1804a85f9fc8f5cadab6, type: 3}
  m_Name: New Universal Renderer Data
  m_EditorClassIdentifier:
  m_RendererFeatures:
  - {fileID: 11400000, guid: 77777777777777777777777777777777, type: 2}
  m_RendererFeatureMap:
  m_UseNativeRenderPass: 0
  m_AssetVersion: 2
  m_OpaqueLayerMask:
    serializedVersion: 2
    m_Bits: 4294967295
  m_TransparentLayerMask:
    serializedVersion: 2
    m_Bits: 4294967295
  m_RenderingMode: 1
  m_DepthPrimingMode: 0
  m_CopyDepthMode: 1
  m_AccurateGbufferNormals: 0
  m_IntermediateTextureMode: 1
";

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!(
        "unitas_urp_pipeline_{name}_{}_{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

#[test]
fn urp_asset_and_renderer_data_parse_from_urp_14_serialized_yaml() -> Result<(), Box<dyn Error>> {
    let asset_stream = unity_yaml::parse(URP_ASSET_YAML)?;
    let asset = parse_urp_pipeline_asset_stream(&asset_stream)?;
    assert_eq!(
        asset.script_guid.as_deref(),
        Some("bf2edee5c58d82540a51f03df9d42094")
    );
    assert_eq!(asset.default_renderer_index, 0);
    assert_eq!(
        asset.renderer_data_guids,
        vec![
            Some(String::from("1234567890abcdef1234567890abcdef")),
            Some(String::from("00000000000000000000000000000000")),
        ]
    );
    assert!(asset.supports_hdr);
    assert_eq!(asset.msaa_sample_count, 4);
    assert!((asset.render_scale - 1.25).abs() < 1e-9);
    assert!(asset.require_depth_texture);
    assert!(!asset.require_opaque_texture);
    assert_eq!(asset.additional_lights_per_object_limit, 4);
    assert!((asset.shadow_distance - 50.0).abs() < 1e-9);

    let renderer_stream = unity_yaml::parse(URP_RENDERER_DATA_YAML)?;
    let renderer = parse_urp_renderer_data_stream(&renderer_stream)?;
    assert_eq!(
        renderer.script_guid.as_deref(),
        Some("de640fe3d0db1804a85f9fc8f5cadab6")
    );
    assert_eq!(renderer.rendering_mode, UrpRenderingMode::Deferred);
    assert_eq!(
        renderer.renderer_feature_guids,
        vec![String::from("77777777777777777777777777777777")]
    );
    assert_eq!(UrpRenderingMode::ForwardPlus.code(), 2);
    assert_eq!(UrpRenderingMode::Deferred.name(), "Deferred");

    assert!(matches!(
        classify_pipeline_asset_stream(&asset_stream),
        PipelineAssetKind::UrpPipelineAsset(_)
    ));
    assert!(matches!(
        classify_pipeline_asset_stream(&renderer_stream),
        PipelineAssetKind::UrpRendererData(_)
    ));
    Ok(())
}

#[test]
fn unknown_pipeline_assets_classify_as_other_without_urp_claims() -> Result<(), Box<dyn Error>> {
    let hdrp_like = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!114 &11400000
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_Script: {fileID: 11500000, guid: f971995892b44a5d867fb9d0da7c8a26, type: 3}
  m_Name: HDRenderPipelineAsset
";
    let stream = unity_yaml::parse(hdrp_like)?;
    assert_eq!(
        classify_pipeline_asset_stream(&stream),
        PipelineAssetKind::Other
    );
    Ok(())
}

#[test]
fn quality_level_custom_render_pipeline_overrides_the_graphics_default(
) -> Result<(), Box<dyn Error>> {
    let graphics_yaml = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!30 &1
GraphicsSettings:
  m_ObjectHideFlags: 0
  serializedVersion: 15
  m_CustomRenderPipeline: {fileID: 11400000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 2}
  m_TransparencySortMode: 0
";
    let quality_yaml = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!47 &1
QualitySettings:
  m_ObjectHideFlags: 0
  serializedVersion: 5
  m_CurrentQuality: 0
  m_QualitySettings:
  - serializedVersion: 4
    name: Performant
    pixelLightCount: 1
    shadows: 0
    shadowResolution: 0
    shadowDistance: 40
    textureQuality: 0
    anisotropicTextures: 0
    antiAliasing: 0
    vSyncCount: 0
    lodBias: 0.7
    customRenderPipeline: {fileID: 11400000, guid: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, type: 2}
  - serializedVersion: 4
    name: Balanced
    pixelLightCount: 2
    shadows: 1
    shadowResolution: 1
    shadowDistance: 60
    textureQuality: 0
    anisotropicTextures: 1
    antiAliasing: 0
    vSyncCount: 0
    lodBias: 1
    customRenderPipeline: {fileID: 0}
  m_TextureMipmapLimitSettings: []
  m_PerPlatformDefaultQuality:
    Standalone: 0
";
    let graphics = parse_graphics_settings_stream(&unity_yaml::parse(graphics_yaml)?)?;
    assert_eq!(
        graphics.custom_render_pipeline_guid(),
        Some(String::from("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))
    );
    let quality = parse_quality_settings_stream(&unity_yaml::parse(quality_yaml)?)?;
    assert_eq!(
        quality.levels[0].custom_render_pipeline_guid.as_deref(),
        Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
        "the per-quality pipeline override must import"
    );
    assert_eq!(
        quality.levels[1].custom_render_pipeline_guid, None,
        "an empty per-quality reference means the GraphicsSettings default applies"
    );
    Ok(())
}

#[test]
fn import_render_pipeline_asset_classifies_by_script_guid_on_disk() -> Result<(), Box<dyn Error>> {
    let dir = temp_dir("import")?;
    let asset_path = dir.join("UniversalRP.asset");
    fs::write(&asset_path, URP_ASSET_YAML)?;
    let imported = import_render_pipeline_asset("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", &asset_path)?;
    assert_eq!(imported.guid, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    assert_eq!(imported.path, asset_path);
    let PipelineAssetKind::UrpPipelineAsset(asset) = &imported.kind else {
        return Err(String::from("expected a URP pipeline asset classification").into());
    };
    assert_eq!(asset.msaa_sample_count, 4);

    assert!(parse_urp_pipeline_asset(&dir.join("missing.asset"))?.is_none());
    assert!(parse_urp_renderer_data(&dir.join("missing.asset"))?.is_none());

    let renderer_path = dir.join("RendererData.asset");
    fs::write(&renderer_path, URP_RENDERER_DATA_YAML)?;
    let renderer = parse_urp_renderer_data(&renderer_path)?;
    let Some(renderer) = renderer else {
        return Err(String::from("expected renderer data to parse from disk").into());
    };
    assert_eq!(renderer.rendering_mode, UrpRenderingMode::Deferred);

    let _ = fs::remove_dir_all(&dir);
    Ok(())
}
