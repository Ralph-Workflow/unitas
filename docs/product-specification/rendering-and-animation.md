# Rendering, Cameras, Lighting, and Animation

## Scope

This specification defines the visible behavior of cameras, geometry,
materials, shaders, lighting, shadows, textures, render targets, animation,
particles, and cinematic sequencing. It describes observable output and
diagnostics, not a rendering architecture.

These are desired end-state requirements, not claims about currently shipped
capability. Stock Unity 2022.3 LTS defines the baseline. URP, Timeline,
Cinemachine, and other package behavior is covered only for explicitly
supported package families, versions, and features.

## Graphics API and target capability

The product does not require a particular graphics API or API version for
rendering. Direct3D, OpenGL, Vulkan, and Metal are possible implementation
choices, not interchangeable promises of Unity compatibility. In particular,
Linux and macOS support does not imply that Direct3D runs on those systems.
ShaderLab and HLSL support describes accepted project content, not the API used
to draw it.

For each supported build target and current host, developers can inspect the
selected graphics API and version, the device and driver where available, and
which declared rendering features are supported or limited. If no usable
graphics device or API is available, startup or build fails with an actionable
reason; it does not silently claim a working renderer. A change of graphics API
must not silently change the specified visible behavior of a supported scene.
Target-specific visual differences or unsupported shader features are reported
as limitations under the compatibility contract.

## Cameras

A Camera renders selected scene content to a game display or render target.
It exposes projection, field of view or orthographic size, clipping range,
viewport, depth order, background and clear behavior, culling mask, target,
enabled state, and supported quality choices.

### Required behavior

* Perspective and orthographic projection agree with scene-view previews and
  coordinate conversion APIs.
* World-to-screen, screen-to-world, viewport, and ray conversions correspond to
  the actual rendered image.
* Multiple cameras contribute in declared order and according to their viewport
  and clear behavior.
* A disabled, culled, targetless, or obscured camera has an inspectable reason
  for producing no visible output.
* Aspect changes, safe areas, dynamic resolution where supported, and display
  selection have explicit effects.
* Render targets display the same camera result expected from their declared
  dimensions, color interpretation, and update behavior.

## Mesh and sprite rendering

Static meshes, skinned meshes, sprites, tilemaps, lines, trails, and supported
particle renderers participate in camera culling, transforms, sorting, layers,
materials, lighting, and shadows according to their feature contracts.

A renderer exposes its source geometry, material slots, bounds, enablement,
sorting state, shadow participation, and visibility reason. Missing geometry,
invalid bounds, absent materials, unsupported topology, or target limitations
are distinct diagnostics.

Skinned meshes reflect bone transforms, bind poses, weights, blend shapes, and
animation. Negative or non-uniform transforms produce defined results. A
renderer does not appear compatible merely because a fallback shape is visible.

## Materials

Materials retain their shader relationship and supported textures, colors,
numbers, vectors, keywords, render queue, surface mode, transparency, alpha
clipping, culling, depth behavior, tiling, and offsets.

The product supports the observable behavior of Unity Built-in standard
materials and declared Universal Render Pipeline materials. Metallic,
smoothness, normal, occlusion, emission, transparency, and alpha-clipping
controls have compatible visible meaning within scope.

Editing a shared material affects every renderer using that asset. Accessing or
creating an instance-specific material produces a distinct editable identity
according to the compatible API contract. Missing properties never silently
accept values that have no visible effect.

## Shaders

Supported ShaderLab and HLSL assets retain their shader name, properties,
subshaders, passes, tags, render states, variants, keywords, and declared visual
role within scope.

Unsupported syntax or behavior identifies the shader, pass, variant, property,
and target where known. A failed shader is unmistakable in the scene and
diagnostics. It never renders a plausible substitute while being reported as
fully correct.

Shader selection respects material keywords, project quality, renderer state,
camera context, and target capability as declared. Variant absence is distinct
from missing source or invalid material data.

## Lighting and environment

Directional, point, and spot lights expose color, intensity, range or angle,
enablement, layers, shadow choice, and supported quality settings. The visible
result respects object transforms and material response.

Environment lighting includes supported ambient, sky, reflection, fog, and
exposure behavior. A project can inspect which environment settings affect a
camera and scene. Baked, mixed, or realtime lighting states, where supported,
are identified accurately rather than treated as interchangeable.

Light limits, target limitations, and quality reductions are visible in
diagnostics. A light ignored because of a layer, range, disabled state, or limit
can be distinguished from an invalid light.

## Shadows

Eligible renderers cast and receive shadows according to material, renderer,
light, layer, distance, and quality settings. Directional, point, and spot
shadow behavior is stated per supported scope.

Shadow bias, normal bias, strength, softness, distance, resolution, and cascade
choices have visible and inspectable effects. Missing shadows identify whether
the cause is caster state, receiver material, light state, culling, quality,
target support, or resource limits.

