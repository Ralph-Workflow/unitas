# Rendering, Cameras, Lighting, and Animation

## Scope

This specification defines the visible behavior of cameras, geometry,
materials, shaders, render pipelines, scripted rendering, lighting, shadows,
textures, render targets, 2D rendering, frame presentation, animation,
particles, and cinematic sequencing. It describes observable output and
diagnostics, not a rendering architecture.

These are desired end-state requirements, not claims about currently shipped
capability. The compatibility baseline is Unity 2022.3 LTS, as defined in the
[Compatibility contract](compatibility-contract.md); projects saved by earlier
releases open after their serialized forms migrate to that baseline. URP and
Shader Graph 14.x, Post Processing (v2) 3.x, the 2D packages, Timeline 1.x,
Cinemachine 2.x, and other package content are covered only for the package
families and version lines listed in the supported package matrix (CMP-002,
owned by the [Compatibility contract](compatibility-contract.md)).

In this document, "within scope" means the render pipelines named in
[Render pipeline scope](#render-pipeline-scope) combined with the package
versions that the supported package matrix lists. Behavior outside that scope
carries one of the compatibility states defined in the
[Compatibility contract](compatibility-contract.md).

Every capability in this document, including inspection, preview, capture, and
the diagnostics below, is available to authorized agents through automation as
defined in [Automation and MCP](automation-and-mcp.md), in the visual editor,
and in headless sessions.

### Compatibility tiers

The tiers below are recorded in the [Feature catalog](feature-catalog.md) and
defined in the [Compatibility contract](compatibility-contract.md):

| ID | Tier |
|---|---|
| REN-001 | Native |
| REN-002 | Native |
| REN-003 | Native |
| REN-004 | Familiar |
| REN-005 | Native |
| REN-006 | Native |
| REN-007 | Native |
| ANI-001 | Familiar |
| ANI-002 | Familiar |
| CIN-001 | Familiar |

* **Native rendering (REN-001, REN-002, REN-003, REN-005, REN-006, REN-007):**
  unitas defines how scenes are rendered. Unity render pipeline assets,
  renderer data, shaders, materials, textures, lighting data, and rendering
  settings are imported and translated into unitas rendering. Pixel identity
  with Unity is not promised. Scripting-visible rendering APIs keep their Unity
  2022.3 names, signatures, and documented meaning; for example,
  `Camera.WorldToScreenPoint` returns the pixel at which the point appears in
  the image unitas renders.
* **Familiar animation, particles, and cinematics (REN-004, ANI-001, ANI-002,
  CIN-001):** Unity clips, controllers, avatars, particle systems, timelines,
  and camera rigs import with their identity, and public types and members
  keep their Unity 2022.3 names and signatures. The behavior is the one this
  document states. Every difference from Unity 2022.3 is listed in the API
  reference owned by AUT-006 in [Automation and MCP](automation-and-mcp.md)
  and in the project compatibility report.

### Visual acceptance

Visual acceptance in this document uses TST-002 visual validation, owned by
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md):
a capture is compared with a unitas-approved reference at the same target,
resolution, quality level, camera, scene state, and time, with a tolerance
declared in the scenario or reference as TST-002 defines. Draw order, the
presence of every expected object, the shader pass
selected for each renderer, and the set of lights affecting each renderer are
compared exactly. A comparison without a declared tolerance is reported as not
evaluable and is never reported as matching. Unity screenshots are not
acceptance references.

## Graphics API and target capability

The product does not require a particular graphics API or API version for
rendering. Direct3D, OpenGL, Vulkan, and Metal are possible implementation
choices, not interchangeable promises. In particular, Linux and macOS support
does not imply that Direct3D runs on those systems. ShaderLab and HLSL support
describes accepted project content, not the API used to draw it.

For each supported build target and current host, developers can inspect the
selected graphics API and version, the device and driver where the host
reports them, and which rendering features named in this document are
Supported or Platform-limited there. If no usable graphics device or API is
available, startup or build fails with an actionable reason; it does not
silently claim a working renderer. A change of graphics API does not change the
draw order, object presence, shader pass selection, or light assignment of a
scene. Target-specific visual differences or unsupported shader features are
reported as limitations under the compatibility contract.

## Render pipeline scope

unitas renders projects authored for the Built-in Render Pipeline or for the
Universal Render Pipeline (URP) 14.x. The project's pipeline is determined by
the Graphics settings render pipeline asset and the per-quality-level
override; the effective pipeline asset for each quality level and target is
inspectable.

* **Built-in Render Pipeline projects:** the forward, deferred, and legacy
  vertex-lit rendering path settings, camera rendering path overrides, and
  Built-in quality settings are imported and translated. Each path keeps the
  limits it states in the project settings, such as the forward pixel light
  count.
* **URP 14.x projects:** URP assets, Universal Renderer data, 2D Renderer
  data, renderer lists, the Forward, Forward+, and Deferred rendering paths,
  camera stacking (base and overlay cameras), render scale, HDR, MSAA, depth
  and opaque texture options, and per-camera renderer selection are imported
  and translated. In Forward, the per-object additional light limit of the URP
  asset applies; Forward+ and Deferred do not apply it; Deferred does not use
  MSAA.
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
* `WorldToScreenPoint`, `ScreenToWorldPoint`, `WorldToViewportPoint`,
  `ViewportToWorldPoint`, `ScreenPointToRay`, and `ViewportPointToRay`
  correspond to the image unitas renders: a world point converted to screen
  coordinates lands on the pixel where that point is drawn.
* Multiple cameras contribute in ascending `Camera.depth` order (Built-in
  projects) or in stack order, base camera first (URP projects), each
  according to its viewport and clear flags.
* `Camera.main` returns the first enabled camera tagged `MainCamera` and
  returns null when there is none.
* A disabled, culled, targetless, or obscured camera has an inspectable reason
  for producing no visible output.
