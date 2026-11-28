use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_render::cube;
use unitas_runtime::{
    EffectiveRenderPipeline, PipelineSource, Runtime, UnsupportedRenderFeature,
    effective_render_pipeline, test_read_error_message, test_read_render_buffer,
    test_write_path_buffer, unitas_runtime_load_scene, unitas_runtime_new,
    unitas_runtime_path_buffer, unitas_runtime_render_buffer, unitas_runtime_render_camera_create,
    unitas_runtime_render_camera_update, unitas_runtime_render_frame,
    unitas_runtime_render_light_create, unitas_runtime_render_light_update,
    unitas_runtime_render_material_create, unitas_runtime_render_mesh_create,
    unitas_runtime_render_renderer_create, unitas_runtime_render_renderer_set_enabled,
};

const FRAME_SIZE: u32 = 64;
const BACKGROUND: [u8; 3] = [26, 51, 77];
const PATH_BUFFER_SIZE: u64 = 1_048_576;

const URP_ASSET_GUID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const RENDERER_DATA_GUID: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const FORWARD_ASSET_GUID: &str = "cccccccccccccccccccccccccccccccc";
const FORWARD_RENDERER_GUID: &str = "dddddddddddddddddddddddddddddddd";

const GRAPHICS_SETTINGS_YAML: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!30 &1
GraphicsSettings:
  m_ObjectHideFlags: 0
  serializedVersion: 15
  m_CustomRenderPipeline: {fileID: 11400000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 2}
  m_TransparencySortMode: 0
";

const QUALITY_SETTINGS_YAML: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!47 &1
QualitySettings:
  m_ObjectHideFlags: 0
  serializedVersion: 5
  m_CurrentQuality: 0
  m_QualitySettings:
  - serializedVersion: 4
    name: URP Deferred
    pixelLightCount: 4
    shadows: 2
    shadowResolution: 2
    shadowDistance: 50
    textureQuality: 0
    anisotropicTextures: 1
    antiAliasing: 0
    vSyncCount: 1
    lodBias: 1
    customRenderPipeline: {fileID: 0}
  - serializedVersion: 4
    name: URP Forward
    pixelLightCount: 4
    shadows: 2
    shadowResolution: 2
    shadowDistance: 50
    textureQuality: 0
    anisotropicTextures: 1
    antiAliasing: 0
    vSyncCount: 1
    lodBias: 1
    customRenderPipeline: {fileID: 11400000, guid: cccccccccccccccccccccccccccccccc, type: 2}
  m_PerPlatformDefaultQuality:
    Standalone: 0
";

const URP_ASSET_YAML: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!114 &11400000
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 0}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: bf2edee5c58d82540a51f03df9d42094, type: 3}
  m_Name: UniversalRP
  m_EditorClassIdentifier:
  k_AssetVersion: 11
  m_RendererDataList:
  - {fileID: 11400000, guid: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, type: 2}
  m_DefaultRendererIndex: 0
  m_RequireDepthTexture: 1
  m_RequireOpaqueTexture: 0
  m_SupportsHDR: 1
  m_MSAA: 4
  m_RenderScale: 1
  m_AdditionalLightsPerObjectLimit: 4
  m_ShadowDistance: 50
";

const RENDERER_DATA_YAML: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!114 &11400000
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 0}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: de640fe3d0db1804a85f9fc8f5cadab6, type: 3}
  m_Name: Renderer Data
  m_EditorClassIdentifier:
  m_RendererFeatures:
  - {fileID: 11400000, guid: 77777777777777777777777777777777, type: 2}
  m_RendererFeatureMap:
  m_AssetVersion: 2
  m_RenderingMode: 1
  m_DepthPrimingMode: 0
  m_CopyDepthMode: 1
  m_IntermediateTextureMode: 1
";

