# Rendering, Cameras, Lighting, and Animation

## Scope

This specification defines the visible behavior of cameras, geometry,
materials, shaders, render pipelines, scripted rendering, lighting, shadows,
textures, render targets, 2D rendering, frame presentation, animation,
particles, and cinematic sequencing. It describes observable output and
diagnostics, not a rendering architecture.

These are desired end-state requirements, not claims about currently shipped
capability. The project's Unity release, from Unity 2020.3 LTS through Unity
2022.3 LTS, is the behavioral baseline, with Unity 2022.3 LTS as the principal
reference, as defined in the [Compatibility contract](compatibility-contract.md).
URP, Shader Graph, Post Processing (v2), the 2D packages, Timeline,
Cinemachine, and other package behavior is covered only for the package
families and version lines listed in the supported package matrix (CMP-002,
owned by the [Compatibility contract](compatibility-contract.md)).

In this document, "within scope" means the render pipelines named in
[Render pipeline scope](#render-pipeline-scope) combined with the package
versions that the supported package matrix lists for the project's baseline
editor release. Behavior outside that scope carries one of the compatibility
states defined in the [Compatibility contract](compatibility-contract.md).

Every capability in this document, including inspection, preview, capture, and
the diagnostics below, is available to authorized agents through automation as
defined in [Automation and MCP](automation-and-mcp.md), in the visual editor,
and in headless sessions.

## Graphics API and target capability

The product does not require a particular graphics API or API version for
rendering. Direct3D, OpenGL, Vulkan, and Metal are possible implementation
choices, not interchangeable promises of Unity compatibility. In particular,
Linux and macOS support does not imply that Direct3D runs on those systems.
ShaderLab and HLSL support describes accepted project content, not the API used
to draw it.

For each supported build target and current host, developers can inspect the
selected graphics API and version, the device and driver where the host
reports them, and which rendering features named in this document are
Supported or Platform-limited there. If no usable graphics device or API is
available, startup or build fails with an actionable reason; it does not
silently claim a working renderer. A change of graphics API does not silently
change the specified visible behavior of a supported scene. Target-specific
visual differences or unsupported shader features are reported as limitations
under the compatibility contract.

## Render pipeline scope

A project renders with the Built-in Render Pipeline or with the Universal
Render Pipeline (URP). The active pipeline is determined by the Graphics
settings render pipeline asset and the per-quality-level override, as in
Unity; the effective pipeline asset for each quality level and target is
inspectable.

* **Built-in Render Pipeline:** forward and deferred rendering paths, the
  legacy vertex-lit path where the Camera or Graphics settings select it,
  camera rendering path overrides, and Built-in quality settings are
  Supported.
* **URP:** the URP 10.x line on Unity 2020.3, 12.x on Unity 2021.3, and 14.x on
  Unity 2022.3, as listed in the supported package matrix. The Forward
  rendering path is Supported on every listed line. The Deferred rendering path
  is Supported on the URP lines that offer it (12.x and 14.x). The Forward+
  rendering path is Supported on URP 14.x. Universal Renderer data, 2D
  Renderer data, renderer lists, camera stacking (base and overlay cameras),
  render scale, HDR, MSAA, depth and opaque texture options, and per-camera
  renderer selection retain their Unity meaning.
* **High Definition Render Pipeline (HDRP):** HDRP assets, materials, volumes,
  and settings are recognized and reported as Unsupported, as listed in the
  [index](INDEX.md). Objects using HDRP materials remain present and
  inspectable and render as an unmistakable failed-shader result.
* **Custom scriptable render pipelines:** a project whose render pipeline asset
  is a project-defined `RenderPipelineAsset` subclass, rather than URP, is
  reported as Unsupported. The rest of the project remains usable for
  inspection, editing, simulation, and non-rendering tests.

A render pipeline asset whose package is absent or whose version is outside the
matrix is reported with the affected quality levels and cameras rather than
silently falling back to another pipeline.

## Cameras

A Camera renders selected scene content to a game display or render target.
It exposes projection, field of view or orthographic size, clipping range,
viewport, depth, background and clear behavior, culling mask, target texture,
target display, enabled state, and quality choices.

### Required behavior

* Perspective and orthographic projection agree with scene-view previews and
  coordinate conversion APIs.
* World-to-screen, screen-to-world, viewport, and ray conversions correspond to
  the actual rendered image.
* Multiple cameras contribute in `Camera.depth` order (Built-in) or stack order
  (URP base and overlay cameras) and according to their viewport and clear
  behavior.
* `Camera.main` returns the first enabled camera tagged `MainCamera`, as in
  Unity, and returns null when there is none.
* A disabled, culled, targetless, or obscured camera has an inspectable reason
  for producing no visible output.
* Aspect changes, safe areas, dynamic resolution on targets that report support
  for it, and display selection have explicit effects.
* Render targets display the camera result implied by their dimensions,
  format, color space, and update behavior.
* `Camera.Render` and manual camera rendering produce the same image as the
  automatic render of the same camera state.

### Camera callbacks and depth textures

* On the Built-in Render Pipeline, `OnPreCull`, `OnPreRender`, `OnPostRender`,
  and `OnRenderImage` on scripts attached to a camera, and the
  `Camera.onPreCull`, `Camera.onPreRender`, and `Camera.onPostRender`
  delegates, run at Unity's points in that camera's render. `OnRenderImage`
  receives the source and destination render textures and its output becomes
  the camera's image.
* `Camera.depthTextureMode` (Depth, DepthNormals, and MotionVectors) makes
  `_CameraDepthTexture`, `_CameraDepthNormalsTexture`, and
  `_CameraMotionVectorsTexture` available to shaders as in Unity.
* On URP, these Built-in camera callbacks are not invoked, as in Unity. The
  project compatibility report names every script that defines them in a URP
  project, with the consequence that they do not run.
* On URP, `RenderPipelineManager.beginFrameRendering`, `endFrameRendering`,
  `beginCameraRendering`, `endCameraRendering`, and, in releases that define
  them, `beginContextRendering` and `endContextRendering` fire in Unity order.

## Mesh and sprite rendering

Static meshes, skinned meshes, sprites, tilemaps, lines, trails, and particle
renderers participate in camera culling, transforms, sorting, layers,
materials, lighting, and shadows according to their feature contracts.

A renderer exposes its source geometry, material slots, bounds, enablement,
sorting state, shadow participation, and visibility reason. Missing geometry,
invalid bounds, absent materials, unsupported topology, or target limitations
are distinct diagnostics.

Skinned meshes reflect bone transforms, bind poses, weights, blend shapes, and
animation. Negative or non-uniform transforms produce defined results. A
renderer does not appear compatible merely because a fallback shape is visible.

### Runtime meshes and mesh instances

* Meshes created or modified at runtime through the `Mesh` API (vertices,
  indices, submeshes, normals, tangents, UVs, colors, bounds recalculation,
  and `MarkDynamic`) render the data the script supplied.
* `Mesh.indexFormat` selects 16-bit or 32-bit indices; a 16-bit mesh with more
  than 65,535 vertices, or an index outside the vertex range, fails with
  Unity's compatible error naming the mesh.
* `Mesh.CombineMeshes`, `SetVertexBufferParams`, `SetVertexBufferData`,
  `SetIndexBufferParams`, `SetIndexBufferData`, `SetSubMesh`, and the
  `Mesh.MeshData` APIs (`AcquireReadOnlyMeshData`, `AllocateWritableMeshData`,
  and `ApplyAndDisposeWritableMeshData`) produce the same mesh as in Unity.
* Reading mesh data that was imported without Read/Write enabled fails in a
  player with a compatible error naming the mesh.
* `MeshFilter.mesh` creates a per-renderer copy of the shared mesh on first
  access, and `MeshFilter.sharedMesh` reads and edits the asset. Copies live
  until destroyed or released by `Resources.UnloadUnusedAssets`, and appear
  in residency inspection as runtime instances distinct from the asset.

## Materials

Materials retain their shader relationship and supported textures, colors,
numbers, vectors, keywords, render queue, surface mode, transparency, alpha
clipping, culling, depth behavior, tiling, and offsets.

The product supports the observable behavior of the Built-in shaders listed in
[Built-in shaders](#built-in-shaders) and the URP Lit, Simple Lit, Unlit,
Baked Lit, and 2D sprite materials of the listed URP lines. Metallic,
smoothness, normal, occlusion, emission, transparency, and alpha-clipping
controls have Unity's visible meaning within scope.

Editing a shared material affects every renderer using that asset.
`Renderer.material` and `Renderer.materials` create per-renderer copies on
first access, while `Renderer.sharedMaterial` and `Renderer.sharedMaterials`
read and edit the asset. Material copies live until destroyed or released by
`Resources.UnloadUnusedAssets`, and appear in residency inspection as runtime
instances. Missing properties never silently accept values that have no
visible effect.

A `MaterialPropertyBlock` applied to a renderer overrides the named properties
for that renderer only, without creating a material instance and without
changing the shared material. The effective value of each property on a
renderer, and whether it comes from the material or a property block, is
inspectable.

## Color space

The Player color space setting (Gamma or Linear), reported at runtime through
`QualitySettings.activeColorSpace`, governs sRGB texture sampling, lighting
and blending math, render texture sRGB read and write, and UI and vertex color
conversion as in Unity. Texture import sRGB flags and `RenderTexture`
read-write modes keep their meaning in both color spaces. A target whose
graphics API cannot provide Linear rendering reports Linear color space as
Platform-limited, naming the target and graphics API; it does not silently
render in Gamma.

## Textures at runtime

Textures expose dimensions, format, mipmap count, filter mode, wrap mode,
anisotropy, color space interpretation, readability, and residency.

* `Texture2D.SetPixel`, `SetPixels`, `SetPixels32`, and `SetPixelData` change
  CPU-side texture data; the visible texture changes only after `Apply`, with
  mipmap regeneration and the `makeNoLongerReadable` choice behaving as in
  Unity.
* `GetPixel`, `GetPixels`, `GetPixels32`, `GetPixelData`, and
  `GetRawTextureData` return the stored texel values. Reading a texture whose
  import settings leave it non-readable fails with a compatible error naming
  the texture.
* `Texture2D.ReadPixels` copies from the active render target into the texture
  at the requested rectangle and offset; the result becomes visible after
  `Apply`.
* `ImageConversion.LoadImage` decodes PNG and JPG data, and `EncodeToPNG`,
  `EncodeToJPG`, `EncodeToTGA`, and `EncodeToEXR` produce files whose decoded
  texels match the source exactly for lossless formats and within the
  requested quality for JPG.
* `Cubemap`, `Texture2DArray`, `Texture3D`, `CubemapArray`, and
  `CustomRenderTexture` (with its initialization, update mode, update zones,
  and double buffering) are created, written, and sampled as in Unity.
* `Sprite.Create` produces a sprite with the given rectangle, pivot,
  pixels-per-unit, mesh type, and border.
* Mipmap streaming honors the quality settings memory budget, per-texture
  priority, and `Texture2D.requestedMipmapLevel`; the loaded and desired mip
  levels of each streamed texture are inspectable.
* `AsyncGPUReadback.Request` completes on a later frame with `done`, `hasError`,
  and `GetData` behaving as in Unity; a target without readback support
  reports the request as Platform-limited and completes it with an error.
* `RenderTexture` creation, `Create`, `Release`, `IsCreated`,
  `RenderTexture.active`, temporary render textures (`GetTemporary` and
  `ReleaseTemporary`), depth buffers, MSAA, and random-write enablement behave
  as in Unity. A format that the target cannot create is reported as
  Platform-limited with the format and target named; it is not silently
  replaced with a different format.
* `WebCamTexture` plays the selected camera device after the target's camera
  permission is granted. A denied permission or an absent camera returns an
  explicit status and an empty `WebCamTexture.devices` list or a texture that
  reports it is not playing; targets without camera access report
  `WebCamTexture` as Platform-limited.
* Texture formats and compression that a target does not support are reported
  at import or build with the asset and target named, together with the
  fallback format Unity would use where one applies.

## Shaders

Supported ShaderLab and HLSL assets retain their shader name, properties,
subshaders, passes, tags, render states, variants, and keywords.

Unsupported syntax or behavior identifies the shader, pass, variant, property,
and target where known. A failed shader is unmistakable in the scene and
diagnostics. It never renders a plausible substitute while being reported as
fully correct.

Shader selection respects material keywords, global keywords, project quality,
renderer state, camera context, pipeline tags, `Shader.globalMaximumLOD` and
per-shader LOD, and target capability as in Unity. Variant absence is distinct
from missing source or invalid material data. Shader variant stripping at
build time reports which variants were excluded and why.

### ShaderLab and HLSL content

The following ShaderLab and shader-program forms are Supported:

* Built-in surface shaders (`#pragma surface`) with the Standard,
  StandardSpecular, Lambert, BlinnPhong, and custom lighting models, their
  options, and the forward, deferred, shadow-caster, and meta passes that Unity
  generates from them.
* `CGPROGRAM` and `HLSLPROGRAM` blocks, vertex, fragment, geometry, and
  tessellation programs on targets whose graphics API offers those stages,
  and `#pragma target` levels with the target-capability check Unity applies.
* Built-in includes (`UnityCG.cginc`, `Lighting.cginc`, `AutoLight.cginc`,
  `UnityShaderVariables.cginc`, and the other Built-in shader includes) and the
  URP ShaderLibrary of each listed URP line.
* `multi_compile`, `shader_feature`, their `_local` forms, and their
  vertex-only and fragment-only variants.
* `Fallback`, `UsePass`, `GrabPass`, `Category`, `LOD`, and fixed-function
  render state (blend, blend operation, color mask, cull, depth test and
  write, offset, stencil, and alpha-to-mask).

A construct that the active pipeline ignores in Unity, such as `GrabPass` or a
Built-in-only `LightMode` pass under URP, is ignored in the same way and is
reported with that consequence for the shader and pass.

### Built-in shaders

The Built-in shader families Standard, Standard (Specular setup), Autodesk
Interactive, `Legacy Shaders/*`, `Mobile/*`, `Unlit/*`, `Particles/*`,
`Sprites/*`, `UI/*`, `Skybox/*`, and `Nature/*` are Supported with Unity's
visible results on the Built-in Render Pipeline, and with Unity's behavior
under URP for the ones URP renders.

`Shader.Find` returns the shader in the editor and in a player when that shader
is referenced by content included in the build or listed in the Graphics
settings Always Included Shaders. Otherwise it returns null in a player, as in
Unity. The build report lists the shaders reachable by `Shader.Find`.

### Shader globals, keywords, and warm-up

* `Shader.SetGlobalFloat`, `SetGlobalInt`, `SetGlobalVector`,
  `SetGlobalColor`, `SetGlobalMatrix`, `SetGlobalTexture`, `SetGlobalBuffer`,
  and their array forms set values that every shader not overriding the
  property observes. `Shader.PropertyToID` returns a stable identifier for the
  play session.
* `Shader.EnableKeyword`, `Shader.DisableKeyword`, `Material.EnableKeyword`,
  `Material.DisableKeyword`, `Material.shaderKeywords`, and, in releases that
  define them, `LocalKeyword`, `GlobalKeyword`, and `Material.SetKeyword`
  select variants as in Unity.
* Exceeding the keyword limit of the project's release produces Unity's
  compatible error naming the shader and keyword.
* `ShaderVariantCollection.WarmUp` and `Shader.WarmupAllShaders` complete the
  preparation of the listed variants before returning, so the first draw that
  uses them does not incur that preparation.

## Shader Graph

Shader Graph assets of the versions that the supported package matrix lists are
treated as shader content for the URP and Built-in targets that their graph
settings select. A Shader Graph asset retains its exposed properties,
keywords, sub graph references, graph target, surface options, and material
interface, and a material using it behaves as a material using an equivalent
hand-written shader.

* Sub graphs, keywords (boolean and enum), exposed and hidden properties,
  custom interpolators where the listed version offers them, and Custom
  Function nodes with inline or file-based HLSL are Supported.
* Excluded, and reported as Unsupported: graph targets for HDRP and for Visual
  Effect Graph, as listed in the [index](INDEX.md).
* Excluded per graph and node: a graph that uses a node unitas cannot
  represent is reported with the graph asset, the node type, the node identity
  within the graph, and every material affected. The graph is not reported as
  Supported, and the material renders as a failed shader.
* Shader Graph authoring user interface is not required; graphs are accepted
  as project content and their properties are editable through materials.

## Scripted rendering

Projects can extend rendering with scripts. Scripted rendering obeys the same
diagnostics and failure rules as built-in rendering: a command that cannot run
on the target reports the command, the script location, and the target.

* **URP renderer features:** `ScriptableRendererFeature` and
  `ScriptableRenderPass` subclasses added to a Universal Renderer or 2D
  Renderer data asset run at their `RenderPassEvent`, with their configured
  inputs, render targets, and clear behavior. Disabled features do not run. A
  feature that references a missing script is reported against the renderer
  data asset.
* **Decals:** the URP Decal Renderer Feature and `DecalProjector` are Supported
  on URP 12.x and 14.x with their technique (DBuffer or screen space),
  projection size, pivot, fade, and decal layers. The Built-in Render Pipeline
  has no decal system, as in Unity.
* **Command buffers:** `CommandBuffer` contents execute in recorded order
  through `Graphics.ExecuteCommandBuffer`, through camera and light events on
  the Built-in Render Pipeline (`Camera.AddCommandBuffer`,
  `Light.AddCommandBuffer`), and through `ScriptableRenderContext` in renderer
  passes. Command buffers attached to a camera are inspectable on that camera.
* **Immediate drawing:** `Graphics.Blit`, `Graphics.DrawMesh`,
  `Graphics.DrawMeshInstanced`, `Graphics.DrawMeshInstancedIndirect`,
  `Graphics.DrawProcedural`, and `Graphics.CopyTexture` produce the same
  visible result as in Unity for the active pipeline, including camera
  selection, layer, and material property block arguments.
* **GPU instancing:** materials with instancing enabled render many renderers
  that share mesh and material as instances, with per-instance properties
  from `MaterialPropertyBlock`. Instanced and non-instanced rendering of the
  same scene produce the same image. Whether instancing actually occurred is
  a reported performance property.
* **Compute shaders:** `ComputeShader` assets, kernels, `Dispatch`,
  `ComputeBuffer` and `GraphicsBuffer` data, and random-write textures are
  Supported on targets whose graphics API offers compute. On a target without
  compute, `SystemInfo.supportsComputeShaders` returns false, compute usage is
  reported as Platform-limited with the asset and target named, and dispatch
  fails with a compatible error instead of silently doing nothing.
* **SRP Batcher and batching:** whether the SRP Batcher, static batching, or
  dynamic batching combined draws is a performance property reported through
  profiling ([Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md));
  it never changes the visible result and is never a compatibility failure.

## Lighting and environment

Directional, point, and spot lights expose color, intensity, range or angle,
enablement, culling mask, shadow choice, render mode, and quality settings.
The visible result respects object transforms and material response.

* Light cookies, Built-in light halos and the `Halo` component, and Built-in
  `Flare` assets with a camera Flare Layer render as in Unity. On URP 14.x the
  Lens Flare (SRP) component renders its lens flare data asset.
* On the Built-in Render Pipeline, the Important and Not Important light render
  modes and the quality pixel light count decide per-pixel or per-vertex
  lighting as in Unity. On URP, the main light, the additional lights per
  object limit, and per-vertex or per-pixel additional lights follow the URP
  asset.
* `Light.cullingMask` excludes layers from a light. URP light layers (12.x)
  and rendering layers (14.x) restrict which renderers a light affects.
* Area lights are Supported as baked-only lights, contributing through
  Unity-baked lighting data.
* Enlighten realtime global illumination data is Unsupported; baked lighting
  data remains Supported as defined below.

Environment lighting includes skybox materials (`Skybox/Procedural`,
`Skybox/6 Sided`, `Skybox/Cubemap`, and `Skybox/Panoramic`), ambient source
(skybox, gradient, or color) and intensity, environment reflection source,
resolution, intensity, and bounces, fog, and exposure behavior. Changing
`RenderSettings` at runtime and calling `DynamicGI.UpdateEnvironment` updates
ambient and reflection lighting as in Unity. A project can inspect which
environment settings affect a camera and scene. Baked, mixed, and realtime
lighting modes are identified accurately rather than treated as
interchangeable.

Light limits, target limitations, and quality reductions are visible in
diagnostics. A light ignored because of a layer, range, disabled state, or limit
can be distinguished from an invalid light.

## Shadows

Eligible renderers cast and receive shadows according to material, renderer,
light, layer, distance, and quality settings. Directional, point, and spot
shadow behavior follows the active pipeline: Built-in and URP shadow casting
for every light type that the pipeline line supports.

Shadow bias, normal bias, strength, softness, distance, resolution, and cascade
choices have visible and inspectable effects. Missing shadows identify whether
the cause is caster state, receiver material, light state, culling, quality,
target support, or resource limits.

## Quality levels

`QualitySettings.SetQualityLevel(level, applyExpensiveChanges)` switches the
active quality level with Unity's immediate and deferred effects: settings that
Unity applies only when `applyExpensiveChanges` is true (such as anti-aliasing
changes that recreate render targets) wait in the same way.

Each quality level's pixel light count, texture quality and mipmap limit,
anisotropic textures, anti-aliasing, soft particles, realtime reflection
probes, shadow settings, LOD bias, maximum LOD level, skin weights, and render
pipeline asset take effect as in Unity. A setting that the active target or
pipeline ignores in Unity is ignored in the same way and reported with the
quality level and target.

## Baked lighting data, probes, fog, LOD, and occlusion

Lightmaps, light probes, reflection probe captures, and occlusion culling data
baked by Unity are Supported project content. Rendering uses them with their
renderer assignments, lightmap indices and scale-offsets, contribution modes
(contribute global illumination, receive from lightmaps or light probes),
mixed lighting modes (Baked Indirect, Shadowmask, Subtractive), shadow masks,
and the visible relationship between baked and realtime lights. Light probe
groups expose their probe positions and interpolation; reflection probes
expose their influence volume, blending, box projection, importance, and
captured result. Realtime reflection probes update according to their refresh
mode and time-slicing settings.

Baking lightmaps and occlusion culling data inside unitas is Unsupported, as
listed in the [index](INDEX.md). A request to generate lighting or bake
occlusion is recognized and reports that state.

When an edit invalidates baked data, for example moving or changing a static
renderer that contributes to global illumination, changing a baked light, or
changing an occluder, the affected baked data is marked stale. The stale
state names the affected lighting data asset, occlusion data, and every
renderer whose baked result no longer matches. Stale data continues to render
as it was baked until it is replaced; it is not silently discarded. A missing
bake, a stale bake, and baked data in a form that unitas cannot read remain
three distinct states.

Fog retains its mode, color, density, and range settings and its camera
participation. Level-of-detail groups expose renderer sets, transitions,
cross-fade state, bounds, and current level. Occlusion status is
distinguishable from layer or frustum culling, LOD, disabled state, and missing
geometry.

## Post-processing

Post-processing is Supported through the Post Processing (v2) package 3.x on the
Built-in Render Pipeline and through URP Volumes on the listed URP lines. Each
system retains its own content: PPv2 Post-process Layer and Post-process Volume
components with profiles, and URP Volume components with Volume Profiles.

Volumes retain global or local mode, collider-defined extent, priority, blend
distance, weight, and the layer mask or camera participation that selects them.
The blended value of every overridden parameter for a given camera and position
is inspectable, together with the volumes that contributed to it.

The following effects are Supported in both systems with their Unity
parameters, except where noted:

* Bloom, including threshold, intensity, scatter or diffusion, tint, and dirt
  texture.
* Tonemapping (neutral and ACES modes) and color adjustments: post exposure,
  contrast, color filter, hue shift, saturation, white balance, channel
  mixer, lift-gamma-gain, shadows-midtones-highlights, and color curves (PPv2
  Color Grading; URP Color Adjustments and related overrides).
* Vignette.
* Depth of field (PPv2 Depth of Field; URP Gaussian and Bokeh modes).
* Motion blur (camera motion blur, the form both systems offer in the listed
  versions).
* Ambient occlusion (PPv2 Ambient Occlusion; URP Screen Space Ambient
  Occlusion renderer feature).
* Chromatic aberration.
* Film grain (PPv2 Grain; URP Film Grain).
* Lens distortion.

An effect or override type that is not in this list is reported with the
volume profile, the effect type, and its state; it does not silently render as
if it were disabled while the profile is reported as Supported.

## Lines, trails, and render textures

Line and trail renderers retain points, width curve, color gradient, material,
alignment, texture mode, sorting, lifetime, and visibility within scope.
Render textures expose size, format, depth, update state, usage, and content
validity. Feedback cycles and unsupported formats are diagnosed rather than
producing unexplained stale imagery.

## Transparency and ordering

Opaque, alpha-tested, and transparent content participates in depth, sorting
layers, order values, render queues, distance sorting, and camera order as in
Unity for the active pipeline. The editor and automation expose the effective
ordering inputs for a selected renderer.

Known limitations involving intersecting transparent surfaces, precision, or
target variation are reported as limitations rather than hidden as deterministic
correctness.

## 2D rendering

The 2D Sprite, 2D Tilemap Editor, 2D Tilemap Extras, 2D Animation, 2D Pixel
Perfect, and 2D PSD Importer packages are covered at the versions the supported
package matrix lists.

### Sprites and sprite renderers

* Sprite assets retain their texture, rectangle, pivot, pixels-per-unit,
  borders (nine-slice), mesh type, custom outline, and custom physics shape.
  The physics shape is consumed by 2D colliders as defined in
  [Physics, navigation, and terrain](physics-navigation-terrain.md).
* `SpriteRenderer` supports Simple, Sliced, and Tiled draw modes (with
  continuous and adaptive tile modes), size, color, flipX and flipY, mask
  interaction, material, sorting layer, and order in layer.
* Sprites imported from PSD/PSB files through the 2D PSD Importer retain their
  layers, generated sprites, and generated prefab hierarchy.

### Sprite Atlas

* Sprite Atlas assets pack their included sprites, folders, and textures; a
  sprite renders from its packed atlas in players and in play mode when
  packing is enabled.
* Late binding is Supported: an atlas marked to not be included in the build
  delivers its sprites only after the project answers
  `SpriteAtlasManager.atlasRequested`; until then the sprite renders as
  missing and the pending request is inspectable.
* Variant atlases and the include-in-build flag retain their Unity meaning. A
  sprite that belongs to multiple atlases is reported as ambiguous.

### Sorting and masking

* Sorting layers, order in layer, sorting axis (transparency sort mode and
  axis), and `SortingGroup` determine 2D draw order as in Unity. A Sorting
  Group sorts its children as one unit and its own sorting values are
  inspectable alongside each child's effective order.
* `SpriteMask` with front and back sorting ranges and alpha cutoff limits
  drawing of sprites whose mask interaction is Visible Inside Mask or Visible
  Outside Mask.

### 2D lighting

* The URP 2D Renderer supports 2D lights (global, freeform, sprite, and
  spot/point light types of the listed URP line), light blend styles, target
  sorting layers, normal maps, shadow casters (`ShadowCaster2D`), and
  intensity and falloff controls. A sprite outside every light's target
  sorting layers receives only global light, and the reason is inspectable.

### Tilemaps

* `Grid` (rectangle, hexagonal, and isometric layouts, cell size, gap, and
  swizzle) and `Tilemap` retain every placed tile, its transform matrix,
  color, and flags.
* Tile, AnimatedTile, RuleTile, RuleOverrideTile, and other tile types in the
  2D Tilemap Extras package resolve the sprite, collider type, and animation
  that their rules select for each cell. A tile asset whose script type is
  missing is preserved and reported per affected cell range.
* `TilemapRenderer` supports Chunk and Individual modes, detect-chunk-culling
  bounds, sort order, and mask interaction, with the same draw-order rules
  as Unity for each mode.
* Runtime `Tilemap` APIs (`SetTile`, `SetTiles`, `GetTile`, `BoxFill`,
  `RefreshTile`, and bounds compression) change the rendered and queried
  tile data immediately; collider updates follow
  [Physics, navigation, and terrain](physics-navigation-terrain.md).

### Pixel Perfect Camera

* The Pixel Perfect Camera component (2D Pixel Perfect package, and the URP
  2D variant on the listed URP lines) retains Assets Pixels Per Unit,
  reference resolution, upscale render texture, pixel snapping, crop frame,
  and stretch fill. The effective camera size and integer zoom factor are
  inspectable, and sprites land on pixel-grid positions as in Unity.

### Skinned sprites

* 2D Animation content retains sprite bones, skin weights, sprite skins
  (`SpriteSkin`), sprite library assets, sprite resolvers, and inverse
  kinematics solvers of the listed package version. Animating bones deforms
  the sprite mesh as in Unity; swapping a sprite through a sprite resolver
  changes the displayed sprite at the frame the label changes.

## Frame presentation and display output

* `Application.targetFrameRate` and `QualitySettings.vSyncCount` determine
  frame pacing as in Unity: on desktop targets a nonzero `vSyncCount`
  synchronizes presentation to the display refresh and takes precedence over
  `targetFrameRate`; on mobile and browser targets `vSyncCount` is ignored as
  in Unity and `targetFrameRate` paces frames within the display refresh. The
  rule in effect on the active target is reported.
* The achieved frame rate, frame time, presentation interval, and missed
  frames are observable during play, in headless sessions, and in players,
  through profiling and automation.
* Fullscreen modes (`FullScreenMode.ExclusiveFullScreen`,
  `FullScreenWindow`, `MaximizedWindow`, and `Windowed`), `Screen.SetResolution`,
  `Screen.resolutions`, `Screen.currentResolution`, refresh rate selection,
  and window resizing apply as in Unity. A mode that a target does not offer
  is reported as Platform-limited and the resolved mode is reported.
* Multiple displays are Supported on desktop targets: `Display.displays`,
  `Display.Activate`, camera target display, and canvas target display render
  to the selected outputs. Targets with one display report that limit.
* HDR display output is Supported on targets and devices that report HDR
  output support, with the HDR output state and the selected tonemapping and
  paper-white settings inspectable; otherwise it is Platform-limited.
* In a headless session without a display, frames render offscreen at the
  resolution the agent session or scenario requests, defaulting to the
  project's default player resolution. Frames advance at the rate the session
  requests or, by default, as fast as they complete; when
  `Application.targetFrameRate` is set, `Time` values are computed from it so
  that simulated time matches a player running at that rate.

## Animation clips

Animation clips drive numeric, vector, color, transform, blend-shape, and
object-reference properties over time. A clip exposes duration, sample rate,
curves, interpolation, wrap behavior, looping, root motion, and events.

The editor and automation can preview, play, pause, and scrub a clip on a
compatible target. Bindings identify their object path, component, and
property. Missing, ambiguous, incompatible, and inactive bindings are distinct.
Preview changes do not silently become saved scene values.

Animation events occur at Unity's times. Events from the same clip that share
a frame fire in their order in the clip; ordering across clips or animators is
guaranteed only where Unity documents it. Loop boundaries, reverse or scaled
playback, and zero-duration clips have defined outcomes.

## Animator controllers

Animator controllers expose layers, states, sub-state machines, entry, exit,
any-state transitions, parameters, conditions, blend trees (1D, 2D simple
directional, 2D freeform directional, 2D freeform cartesian, and direct),
masks, speed, interruption, normalized time, override controllers, and
`StateMachineBehaviour` scripts.

At runtime, boolean, integer, float, and trigger parameters affect transitions
as in Unity. Trigger consumption, transition duration, exit time, interruption
source and ordered interruption, layer weight, override and additive layer
blending, and synced layers behave as in Unity for the project's release. The
editor and automation show active states, transitions, parameters, and weights
during play.

`StateMachineBehaviour` callbacks (`OnStateEnter`, `OnStateUpdate`,
`OnStateExit`, `OnStateMove`, `OnStateIK`, `OnStateMachineEnter`, and
`OnStateMachineExit`) run at the points Unity defines relative to the
transition that causes them.

### Write Defaults and state reset

* A state with Write Defaults on writes the controller's captured default
  value to every property that other states in the controller animate and
  this state does not. A state with Write Defaults off leaves those properties
  at their last written value.
* Disabling and re-enabling the Animator, assigning
  `runtimeAnimatorController`, and calling `Rebind` reset state machine state
  and restore captured defaults as in Unity, unless the Keep Animator State On
  Disable option (`keepAnimatorStateOnDisable`, together with
  `writeDefaultValuesOnDisable` in the releases that define it) preserves
  state on disable.

### Update timing and scripting

* Update modes Normal, Animate Physics, and Unscaled Time advance the Animator
  at Unity's points: Normal after `Update` and before `LateUpdate` using
  scaled time, Animate Physics in the fixed time step, and Unscaled Time after
  `Update` using unscaled time.
* Culling modes Always Animate, Cull Update Transforms, and Cull Completely
  decide, as in Unity, whether an off-screen Animator advances its state,
  writes transforms, and fires animation events and root motion.
* `Play`, `CrossFade`, `CrossFadeInFixedTime`, `GetCurrentAnimatorStateInfo`,
  `GetNextAnimatorStateInfo`, `IsInTransition`, `Animator.StringToHash`,
  parameter setters, and `SetLayerWeight` take effect at Unity's evaluation
  point, so a value set in `Update` is observed in that frame's evaluation and
  state info queried before evaluation reports the previous frame's result.

Missing motions, invalid parameter references, unreachable states, invalid
bindings, and conflicting transition behavior produce specific diagnostics.

## Skeletal animation and root motion

Skinned content retains bone hierarchy, skin weights, bind poses, rig and
avatar information, blend shapes, and root motion. Animation affects the
rendered skin, attached child objects, queried bone transforms, and relevant
colliders consistently.

Root motion identifies whether movement applies to the GameObject, remains in
the animation, or is handled by gameplay code through `OnAnimatorMove`,
according to `Animator.applyRootMotion` and clip settings. Looping does not
introduce unexplained position or rotation discontinuities.

## Avatars, humanoid retargeting, and inverse kinematics

* Generic and humanoid `Avatar` assets retain their bone mapping, root node,
  human description, and muscle settings. An avatar whose definition is
  invalid is reported with the missing or misassigned bones.
* Humanoid clips retarget across avatars with different proportions as in
  Unity, honoring per-muscle limits, translation degrees of freedom, arm and
  leg stretch, feet spacing, and foot IK from the clip settings. Muscle values
  read through `HumanPoseHandler` match the applied pose.
* Avatar masks restrict which humanoid body parts or transform paths a layer
  or clip affects.
* `OnAnimatorIK` runs once per IK-pass-enabled layer, and IK goals, hints,
  look-at weights, and body position set through `Animator` APIs affect the
  final pose. `OnAnimatorMove` replaces automatic root-motion application when
  present, as in Unity.
* The legacy `Animation` component plays legacy clips with `Play`,
  `CrossFade`, `Blend`, `PlayQueued`, animation states, layers, weights,
  wrap modes, and culling type as in Unity. A non-legacy clip assigned to it
  is reported as in Unity rather than silently played.

## Playable graphs

The Playables API is Supported for animation, audio, and script playables:

* `PlayableGraph` creation, evaluation, time update modes (game time, unscaled
  game time, DSP clock, and manual), `Play`, `Stop`, `Evaluate`, and
  `Destroy`; playable time, speed, duration, done state, and input weights.
* Animation playables: `AnimationClipPlayable`, `AnimationMixerPlayable`,
  `AnimationLayerMixerPlayable` (with layer masks and additive layers),
  `AnimatorControllerPlayable`, and `AnimationPlayableOutput`.
* Audio playables: `AudioClipPlayable`, `AudioMixerPlayable`, and
  `AudioPlayableOutput`, routed through their output's `AudioSource` and that
  source's mixer group as defined by AUD-001 in
  [UI, audio, and input](ui-audio-input.md).
* `ScriptPlayable<T>` with `PlayableBehaviour` callbacks (`OnGraphStart`,
  `OnGraphStop`, `OnBehaviourPlay`, `OnBehaviourPause`, `PrepareFrame`,
  `ProcessFrame`, and `OnPlayableDestroy`) in Unity order.
* Graphs that are not destroyed before their owner is destroyed or a play
  session ends are reported as leaked. Playable graphs, their nodes, outputs,
  weights, and time are inspectable during play.

## Particle effects

Particle systems expose duration, looping, start state, emission, lifetime,
shape, velocity, force, color, size, rotation, noise, collision, triggers,
sub-emitters, texture sheet animation, trails, lights, custom data, renderer
state (including custom vertex streams and mesh GPU instancing), simulation
space, scaling mode, culling mode, and prewarm behavior within scope.

* `Play`, `Stop` (with `ParticleSystemStopBehavior` emitting-only or
  emitting-and-clear), `Pause`, `Clear`, `Emit` (count or `EmitParams`),
  `Simulate`, `GetParticles`, `SetParticles`, `GetCustomParticleData`, and
  `SetCustomParticleData` behave as in Unity.
* Stop actions None, Disable, Destroy, and Callback run when the system and
  its sub-emitters finish; Callback invokes `OnParticleSystemStopped`.
* Culling modes (automatic, pause and catch-up, pause, and always simulate)
  decide off-screen simulation as in Unity.
* With Send Collision Messages enabled, `OnParticleCollision` runs on the
  particle system's and the collided object's scripts, and
  `GetCollisionEvents` returns the contacts. `OnParticleTrigger` runs for
  trigger module events, and `GetTriggerParticles` returns the particles in
  each event category.
* `ParticleSystemForceField` components and the External Forces module
  (with its influence filter and layer mask) affect particles as in Unity.

Effects can be previewed, restarted, paused, and scrubbed in the editor and
through automation for systems whose simulation is time-deterministic under
their random seed. Runtime control reports whether a system is playing,
emitting, paused, stopped, or complete. Bounds, sorting, material, layer, camera
visibility, and target limits are inspectable. Visual Effect Graph assets are
Unsupported as listed in the [index](INDEX.md).

## Sequencing and cinematics

Time-based sequences built with Timeline 1.x coordinate animation, audio,
activation, cameras, signals, control tracks, and property tracks. A sequence
exposes duration, tracks, clips, blends, extrapolation, markers, bindings, and
evaluation time.

The editor and automation can preview and scrub without persisting preview
state. Runtime `PlayableDirector` play, pause, resume, stop, seek, wrap mode
(hold, loop, and none), completion (`stopped` event), time update mode, and
nested sequence behavior are explicit. Signals fire their receivers' reactions
once per crossing as in Unity. Missing bindings identify the track and
intended target. Custom tracks, clips, and mixers implemented with playable
scripts behave as defined in [Playable graphs](#playable-graphs).

## Camera behavior

Camera rigs built with Cinemachine 2.x behave as in Unity: virtual cameras
with priority, Follow and LookAt targets, body and aim behaviors, noise
profiles, lens settings, extensions (including confiners and collision
avoidance), state-driven, clear-shot, mixing, and blend-list cameras, and
target groups. The `CinemachineBrain` selects the live camera by priority and
recency, applies default and custom blends, and updates in the configured
update method (smart, fixed, late, or manual).

Camera cuts and blends, whether driven by Cinemachine or by Timeline camera
tracks, produce defined output and input-control transitions. The live virtual
camera, the blend in progress, and the reason a camera is not live are
inspectable during play.

## Rendering diagnostics

Users and authorized agents can inspect camera contribution, visible and culled
renderers, geometry counts, material and shader selection, active lights,
shadow participation, active volumes, render targets, renderer features and
command buffers that ran, texture residency, runtime mesh and material
instances, animation state, playable graphs, and frame cost.

Diagnostics navigate to responsible project content and distinguish correctness
errors from performance warnings. A captured image records target, resolution,
aspect, quality level, color space, camera, render pipeline and renderer, scene
state, and relevant color interpretation.

Gizmos, `Debug.DrawLine`, and `Debug.DrawRay` visualization, frame capture as
structured data, and profiling of rendering cost are owned by
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md);
their drawing in the scene and game views is owned by
[Editor and authoring](editor-and-authoring.md).

## Acceptance outcomes

1. Camera coordinate conversions agree with the rendered image, multiple
   cameras and URP camera stacks contribute in depth and stack order,
   `Camera.main` resolves the first enabled `MainCamera`, and a camera without
   visible output reports its reason. (REN-001)
2. On the Built-in Render Pipeline, `OnRenderImage` and `Camera.onPreRender`
   run and alter the image as in Unity; in a URP project they do not run, the
   compatibility report names the scripts that define them, and
   `RenderPipelineManager.beginCameraRendering` fires instead. (REN-001,
   REN-006)
3. Built-in shader-family and URP materials preserve their visible properties
   across editor and player, and a `MaterialPropertyBlock` changes one
   renderer without creating a material instance. (REN-002)
4. Accessing `Renderer.material` and `MeshFilter.mesh` creates runtime
   instances visible in residency inspection that `Resources.UnloadUnusedAssets`
   releases after they are unreferenced, while `sharedMaterial` and
   `sharedMesh` edits change the asset; a 32-bit-index mesh with more than
   65,535 vertices renders and an out-of-range index fails with Unity's error.
   (REN-002)
5. A script writes pixels with `SetPixels`, calls `Apply`, reads them back with
   `GetPixels`, copies a render target with `ReadPixels`, and reads a render
   texture with `AsyncGPUReadback`; reading a non-readable texture fails with a
   compatible error naming it. (REN-002)
6. The same scene rendered in Gamma and Linear color space matches Unity's
   output for each, and a target without Linear support reports it as
   Platform-limited naming the graphics API. (REN-002)
7. `WebCamTexture` plays after camera permission is granted and returns an
   explicit status when permission is denied or no camera exists. (REN-002)
8. A surface shader, a `CGPROGRAM` shader using `UnityCG.cginc` and
   `multi_compile` keywords, and a shader with `GrabPass` render as in Unity on
   the Built-in Render Pipeline; under URP the `GrabPass` is reported as
   ignored. (REN-002, REN-006)
9. `Shader.Find` for a shader in Always Included Shaders succeeds in a player,
   for an unreferenced shader returns null, and the build report lists the
   shaders reachable by `Shader.Find`. (REN-002)
10. Missing visual output has an inspectable cause that distinguishes renderer,
    material, shader, light, culling, and target reasons. (REN-001, REN-002,
    REN-003)
11. A scene with Unity-baked lightmaps, light probes, reflection probes, and
    occlusion data renders with that data in editor and player; moving a
    static contributing renderer marks the data stale and names the affected
    renderers; requesting a bake reports Unsupported. (REN-003)
12. Switching skybox and ambient settings through `RenderSettings` followed by
    `DynamicGI.UpdateEnvironment` changes ambient and reflection lighting as in
    Unity; light cookies, flares, light culling masks, URP light layers, and
    Built-in pixel light count produce Unity's result; Enlighten realtime GI
    data is reported Unsupported. (REN-003)
13. `QualitySettings.SetQualityLevel` with `applyExpensiveChanges` false and
    then true applies each quality setting at Unity's point, and a setting the
    target ignores is reported. (REN-003)
14. Each listed PPv2 and URP Volume effect renders with its blended parameters,
    and the contributing volumes for a camera position are inspectable; an
    unlisted effect type is reported rather than silently ignored. (REN-003)
15. Particle systems, trails, and line renderers report their playback state
    and render the same in preview, play mode, and player, while a Visual
    Effect Graph asset is reported Unsupported. (REN-004)
16. `Emit` with `EmitParams`, `SetParticles`, custom data, a Destroy stop action,
    a Callback stop action invoking `OnParticleSystemStopped`,
    `OnParticleCollision`, `OnParticleTrigger`, and a force field behave as in
    Unity. (REN-004)
17. A 2D scene with sliced and tiled sprites, a late-bound Sprite Atlas, a
    Sorting Group, a Sprite Mask, URP 2D lights, a rule-tile Tilemap, a Pixel
    Perfect Camera, and a skinned 2D character renders with the same draw order
    and pixel placement as the Unity reference. (REN-005)
18. A project on each listed URP line renders with Forward, with Deferred where
    the line offers it, and with Forward+ on URP 14; an HDRP project and a
    custom SRP project are each reported Unsupported while remaining
    inspectable. (REN-006)
19. A Shader Graph material renders equivalently to its Unity result; an HDRP
    or Visual Effect Graph target is reported Unsupported; and a graph
    containing an unrepresentable node is reported with the graph, node, and
    affected materials. (REN-006)
20. A `ScriptableRendererFeature`, a URP `DecalProjector` on 12.x and 14.x, a
    camera `CommandBuffer`, `Graphics.DrawMeshInstanced`, and a compute shader
    dispatch each produce the Unity-equivalent image; on a target without
    compute the dispatch is reported Platform-limited; enabling the SRP
    Batcher changes only reported performance. (REN-006)
21. Global shader properties and keywords set from script select the same
    variants as in Unity, exceeding the keyword limit produces Unity's error,
    and after `ShaderVariantCollection.WarmUp` returns the listed variants are
    ready. (REN-006)
22. Failed or unsupported shaders cannot appear as successful compatibility.
    (REN-002, REN-006)
23. `Application.targetFrameRate`, `QualitySettings.vSyncCount`, fullscreen
    mode, resolution changes, and a second display take effect in a desktop
    player, and the achieved frame pacing is observable through automation.
    (REN-007)
24. A headless session renders frames at the requested resolution, or at the
    project's default player resolution when none is requested, and with
    `Application.targetFrameRate` set reports `Time` values computed from that
    rate. (REN-007)
25. Visual comparison captures identify all conditions needed for evaluation,
    including target, resolution, quality level, color space, camera, and
    render pipeline. (REN-001, REN-006, REN-007)
26. On each supported build target, a developer or agent can identify the
    effective graphics API and diagnose missing device capabilities without
    assuming Direct3D on Linux or macOS. (REN-006, REN-007)
27. Animation preview, play mode, and built-player playback produce the same
    pose and animation event sequence for the same sequence of frame times.
    (ANI-001)
28. Animator state, transition, blend-tree, and `StateMachineBehaviour` behavior
    is observable during runtime. (ANI-001)
29. A controller mixing Write Defaults on and off states leaves properties with
    Unity's values after each transition, and disabling and re-enabling the
    Animator resets or keeps state according to Keep Animator State On
    Disable. (ANI-001)
30. Animators in Normal, Animate Physics, and Unscaled Time update modes advance
    at Unity's points with `Time.timeScale` at 0 and 1; culling modes decide
    off-screen state and events as in Unity; `CrossFade` and parameter setters
    called in `Update` are observed in that frame's evaluation. (ANI-001)
31. A humanoid clip retargets onto a differently proportioned avatar with
    muscle limits honored, `OnAnimatorIK` and `OnAnimatorMove` alter the pose
    and motion, and an avatar mask restricts a layer. (ANI-002)
32. A `PlayableGraph` mixing two `AnimationClipPlayable` inputs and a
    `ScriptPlayable` produces the Unity-equivalent pose and callback order, an
    `AudioPlayableOutput` plays through its `AudioSource` and mixer group, and
    the legacy `Animation` component cross-fades legacy clips. (ANI-002)
33. A Timeline with animation, audio, activation, signal, and Cinemachine shot
    tracks plays, pauses, seeks, and completes as in Unity, and the live
    virtual camera and active blend are inspectable during play. (CIN-001)