* Aspect changes, safe areas, dynamic resolution on targets that report support
  for it, and display selection have explicit effects.
* Render targets receive the camera result at their dimensions, format, color
  space, and update behavior.
* `Camera.Render` and manual camera rendering produce the same image as the
  automatic render of the same camera state.

### Camera callbacks and depth textures

* In Built-in Render Pipeline projects, for each camera, `OnPreCull` and
  `Camera.onPreCull` run before culling, `OnPreRender` and
  `Camera.onPreRender` run after culling and before drawing, `OnPostRender`
  and `Camera.onPostRender` run after drawing, and `OnRenderImage` runs last
  with source and destination render textures; its output becomes the
  camera's image.
* `Camera.depthTextureMode` (Depth, DepthNormals, and MotionVectors) makes
  `_CameraDepthTexture`, `_CameraDepthNormalsTexture`, and
  `_CameraMotionVectorsTexture` available to shaders for that camera.
* In URP projects these Built-in camera callbacks are not invoked. The project
  compatibility report names every script that defines them, with the
  consequence that they do not run.
* In URP projects, `RenderPipelineManager.beginContextRendering`,
  `beginCameraRendering`, `endCameraRendering`, and `endContextRendering` fire
  in that order for each frame, with the camera events once per camera in
  render order.

## Mesh and sprite rendering

Static meshes, skinned meshes, sprites, tilemaps, lines, trails, and particle
renderers participate in camera culling, transforms, sorting, layers,
materials, lighting, and shadows according to their feature contracts.

A renderer exposes its source geometry, material slots, bounds, enablement,
sorting state, shadow participation, and visibility reason. Missing geometry,
invalid bounds, absent materials, unsupported topology, or target limitations
are distinct diagnostics.

Skinned meshes reflect bone transforms, bind poses, weights, blend shapes, and
animation. Negative scale flips triangle winding so front faces remain visible,
and non-uniform scale transforms normals with the inverse transpose. A
renderer does not appear compatible merely because a fallback shape is visible.

### Runtime meshes and mesh instances

* Meshes created or modified at runtime through the `Mesh` API (vertices,
  indices, submeshes, normals, tangents, UVs, colors, bounds recalculation,
  and `MarkDynamic`) render the data the script supplied.
* `Mesh.indexFormat` selects 16-bit or 32-bit indices. Assigning more than
  65,535 vertices to a 16-bit mesh, or an index outside the vertex range,
  fails with an error naming the mesh and leaves the mesh unchanged.
* `Mesh.CombineMeshes`, `SetVertexBufferParams`, `SetVertexBufferData`,
  `SetIndexBufferParams`, `SetIndexBufferData`, `SetSubMesh`, and the
  `Mesh.MeshData` APIs (`AcquireReadOnlyMeshData`, `AllocateWritableMeshData`,
  and `ApplyAndDisposeWritableMeshData`) produce meshes whose vertex, index,
  and submesh data read back exactly as written.
* Reading mesh data that was imported without Read/Write enabled fails in a
  player with an error naming the mesh.
* `MeshFilter.mesh` creates a per-renderer copy of the shared mesh on first
  access, and `MeshFilter.sharedMesh` reads and edits the asset. Copies live
  until destroyed or released by `Resources.UnloadUnusedAssets`, and appear
  in residency inspection as runtime instances distinct from the asset.

## Materials

Materials retain their shader relationship and their textures, colors,
numbers, vectors, keywords, render queue, surface mode, transparency, alpha
clipping, culling, depth behavior, tiling, and offsets.