const FORWARD_ASSET_YAML: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!114 &11400000
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 0}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: bf2edee5c58d82540a51f03df9d42094, type: 3}
  m_Name: ForwardRP
  m_EditorClassIdentifier:
  k_AssetVersion: 11
  m_RendererDataList:
  - {fileID: 11400000, guid: dddddddddddddddddddddddddddddddd, type: 2}
  m_DefaultRendererIndex: 0
  m_RequireDepthTexture: 0
  m_RequireOpaqueTexture: 0
  m_SupportsHDR: 1
  m_MSAA: 1
  m_RenderScale: 1
  m_AdditionalLightsPerObjectLimit: 4
  m_ShadowDistance: 50
";

const FORWARD_RENDERER_YAML: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!114 &11400000
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 0}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: de640fe3d0db1804a85f9fc8f5cadab6, type: 3}
  m_Name: Forward Renderer
  m_EditorClassIdentifier:
  m_RendererFeatures: []
  m_RendererFeatureMap:
  m_AssetVersion: 2
  m_RenderingMode: 0
  m_DepthPrimingMode: 0
  m_CopyDepthMode: 1
  m_IntermediateTextureMode: 1
";

const URP_SCENE_YAML: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: MainCamera
--- !u!4 &1001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: -10}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!20 &1002
Camera:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  m_Enabled: 1
  m_BackGroundColor: {r: 0.1, g: 0.2, b: 0.3, a: 0}
  near clip plane: 0.3
  far clip plane: 1000
  field of view: 60
  orthographic: 0
  orthographic size: 5
--- !u!1 &2000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: OverlayCamera
--- !u!4 &2001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 2000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: -10}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!20 &2002
Camera:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 2000}
  m_Enabled: 1
  m_BackGroundColor: {r: 0.1, g: 0.2, b: 0.3, a: 0}
  near clip plane: 0.3
  far clip plane: 1000
  field of view: 60
  orthographic: 0
  orthographic size: 5
--- !u!114 &2003
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 2000}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: a79441f348de89743a2939f4d699eac1, type: 3}
  m_Name:
  m_EditorClassIdentifier:
  m_RenderType: 1
  m_CameraStack: []
";

const RENDER_SCENE_YAML: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: MainCamera
--- !u!4 &1001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: -10}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!20 &1002
Camera:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  m_Enabled: 1
  m_BackGroundColor: {r: 0.1, g: 0.2, b: 0.3, a: 0}
  near clip plane: 0.3
  far clip plane: 1000
  field of view: 60
  orthographic: 0
  orthographic size: 5
--- !u!1 &2000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: KeyLight
--- !u!4 &2001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 2000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 5, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!108 &2002
Light:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 2000}
  m_Enabled: 1
  m_Type: 1
  m_Color: {r: 1, g: 1, b: 1, a: 1}
  m_Intensity: 1
--- !u!1 &3000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: Cube
--- !u!4 &3001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 3000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
";

fn temp_project(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "unitas_urp_render_{name}_{}_{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join("ProjectSettings"))?;
    std::fs::create_dir_all(root.join("Assets").join("Settings"))?;
    std::fs::create_dir_all(root.join("Assets").join("Scenes"))?;
    std::fs::write(
        root.join("ProjectSettings/GraphicsSettings.asset"),
        GRAPHICS_SETTINGS_YAML,
    )?;
    std::fs::write(
        root.join("ProjectSettings/QualitySettings.asset"),
        QUALITY_SETTINGS_YAML,
    )?;
    write_asset(
        &root,
        "Settings/UniversalRP.asset",
        URP_ASSET_GUID,
        URP_ASSET_YAML,
    )?;
    write_asset(
        &root,
        "Settings/RendererData.asset",
        RENDERER_DATA_GUID,
        RENDERER_DATA_YAML,
    )?;
    write_asset(
        &root,
        "Settings/ForwardRP.asset",
        FORWARD_ASSET_GUID,
        FORWARD_ASSET_YAML,
    )?;
    write_asset(
        &root,
        "Settings/ForwardRenderer.asset",
        FORWARD_RENDERER_GUID,
        FORWARD_RENDERER_YAML,
    )?;
    Ok(root)
}

