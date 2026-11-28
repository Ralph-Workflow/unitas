# Rendering, Cameras, Lighting, and Animation

## Scope

This specification defines the visible behavior of cameras, geometry,
materials, shaders, render pipelines, scripted rendering, lighting, shadows,
textures, render targets, 2D rendering, frame presentation, animation,
particles, and cinematic sequencing. It describes observable output and
diagnostics; the structure of rendering and animation is stated in
[Architecture](#architecture).

The compatibility baseline is Unity 2022.3 LTS, as defined in the
[Compatibility contract](compatibility-contract.md); projects saved by earlier
releases open after their serialized forms migrate to that baseline. URP,
HDRP, Scriptable Render Pipeline Core, Shader Graph, and Visual Effect Graph
14.x, Post Processing (v2) 3.x, the 2D packages, Timeline 1.x, Cinemachine
2.x, and other package content are covered only for the package
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
declared in the scenario or reference as TST-002 defines. The order of draws
whose order is visible (transparent, 2D, UI, and sorting-group content), each
renderer's reported sorting position, the presence of every expected object,
the shader pass selected for each renderer, and the set of lights affecting
each renderer are compared exactly. A comparison without a declared tolerance is reported as not
evaluable and is never reported as matching. Unity screenshots are not
acceptance references.

## Architecture

Rendering and animation follow the [Engine architecture](architecture.md).

* **Render proxies.** The renderer never reads scene objects. Change tracking
  keeps a render proxy for each renderer, light, camera, probe, decal, canvas
  batch, and particle system (transform, bounds, mesh and material handles,
  masks, sorting and lighting inputs), and the spatial structure that culls
  them, current as their sources change.
* **Frame handoff.** After `LateUpdate` and canvas rebuilds the frame's
  changes close; pipeline code records against that state, and `Submit` hands
  the work to render threads, which encode and submit GPU commands while the
  main thread simulates the next frame. The main thread waits on the GPU only
  where an API is synchronous by contract, such as a CPU read after
  `ReadPixels`.
* **One renderer, every pipeline.** URP, HDRP, and custom pipelines run their
  own unmodified C# code, which records through `ScriptableRenderContext`,
  `CommandBuffer`, and the SRP Core render graph into one native render graph;
  the Built-in pipeline is built from the same passes. Built-in features in
  scriptable pipelines, backend emulation, and capture outputs are passes in
  it. The graph orders, synchronizes, aliases, and merges passes from their
  declared reads and writes, drops unread ones, records independent ones in
  parallel, and is what rendering diagnostics show.
* **Culling and batching.** Frustum, layer, LOD, occlusion, and per-eye
  culling run as jobs and produce `CullingResults`. Sorting fixes only the
  order the sorting rules make visible. Per-object and per-material constants
  live in persistent GPU buffers rewritten only where something changed, and
  compatible draws are instanced or indirect where the device allows; none of
  this alters the image.
* **Shaders and resources.** Shaders translate at import into cooked
  per-backend variants; a player never compiles shader source. GPU pipelines
  are built from cooked variants, cached per device and driver, and
  pre-created by warm-up. Meshes and textures upload asynchronously; mipmap
  streaming follows the visible set and quality budget.
* **Animation and particles.** Transitions, parameters, and graph topology
  resolve on the main thread at each system's Unity update point; sampling,
  blending, retargeting, and IK run as jobs between the script callbacks this
  document orders and write back to native transforms in one pass before the
  next observing callback. Events and callbacks dispatch in an order
  independent of job scheduling. Skinning runs on the GPU from the pose CPU
  queries read. Particle systems simulate as jobs; Visual Effect Graph
  simulates in render-graph compute passes.
* **Editor, headless, player.** The worker renders the edited project's game,
  scene, and preview views with the project's pipeline; the editor renders its
  own interface with the same renderer. Headless sessions, captures, identity
  maps, and bakers use the same graph and translated shaders. Per-frame work
  scales with change and visibility, and the engine's render path allocates
  no managed memory per frame.

## Graphics API and target capability

Rendering draws through `wgpu` on every target. Each target's graphics API is
the `wgpu` backend it uses there, such as Vulkan, Metal, Direct3D 12, OpenGL
ES, WebGL 2, or WebGPU. Support for a target does not imply that any other
target's API runs on it. ShaderLab and HLSL describe accepted project content,
not the API used to draw it.

For each supported build target and current host, developers can inspect the
selected graphics API and version, the device and driver where the host
reports them, and, for each rendering feature named in this document, whether
the graphics API provides it directly, unitas provides it on that device by
other means, or the device's hardware lacks it. If no usable graphics device or
API is available, startup or build fails with an actionable reason; it does not
silently claim a working renderer. A change of graphics API does not change the
visible draw order, object presence, shader pass selection, or light assignment of a
scene.

A feature that a graphics API lacks but the device can perform is provided by
unitas on that API. For example, geometry and tessellation programs run on
graphics APIs that have no such stages, and Linear color space renders on
graphics APIs without sRGB formats. The result passes the same visual
validation as on an API with the feature. Only a feature that the device's
hardware cannot perform, such as hardware ray tracing on a device without it,
is reported as Platform-limited. That report names the device, graphics API,
and feature, and states what renders instead.

## Render pipeline scope

unitas renders projects authored for the Built-in Render Pipeline, the
Universal Render Pipeline (URP) 14.x, the High Definition Render Pipeline
(HDRP) 14.x, or a project-defined scriptable render pipeline. The project's
pipeline is determined by the Graphics settings render pipeline asset and the
per-quality-level override; the effective pipeline asset for each quality
level and target is inspectable. Assigning `GraphicsSettings.renderPipelineAsset`
or `QualitySettings.renderPipeline`, or switching to a quality level with a
different pipeline asset, changes the pipeline from the next rendered frame.

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
* **HDRP 14.x projects:** HDRP assets, settings, materials, lights, and
  volumes are imported and translated as defined in
  [High Definition Render Pipeline](#high-definition-render-pipeline).
* **Custom scriptable render pipelines:** a project whose render pipeline asset
  is a project-defined `RenderPipelineAsset` subclass renders through that
  pipeline's scripts, as defined in
  [Custom scriptable render pipelines](#custom-scriptable-render-pipelines).

A render pipeline asset whose package is absent or whose version is outside the
matrix is reported with the affected quality levels and cameras rather than
silently falling back to another pipeline.

### Built-in features in URP and HDRP projects

Unity 2022.3 does not run some Built-in Render Pipeline features under URP or
HDRP. unitas runs them as unitas behavior: the Built-in camera callbacks
([Camera callbacks and depth textures](#camera-callbacks-and-depth-textures)),
`GrabPass` and passes whose `LightMode` tag is Built-in only
([ShaderLab and HLSL content](#shaderlab-and-hlsl-content)), and the
`Projector` component ([Scripted rendering](#scripted-rendering)).

The project setting Built-in features in scriptable pipelines controls this
and is on by default. When it is off, unitas keeps the Unity 2022.3 behavior:
those features do not run, and the project compatibility report names every
affected script, shader, pass, and component with that consequence. When it is
on, the report still lists them, so a project that worked around their absence
in Unity can find the workaround and turn the setting off. The setting does not
apply to custom scriptable render pipelines, whose scripts define the frame.

### High Definition Render Pipeline

HDRP 14.x content renders with its documented meaning, and its public types
and members keep their Unity 2022.3 names, signatures, and documented meaning.

* **Assets and settings:** HDRP assets per quality level, HDRP Global
  Settings, default and per-camera Frame Settings, the Lit shader mode
  (forward, deferred, or both), diffusion profile lists, rendering layers, the
  default volume profile, and dynamic resolution settings are imported and
  translated. `HDAdditionalCameraData`, `HDAdditionalLightData`, and
  `HDAdditionalReflectionData` keep their serialized values and effects.
* **Materials:** HDRP Lit, Layered Lit, their tessellation variants, Unlit,
  StackLit, Fabric, Hair, Eye, AxF, Decal, and Terrain Lit materials, and HDRP
  Shader Graph materials, are translated with their surface type, blend mode,
  material type (standard, subsurface scattering, translucent, anisotropy,
  iridescence, and specular color), double-sided mode, displacement mode,
  refraction, distortion, transparent depth prepass and postpass, and sorting
  priority. Subsurface scattering and transmission use the assigned diffusion
  profile. A material whose diffusion profile is missing from the active list
  is reported against the material and renders with the default profile.
* **Lights and shadows:** physical light units (lumen, candela, lux, nits, and
  EV100), color temperature, rectangle, tube, and disc area lights in
  realtime, IES profiles, cookies, light layers, shadow filtering quality,
  contact shadows, micro shadows, and shadow update modes (every frame, on
  enable, and on demand) take effect.
* **Volumes:** HDRP volume overrides blend by the rules in
  [Post-processing](#post-processing). They include HDRI, physically based,
  and gradient skies; cloud layer and volumetric clouds; fog and volumetric fog,
  with Local Volumetric Fog components; exposure (fixed, automatic, curve
  mapping, automatic histogram, and physical camera); visual environment;
  indirect lighting controller; screen space ambient occlusion, reflection,
  global illumination, and refraction; shadows and contact shadows; and the
  HDRP post-processing effects: bloom, tonemapping, color adjustments, white
  balance, lift-gamma-gain, shadows-midtones-highlights, color curves, channel
  mixer, split toning, vignette, physically based depth of field, per-object
  motion blur, panini projection, lens distortion, chromatic aberration, and
  film grain.
* **Probes, decals, and water:** reflection probes, planar reflection probes,
  light probe groups, probe volumes, HDRP decal projectors with decal layers,
  and water surfaces render with their settings.
* **Ray tracing:** ray-traced shadows, ambient occlusion, reflections, global
  illumination, subsurface scattering, recursive rendering, and the path
  tracer render on devices with ray tracing hardware. On a device without it,
  each ray-traced effect is reported as Platform-limited naming the device,
  and renders through the effect's screen-space mode.
* **Custom passes and custom post-processes:** Custom Pass Volumes,
  full-screen and draw-renderers custom passes, `CustomPass` subclasses, and
  `CustomPostProcessVolumeComponent` subclasses listed in the global settings
  run at their injection points.
* **Anti-aliasing and upscaling:** temporal anti-aliasing, SMAA, FXAA, MSAA in
  forward mode, and dynamic resolution with TAA Upscale and FidelityFX Super
  Resolution 1 take effect. Vendor upscalers render on devices and drivers
  whose vendor provides them; elsewhere the camera reports the upscaler
  Externally unavailable and uses the next upscaler its settings allow.

### Custom scriptable render pipelines

A project-defined pipeline runs through the scriptable render pipeline API
with its Unity 2022.3 names, signatures, and documented meaning. The image is
the one the pipeline's recorded commands produce.

* `RenderPipelineAsset.CreatePipeline` runs before the first frame that needs
  the pipeline. Changing the active pipeline asset, or validating the asset in
  the editor, disposes the current `RenderPipeline` instance and creates a new
  one. `RenderPipeline.Dispose` and `disposed` keep their meaning.
* `RenderPipeline.Render(ScriptableRenderContext, List<Camera>)` runs once per
  frame with the cameras to render in that frame, including scene view,
  preview, and reflection probe cameras in the editor. `BeginContextRendering`,
  `BeginCameraRendering`, `EndCameraRendering`, and `EndContextRendering`
  raise the matching `RenderPipelineManager` events.
* `ScriptableRenderContext` records `SetupCameraProperties`, `DrawRenderers`,
  `DrawRendererList`, `DrawShadows`, `DrawSkybox`, `DrawGizmos`,
  `DrawUIOverlay`, `DrawWireOverlay`, `ExecuteCommandBuffer`,
  `ExecuteCommandBufferAsync`, `BeginRenderPass`, `BeginSubPass`,
  `EndSubPass`, `EndRenderPass`, and `InvokeOnRenderObjectCallback`. Recorded
  work executes in recorded order at `Submit`. Work recorded but never
  submitted does not render, and the camera reports that nothing was
  submitted.
* `Camera.TryGetCullingParameters`, `ScriptableCullingParameters`,
  `ScriptableRenderContext.Cull`, and `CullingResults` return the visible
  renderers, visible lights and their index map, visible reflection probes,
  shadow caster bounds, and the shadow matrices and split data computed by
  `ComputeDirectionalShadowMatricesAndCullingPrimitives`,
  `ComputeSpotShadowMatricesAndCullingPrimitives`, and
  `ComputePointShadowMatricesAndCullingPrimitives`. Culling honors culling
  masks, culling options, occlusion culling data, and LOD.
* `DrawingSettings`, `SortingSettings`, `FilteringSettings`,
  `RenderStateBlock`, `ShaderTagId`, `RendererListDesc`, and per-object data
  flags select passes by `LightMode` tag, with a pass that has no tag matching
  `SRPDefaultUnlit`. They sort, filter by render queue range, layer mask, and
  rendering layer mask, and supply the requested per-object data: lightmaps,
  light probes, reflection probes, light indices, and motion vectors.
* `CommandBuffer` contents recorded by the pipeline run as defined in
  [Scripted rendering](#scripted-rendering).
* `GraphicsSettings.lightsUseLinearIntensity`, `lightsUseColorTemperature`,
  `useScriptableRenderPipelineBatching`, `SupportedRenderingFeatures`,
  `Shader.globalRenderPipeline`, `RenderPipelineGlobalSettings`, and the
  Scriptable Render Pipeline Core 14.x content a pipeline uses, such as
  `RTHandles`, the render graph, the volume framework, and the debug manager,
  keep their meaning.
* An exception thrown from pipeline code is logged with its stack trace and
  script location. The cameras of that frame show the work submitted before
  the exception, and the next frame calls `Render` again. A pipeline asset
  whose `CreatePipeline` returns null renders nothing and is reported with the
  asset named.

### Stereo rendering

Stereo rendering for XR devices follows XR-001 in
[Builds, platforms, XR, and networking](build-platforms-xr-networking.md),
which owns the devices, runtimes, and render views. This section owns how each
render pipeline draws those views.

* Single-pass instanced and multi-pass stereo render in Built-in Render
  Pipeline, URP 14.x, and HDRP 14.x projects, selected by the XR provider
  settings for each target. In a custom scriptable render pipeline, stereo
  follows the commands the pipeline records, through
  `ScriptableRenderContext.StartMultiEye`, `StereoEndRender`, and
  `StopMultiEye` and the XR display subsystem's render passes.
* The active stereo mode is reported for each camera and target, with the
  reason when it differs from the requested mode. A device whose runtime
  cannot accept single-pass instanced output renders multi-pass and reports
  that substitution.
* Both modes produce the same image for each eye. Every translated shader
  renders in both eyes in single-pass instanced mode, including a project
  shader that lacks the stereo instancing declarations. Such a shader is listed
  in the compatibility report as relying on unitas behavior, because in Unity
  2022.3 it draws in only one eye.
* Culling runs for each eye, or once with the combined eye frustum, as the
  pipeline and mode select. No renderer visible in either eye is culled.
  `Camera.stereoActiveEye`, `GetStereoViewMatrix`,
  `GetStereoProjectionMatrix`, `stereoSeparation`, and `stereoConvergence`
  report each eye's values.
* Post-processing runs for each eye with that eye's view, projection, and
  depth. Screen-space effects never sample across the boundary between eyes.
* Screen Space - Camera and World Space canvases draw in both eyes at their
  depth. As unitas behavior, a Screen Space - Overlay canvas also draws in
  both eyes, as a head-locked plane at the distance the project's XR overlay
  canvas setting gives, and receives pointer input from the XR interaction
  that targets it. Turning that setting off keeps the Unity 2022.3 behavior,
  in which overlay canvases draw only to the desktop mirror view. The
  compatibility report names each overlay canvas in an XR scene.
* Foveated rendering, fixed or eye-tracked, takes effect on devices whose
  runtime exposes it, controlled through the XR display subsystem's foveated
  rendering level and flags. On a device without it, the requested level is
  reported as Platform-limited and the frame renders at full resolution.
* Test captures can record the left eye, the right eye, both eyes side by
  side, or the mirror view. Each capture records the stereo mode and eye with
  the conditions in [Visual acceptance](#visual-acceptance). A headless
  session renders stereo through a simulated device whose eye poses,
  resolution, and stereo mode the scenario declares.

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
* In URP and HDRP projects with Built-in features in scriptable pipelines on
  (see [Built-in features in URP and HDRP projects](#built-in-features-in-urp-and-hdrp-projects)),
  these callbacks also run for each camera. `OnPreCull` runs before the camera
  culls, `OnPreRender` after culling and before drawing, and `OnPostRender`
  after the camera's drawing and before its post-processing. `OnRenderImage`
  receives the camera's color after the pipeline's post-processing, and its
  output becomes the camera's image. In a URP camera stack, the callbacks run
  for each camera in stack order, and `OnRenderImage` on the base camera
  receives the stack's final color. With the setting off, they are not
  invoked, and the project compatibility report names every script that
  defines them, with the consequence that they do not run.
* In URP and HDRP projects, `RenderPipelineManager.beginContextRendering`,
  `beginCameraRendering`, `endCameraRendering`, and `endContextRendering` fire
  in that order for each frame, with the camera events once per camera in
  render order.

## Mesh and sprite rendering

Static meshes, skinned meshes, sprites, tilemaps, lines, trails, and particle
renderers participate in camera culling, transforms, sorting, layers,
materials, lighting, and shadows according to their feature contracts.

A renderer exposes its source geometry, material slots, bounds, enablement,
sorting state, shadow participation, and visibility reason. Meshes of every
`MeshTopology` (triangles, quads, lines, line strips, and points) render on
every target, including graphics APIs without quad primitives. Missing
geometry, invalid bounds, absent materials, index data inconsistent with its
topology, and device limitations are distinct diagnostics.

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
[Built-in shaders](#built-in-shaders), the URP Lit, Complex Lit, Simple Lit,
Unlit, Baked Lit, Terrain Lit, Particles (Lit, Simple Lit, and Unlit), Decal,
SpeedTree, Autodesk Interactive, and 2D sprite materials, and the HDRP
materials listed in [High Definition Render Pipeline](#high-definition-render-pipeline)
are translated to unitas materials. Metallic,
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
these meanings. Linear color space renders on every target. On a graphics API
without sRGB texture or render-target formats, unitas performs the sRGB
conversions itself, and the result passes the same visual validation as on an
API with those formats. Linear never silently renders as Gamma.

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
  `hasError` reports failure, and `GetData` returns the texels. On a graphics
  API without asynchronous readback, the request still completes on a later
  frame with the texels, `SystemInfo.supportsAsyncGPUReadback` returns true,
  and the stall the copy costs is reported as a performance property.
  `hasError` is true only when the source is released or invalid before the
  request completes.
* `RenderTexture` creation, `Create`, `Release`, `IsCreated`,
  `RenderTexture.active`, temporary render textures (`GetTemporary` and
  `ReleaseTemporary`), depth buffers, MSAA, and random-write enablement keep
  their documented meaning. When the device cannot create a requested format
  directly, unitas stores it in a device format that holds every requested
  channel at the requested precision or higher. Reads, writes, and sampling
  behave as the requested format, and the storage format is reported as a
  performance property. Only a format whose channels or precision no device
  format can hold, such as a floating-point render target on hardware without
  one, is reported as Platform-limited with the format and device named. It is
  never silently replaced with a lower-precision format.
* Render textures expose size, format, depth, update state, usage, and content
  validity. Feedback cycles and formats no device format can hold are
  diagnosed rather than producing unexplained stale imagery.
* `WebCamTexture` plays the selected camera device after the target's camera
  permission is granted, on desktop, mobile, and browser targets, and on
  console targets when the platform SDK is installed. A denied permission or
  an absent camera returns an explicit status and an empty
  `WebCamTexture.devices` list or a texture that reports it is not playing.
* Texture formats and compression that a target does not support are
  transcoded at import or build into a format the target supports, and the
  texture renders through it. The asset, the target, and the format used are
  reported.

## Shaders

ShaderLab and HLSL assets are imported and translated. A translated shader
retains its shader name, properties, subshaders, passes, tags, render states,
variants, and keywords.

Every ShaderLab and HLSL construct that Unity 2022.3 compiles for a target
translates for that target. A shader that fails to compile, because of a
syntax or semantic error, a missing include, or a missing referenced asset,
identifies the shader, pass, variant, property, and target where known. A
failed shader is unmistakable in the scene
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
  tessellation programs, and `#pragma target` and `#pragma require` levels.
  Geometry and tessellation programs run on every device that can run compute
  shaders, including graphics APIs that have no such stages. On WebGL 2, which
  has no compute stage, they follow the compute rule of the Web target in
  [Web target](web-target.md#render-pipelines-and-features-on-the-web). A shader whose
  target level exceeds the device's capability falls through to the next
  subshader or its `Fallback`.
* Built-in includes (`UnityCG.cginc`, `Lighting.cginc`, `AutoLight.cginc`,
  `UnityShaderVariables.cginc`, and the other Built-in shader includes), the
  URP 14.x and HDRP 14.x ShaderLibrary, and the Scriptable Render Pipeline
  Core 14.x shader includes.
* Ray tracing shaders (`.raytrace` assets and ray tracing passes in
  ShaderLab) on devices with ray tracing hardware. On a device without it,
  `SystemInfo.supportsRayTracing` returns false and ray tracing work is
  reported as Platform-limited with the device named.
* `multi_compile`, `shader_feature`, their `_local` forms, and their
  vertex-only and fragment-only variants.
* `Fallback`, `UsePass`, `GrabPass`, `Category`, `LOD`, and fixed-function
  render state (blend, blend operation, color mask, cull, depth test and
  write, offset, stencil, and alpha-to-mask).

In URP and HDRP projects with Built-in features in scriptable pipelines on
(see [Built-in features in URP and HDRP projects](#built-in-features-in-urp-and-hdrp-projects)),
a `GrabPass` captures the camera color at the point where the pass draws, into
the texture name it declares or `_GrabTexture`. Passes whose `LightMode` tag is
Built-in only, such as `ForwardBase`, `ForwardAdd`, `Vertex`, `VertexLM`, and
`Always`, render in the pipeline's forward drawing, including the passes that
surface shaders generate. Their Built-in lighting variables, ambient, and
lightmap inputs are filled from the pipeline's lights and environment:
`ForwardBase` draws once per renderer with the main light, and `ForwardAdd`
draws once for each additional light that affects the renderer. A shader with a
subshader for the active pipeline uses that subshader, so a shader that already
provides URP or HDRP passes is unaffected. With the setting off, these passes
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
* A player never compiles shader source. First use of a variant that was not
  warmed up creates only its GPU pipeline from cooked data, and each such
  creation during play is reported in profiling with the shader, pass,
  variant, and duration.

## Shader Graph

Shader Graph 14.x assets are imported as shader content for the Built-in,
URP, HDRP, and Visual Effect Graph targets that their graph settings select. A
Shader Graph asset retains its exposed properties, keywords, sub graph
references, graph targets, surface options, and material interface, and a
material using it behaves as a material using an equivalent hand-written
shader.

* Sub graphs, keywords (boolean and enum), exposed and hidden properties,
  custom interpolators, and Custom Function nodes with inline or file-based
  HLSL are Supported.
* Every graph target of Shader Graph 14.x is Supported: the Built-in, URP,
  and HDRP targets with each of their material types, including the HDRP
  Lit, StackLit, Hair, Fabric, Eye, Unlit, Decal, Fullscreen, and Water
  materials, and the Visual Effect Graph target used by Visual Effect Graph
  Shader Graph outputs ([Visual Effect Graph](#visual-effect-graph)).
* Every node that Shader Graph 14.x provides is Supported. A graph that
  references a node type whose package or script is missing is reported with
  the graph asset, the node type, the node identity within the graph, and
  every material affected. The graph is not reported as Supported, and the
  material renders as a failed shader.
* The unitas editor draws and edits Shader Graph assets and sub graphs in a
  Shader Graph editor. It shows every node with its input and output ports,
  port types, and default values; the connections between ports; groups and
  sticky notes; the blackboard with exposed and hidden properties, keywords,
  and categories; and the graph inspector with targets, material settings,
  and node settings. Users create, move, delete, and connect nodes, edit port
  defaults, add and reorder properties and keywords, create a sub graph from a
  selection, and open a sub graph node to edit that sub graph. A main preview
  on a selectable mesh and a preview on each node show the result as the
  graph changes. Edits take part in undo and redo (EDT-005), and saving the
  graph updates every material that uses it.
* Authorized agents edit the same graphs through MCP, as defined in
  [Automation and MCP](automation-and-mcp.md). They read and write a graph as
  structured data: nodes with type, identity, position, settings, and port
  defaults; connections from an output port to an input port; properties;
  keywords; targets; and sub graph references. Agent edits are validated by
  the same rules as editor edits and appear in the editor as they apply.
* An edit that connects ports of incompatible types, creates a cycle,
  references a missing sub graph, or duplicates a property or keyword
  reference name is refused with the reason, and the graph is left unchanged.
* A saved graph remains a Shader Graph 14.x asset that Unity 2022.3 LTS opens
  with the same nodes, connections, properties, keywords, targets, positions,
  groups, and sticky notes. Data that unitas does not edit is preserved.

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
  size, pivot, fade, and decal layers. HDRP decals are defined in
  [High Definition Render Pipeline](#high-definition-render-pipeline).
* **Projectors:** in Built-in Render Pipeline projects, and in URP and HDRP
  projects with Built-in features in scriptable pipelines on, the `Projector`
  component projects its material onto renderers inside its frustum. It
  honors near and far clip planes, field of view or orthographic size, aspect
  ratio, and ignored layers. With the setting off in URP and HDRP projects,
  projectors do not render and each is reported with that consequence.
* **Command buffers:** `CommandBuffer` contents execute in recorded order
  through `Graphics.ExecuteCommandBuffer`, through camera and light events in
  Built-in Render Pipeline projects (`Camera.AddCommandBuffer`,
  `Light.AddCommandBuffer`), and through `ScriptableRenderContext` in renderer
  passes, HDRP custom passes, and custom scriptable render pipelines. Command
  buffers attached to a camera are inspectable on that camera.
* **Ray tracing from script:** `RayTracingAccelerationStructure`,
  `RayTracingShader`, and `CommandBuffer.DispatchRays` build acceleration
  structures and dispatch rays on devices with ray tracing hardware, with the
  Platform-limited state stated in [ShaderLab and HLSL content](#shaderlab-and-hlsl-content).
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
* **Draw efficiency:** draws that share a shader pass reuse persistent
  per-object and per-material GPU data, and compatible renderers are instanced
  wherever the device supports it, whether or not the project enabled
  instancing on the material. Draw, batch, and state-change counts are
  measured against the release's declared frame-time budgets (PRF-002).
* **Compute shaders:** `ComputeShader` assets, kernels, `Dispatch`,
  `ComputeBuffer` and `GraphicsBuffer` data, and random-write textures are
  Supported on every device that can run compute work, on desktop, mobile,
  console, and browser targets. On WebGL 2, which has no compute stage, compute
  passes are lowered by the render graph to passes the API can run, as the
  [Web target](web-target.md) defines; there is no second implementation. On a device whose hardware cannot run compute work,
  `SystemInfo.supportsComputeShaders` returns false, compute usage is
  reported as Platform-limited with the asset and device named, and dispatch
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
* Area lights (rectangle and disc) contribute through baked lighting data,
  whether Unity or unitas baked it. In HDRP projects they also render in
  realtime with shadows. In Built-in and URP projects, as unitas behavior, an
  area light whose mode is Realtime or Mixed also renders in realtime, with
  its shape, size, range, and shadows. Turning off the project setting
  Realtime area lights keeps the Unity 2022.3 behavior for those pipelines, in
  which area lights contribute only through baked data. The compatibility
  report names each area light whose result depends on this setting.
* Enlighten realtime global illumination is Supported in every pipeline when
  the lighting settings enable Realtime Global Illumination. Static renderers
  that contribute global illumination receive realtime indirect lighting from
  realtime and mixed lights, from emissive materials whose global illumination
  flag is Realtime, and from the environment. Dynamic renderers receive it
  through light probes. Changing a light's color, intensity, indirect
  multiplier, direction, or position, calling `DynamicGI.SetEmissive`,
  `Renderer.UpdateGIMaterials`, or `DynamicGI.UpdateEnvironment` updates
  indirect lighting over the following frames, as throttled by
  `DynamicGI.updateThreshold` and `DynamicGI.materialUpdateTimeSlice`.
  `DynamicGI.indirectScale` scales the result, and `DynamicGI.isConverged`
  reports when updates have settled. `Renderer.realtimeLightmapIndex` and
  `realtimeLightmapScaleOffset` keep their meaning.
* Realtime global illumination uses the precomputed data of a Unity or unitas
  bake. A scene without that data renders without realtime indirect lighting
  and reports the missing precompute with the scene named; stale precompute is
  reported as defined below.

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
lights cast shadows in Built-in, URP, HDRP, and custom pipeline projects, and
area lights cast realtime shadows wherever they render in realtime.

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

Lightmaps, light probes, reflection probe captures, realtime global
illumination precompute, and occlusion culling data baked by Unity or by
unitas are Supported project content. Rendering uses them with their
renderer assignments, lightmap indices and scale-offsets, contribution modes
(contribute global illumination, receive from lightmaps or light probes),
mixed lighting modes (Baked Indirect, Shadowmask, Subtractive), shadow masks,
and the relationship between baked and realtime lights. Light probe groups
expose their probe positions and interpolation; reflection probes expose their
influence volume, blending, box projection, importance, and captured result.
Realtime reflection probes update according to their refresh mode and
time-slicing settings.

unitas bakes lighting and occlusion data in the editor and in headless
sessions. Users start a bake from the Lighting and Occlusion Culling windows;
scripts use the `Lightmapping` and `StaticOcclusionCulling` editor APIs; and
authorized agents and continuous integration use automation and the command
line (BLD-002). Every route produces the same data.

* **Lightmaps:** a bake uses the scene's Lighting Settings asset (lightmapper,
  sample counts, bounces, lightmap resolution, padding, maximum lightmap size,
  compression, directional mode, ambient occlusion, and filtering), each
  renderer's lightmap parameters and scale in lightmap, each mesh's lightmap
  UVs (generated when its import settings request them), the mixed lighting
  mode, and each light's mode. It produces lightmaps, directional maps, and
  shadowmasks. `Lightmapping.Bake`, `BakeAsync`, `BakeMultipleScenes`,
  `Cancel`, `Clear`, `isRunning`, `bakeStarted`, and `bakeCompleted` keep their
  meaning. With Auto Generate on, unitas rebakes stale data in the background
  after each edit that makes it stale.
* **Light probes and reflection probes:** a bake computes every light probe
  group's probes and tetrahedralization and captures every baked reflection
  probe. `Lightmapping.BakeReflectionProbe` captures one probe to a file, and
  custom reflection probes keep their assigned cubemaps.
* **Realtime global illumination precompute:** with Realtime Global
  Illumination enabled, a bake produces the precompute that
  [Lighting and environment](#lighting-and-environment) uses.
* **Occlusion culling:** `StaticOcclusionCulling.Compute`,
  `GenerateInBackground`, `Cancel`, `Clear`, and `isRunning` bake occlusion
  data from occluder and occludee static flags, occlusion areas, occlusion
  portals, and the smallest occluder, smallest hole, and backface threshold
  parameters.
* **Output:** the lighting data asset, lightmap, directional, and shadowmask
  textures, reflection probe cubemaps, and occlusion culling data are written
  into the project in the forms and locations Unity 2022.3 uses, and assigned
  to their scenes. Unity 2022.3 opening the project renders with that data
  without rebaking. A bake passes visual validation against unitas-approved
  references; pixel identity with a Unity bake is not promised.
* **Progress:** a running bake reports its stage, fraction complete, elapsed
  time, estimated time remaining, and the scene and object count of the
  current stage. The editor shows it as a progress indicator,
  `Lightmapping.buildProgress` returns the fraction, automation receives
  progress events, and a headless session writes progress to its output.
* **Cancellation and failure:** cancelling a bake leaves the previously baked
  data unchanged. A failed bake names the cause and the affected objects, for
  example overlapping or missing lightmap UVs or insufficient memory, and also
  leaves the previous data unchanged.
* **Hosts and editing:** bakes run on hosts with or without a GPU. Edits made
  while a background bake runs do not change that bake's input; when it
  completes, the data those edits affect is marked stale.

When an edit invalidates baked data, for example moving or changing a static
renderer that contributes to global illumination, changing a baked light, or
changing an occluder, the affected baked data is marked stale. The stale
state names the affected lighting data asset, occlusion data, and every
renderer whose baked result no longer matches. Stale data continues to render
as it was baked until it is replaced; it is not silently discarded. A missing
bake, a stale bake, and baked data that is corrupt or does not match its scene
remain three distinct states.

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
* Auto exposure and screen space reflections (PPv2).
* Color lookup (PPv2 Color Grading LUT modes; URP Color Lookup), split
  toning, and Panini projection (URP).
* Anti-aliasing: FXAA, SMAA, and temporal anti-aliasing, selected on the PPv2
  Post-process Layer or the URP camera.

Project-defined effects run with their blended parameters in the same way:
PPv2 `PostProcessEffectSettings` and `PostProcessEffectRenderer` pairs at
their injection point, and URP `VolumeComponent` subclasses read by renderer
features or by `VolumeManager.instance.stack`. HDRP effects are listed in
[High Definition Render Pipeline](#high-definition-render-pipeline). An effect
or override whose script type is missing is preserved and reported with the
volume profile, the effect type, and its state. It does not silently render as
if it were disabled while the profile is reported as Supported.

## Lines and trails

Line and trail renderers retain points, width curve, color gradient, material,
alignment, texture mode, sorting, lifetime, and visibility within scope.

## Transparency and ordering

Within each camera, content is ordered first by render queue, then by sorting
layer and order in layer, then by distance along the camera's sort axis:
transparent content back to front, and opaque content by its requested sorting
criteria. Any further reordering of opaque draws, for depth rejection or for
state coherence, cannot be observed and is not compared. Alpha-tested content
writes depth like opaque content. The editor and automation expose the
effective ordering inputs and resulting position in the draw order for a
selected renderer.

Transparent renderers sort per renderer, as in Unity 2022.3, so two
intersecting transparent renderers draw in the order their sorting inputs
give. The editor and automation identify transparent renderers whose bounds
intersect, so authors can adjust their sorting. Precision and target
variation follow the tier rules of the compatibility contract.

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
* Multiple displays are Supported on desktop targets, and on mobile and
  console targets whose device has secondary or external displays:
  `Display.displays`, `Display.Activate`, camera target display, and canvas
  target display render to the selected outputs. A device with one display
  reports one display in `Display.displays`.
* HDR display output is Supported on every target whose device and display
  report HDR output support, with the HDR output state and the selected
  tonemapping and paper-white settings inspectable. On a device or display
  without HDR output, the HDR output state reports it unavailable and frames
  present in SDR.
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
pose. Events and `StateMachineBehaviour` callbacks of different animators in
one frame are dispatched on the main thread in a stable order that does not
depend on job scheduling; the AUT-006 API reference states that order.

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
* Visibility for animator culling means visible to any camera in the most
  recent completed culling, which is the previous rendered frame's. Culling
  mode Always Animate advances state and writes transforms when no
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
* Off-screen for particle culling means not visible to any camera in the most
  recent completed culling, which is the previous rendered frame's. Culling
  mode Automatic pauses looping systems and keeps simulating non-looping
  systems when off-screen; Pause And Catch-up pauses and then
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
visibility, and target limits are inspectable. Visual Effect Graph effects are
defined in [Visual Effect Graph](#visual-effect-graph).

### Visual Effect Graph

Visual Effect Graph 14.x effects simulate and render on the GPU in Built-in,
URP, and HDRP projects. Public types and members keep their Unity 2022.3
names and signatures, and every behavior difference is listed in the AUT-006
API reference.

* Visual effect assets and subgraphs import with their systems, contexts
  (Event, Spawn, GPU Event, Initialize Particle, Update Particle, and Output),
  blocks, operators, custom attributes, capacity, bounds mode, simulation
  space, and exposed properties.
* Output contexts render quads, triangles, octagons, lines, meshes, particle
  strips, points, decals, and distortion, and Shader Graph outputs render
  with their graph, each with its blend mode, sorting, culling, and lighting.
* The `VisualEffect` component supports `Play`, `Stop`, `Reinit`,
  `SendEvent` with `VFXEventAttribute`, `pause`, `playRate`, `Simulate`,
  `startSeed`, `resetSeedOnPlay`, `aliveParticleCount`, `culled`,
  `HasAnySystemAwake`, exposed property getters and setters, and
  `outputEventReceived`. Property binders and event binders apply their
  bindings every frame.
* Spawn contexts emit at constant rate, in bursts, in periodic bursts, at
  variable rate, and through `VFXSpawnerCallbacks` scripts. GPU events trigger
  on die, always, and over time, and spawn into their target systems.
* Collision with primitive shapes, the depth buffer, and signed distance
  fields, point caches, mesh and skinned mesh sampling, and texture sampling
  take effect. `VFXManager` fixed time step and maximum delta time govern
  simulation steps.
* Timeline Visual Effect Activation tracks send their clip events at clip
  start and end ([Sequencing and cinematics](#sequencing-and-cinematics)).
* A system outside the camera frustum by its bounds stops simulating while
  culled, as its culling flags state.
* Effects can be previewed, restarted, paused, and scrubbed in the editor and
  through automation. With a fixed seed, the same effect produces the same
  particle count on the same device in preview, play mode, and player.
* The unitas editor draws and edits visual effect assets and subgraphs in a
  Visual Effect Graph editor. It shows contexts with their blocks, operators
  with their input and output ports, flow connections between contexts and
  property connections between ports, groups and sticky notes, and the
  blackboard with exposed properties, custom attributes, and categories.
  Users create, delete, reorder, and connect contexts, blocks, and operators,
  edit settings and port values, create subgraph blocks and operators from a
  selection, and open a subgraph to edit it. A visual effect in the scene, or
  a preview effect, plays the graph as it changes, restarting when an edit
  changes a system's structure. Edits take part in undo and redo (EDT-005).
* Authorized agents edit the same graphs through MCP as structured contexts,
  blocks, operators, settings, flow connections, property connections,
  blackboard entries, and subgraph references, validated by the same rules as
  editor edits. An edit that connects incompatible ports or contexts, or
  references a missing subgraph, is refused with the reason, and the graph is
  left unchanged.
* A saved graph remains a Visual Effect Graph 14.x asset that Unity 2022.3 LTS
  opens with the same contexts, blocks, operators, connections, properties,
  positions, groups, and sticky notes. Data that unitas does not edit is
  preserved.
* On a device whose hardware cannot run compute work, each `VisualEffect` is
  reported as Platform-limited with the device named, stays present and
  inspectable, and reports that it is not playing.

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

## Cinemachine camera rigs

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
instances, animation state, playable graphs, CPU and GPU time per camera and
per pass, draw, batch, and state-change counts, and GPU memory by resource
type.

Diagnostics navigate to responsible project content and distinguish correctness
errors from performance warnings. A captured image records target, resolution,
aspect, quality level, color space, camera, render pipeline and renderer, stereo
mode and eye, scene state, and relevant color interpretation.

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
   `OnRenderImage` output becomes the camera image; in a URP project they run
   at the points this document states, and with Built-in features in
   scriptable pipelines off they do not run and the compatibility report names
   the scripts that define them; `RenderPipelineManager.beginCameraRendering`
   fires in the URP project in both cases. (REN-001, REN-006)
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
   its linear value in Linear, and the Linear scene passes the same validation
   on a graphics API without sRGB formats. (REN-002)
7. `WebCamTexture` plays after camera permission is granted and returns an
   explicit status when permission is denied or no camera exists. (REN-002)
8. A surface shader, a `CGPROGRAM` shader using `UnityCG.cginc` and
   `multi_compile` keywords, and a shader with `GrabPass` are translated and
   pass visual validation in a Built-in project, with the selected pass and
   variant compared exactly; in a URP project the `GrabPass` and the surface
   shader's `ForwardBase` and `ForwardAdd` passes render and pass visual
   validation, and with Built-in features in scriptable pipelines off they are
   reported as not rendered. (REN-002, REN-006)
9. `Shader.Find` for a shader in Always Included Shaders succeeds in a player,
   for an unreferenced shader returns null, and the build report lists the
   shaders reachable by `Shader.Find`. (REN-002)
10. Missing visual output has an inspectable cause that distinguishes renderer,
    material, shader, light, culling, and target reasons. (REN-001, REN-002,
    REN-003)
11. A scene with Unity-baked lightmaps, light probes, reflection probes, and
    occlusion data renders with that data in editor and player; moving a
    static contributing renderer marks the data stale and names the affected
    renderers; a bake of lightmaps, light probes, reflection probes, and
    occlusion data started in the editor replaces the stale data, and the
    result passes visual validation. (REN-003)
12. Switching skybox and ambient settings through `RenderSettings` followed by
    `DynamicGI.UpdateEnvironment` changes ambient and reflection lighting on
    the next frame; light cookies, flares, light culling masks, URP rendering
    layers, and the Built-in pixel light count produce the light assignment
    this document states, compared exactly; with Realtime Global Illumination
    enabled, changing a realtime light's color changes indirect lighting on
    static and probe-lit renderers until `DynamicGI.isConverged` is true.
    (REN-003)
13. `QualitySettings.SetQualityLevel` with `applyExpensiveChanges` false defers
    the anti-aliasing change and applies all other settings, the same call
    with true applies anti-aliasing, and a setting the target does not use is
    reported. (REN-003)
14. Each listed PPv2 and URP Volume effect passes visual validation with its
    blended parameters, and the contributing volumes for a camera position are
    inspectable; a project-defined PPv2 effect and a custom URP volume
    component run with their blended parameters, and an effect whose script
    type is missing is preserved and reported rather than silently ignored.
    (REN-003)
15. Particle systems produce the same particle count and positions under a
    fixed random seed in preview, play mode, and player on the same target,
    and trails and line renderers report their state. (REN-004)
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
    only in Forward; an HDRP 14.x project and a custom scriptable render
    pipeline project each render and pass visual validation. (REN-006)
19. A Shader Graph material for each of the Built-in, URP, and HDRP targets
    passes visual validation against the approved reference of an equivalent
    hand-written shader; a Visual Effect Graph Shader Graph output renders;
    and a graph that references a node type whose package or script is missing
    is reported with the graph, node, and affected materials. (REN-006)
20. A `ScriptableRendererFeature`, a URP `DecalProjector`, a camera
    `CommandBuffer`, `Graphics.DrawMeshInstanced`, and a compute shader
    dispatch each pass visual validation; on a device whose hardware cannot
    run compute work the dispatch fails with an error and is reported
    Platform-limited naming the device; enabling the SRP Batcher changes only
    reported performance. (REN-006)
21. Global shader properties and keywords set from script select the variant
    compiled for the enabled keyword set, exceeding the stated keyword limit
    produces an error naming the shader and keyword, and after
    `ShaderVariantCollection.WarmUp` returns the listed variants are ready.
    (REN-006)
22. Failed shaders and shaders with missing content cannot appear as
    successful compatibility.
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
    effective graphics backend and API and diagnose missing device
    capabilities. (REN-006, REN-007)
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
35. An HDRP 14.x scene with Lit, StackLit, and subsurface-scattering
    materials, realtime area lights in physical light units, a physically
    based sky, volumetric fog, automatic exposure, an HDRP decal, and a
    full-screen custom pass passes visual validation, with blended volume
    parameters inspectable; on a device without ray tracing hardware, ray
    traced reflections are reported Platform-limited naming the device and
    render in screen-space mode. (REN-003, REN-006)
36. A custom scriptable render pipeline project calls `CreatePipeline` once,
    receives every camera of the frame in `Render`, culls and draws with
    `DrawRenderers` so that the passes its `ShaderTagId` list selects are
    compared exactly, disposes and recreates the pipeline when the asset
    changes at runtime, and after an exception in `Render` logs it with the
    script location and renders the next frame. (REN-006)
37. A lightmap, light probe, reflection probe, and occlusion bake started from
    the command line in a headless session on a host without a GPU completes
    with stage, fraction, and estimated time remaining reported; Unity 2022.3
    opens the project and renders with the baked data without rebaking; a
    bake cancelled midway leaves the previous data unchanged; and a bake with
    Realtime Global Illumination enabled produces the precompute that realtime
    indirect lighting uses. (REN-003)
38. A Visual Effect Graph effect with spawn, GPU event, collision, mesh
    output, Shader Graph output, property binders, and a Timeline activation
    track plays, pauses, reinitializes, receives `SendEvent`, and passes
    visual validation in a URP and an HDRP project, and produces the same
    particle count under a fixed seed in preview, play mode, and player on the
    same device; on a device without compute it is reported Platform-limited
    and stays inspectable. (REN-004)
39. A Shader Graph and a visual effect graph created and edited in the editor
    and through automation preview their result, save, and open in Unity
    2022.3 with the same nodes, connections, and settings. (REN-004, REN-006)
40. A geometry shader and a tessellation shader pass visual validation on a
    graphics API without those stages; `AsyncGPUReadback` returns the texels
    on a graphics API without asynchronous readback; and a render texture in
    a format the device cannot create directly reads back the written values
    at the requested precision. (REN-002, REN-006)
41. A Realtime area light in a URP project lights and shadows renderers in
    realtime, and with the Realtime area lights setting off it contributes
    only through baked data; a Built-in project `Projector` projects its
    material onto renderers in its frustum and honors ignored layers; and a
    mesh with line, line strip, point, and quad topology renders each
    submesh. (REN-002, REN-003, REN-006)
42. An XR scene renders in single-pass instanced and multi-pass stereo in a
    Built-in, a URP 14.x, and an HDRP 14.x project, with the active stereo
    mode reported for each camera, and each eye's capture passes visual
    validation against the same reference in both modes, including a project
    shader without stereo instancing declarations. (REN-006, XR-001)
43. In a stereo scene, a renderer visible in only one eye is not culled,
    bloom and ambient occlusion sample only within each eye, a World Space
    canvas draws in both eyes at its depth, and a Screen Space - Overlay
    canvas draws as a head-locked plane, or only to the mirror view with the
    XR overlay canvas setting off. (REN-003, REN-006, XR-001)
44. On a device whose runtime exposes foveated rendering, setting the
    foveated rendering level reduces shading resolution outside the focus
    region as the runtime reports; on a device without it, the level is
    reported Platform-limited and frames render at full resolution; a
    headless session captures the left eye, right eye, side-by-side, and
    mirror view of a simulated stereo device, and each capture records its
    stereo mode and eye. (REN-006, REN-007, XR-001)
45. In the Shader Graph editor, a user builds a URP Lit graph with a texture
    property, a boolean keyword, and a sub graph created from a selection;
    the node previews and the main preview update as ports are connected;
    undo and redo restore each step; saving updates the materials that use
    the graph; and Unity 2022.3 LTS opens the saved graph and sub graph with
    the same nodes, connections, blackboard entries, and positions. (REN-006)
46. In the Visual Effect Graph editor, a user adds a spawn, initialize,
    update, and output context, blocks, operators, an exposed property, and a
    subgraph operator; the scene effect plays each change as it is made; and
    Unity 2022.3 LTS opens the saved graph with the same contexts, blocks,
    operators, connections, and properties. (REN-004)
47. An agent creates a Shader Graph and a visual effect graph through MCP as
    structured nodes and connections, and the editor shows the same graphs;
    an agent edit that connects incompatible ports is refused with the reason
    and leaves the graph unchanged; and the agent reads back every node,
    connection, property, and keyword it wrote. (REN-004, REN-006)