Materials using the Built-in shaders listed in
[Built-in shaders](#built-in-shaders) and the URP Lit, Simple Lit, Unlit,
Baked Lit, and 2D sprite materials are translated to unitas materials. Metallic,
smoothness, normal, occlusion, emission, transparency, and alpha-clipping
controls keep their role: increasing metallic reduces diffuse response,
increasing smoothness sharpens highlights and reflections, emission adds light
independent of scene lighting, and alpha clipping discards fragments below the
threshold.

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
`QualitySettings.activeColorSpace`, governs rendering. In Linear, textures
marked sRGB are converted to linear on sampling, lighting and blending are
computed in linear values, sRGB render targets convert on write, and UI and
vertex colors are converted from sRGB to linear. In Gamma, no conversion takes
place. Texture import sRGB flags and `RenderTexture` read-write modes keep
these meanings. A target whose graphics API cannot provide Linear rendering
reports Linear color space as Platform-limited, naming the target and graphics
API; it does not silently render in Gamma.

## Textures at runtime

Textures expose dimensions, format, mipmap count, filter mode, wrap mode,
anisotropy, color space interpretation, readability, and residency.

* `Texture2D.SetPixel`, `SetPixels`, `SetPixels32`, and `SetPixelData` change
  CPU-side texture data; the visible texture changes only after `Apply`.
  `Apply(updateMipmaps)` regenerates mipmaps when true, and
  `makeNoLongerReadable` releases the CPU copy so later reads fail.
* `GetPixel`, `GetPixels`, `GetPixels32`, `GetPixelData`, and
  `GetRawTextureData` return the stored texel values. Reading a texture whose
  import settings leave it non-readable fails with an error naming the
  texture.
* `Texture2D.ReadPixels` copies from the active render target into the texture
  at the requested rectangle and offset; the result becomes visible after
  `Apply`.
* `ImageConversion.LoadImage` decodes PNG and JPG data, and `EncodeToPNG`,
  `EncodeToJPG`, `EncodeToTGA`, and `EncodeToEXR` produce files whose decoded
  texels match the source exactly for lossless formats and within the
  requested quality for JPG.
* `Cubemap`, `Texture2DArray`, `Texture3D`, `CubemapArray`, and
  `CustomRenderTexture` (with its initialization, update mode, update zones,
  and double buffering) can be created, written, and sampled.
* `Sprite.Create` produces a sprite with the given rectangle, pivot,
  pixels-per-unit, mesh type, and border.
* Mipmap streaming honors the quality settings memory budget, per-texture
  priority, and `Texture2D.requestedMipmapLevel`; the loaded and desired mip
  levels of each streamed texture are inspectable.
* `AsyncGPUReadback.Request` completes on a later frame; `done` becomes true,
  `hasError` reports failure, and `GetData` returns the texels. A target
  without readback support reports the request as Platform-limited and
  completes it with `hasError` true.
* `RenderTexture` creation, `Create`, `Release`, `IsCreated`,
  `RenderTexture.active`, temporary render textures (`GetTemporary` and
  `ReleaseTemporary`), depth buffers, MSAA, and random-write enablement keep
  their documented meaning. A format that the target cannot create is reported
  as Platform-limited with the format and target named; it is not silently
  replaced with a different format.
* `WebCamTexture` plays the selected camera device after the target's camera
  permission is granted. A denied permission or an absent camera returns an
  explicit status and an empty `WebCamTexture.devices` list or a texture that
  reports it is not playing; targets without camera access report
  `WebCamTexture` as Platform-limited.
* Texture formats and compression that a target does not support are reported
  at import or build with the asset and target named, together with the
  fallback format unitas uses.

## Shaders

ShaderLab and HLSL assets are imported and translated. A translated shader
retains its shader name, properties, subshaders, passes, tags, render states,
variants, and keywords.

Untranslatable syntax or behavior identifies the shader, pass, variant,
property, and target where known. A failed shader is unmistakable in the scene
and diagnostics. It never renders a plausible substitute while being reported
as fully correct.

Shader selection uses material keywords, global keywords, project quality,
renderer state, camera context, pipeline tags, `Shader.globalMaximumLOD` and
per-shader LOD, and target capability: the first subshader whose LOD, tags,
and target requirements are met is used, then the variant matching the active
keywords. Variant absence is distinct from missing source or invalid material
data. Shader variant stripping at build time reports which variants were
excluded and why.

### ShaderLab and HLSL content

The following ShaderLab and shader-program forms are translated and Supported:

* Built-in surface shaders (`#pragma surface`) with the Standard,
  StandardSpecular, Lambert, BlinnPhong, and custom lighting models and their
  options, producing forward, deferred, shadow-caster, and meta passes.
* `CGPROGRAM` and `HLSLPROGRAM` blocks, vertex, fragment, geometry, and
  tessellation programs on targets whose graphics API offers those stages,
  and `#pragma target` levels; a shader whose target level exceeds the
  device's capability falls through to the next subshader or its `Fallback`.
* Built-in includes (`UnityCG.cginc`, `Lighting.cginc`, `AutoLight.cginc`,
  `UnityShaderVariables.cginc`, and the other Built-in shader includes) and the
  URP 14.x ShaderLibrary.
* `multi_compile`, `shader_feature`, their `_local` forms, and their
  vertex-only and fragment-only variants.
* `Fallback`, `UsePass`, `GrabPass`, `Category`, `LOD`, and fixed-function
  render state (blend, blend operation, color mask, cull, depth test and
  write, offset, stencil, and alpha-to-mask).

In URP projects, `GrabPass` and passes whose `LightMode` tag is Built-in only
are not rendered, and each such shader and pass is reported with that
consequence.

### Built-in shaders

The Built-in shader families Standard, Standard (Specular setup), Autodesk
Interactive, `Legacy Shaders/*`, `Mobile/*`, `Unlit/*`, `Particles/*`,
`Sprites/*`, `UI/*`, `Skybox/*`, and `Nature/*` are translated to unitas
shaders that keep each family's properties and visual role: lit, unlit,
transparent, cutout, additive, particle, sprite, UI, skybox, and foliage.

`Shader.Find` returns the shader in the editor and in a player when that shader
is referenced by content included in the build or listed in the Graphics
settings Always Included Shaders. Otherwise it returns null in a player. The
build report lists the shaders reachable by `Shader.Find`.

### Shader globals, keywords, and warm-up

* `Shader.SetGlobalFloat`, `SetGlobalInt`, `SetGlobalVector`,
  `SetGlobalColor`, `SetGlobalMatrix`, `SetGlobalTexture`, `SetGlobalBuffer`,
  and their array forms set values that every shader not overriding the
  property observes. `Shader.PropertyToID` returns a stable identifier for the
  play session.
* `Shader.EnableKeyword`, `Shader.DisableKeyword`, `Material.EnableKeyword`,
  `Material.DisableKeyword`, `Material.shaderKeywords`, `LocalKeyword`,
  `GlobalKeyword`, and `Material.SetKeyword` select the variant compiled for
  the enabled keyword set.
* Declaring more keywords than the keyword limit stated in the AUT-006 API
  reference produces an error naming the shader and keyword.
* `ShaderVariantCollection.WarmUp` and `Shader.WarmupAllShaders` complete the
  preparation of the listed variants before returning, so the first draw that
  uses them does not incur that preparation.

## Shader Graph

Shader Graph 14.x assets are imported as shader content for the URP and
Built-in targets that their graph settings select. A Shader Graph asset retains
its exposed properties, keywords, sub graph references, graph target, surface
options, and material interface, and a material using it behaves as a material
using an equivalent hand-written shader.

* Sub graphs, keywords (boolean and enum), exposed and hidden properties,
  custom interpolators, and Custom Function nodes with inline or file-based
  HLSL are Supported.
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
* **Decals:** in URP projects the Decal Renderer Feature and `DecalProjector`
  are Supported with their technique (DBuffer or screen space), projection
  size, pivot, fade, and decal layers. Built-in Render Pipeline projects have
  no decal system.
* **Command buffers:** `CommandBuffer` contents execute in recorded order
  through `Graphics.ExecuteCommandBuffer`, through camera and light events in
  Built-in Render Pipeline projects (`Camera.AddCommandBuffer`,
  `Light.AddCommandBuffer`), and through `ScriptableRenderContext` in renderer
  passes. Command buffers attached to a camera are inspectable on that camera.
* **Immediate drawing:** `Graphics.Blit`, `Graphics.DrawMesh`,
  `Graphics.DrawMeshInstanced`, `Graphics.DrawMeshInstancedIndirect`,
  `Graphics.DrawProcedural`, and `Graphics.CopyTexture` draw or copy with the
  camera, layer, material, and material property block arguments they are
  given.
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
  fails with an error instead of silently doing nothing.
* **SRP Batcher and batching:** whether the SRP Batcher, static batching, or
  dynamic batching combined draws is a performance property reported through
  profiling ([Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md));
  it never changes the visible result and is never a compatibility failure.

## Lighting and environment

Directional, point, and spot lights expose color, intensity, range or angle,
enablement, culling mask, shadow choice, render mode, and quality settings.
The visible result respects object transforms and material response.

* Light cookies, Built-in light halos and the `Halo` component, and Built-in
  `Flare` assets with a camera Flare Layer are translated and rendered. In URP
  projects the Lens Flare (SRP) component renders its lens flare data asset.
* In Built-in Render Pipeline projects, lights marked Important render per
  pixel, lights marked Not Important render per vertex, and Auto lights fill
  the quality pixel light count in order of brightness and distance. In URP
  projects, the main light, the additional lights per object limit (Forward
  only), and per-vertex or per-pixel additional lights follow the URP asset.
* `Light.cullingMask` excludes layers from a light, and URP rendering layers
  restrict which renderers a light affects.
* Area lights are Supported as baked-only lights, contributing through
  Unity-baked lighting data.
* Enlighten realtime global illumination data is Unsupported, as listed in the
  [index](INDEX.md); baked lighting data remains Supported as defined below.

Environment lighting includes skybox materials (`Skybox/Procedural`,
`Skybox/6 Sided`, `Skybox/Cubemap`, and `Skybox/Panoramic`), ambient source
(skybox, gradient, or color) and intensity, environment reflection source,
resolution, intensity, and bounces, fog, and exposure. Changing
`RenderSettings` at runtime changes the next rendered frame, and calling
`DynamicGI.UpdateEnvironment` recomputes ambient probe and default reflection
lighting from the current skybox. A project can inspect which environment
settings affect a camera and scene. Baked, mixed, and realtime lighting modes
are identified accurately rather than treated as interchangeable.

Light limits, target limitations, and quality reductions are visible in
diagnostics. A light ignored because of a layer, range, disabled state, or limit
can be distinguished from an invalid light.

## Shadows

Eligible renderers cast and receive shadows according to material, renderer,
light, layer, distance, and quality settings. Directional, point, and spot
lights cast shadows in both Built-in and URP projects.

Shadow bias, normal bias, strength, softness, distance, resolution, and cascade
choices have visible and inspectable effects. Missing shadows identify whether
the cause is caster state, receiver material, light state, culling, quality,
target support, or resource limits.

## Quality levels

`QualitySettings.SetQualityLevel(level, applyExpensiveChanges)` switches the
active quality level. Settings that require recreating render targets
(anti-aliasing) apply only when `applyExpensiveChanges` is true; all other
settings of the level apply from the next frame.

Each quality level's pixel light count, texture quality and mipmap limit,
anisotropic textures, anti-aliasing, soft particles, realtime reflection
probes, shadow settings, LOD bias, maximum LOD level, skin weights, and render
pipeline asset take effect when the level is active. A setting that the active
target or pipeline does not use is ignored and reported with the quality level
and target.

## Baked lighting data, probes, fog, LOD, and occlusion

Lightmaps, light probes, reflection probe captures, and occlusion culling data
baked by Unity are Supported project content. Rendering uses them with their
renderer assignments, lightmap indices and scale-offsets, contribution modes
(contribute global illumination, receive from lightmaps or light probes),
mixed lighting modes (Baked Indirect, Shadowmask, Subtractive), shadow masks,
and the relationship between baked and realtime lights. Light probe groups
expose their probe positions and interpolation; reflection probes expose their
influence volume, blending, box projection, importance, and captured result.
Realtime reflection probes update according to their refresh mode and
time-slicing settings.

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

Post-processing content from the Post Processing (v2) package 3.x (Built-in
Render Pipeline projects) and URP 14.x Volumes is imported and translated. Each
system retains its own content: PPv2 Post-process Layer and Post-process Volume
components with profiles, and URP Volume components with Volume Profiles.

Volumes retain global or local mode, collider-defined extent, priority, blend
distance, weight, and the layer mask or camera participation that selects them.
Higher-priority volumes override lower ones, and local volumes blend in by
distance within their blend distance. The blended value of every overridden
parameter for a given camera and position is inspectable, together with the
volumes that contributed to it.

The following effects are Supported in both systems with their source
parameters translated, except where noted:

* Bloom, including threshold, intensity, scatter or diffusion, tint, and dirt
  texture.
* Tonemapping (neutral and ACES modes) and color adjustments: post exposure,
  contrast, color filter, hue shift, saturation, white balance, channel
  mixer, lift-gamma-gain, shadows-midtones-highlights, and color curves (PPv2
  Color Grading; URP Color Adjustments and related overrides).
* Vignette.
* Depth of field (PPv2 Depth of Field; URP Gaussian and Bokeh modes).
* Motion blur (camera motion blur, the form both systems offer).
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

Opaque content draws front to back and transparent content back to front within
each camera, ordered first by render queue, then by sorting layer and order in
layer, then by distance along the camera's sort axis. Alpha-tested content
writes depth like opaque content. The editor and automation expose the
effective ordering inputs and resulting position in the draw order for a
selected renderer.

Known limitations involving intersecting transparent surfaces, precision, or
target variation are reported as limitations rather than hidden as deterministic
correctness.

## 2D rendering

Content from the 2D Sprite, 2D Tilemap Editor, 2D Tilemap Extras, 2D Animation,
2D Pixel Perfect, and 2D PSD Importer packages, at the versions the supported
package matrix lists, is imported and rendered.

### Sprites and sprite renderers

* Sprite assets retain their texture, rectangle, pivot, pixels-per-unit,
  borders (nine-slice), mesh type, custom outline, and custom physics shape.
  The physics shape is consumed by 2D colliders as defined in
  [Physics, navigation, and terrain](physics-navigation-terrain.md).
* `SpriteRenderer` supports Simple, Sliced, and Tiled draw modes (with
  continuous and adaptive tile modes), size, color, flipX and flipY, mask
  interaction, material, sorting layer, and order in layer. Sliced mode keeps
  border regions at their pixel size and stretches the center; Tiled mode
  repeats the center.
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
* Variant atlases and the include-in-build flag retain their meaning. A
  sprite that belongs to multiple atlases is reported as ambiguous.

### Sorting and masking

* 2D draw order is determined by sorting layer, then order in layer, then
  `SortingGroup` membership, then distance along the transparency sort axis.
  A Sorting Group sorts its children as one unit at the group's sorting layer
  and order, and its children sort among themselves inside it; each child's
  effective order is inspectable.
* `SpriteMask` with front and back sorting ranges and alpha cutoff limits
  drawing of sprites whose mask interaction is Visible Inside Mask or Visible
  Outside Mask.

### 2D lighting

* The URP 2D Renderer supports 2D lights (global, freeform, sprite, and spot
  light types), light blend styles, target sorting layers, normal maps, shadow
  casters (`ShadowCaster2D`), and intensity and falloff controls. A sprite
  outside every light's target sorting layers receives only global light, and
  the reason is inspectable.

### Tilemaps

* `Grid` (rectangle, hexagonal, and isometric layouts, cell size, gap, and
  swizzle) and `Tilemap` retain every placed tile, its transform matrix,
  color, and flags.
* Tile, AnimatedTile, RuleTile, RuleOverrideTile, and other tile types in the
  2D Tilemap Extras package resolve the sprite, collider type, and animation
  that their rules select for each cell. A tile asset whose script type is
  missing is preserved and reported per affected cell range.
* `TilemapRenderer` in Chunk mode sorts the whole tilemap as one renderer; in
  Individual mode each tile sorts separately with other renderers by its
  position along the sort axis. Detect-chunk-culling bounds, sort order, and
  mask interaction apply in both modes.
* Runtime `Tilemap` APIs (`SetTile`, `SetTiles`, `GetTile`, `BoxFill`,
  `RefreshTile`, and bounds compression) change the rendered and queried
  tile data immediately; collider updates follow
  [Physics, navigation, and terrain](physics-navigation-terrain.md).

### Pixel Perfect Camera

* The Pixel Perfect Camera component (2D Pixel Perfect package, and the URP 2D
  variant) retains Assets Pixels Per Unit, reference resolution, upscale
  render texture, pixel snapping, crop frame, and stretch fill. The camera
  uses the largest integer zoom at which the reference resolution fits the
  screen, the effective camera size and zoom factor are inspectable, and with
  pixel snapping on, sprite positions snap to the grid defined by Assets
  Pixels Per Unit.

### Skinned sprites

* 2D Animation content retains sprite bones, skin weights, sprite skins
  (`SpriteSkin`), sprite library assets, sprite resolvers, and inverse
  kinematics solvers. Animating bones deforms the sprite mesh by its skin
  weights; swapping a sprite through a sprite resolver changes the displayed
  sprite at the frame the label changes.

## Frame presentation and display output

* `Application.targetFrameRate` and `QualitySettings.vSyncCount` determine
  frame pacing. On desktop targets a nonzero `vSyncCount` synchronizes
  presentation to every nth display refresh and takes precedence over
  `targetFrameRate`. On mobile and browser targets `vSyncCount` is ignored and
  `targetFrameRate` paces frames within the display refresh. The rule in
  effect on the active target is reported.
* The achieved frame rate, frame time, presentation interval, and missed
  frames are observable during play, in headless sessions, and in players,
  through profiling and automation.
* Fullscreen modes (`FullScreenMode.ExclusiveFullScreen`,
  `FullScreenWindow`, `MaximizedWindow`, and `Windowed`), `Screen.SetResolution`,
  `Screen.resolutions`, `Screen.currentResolution`, refresh rate selection,
  and window resizing apply from the next frame. A mode that a target does not
  offer is reported as Platform-limited and the resolved mode is reported.
* Multiple displays are Supported on desktop targets: `Display.displays`,
  `Display.Activate`, camera target display, and canvas target display render
  to the selected outputs. Targets with one display report that limit.
* HDR display output is Supported on targets and devices that report HDR
  output support, with the HDR output state and the selected tonemapping and
  paper-white settings inspectable; otherwise it is Platform-limited.
* In a headless session without a display, frames render offscreen at the
  resolution the agent session or scenario requests, defaulting to the
  project's default player resolution. Frame pacing and `Time` follow RUN-004
  in [Scripting runtime](scripting-runtime.md) and the time-control conditions
  owned by AUT-002 in [Automation and MCP](automation-and-mcp.md). With no
  condition requested, frames advance as fast as they complete and
  `Application.targetFrameRate` limits the rate as in a player.

## Animation clips

Animation clips imported from Unity drive numeric, vector, color, transform,
blend-shape, and object-reference properties over time. A clip exposes
duration, sample rate, curves, interpolation, wrap behavior, looping, root
motion, and events.

The editor and automation can preview, play, pause, and scrub a clip on a
compatible target. Bindings identify their object path, component, and
property. Missing, ambiguous, incompatible, and inactive bindings are distinct.
Preview changes do not silently become saved scene values.

An animation event fires during the evaluation of the frame in which playback
time crosses the event's time. When one frame crosses several events of a
clip, they fire in time order, and events with equal times fire in their order
in the clip. When looping crosses the clip end, the events up to the end fire
before the events from the start. Reverse playback fires events in descending
time order. A zero-duration clip fires its events once and holds its single
pose. Ordering of events across different clips or animators in the same frame
follows the order in which those animators are evaluated, which the AUT-006
API reference states.

## Animator controllers

Animator controllers expose layers, states, sub-state machines, entry, exit,
any-state transitions, parameters, conditions, blend trees (1D, 2D simple
directional, 2D freeform directional, 2D freeform cartesian, and direct),
masks, speed, interruption, normalized time, override controllers, and
`StateMachineBehaviour` scripts.

At runtime, a transition starts when all of its conditions on boolean,
integer, float, and trigger parameters are met and, when it has exit time,
the source state has reached that normalized time. A trigger stays set until
a transition consumes it or `ResetTrigger` clears it. Transition duration,
interruption source and ordered interruption, layer weight, override and
additive layer blending, and synced layers behave as this section and the
AUT-006 API reference state. The editor and automation show active states,
transitions, parameters, and weights during play.

`StateMachineBehaviour` callbacks run as follows: `OnStateEnter` on the first
frame a state is entered (the destination of a starting transition),
`OnStateUpdate` on every evaluated frame except the first and last,
`OnStateExit` on the frame a state is left (when its outgoing transition
completes), `OnStateMove` and `OnStateIK` during root motion and IK
processing for that state, and `OnStateMachineEnter` and
`OnStateMachineExit` when a transition enters or leaves a sub-state machine.

### Write Defaults and state reset

* A state with Write Defaults on writes the controller's captured default
  value to every property that other states in the controller animate and
  this state does not. A state with Write Defaults off leaves those properties
  at their last written value.
* Disabling and re-enabling the Animator, assigning
  `runtimeAnimatorController`, and calling `Rebind` reset every layer to its
  default state, clear parameters to their default values, and restore the
  captured defaults. When `keepAnimatorStateOnDisable` is true, disabling and
  re-enabling keeps the current states and parameter values instead.

### Update timing and scripting

* Update mode Normal advances the Animator once per frame after `Update` and
  before `LateUpdate`, using scaled delta time. Animate Physics advances it in
  each fixed time step. Unscaled Time advances it after `Update` using
  unscaled delta time, so it continues when `Time.timeScale` is 0.
* Culling mode Always Animate advances state and writes transforms when no
  renderer is visible. Cull Update Transforms advances state, fires events,
  and computes root motion off-screen but skips retargeting, IK, and transform
  writes. Cull Completely stops advancing state and firing events while no
  renderer is visible.
* `Play`, `CrossFade`, `CrossFadeInFixedTime`, parameter setters, and
  `SetLayerWeight` called during `Update` take effect in that frame's
  evaluation. `GetCurrentAnimatorStateInfo`, `GetNextAnimatorStateInfo`, and
  `IsInTransition` queried before that evaluation report the previous
  evaluation's result. `Animator.StringToHash` returns the same identifier for
  the same name in every session.

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

* Generic and humanoid `Avatar` assets import with their bone mapping, root
  node, human description, and muscle settings. An avatar whose definition is
  invalid is reported with the missing or misassigned bones.
* Humanoid clips are stored and applied in muscle space: the same muscle
  values produce the same body pose on avatars of different proportions.
  Retargeting honors per-muscle limits, translation degrees of freedom, arm
  and leg stretch, feet spacing, and foot IK from the clip settings. Muscle
  values read through `HumanPoseHandler` equal those of the applied pose.
* Avatar masks restrict which humanoid body parts or transform paths a layer
  or clip affects.
* `OnAnimatorIK(layerIndex)` runs once per frame for each layer with IK Pass
  enabled, after that layer's pose is evaluated; IK goals, hints, look-at
  weights, and body position set during it shape the final pose. When a script
  on the Animator's GameObject defines `OnAnimatorMove`, root motion is not
  applied automatically and the script receives it through
  `Animator.deltaPosition` and `deltaRotation`.
* The legacy `Animation` component plays clips marked legacy with `Play`,
  `CrossFade`, `Blend`, `PlayQueued`, animation states, layers, weights, wrap
  modes, and culling type. A clip not marked legacy assigned to it does not
  play and is reported with an error naming the clip.

## Playable graphs

The Playables API is Supported for animation, audio, and script playables:

* `PlayableGraph` creation, evaluation, time update modes (game time, unscaled
  game time, DSP clock, and manual), `Play`, `Stop`, `Evaluate`, and
  `Destroy`; playable time, speed, duration, done state, and input weights.
* Animation playables: `AnimationClipPlayable`, `AnimationMixerPlayable`,
  `AnimationLayerMixerPlayable` (with layer masks and additive layers),
  `AnimatorControllerPlayable`, and `AnimationPlayableOutput`. A mixer's
  output is the weighted blend of its inputs.
* Audio playables: `AudioClipPlayable`, `AudioMixerPlayable`, and
  `AudioPlayableOutput`, routed through their output's `AudioSource` and that
  source's mixer group as defined by AUD-001 in
  [UI, audio, and input](ui-audio-input.md).
* `ScriptPlayable<T>` with `PlayableBehaviour` callbacks: `OnGraphStart` when
  the graph starts playing, `OnBehaviourPlay` and `OnBehaviourPause` when the
  playable's play state changes, `PrepareFrame` for every playable before any
  `ProcessFrame` of the same frame, `OnGraphStop` when the graph stops, and
  `OnPlayableDestroy` when the playable is destroyed.
* Graphs that are not destroyed before their owner is destroyed or a play
  session ends are reported as leaked. Playable graphs, their nodes, outputs,
  weights, and time are inspectable during play.

## Particle effects

Particle systems import with duration, looping, start state, emission,
lifetime, shape, velocity, force, color, size, rotation, noise, collision,
triggers, sub-emitters, texture sheet animation, trails, lights, custom data,
renderer state (including custom vertex streams and mesh GPU instancing),
simulation space, scaling mode, culling mode, and prewarm behavior within
scope.

* `Play` starts emission and simulation, including child systems when
  `withChildren` is true. `Stop` with `StopEmitting` ends emission and lets
  live particles finish; with `StopEmittingAndClear` it also removes them.
  `Pause` freezes simulation; `Clear` removes live particles; `Emit(count)`
  spawns particles immediately and `Emit(EmitParams, count)` spawns them with
  the given overrides; `Simulate(t, withChildren, restart)` advances by `t`
  seconds; `GetParticles` and `SetParticles` read and replace live particles;
  `GetCustomParticleData` and `SetCustomParticleData` read and write custom
  data streams.
* Stop actions run when the system and its sub-emitters have no live
  particles after stopping: None does nothing, Disable deactivates the
  GameObject, Destroy destroys it, and Callback invokes
  `OnParticleSystemStopped` on its scripts.
* Culling mode Automatic pauses looping systems and keeps simulating
  non-looping systems when off-screen; Pause And Catch-up pauses and then
  advances by the elapsed time when visible again; Pause pauses; Always
  Simulate keeps simulating.
* With Send Collision Messages enabled, `OnParticleCollision` runs on the
  particle system's and the collided object's scripts, and
  `GetCollisionEvents` returns the contacts. `OnParticleTrigger` runs for
  trigger module events, and `GetTriggerParticles` returns the particles in
  each event category.
* `ParticleSystemForceField` components and the External Forces module
  (with its influence filter and layer mask) apply their forces to particles
  inside their shapes.

Effects can be previewed, restarted, paused, and scrubbed in the editor and
through automation for systems whose simulation is time-deterministic under
their random seed. Runtime control reports whether a system is playing,
emitting, paused, stopped, or complete. Bounds, sorting, material, layer, camera
visibility, and target limits are inspectable. Visual Effect Graph assets are
Unsupported as listed in the [index](INDEX.md).

## Sequencing and cinematics

Timeline 1.x assets import with their tracks and coordinate animation, audio,
activation, cameras, signals, control tracks, and property tracks. A sequence
exposes duration, tracks, clips, blends, extrapolation, markers, bindings, and
evaluation time.

The editor and automation can preview and scrub without persisting preview
state. At runtime, `PlayableDirector` play, pause, resume, stop, and seek
(`time` assignment) take effect at the next evaluation. Wrap mode Hold keeps the
last frame at the end, Loop restarts from zero, and None stops and raises the
`stopped` event. The time update mode selects game, unscaled, DSP, or manual
time. Nested sequences through control tracks play in step with their parent.
A signal fires its receiver's reactions once each time playback crosses it;
a retroactive signal also fires once when playback starts after it, and an
emit-once signal fires only on its first crossing. Missing bindings identify
the track and intended target. Custom tracks, clips, and mixers implemented
with playable scripts behave as defined in [Playable graphs](#playable-graphs).

## Camera behavior

Cinemachine 2.x camera rigs import with their virtual cameras, priority,
Follow and LookAt targets, body and aim behaviors, noise profiles, lens
settings, extensions (including confiners and collision avoidance),
state-driven, clear-shot, mixing, and blend-list cameras, and target groups.
The `CinemachineBrain` makes the highest-priority active virtual camera live,
choosing the most recently activated one among equal priorities. A change of
live camera blends with the matching custom blend from the brain's blend asset
or else the default blend. The brain updates in its configured update method:
smart, fixed, late, or manual.

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

Items that compare images use the rule in [Visual acceptance](#visual-acceptance).

1. Camera coordinate conversions land on the pixels where the converted points
   are drawn, multiple cameras and URP camera stacks contribute in depth and
   stack order, `Camera.main` resolves the first enabled `MainCamera`, and a
   camera without visible output reports its reason. (REN-001)
2. In a Built-in Render Pipeline project, `OnRenderImage` and
   `Camera.onPreRender` run in the order this document states and
   `OnRenderImage` output becomes the camera image; in a URP project they do
   not run, the compatibility report names the scripts that define them, and
   `RenderPipelineManager.beginCameraRendering` fires instead. (REN-001,
   REN-006)
3. Scenes using Built-in shader-family and URP materials pass visual validation
   against unitas-approved references in editor and player, and a
   `MaterialPropertyBlock` changes one renderer without creating a material
   instance. (REN-002)
4. Accessing `Renderer.material` and `MeshFilter.mesh` creates runtime
   instances visible in residency inspection that `Resources.UnloadUnusedAssets`
   releases after they are unreferenced, while `sharedMaterial` and
   `sharedMesh` edits change the asset; a 32-bit-index mesh with more than
   65,535 vertices renders and an out-of-range index fails with an error naming
   the mesh. (REN-002)
5. A script writes pixels with `SetPixels`, calls `Apply`, reads them back with
   `GetPixels`, copies a render target with `ReadPixels`, and reads a render
   texture with `AsyncGPUReadback`, each returning the written texels; reading
   a non-readable texture fails with an error naming it. (REN-002)
6. A scene rendered in Gamma and in Linear color space passes visual validation
   against the approved reference for each, a mid-gray sRGB texel samples as
   its linear value in Linear, and a target without Linear support reports it
   as Platform-limited naming the graphics API. (REN-002)
7. `WebCamTexture` plays after camera permission is granted and returns an
   explicit status when permission is denied or no camera exists. (REN-002)
8. A surface shader, a `CGPROGRAM` shader using `UnityCG.cginc` and
   `multi_compile` keywords, and a shader with `GrabPass` are translated and
   pass visual validation in a Built-in project, with the selected pass and
   variant compared exactly; in a URP project the `GrabPass` is reported as not
   rendered. (REN-002, REN-006)
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
    `DynamicGI.UpdateEnvironment` changes ambient and reflection lighting on
    the next frame; light cookies, flares, light culling masks, URP rendering
    layers, and the Built-in pixel light count produce the light assignment
    this document states, compared exactly; Enlighten realtime GI data is
    reported Unsupported. (REN-003)
13. `QualitySettings.SetQualityLevel` with `applyExpensiveChanges` false defers
    the anti-aliasing change and applies all other settings, the same call
    with true applies anti-aliasing, and a setting the target does not use is
    reported. (REN-003)
14. Each listed PPv2 and URP Volume effect passes visual validation with its
    blended parameters, and the contributing volumes for a camera position are
    inspectable; an unlisted effect type is reported rather than silently
    ignored. (REN-003)
15. Particle systems, trails, and line renderers report their playback state
    and produce the same particle count and positions under a fixed random
    seed in preview, play mode, and player, while a Visual Effect Graph asset
    is reported Unsupported. (REN-004)
16. `Emit` with `EmitParams`, `SetParticles`, custom data, a Destroy stop
    action, a Callback stop action invoking `OnParticleSystemStopped`,
    `OnParticleCollision`, `OnParticleTrigger`, and a force field each behave
    as this document states. (REN-004)
17. A 2D scene with sliced and tiled sprites, a late-bound Sprite Atlas, a
    Sorting Group, a Sprite Mask, URP 2D lights, a rule-tile Tilemap, a Pixel
    Perfect Camera, and a skinned 2D character passes visual validation, with
    draw order and pixel-snapped sprite positions compared exactly. (REN-005)
18. A URP 14.x project renders with each of Forward, Forward+, and Deferred and
    passes visual validation for each, with the per-object light limit applied
    only in Forward; an HDRP project and a custom SRP project are each reported
    Unsupported while remaining inspectable. (REN-006)
19. A Shader Graph material passes visual validation against the approved
    reference of an equivalent hand-written shader; an HDRP or Visual Effect
    Graph target is reported Unsupported; and a graph containing an
    unrepresentable node is reported with the graph, node, and affected
    materials. (REN-006)
20. A `ScriptableRendererFeature`, a URP `DecalProjector`, a camera
    `CommandBuffer`, `Graphics.DrawMeshInstanced`, and a compute shader
    dispatch each pass visual validation; on a target without compute the
    dispatch is reported Platform-limited; enabling the SRP Batcher changes
    only reported performance. (REN-006)
21. Global shader properties and keywords set from script select the variant
    compiled for the enabled keyword set, exceeding the stated keyword limit
    produces an error naming the shader and keyword, and after
    `ShaderVariantCollection.WarmUp` returns the listed variants are ready.
    (REN-006)
22. Failed or unsupported shaders cannot appear as successful compatibility.
    (REN-002, REN-006)
23. `Application.targetFrameRate`, `QualitySettings.vSyncCount`, fullscreen
    mode, resolution changes, and a second display take effect in a desktop
    player, and the achieved frame pacing is observable through automation.
    (REN-007)
24. A headless session renders frames at the requested resolution, or at the
    project's default player resolution when none is requested; with a
    time-control condition requested through AUT-002, frames and `Time`
    follow that condition; with none, frames advance as fast as they complete
    and `Application.targetFrameRate` limits the rate. (REN-007)
25. Visual comparison captures record every condition needed for evaluation,
    including target, resolution, quality level, color space, camera, and
    render pipeline, and a comparison without a declared tolerance is reported
    as not evaluable. (REN-001, REN-006, REN-007)
26. On each supported build target, a developer or agent can identify the
    effective graphics API and diagnose missing device capabilities without
    assuming Direct3D on Linux or macOS. (REN-006, REN-007)
27. Animation preview, play mode, and built-player playback produce the same
    pose and animation event sequence for the same sequence of frame times,
    including looping and reverse playback across events. (ANI-001)
28. Animator state, transition, trigger, blend-tree, and `StateMachineBehaviour`
    callback order follow this document and are observable during runtime.
    (ANI-001)
29. A controller mixing Write Defaults on and off states leaves each property at
    the value this document states after each transition, and disabling and
    re-enabling the Animator resets state, or keeps it when
    `keepAnimatorStateOnDisable` is true. (ANI-001)
30. Animators in Normal, Animate Physics, and Unscaled Time update modes advance
    at the points this document states with `Time.timeScale` at 0 and 1;
    culling modes decide off-screen state and events as stated; `CrossFade`
    and parameter setters called in `Update` are observed in that frame's
    evaluation. (ANI-001)
31. A humanoid clip applied to two avatars of different proportions produces
    equal muscle values on both with muscle limits honored, `OnAnimatorIK` and
    `OnAnimatorMove` alter the pose and motion, and an avatar mask restricts a
    layer. (ANI-002)
32. A `PlayableGraph` mixing two `AnimationClipPlayable` inputs at weights 0.25
    and 0.75 produces the weighted pose, a `ScriptPlayable` receives its
    callbacks in the stated order, an `AudioPlayableOutput` plays through its
    `AudioSource` and mixer group, and the legacy `Animation` component
    cross-fades legacy clips. (ANI-002)
33. A Timeline with animation, audio, activation, signal, and Cinemachine shot
    tracks plays, pauses, seeks, and completes under each wrap mode as stated,
    signals fire once per crossing, and the live virtual camera and active
    blend are inspectable during play. (CIN-001)
34. Every behavior difference between unitas and Unity 2022.3 in particles,
    animation, playables, Timeline, and Cinemachine appears in the AUT-006 API
    reference and in the project compatibility report for a project that uses
    the affected member. (REN-004, ANI-001, ANI-002, CIN-001)