fn write_asset(root: &Path, relative: &str, guid: &str, yaml: &str) -> Result<(), Box<dyn Error>> {
    let path = root.join("Assets").join(relative);
    std::fs::write(&path, yaml)?;
    std::fs::write(
        path.with_extension("asset.meta"),
        format!("fileFormatVersion: 2\nguid: {guid}\n"),
    )?;
    Ok(())
}

fn write_scene(root: &Path, name: &str, yaml: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path = root.join("Assets").join("Scenes").join(name);
    std::fs::write(&path, yaml)?;
    Ok(path)
}

#[test]
fn urp_project_reports_unsupported_features_per_camera_and_quality_level()
-> Result<(), Box<dyn Error>> {
    let root = temp_project("report")?;
    let scene = write_scene(&root, "UrpScene.unity", URP_SCENE_YAML)?;

    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene)?;

    let report = runtime.render_pipeline_report();
    assert_eq!(report.levels.len(), 2);
    assert_eq!(report.levels[0].name, "URP Deferred");
    assert_eq!(report.levels[0].source, PipelineSource::GraphicsDefault);
    assert!(
        matches!(
            report.levels[0].pipeline,
            EffectiveRenderPipeline::Urp { .. }
        ),
        "the GraphicsSettings default URP asset must resolve for the level without an override"
    );
    assert_eq!(report.levels[1].name, "URP Forward");
    assert_eq!(report.levels[1].source, PipelineSource::QualityOverride);
    assert!(
        matches!(
            report.levels[1].pipeline,
            EffectiveRenderPipeline::Urp { .. }
        ),
        "the per-quality override must resolve to the Forward URP asset"
    );
    assert_eq!(
        runtime.render_pipeline_assets().len(),
        4,
        "the URP assets and both renderer data assets must import"
    );

    let has = |level: usize, camera: &str, feature: UnsupportedRenderFeature| {
        report.unsupported.iter().any(|entry| {
            entry.level_index == level && entry.camera == camera && entry.feature == feature
        })
    };
    for camera in ["MainCamera", "OverlayCamera"] {
        assert!(
            has(0, camera, UnsupportedRenderFeature::Msaa),
            "MSAA x4 on the deferred URP asset must be reported for {camera}"
        );
        assert!(
            has(0, camera, UnsupportedRenderFeature::DeferredRendering),
            "the Deferred renderer data must be reported for {camera}"
        );
        assert!(
            has(0, camera, UnsupportedRenderFeature::RendererFeatures),
            "the renderer feature reference must be reported for {camera}"
        );
        assert!(
            !has(1, camera, UnsupportedRenderFeature::Msaa),
            "the Forward asset disables MSAA"
        );
        assert!(
            !has(1, camera, UnsupportedRenderFeature::DeferredRendering),
            "the Forward renderer data renders through Forward"
        );
        assert!(
            !has(1, camera, UnsupportedRenderFeature::RendererFeatures),
            "the Forward renderer data has no renderer features"
        );
    }
    assert!(
        has(0, "OverlayCamera", UnsupportedRenderFeature::CameraStacking),
        "the overlay camera must be reported for camera stacking"
    );
    assert!(
        has(1, "OverlayCamera", UnsupportedRenderFeature::CameraStacking),
        "camera stacking stays reported on the Forward quality level"
    );
    assert!(
        !has(0, "MainCamera", UnsupportedRenderFeature::CameraStacking),
        "the base camera is not an overlay camera"
    );
    let deferred_entry = report
        .unsupported
        .iter()
        .find(|entry| {
            entry.level_index == 0
                && entry.camera == "MainCamera"
                && entry.feature == UnsupportedRenderFeature::DeferredRendering
        })
        .ok_or("expected a deferred-rendering report entry")?;
    assert_eq!(deferred_entry.level_name, "URP Deferred");

    let resolved = effective_render_pipeline(&runtime, 0);
    let EffectiveRenderPipeline::Urp { asset } = &resolved else {
        return Err(String::from("expected the URP pipeline for level 0").into());
    };
    assert_eq!(asset.msaa_sample_count, 4);
    assert_eq!(asset.additional_lights_per_object_limit, 4);
    assert!(asset.require_depth_texture);

    let _ = std::fs::remove_dir_all(&root);
    Ok(())
}