## Baked lighting, probes, fog, LOD, and occlusion

Where supported, baked and mixed lighting preserves lightmaps, renderer
assignments, contribution modes, shadow masks, and the visible relationship
between baked and realtime lights. A missing or stale bake is distinguishable
from an unsupported bake. Light and reflection probes expose their influence,
validity, blending, and captured result.

Supported fog and post-processing retain their visible settings, camera
participation, volumes, priorities, and blend weights. Package-provided effects
are bounded by their declared supported package version.

Level-of-detail groups expose renderer sets, transitions, fade state, bounds,
and current level. Occlusion status is distinguishable from layer or frustum
culling, LOD, disabled state, and missing geometry.

## Lines, trails, and render textures

Line and trail renderers retain points, width, color, material, alignment,
texture behavior, sorting, lifetime, and visibility within scope. Render
textures expose size, format, depth, update state, usage, and content validity.
Feedback cycles and unsupported formats are diagnosed rather than producing
unexplained stale imagery.

## Transparency and ordering

Opaque, alpha-tested, and transparent content participate in depth, sorting
layers, order values, render queues, and camera order predictably. The editor
can expose the effective ordering inputs for a selected renderer.

Known limitations involving intersecting transparent surfaces, precision, or
target variation are reported as limitations rather than hidden as deterministic
correctness.

## Animation clips

Animation clips drive supported numeric, vector, color, transform, blend-shape,
and object-reference properties over time. A clip exposes duration, sample
rate, curves, interpolation, wrap behavior, looping, root motion, and events.

The editor can preview, play, pause, and scrub a clip on a compatible target.
Bindings identify their object path, component, and property. Missing,
ambiguous, incompatible, and inactive bindings are distinct. Preview changes do
not silently become saved scene values.

Animation events occur at compatible times. Events from the same clip that
share a frame preserve their declared order; ordering across clips or animators
is guaranteed only where the compatible API contract defines it. Loop
boundaries, reverse or scaled playback where supported, and zero-duration clips
have defined outcomes.

## Animator controllers

Animator controllers expose layers, states, entry, exit, transitions,
parameters, conditions, blend trees, masks, speed, interruption, normalized
time, and supported state behaviors.

At runtime, boolean, integer, float, and trigger parameters affect transitions
predictably. Trigger consumption, transition duration, exit time, interruption,
and layer weight follow the declared compatibility behavior. The editor can
show active states, transitions, parameters, and weights during play.

Missing motions, invalid parameter references, unreachable states, invalid
bindings, and conflicting transition behavior produce specific diagnostics.

## Skeletal animation and root motion

Skinned content retains bone hierarchy, skin weights, bind poses, supported rig
or avatar information, blend shapes, and root motion. Animation affects the
rendered skin, attached child objects, queried bone transforms, and relevant
colliders consistently.

Root motion identifies whether movement applies to the GameObject, remains in
the animation, or is handled by gameplay code according to project settings.
Looping does not introduce unexplained position or rotation discontinuities.

## Particle effects

Particle systems expose duration, looping, start state, emission, lifetime,
shape, velocity, force, color, size, rotation, collision, triggers,
sub-emitters, trails, lights, renderer state, simulation space, and prewarm
behavior within scope.

Effects can be previewed, restarted, paused, and scrubbed where the behavior is
meaningful. Runtime control reports whether a system is playing, emitting,
paused, stopped, or complete. Bounds, sorting, material, layer, camera
visibility, and target limits are inspectable.

## Sequencing and cinematics

Time-based sequences coordinate animation, audio, activation, cameras, signals,
and supported properties. A sequence exposes duration, tracks, clips, blends,
markers, bindings, and evaluation time.

The editor can preview and scrub without persisting preview state. Runtime play,
pause, seek, loop, completion, and nested sequence behavior are explicit.
Missing bindings identify the track and intended target. Camera cuts and blends
produce defined output and input-control transitions.

## Rendering diagnostics

Users and authorized agents can inspect camera contribution, visible and culled
renderers, geometry counts, material and shader selection, active lights,
shadow participation, render targets, texture residency, animation state, and
frame cost.

Diagnostics navigate to responsible project content and distinguish correctness
errors from performance warnings. A captured image records target, resolution,
aspect, quality, camera, scene state, and relevant color interpretation.

## Acceptance outcomes

1. Camera coordinate conversions agree with the rendered image.
2. Supported standard and URP materials preserve their declared visible
   properties across editor and player.
3. Missing visual output has an inspectable cause.
4. Animation preview, play mode, and built-player timing agree within declared
   platform limits.
5. Animator state and transition behavior is observable during runtime.
6. Failed or unsupported shaders cannot appear as successful compatibility.
7. Visual comparison captures identify all conditions needed for meaningful
   evaluation.
8. On each declared target, a developer can identify the effective graphics API
   and diagnose missing device capabilities without assuming Direct3D on Linux
   or macOS.