fn new_handle() -> Result<u64, Box<dyn Error>> {
    let handle = unitas_runtime_new();
    if handle == 0 {
        return Err(String::from("runtime handle creation failed").into());
    }
    Ok(handle)
}

fn path_buffer(handle: u64) -> Result<u64, Box<dyn Error>> {
    let address = unitas_runtime_path_buffer(handle, PATH_BUFFER_SIZE);
    if address == 0 {
        return Err(String::from("path buffer allocation failed").into());
    }
    Ok(address)
}

fn load_render_scene(handle: u64, path: &str) -> Result<(), Box<dyn Error>> {
    let address = path_buffer(handle)?;
    let bytes = path.as_bytes();
    test_write_path_buffer(handle, bytes)?;
    let code = unitas_runtime_load_scene(handle, address, u64::try_from(bytes.len())?);
    if code != 0 {
        let message = test_read_error_message(handle)?;
        return Err(format!("scene load failed with code {code}: {message}").into());
    }
    Ok(())
}

fn entity_id_for(path: &str, name: &str) -> Result<u64, Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(path))?;
    let graph = runtime
        .scene_graph()
        .ok_or_else(|| String::from("no scene graph found"))?;
    for id in graph.breadth_first_order() {
        if let Some(entity) = graph.entity(id)
            && entity.name == name
        {
            return Ok(id.as_u64());
        }
    }
    Err(format!("named entity {name} not found in scene").into())
}

fn cube_mesh_bytes() -> (Vec<u8>, u32, u32, u32) {
    let mesh = cube(1.0);
    let mut bytes = Vec::new();
    for vertex in &mesh.positions {
        for value in [vertex.x, vertex.y, vertex.z] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    let position_bytes = bytes.len() as u32;
    for normal in &mesh.normals {
        for value in [normal.x, normal.y, normal.z] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    let normal_bytes = bytes.len() as u32 - position_bytes;
    for index in &mesh.indices {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    let index_bytes = bytes.len() as u32 - position_bytes - normal_bytes;
    (bytes, position_bytes / 4, normal_bytes / 4, index_bytes / 4)
}

fn create_cube_mesh(handle: u64, base: u64) -> Result<u64, Box<dyn Error>> {
    let (bytes, position_count, normal_count, index_count) = cube_mesh_bytes();
    test_write_path_buffer(handle, &bytes)?;
    let mesh = unitas_runtime_render_mesh_create(
        handle,
        base as *const f32,
        position_count,
        (base + u64::from(position_count * 4)) as *const f32,
        normal_count,
        (base + u64::from((position_count + normal_count) * 4)) as *const u32,
        index_count,
    );
    Ok(mesh)
}

fn camera_blob_bytes() -> Vec<u8> {
    let values = [0.0f32, 60.0, 0.3, 1000.0, 0.1, 0.2, 0.3, 0.0];
    let mut bytes = Vec::new();
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn bmp_pixel(bytes: &[u8], x: u32, y_top_down: u32) -> [u8; 3] {
    let row = FRAME_SIZE - 1 - y_top_down;
    let offset = 54 + (row as usize) * (FRAME_SIZE as usize * 3) + (x as usize) * 3;
    [bytes[offset + 2], bytes[offset + 1], bytes[offset]]
}

fn lit_pixel_bounds(bytes: &[u8]) -> Option<(u32, u32, u32, u32)> {
    let mut min_x = FRAME_SIZE;
    let mut max_x = 0u32;
    let mut min_y = FRAME_SIZE;
    let mut max_y = 0u32;
    let mut found = false;
    for y in 0..FRAME_SIZE {
        for x in 0..FRAME_SIZE {
            if bmp_pixel(bytes, x, y) != BACKGROUND {
                found = true;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    if found {
        Some((min_x, max_x, min_y, max_y))
    } else {
        None
    }
}

#[test]
fn urp_forward_project_renders_scene_geometry_through_the_lit_path() -> Result<(), Box<dyn Error>> {
    let root = temp_project("frame")?;
    let scene = write_scene(&root, "RenderScene.unity", RENDER_SCENE_YAML)?;
    let scene_text = scene
        .to_str()
        .ok_or_else(|| String::from("non-utf8 temp path"))?
        .to_string();

    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene)?;
    let resolved = effective_render_pipeline(&runtime, 1);
    let EffectiveRenderPipeline::Urp { asset } = &resolved else {
        return Err(String::from("expected the Forward URP pipeline for level 1").into());
    };
    assert_eq!(asset.msaa_sample_count, 1);
    let report = runtime.render_pipeline_report();
    assert!(
        report
            .unsupported
            .iter()
            .all(|entry| entry.level_index != 1),
        "the URP Forward quality level reports no unsupported rendering features: {:?}",
        report.unsupported
    );

    let handle = new_handle()?;
    load_render_scene(handle, &scene_text)?;
    let base = path_buffer(handle)?;
    let mesh = create_cube_mesh(handle, base)?;
    assert_ne!(mesh, 0);
    let material = unitas_runtime_render_material_create(handle);
    assert_ne!(material, 0);
    let camera_entity = entity_id_for(&scene_text, "MainCamera")?;
    let cube_entity = entity_id_for(&scene_text, "Cube")?;
    let light_entity = entity_id_for(&scene_text, "KeyLight")?;
    let camera = unitas_runtime_render_camera_create(handle, camera_entity);
    assert_ne!(camera, 0);
    let blob_address = path_buffer(handle)?;
    test_write_path_buffer(handle, &camera_blob_bytes())?;
    assert_eq!(
        unitas_runtime_render_camera_update(handle, camera, blob_address as *const f32),
        0
    );
    let renderer = unitas_runtime_render_renderer_create(handle, cube_entity, mesh, material);
    assert_ne!(renderer, 0);
    assert_eq!(
        unitas_runtime_render_renderer_set_enabled(handle, renderer, 1),
        0
    );
    let light = unitas_runtime_render_light_create(handle, light_entity);
    assert_ne!(light, 0);
    assert_eq!(
        unitas_runtime_render_light_update(handle, light, 0.0, 0.3, -1.0, 1.0, 1.0, 1.0, 1.0),
        0
    );
    let count = unitas_runtime_render_frame(handle, camera, FRAME_SIZE, FRAME_SIZE);
    assert!(count >= 0, "render frame must succeed: {count}");
    let buffer_address = unitas_runtime_render_buffer(handle, u64::try_from(count)?);
    assert_ne!(buffer_address, 0);
    let mut bytes = vec![0u8; usize::try_from(count)?];
    test_read_render_buffer(handle, &mut bytes)?;
    assert_eq!(&bytes[0..2], b"BM");
    let width = u32::from_le_bytes(bytes[18..22].try_into()?);
    let height = u32::from_le_bytes(bytes[22..26].try_into()?);
    assert_eq!(width, FRAME_SIZE);
    assert_eq!(height, FRAME_SIZE);
    assert!(
        lit_pixel_bounds(&bytes).is_some(),
        "the URP Forward project must render scene geometry through the lit path"
    );

    let _ = std::fs::remove_dir_all(&root);
    Ok(())
}
