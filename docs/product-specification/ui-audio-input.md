# User Interface, Text, Audio, Input, and Video

## Scope

This specification defines player-facing user interfaces, immediate-mode
runtime GUI, text and fonts, input devices and actions, audio playback,
spatialization, mixing, capture, video playback, and the diagnostics used to
validate them.

These are desired end-state requirements, not claims about currently shipped
capability. The compatibility baseline is Unity 2022.3 LTS, as defined in the
[Compatibility contract](compatibility-contract.md). uGUI (`com.unity.ugui`)
1.0.x, TextMeshPro 3.0.x, the Input System 1.x, and other package content are
covered only for the package families and version lines listed in the
supported package matrix (CMP-002, owned by the
[Compatibility contract](compatibility-contract.md)). In this document,
"within scope" means those listed packages together with the built-in modules
described here. UI Toolkit runtime panels and documents are recognized and
reported as Unsupported, as listed in the [index](INDEX.md).

Localized content is owned by LOC-001 in
[Accessibility, localization, and persistence](accessibility-localization-persistence.md).
Persistent `UnityEvent` wiring, `SendMessage`, and delayed invocation are owned
by RUN-008 in [Scripting runtime](scripting-runtime.md).

Every capability in this document, including inspection of UI, input, audio,
and video state and the injection of synthetic input, is available to
authorized agents through automation as defined in
[Automation and MCP](automation-and-mcp.md), in the visual editor, and in
headless sessions.

### Compatibility tiers

The tiers below are recorded in the [Feature catalog](feature-catalog.md) and
defined in the [Compatibility contract](compatibility-contract.md):

| ID | Tier |
|---|---|
| UI-001 | Familiar |
| UI-002 | Familiar |
| TXT-001 | Familiar |
| INP-001 | Familiar |
| INP-002 | Familiar |
| AUD-001 | Familiar |
| MED-001 | Familiar |

Every feature in this document is Familiar: Unity canvases, controls, fonts,
input settings and action assets, audio clips and mixers, and video clips
import with their identity, and public types and members keep their Unity
2022.3 names and signatures. The behavior is the one this document states.
Every difference from Unity 2022.3 is listed in the API reference owned by
AUT-006 in [Automation and MCP](automation-and-mcp.md) and in the project
compatibility report for each project that uses the affected member.

## Canvas and layout

The runtime user-interface feature supports screen-space overlay,
screen-space camera, and world-space canvases. RectTransform content exposes
anchors, pivot, position, size, offsets, rotation, scale, and hierarchy
relationships.

### Required behavior

* A RectTransform's rectangle is computed from its anchors (as fractions of
  the parent rectangle), its offsets from those anchors, and its pivot, and is
  recomputed when the parent size, reference resolution, aspect ratio, screen
  orientation, safe area, or content changes.
* `CanvasScaler` Constant Pixel Size uses its scale factor. Scale With Screen
  Size computes the scale from the reference resolution: Match Width Or Height
  interpolates between the width and height ratios in logarithmic space by the
  match value, Expand uses the smaller ratio, and Shrink uses the larger.
  Constant Physical Size scales by the screen DPI relative to the physical
  unit, using the fallback DPI when the screen reports none.
* Horizontal, vertical, and grid layout groups allocate each child its minimum
  size, then its preferred size, then distribute remaining space by flexible
  size, honoring padding, spacing, alignment, child control, and child force
  expand. `ContentSizeFitter` sets a size to the minimum or preferred size of
  the content. `LayoutElement` overrides those sizes, and
  `AspectRatioFitter` keeps the configured aspect in its mode.
* Nested layout components reach a stable result or report a cycle or conflict
  that names the participating RectTransforms.
* Screen-space overlay, camera-associated, and world-space interfaces remain
  distinct and inspectable, including canvas sort order, override sorting,
  sorting layer, target display, and pixel-perfect settings.
* `Mask` and `RectMask2D` clip drawing and pointer hits to their area.
  `CanvasGroup` multiplies alpha down the hierarchy, and its interactable and
  blocks-raycasts flags apply to descendants unless a nested group sets
  ignore parent groups.
* UI scale and readable size choices apply without corrupting saved layout.

## Controls and event behavior

Supported controls include images, raw images, text, buttons, toggles, toggle
groups, sliders, scrollbars, scroll rects, dropdowns, input fields, masks, and
other `Selectable` controls from uGUI and TextMeshPro within scope.

A control has visible, interactable, highlighted, focused, selected, pressed,
and disabled states, and input fields additionally have a validation state.
Selectable transitions present those states: Color Tint multiplies the target
graphic's color by the state color over the fade duration, Sprite Swap shows
the state's sprite, and Animation sets the state's trigger on the control's
Animator. Pointer, touch, keyboard, gamepad, and automation can interact
through the same control behavior.

Events include pointer enter and exit, down, up, click, initialize-potential-
drag, begin drag, drag, end drag, drop, scroll, select, deselect, update
selected, move, submit, cancel, and value changes. An event identifies its
target and propagation behavior: it is delivered through the
`IEventSystemHandler` interfaces (`IPointerClickHandler`, `IDragHandler`, and
the others in uGUI) to the first object, walking up from the raycast target,
that implements the handler (the `ExecuteEvents` hierarchy walk). A control
cannot report a successful activation when effective pointer hit-testing
routes that input to another element; visibility alone does not imply input
blocking.

### Graphics and mesh effects

* `Image` supports Simple, Sliced, Tiled, and Filled types. Sliced keeps the
  sprite borders at their pixel size, scaled by `pixelsPerUnitMultiplier`, and
  stretches the center; Tiled repeats the center; Filled draws the fraction
  `fillAmount` of the image, sweeping from the fill origin by fill method
  (horizontal, vertical, radial 90, radial 180, or radial 360) in the
  clockwise or counterclockwise direction. `preserveAspect` and fill center
  apply.
* `Image.alphaHitTestMinimumThreshold` restricts pointer hits to texels whose
  alpha is at or above the threshold; it requires a readable sprite texture,
  and a non-readable texture produces an error naming it.
* `RawImage` draws its texture with its `uvRect`.
* `Shadow`, `Outline`, and `PositionAsUV1` effects, and project subclasses of
  `BaseMeshEffect` or implementations of `IMeshModifier`, modify the generated
  UI mesh in component order.

### Rebuild timing and scrolling

* `Canvas.willRenderCanvases` fires once per frame after `LateUpdate` and
  before canvases render; pending layout rebuilds and then graphic rebuilds
  run during it. `Canvas.ForceUpdateCanvases` and
  `LayoutRebuilder.ForceRebuildLayoutImmediate` complete pending rebuilds
  before returning, so RectTransform values read afterwards reflect them.
* `ScrollRect` movement type Unrestricted lets content move freely, Elastic
  returns content to its bounds over the elasticity time, and Clamped never
  lets it leave them. With inertia on, content keeps moving after release with
  velocity reduced by the deceleration rate each second. Scroll sensitivity,
  viewport, and scrollbar visibility modes apply.
* `EventSystem.IsPointerOverGameObject`, with no argument for the mouse and
  with a pointer or touch identifier, returns true when that pointer's most
  recent raycast through the active input module hit an object.

Control callbacks such as `Button.onClick`, `Toggle.onValueChanged`, and
`Slider.onValueChanged` are `UnityEvent`s; their persistent and dynamic
listeners behave as defined by RUN-008 in
[Scripting runtime](scripting-runtime.md).

## Event system, raycasters, and input modules

* When several event systems are enabled, the first one enabled is
  `EventSystem.current` until it is disabled, and a diagnostic names each
  enabled event system. The first selected object, drag threshold, and
  `sendNavigationEvents` apply to the current event system.
* `GraphicRaycaster` hits UI graphics whose raycast target is enabled,
  honoring blocking objects, blocking mask, ignore reversed graphics, canvas
  groups, and masks. `PhysicsRaycaster` and `Physics2DRaycaster` hit 3D and 2D
  colliders through the camera with their event mask, so scene objects
  receive the same pointer events as UI. Results from multiple raycasters are
  ordered by raycaster sort order and render order, then camera depth, then
  hit distance, and the first result is the event target.
* `StandaloneInputModule` drives UI from the legacy Input Manager, and
  `InputSystemUIInputModule` drives UI from Input System actions, including
  per-player multiplayer event systems. A module whose input backend is
  disabled by the project's active input handling setting reports that
  conflict rather than silently delivering no events.
* The current raycast result list for a pointer, the hovered and pressed
  objects, and the reason an object received no event (no raycaster, raycast
  target disabled, blocked by another graphic, outside mask, or canvas group
  not blocking) are inspectable.

## Focus and navigation

The interface maintains one unambiguous selected control per event system.
Directional navigation follows explicit links when supplied. Automatic,
Horizontal, and Vertical navigation modes select, among interactable
selectables in the permitted directions, the one whose offset from the current
control maximizes the dot product with the requested direction divided by the
squared distance. Disabled, hidden, or non-interactable controls are skipped.

Changing input device does not strand focus. Modal content constrains focus to
its intended scope. Opening and closing a modal restores the prior selection
when that control still exists and is interactable. The editor and automation
can show why a control did or did not receive navigation or pointer input.

## Immediate-mode runtime GUI

Runtime IMGUI through `OnGUI` is Supported in play mode, headless sessions, and
players:

* `OnGUI` runs once per GUI event on every enabled `MonoBehaviour` that
  defines it, with a Layout event before each other event and one Repaint
  event per frame. `Event.current` reports the event type (`MouseDown`,
  `MouseUp`, `MouseDrag`, `MouseMove`, `KeyDown`, `KeyUp`, `ScrollWheel`,
  `Repaint`, `Layout`, `Used`, and the other runtime types), mouse position,
  button, key code, character, and modifiers; `Event.Use` marks the event as
  `Used` so later controls and scripts ignore it.
* `GUI` controls (`Label`, `Box`, `Button`, `RepeatButton`, `Toggle`,
  `TextField`, `TextArea`, `PasswordField`, `HorizontalSlider`,
  `VerticalSlider`, `HorizontalScrollbar`, `VerticalScrollbar`, `Toolbar`,
  `SelectionGrid`, `Window`, `DragWindow`, `BeginGroup`, and `BeginScrollView`)
  and their `GUILayout` counterparts with layout options draw and return
  values: `Button` returns true when a press is released over it,
  `RepeatButton` returns true on every frame it is held, toggles, sliders,
  scrollbars, toolbars, and selection grids return the new value, and text
  controls return the edited string.
* `GUILayout` computes positions in the Layout pass and reuses them in later
  passes; a control sequence that differs between passes produces an error
  naming the script and the mismatched control.
* The default skin, `GUISkin` assets, and `GUIStyle` states style controls.
  `GUI.color` tints controls, `GUI.backgroundColor` tints backgrounds,
  `GUI.contentColor` tints text and icons, `GUI.enabled` false disables input
  to following controls, `GUI.changed` becomes true when a control's value
  changes, and `GUI.matrix` transforms drawing and input. Controls named with
  `GUI.SetNextControlName` receive keyboard focus through `GUI.FocusControl`,
  and `GUIUtility.hotControl` and `keyboardControl` report the active and
  focused controls.
* `GUI.depth` orders drawing and input between scripts: lower depth draws on
  top and receives input first. IMGUI draws after all cameras in screen
  coordinates with the origin at the top left.
* IMGUI in editor extensions (custom inspectors and editor windows) is outside
  this feature and is reported as listed in the [index](INDEX.md).

## Text and fonts

Runtime text supports Unicode content, font assets, fallback fonts, size and
auto-sizing, alignment, wrapping, overflow, spacing, style, rich text within
scope, and resolution-aware rendering for the legacy text components (UI
`Text` and `TextMesh`) and for TextMeshPro 3.0.x.

### Legacy fonts and text

* TrueType and OpenType font import retains font size, rendering mode,
  character set (Dynamic, Unicode, ASCII default set, and custom set),
  Include Font Data, ascent calculation mode, and fallback font names.
  Custom fonts (`.fontsettings`) render from their character table and
  material.
* `Font.CreateDynamicFontFromOSFont` returns a font backed by the named
  operating-system font; on targets where operating-system fonts are not
  accessible (the browser target and others that report it), it is
  Platform-limited and the result is reported.
* Legacy UI `Text` Best Fit chooses the largest size between its minimum and
  maximum at which the text fits its rectangle. `Font.RequestCharactersInTexture`
  adds the requested characters to the dynamic font texture,
  `Font.textureRebuilt` fires when that texture is rebuilt, and
  `Font.GetCharacterInfo` returns the metrics of a character in the texture.

### TextMeshPro scope

* `TextMeshProUGUI` and `TextMeshPro` components, `TMP_FontAsset` (static and
  dynamic atlas population, SDF and bitmap render modes, atlas size, padding,
  and multi-atlas), font fallback chains (per font asset and global TMP
  Settings), `TMP_SpriteAsset`, style sheets, material presets, rich-text
  tags, text overflow modes (including linked, page, and ellipsis), word
  wrapping, kerning, character and line spacing, and `TMP_InputField` and
  `TMP_Dropdown` are Supported.
* Mesh, character, word, line, and link information exposed through
  `TMP_Text.textInfo` matches the rendered text; link hit-testing through
  `TMP_TextUtilities` returns the link whose characters lie under the point.
* TMP shaders (distance field, mobile, bitmap, and sprite) render with
  their material properties (face, outline, underlay, glow, and bevel) as
  shader content governed by [Rendering and animation](rendering-and-animation.md).
* A TMP feature outside 3.0.x, or a TMP resource missing from the project
  (for example the TMP Essential Resources), is reported with the affected
  text objects.

### Glyphs, fallback, and scripts

Missing glyphs identify the character, the font, the fallback chain searched,
the locale where one is active, and the affected text object. Fallback is
visible and deterministic: the font asset is searched first, then its
fallback list in order, then the global fallback list in order, so the same
content, font assets, and settings select the same fallback font on every run
and target. Text does not silently disappear, drop unsupported characters, or
substitute them without diagnostic state; an unresolved character is drawn as
the missing-glyph character configured in TMP Settings, or as the font's
missing-glyph shape for legacy text.

Layout uses the displayed glyph metrics and updates when content, font,
scaling, or locale changes. Text layout responds to scaling, long strings,
line breaks, mixed scripts, bidirectional content, and dynamic values within
the language support stated for each locale under LOC-001.

TextMeshPro's right-to-left option reverses character order on each line.
Neither text system performs script shaping (such as Arabic joining forms) or
Unicode bidirectional reordering; this is listed in the AUT-006 API reference.
Basic character display is not represented as complete complex-script
support; per-locale support states are reported as defined by LOC-001.

### Input fields

Input fields distinguish displayed text, placeholder, selection, caret,
composition (IME), content type and validation, character limit, password
presentation, submission, cancellation, and focus loss. Clipboard behavior and
on-screen keyboard availability (`TouchScreenKeyboard`) are stated per
platform.

## Localization

Localized strings, assets, locale selection, and their layout effects are
owned by LOC-001 in
[Accessibility, localization, and persistence](accessibility-localization-persistence.md).

## Legacy input behavior

`UnityEngine.Input` covers keyboard keys, `GetKey`, `GetKeyDown`, and `GetKeyUp`
transitions, `anyKey`, `inputString`, mouse buttons, pointer position and
delta, scrolling, named buttons, named axes from the imported Input Manager
settings, raw axes, touches, `simulateMouseWithTouches`, multi-touch,
acceleration, gyroscope, compass, and location where the device provides
them.

`GetKeyDown`, `GetKeyUp`, `GetButtonDown`, `GetButtonUp`, `GetMouseButtonDown`,
and `GetMouseButtonUp` are true only during the frame in which the transition
occurred. Named axes respect configured sources, dead zone, sensitivity,
gravity, inversion, snapping, and joystick number: `GetAxis` moves toward the
input value at the sensitivity rate and back to zero at the gravity rate,
while `GetAxisRaw` returns the unsmoothed value. Focus loss, cursor lock
(`Cursor.lockState`), cursor visibility, and device disconnection have defined
outcomes. Touch identity remains stable from begin through end or cancel.

When the project's active input handling setting is Input System only, legacy
`Input` APIs throw `InvalidOperationException`; when it is Both, legacy and
action-based input observe the same devices.

## Action-based input

Actions express gameplay intent independently of a specific device. Input
System 1.x action assets import with action maps, actions (Value, Button, and
Pass-Through types), bindings, composites (1D axis, 2D vector, 3D vector, and
button with one or two modifiers), interactions (press, hold, tap, slow tap,
and multi-tap), processors, and control schemes.

### User stories

* As a player, I can use keyboard and mouse, gamepad, touch, pen, or supported
  XR controls according to the control schemes the game's input actions
  define. XR device behavior is owned by XR-001 in
  [Builds, platforms, XR, and networking](build-platforms-xr-networking.md).
* As a player, I can rebind controls and retain my choices.
* As a local multiplayer developer, I can assign distinct devices to players.
* As a tester or agent, I can see which physical control caused an action.

### Required behavior

* An enabled action moves from Waiting to Started when a bound control
  actuates, to Performed when its interaction completes, and to Canceled when
  the control is released or the interaction fails; a disabled action is in
  Disabled. Button and Value actions check initial state on enable, and when
  several bound controls actuate, a Value action follows the most actuated
  one.
* `PlayerInput` notification behaviors deliver actions as follows: Send
  Messages calls `On<ActionName>` methods on the GameObject, Broadcast Messages
  also calls them on children, Invoke Unity Events invokes the per-action
  events, and Invoke C# Events raises `onActionTriggered`. Action map switching
  changes the enabled map, and control scheme auto-switching changes the scheme
  of a single player when a device of another scheme is used.
  `PlayerInputManager` joins players when a button is pressed, when the join
  action triggers, or manually, up to its maximum player count, and with
  split-screen enabled divides the screen into one region per player.
* The generated C# wrapper class for an action asset exposes the same maps and
  actions as the asset.
* Device-level access through `Keyboard.current`, `Mouse.current`,
  `Gamepad.current`, `Touchscreen.current`, `Pen.current`, and
  `InputSystem.devices` returns the most recently used device of each kind and
  all connected devices. Control-level `isPressed`, `wasPressedThisFrame`,
  `wasReleasedThisFrame`, and `ReadValue` report the control's current value
  and are true for a transition only in the input update in which the control
  crossed its press point. `EnhancedTouch` (`Touch.activeTouches` and finger
  tracking) is Supported once enabled.
* On-screen controls (`OnScreenStick` and `OnScreenButton`) feed their bound
  control paths as virtual devices.
* The Input System settings asset applies its update mode (dynamic update,
  fixed update, or manual), background behavior, default dead zones, default
  press point and tap and hold times, supported devices, and filtering of
  noisy controls. The update mode determines when actions observe new values.
* Custom processors, interactions, composites, and device layouts registered
  by project code (`InputSystem.RegisterProcessor`, `RegisterInteraction`,
  `RegisterBindingComposite`, and `RegisterLayout`) participate in binding
  resolution as built-in ones do. A binding that names an unregistered
  processor, interaction, composite, or layout is reported with the action
  and binding.
* Device changes do not duplicate or lose an eligible action;
  `InputSystem.onDeviceChange` reports added, removed, disconnected, and
  reconnected devices. Binding conflicts are visible.
* Interactive rebinding (`PerformInteractiveRebinding`) supports excluded
  controls, cancel controls, timeouts, and expected control types. Binding
  overrides saved with `SaveBindingOverridesAsJson` and restored with
  `LoadBindingOverridesFromJson` identify the action and binding they affect,
  and an override whose action or binding no longer exists is reported as
  stale.
* Gamepad rumble (`SetMotorSpeeds`, `PauseHaptics`, and `ResetHaptics`)
  returns an explicit status on devices without motors.

Input diagnostics show connected devices, layouts, controls, current values,
resolved bindings, action phases, player assignment, focus, and the reason an
action did not fire. Automated input is marked as synthetic in playtest
evidence.

## Audio clips and sources

`AudioSource` and `AudioClip` support one-shot (`PlayOneShot`) and looping
playback with clip, volume, pitch, mute, pause, start time (`time` and
`timeSamples`), priority, output mixer group, bypass settings, and spatial
behavior.

`AudioSource.PlayClipAtPoint` creates a temporary GameObject with an
`AudioSource` at the given position, plays the clip, and destroys the object
when the clip ends; the temporary source is visible in voice inspection while
it plays.

Playback state distinguishes scheduled, loading, playing, paused, virtualized,
stopped, completed, and failed. Seeking and time reporting use the clip's
length in samples and the target's decoder capability. Loop boundaries are
gapless for clips whose load type and encoding allow seamless looping on the
target.

Destroying or disabling the owning object stops its sources. Changing scenes
stops sources on unloaded objects. `AudioListener.pause` pauses every source
except those with `ignoreListenerPause`, and application pause pauses all
output. `Time.timeScale` does not change audio pitch or playback rate.
`AudioListener.volume` scales all output.

### Clip loading

* Decompress On Load decodes a clip to PCM when it loads, Compressed In Memory
  keeps compressed data and decodes during playback, and Streaming decodes
  from storage during playback. Preload Audio Data loads the clip with the
  scene that references it, and Load In Background loads it without blocking.
  `AudioClip.loadState`, `LoadAudioData`, and `UnloadAudioData` report and
  change residency.
* `AudioClip.GetData` returns the clip's samples for uncompressed and
  Decompress On Load clips, and `SetData` writes samples that later playback
  renders. Reading a clip whose load type does not permit it fails with an
  error naming the clip.
* `AudioClip.Create`, including streamed clips with PCM reader and position
  callbacks, produces playable clips with the requested length, channels, and
  frequency.
* Encodings or sample rates that a target cannot play are reported at import
  or build with the clip and target named.

## Spatial audio and listeners

Three-dimensional audio responds to listener and source position, distance
attenuation (logarithmic, linear, and custom rolloff curves between minimum
and maximum distance), spatial blend, source direction, Doppler level, spread,
and reverb zone mix. Two-dimensional audio remains independent of scene
distance.

An `AudioReverbZone` applies its preset or custom room parameters fully within
its minimum distance and fades them to none at its maximum distance; where
zones overlap, their effects mix in proportion to each zone's influence at the
listener.

Spatializer and ambisonic decoder plugins selected in the project's audio
settings are native plugins assessed as defined by CMP-003 in the
[Compatibility contract](compatibility-contract.md); the selected plugin and
its compatibility state are reported. Audio import formats are owned by
[Projects and assets](project-and-assets.md).

The runtime identifies the active listener state. Missing listeners, conflicting
listeners, inaudible range, muted routing, zero volume, paused state, unsupported
output, and voice limits are distinct reasons for silence.

## Mixing and routing

`AudioMixer` assets import with groups, the group hierarchy, sends and
receives, effect chains, ducking, solo, mute, bypass, and views.

* Groups control volume, mute, solo, and effects. The built-in mixer effects
  (attenuation, lowpass, highpass, simple lowpass and highpass, echo, flange,
  distortion, normalize, parametric EQ, pitch shifter, chorus, compressor,
  SFX reverb, send, receive, and duck volume) are Supported with their
  imported parameters.
* Snapshots store parameter states. `AudioMixerSnapshot.TransitionTo`
  interpolates every parameter to the snapshot over the given time, and
  `AudioMixer.TransitionToSnapshots` blends several snapshots by weight.
* Exposed parameters are readable and writable by name through `SetFloat` and
  `GetFloat`; `ClearFloat` returns the parameter to snapshot control. An
  unknown parameter name returns false and is reported in diagnostics.
* Playable audio follows the same routing: an `AudioPlayableOutput` plays
  through its target `AudioSource`, whose output mixer group then applies,
  and an `AudioClipPlayable` honors its clip's load type and its own loop
  setting. Playable graph behavior is owned by ANI-002 in
  [Rendering and animation](rendering-and-animation.md).
* Mixer effects provided by native audio plugins are project native plugins,
  assessed as defined by CMP-003 in the
  [Compatibility contract](compatibility-contract.md).
* When more sources play than the real voice count in the audio settings, the
  sources with the lowest priority (highest priority number), then the
  quietest, are virtualized; beyond the virtual voice count, they are stopped.
  The affected sources are reported.

The developer and authorized agents can monitor active voices, levels, routing,
clipping, latency where the platform reports it, and the object that owns each
source. Preview audio is distinguished from runtime audio.

## Audio filters and custom processing

* Filter components (`AudioLowPassFilter`, `AudioHighPassFilter`,
  `AudioEchoFilter`, `AudioDistortionFilter`, `AudioReverbFilter`, and
  `AudioChorusFilter`) on an `AudioSource` or `AudioListener` process audio in
  component order from top to bottom.
* `OnAudioFilterRead(float[] data, int channels)` runs on the audio thread and
  receives interleaved samples at the output sample rate at its component
  position in the filter chain. A script on a GameObject without an
  `AudioSource` acts as a generator. Unity APIs called from this callback are
  governed by the main-thread rules of RUN-004 in
  [Scripting runtime](scripting-runtime.md). An exception in the callback is
  reported with the script and does not stop other audio.
* `AudioSource.GetOutputData` and `GetSpectrumData`, and the `AudioListener`
  equivalents, return the most recent output samples and the magnitude
  spectrum computed with the requested `FFTWindow`.

## Audio timing and scheduling

* `AudioSettings.dspTime` advances with the number of samples output by the
  audio system, independent of `Time.timeScale` and frame rate.
  `AudioSettings.outputSampleRate`, `GetDSPBufferSize`, and `GetConfiguration`
  report the active configuration; `Reset` applies a new configuration and
  restarts output; `OnAudioConfigurationChanged` fires after a configuration
  or output device change, reporting whether the device changed.
* `AudioSource.PlayScheduled(dspTime)` starts playback at the output sample
  corresponding to that time, and `SetScheduledStartTime` and
  `SetScheduledEndTime` adjust a scheduled source. Two clips scheduled back to
  back at exact sample boundaries play with no gap and no overlap. A time
  already in the past starts playback at the next audio buffer.
* In headless sessions, audio renders at the project's audio output sample
  rate to a capturable output. Audio capture (listener or mixer output, time
  range, samples, channel layout, DSP start time, and contributing voices) is
  defined by AUT-002 in [Automation and MCP](automation-and-mcp.md), and audio
  validation by TST-002 in
  [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).

## Microphone capture

* `Microphone.devices` lists device names. `Microphone.Start` records from a
  device into a clip of the given length in seconds at the given frequency,
  looping or not; `End` stops recording, `IsRecording` reports state,
  `GetPosition` returns the current sample position, and `GetDeviceCaps`
  returns the device's frequency range. The recorded `AudioClip` contains the
  captured samples.
* Capture requires the target's permission: the project requests it with
  `Application.RequestUserAuthorization` or the Android microphone
  permission, and a denied, revoked, or not-yet-granted permission returns an
  explicit status and an empty device list or failed start rather than
  silence presented as recording.
* The browser target does not provide the `Microphone` class; there,
  microphone capture is Platform-limited.

## Video playback

`VideoPlayer` plays video from a `VideoClip` asset or a URL (local file,
StreamingAssets path, or network address).

* **Control:** `Prepare` loads the source and `isPrepared` becomes true when
  the first frame can be shown; `Play`, `Pause`, and `Stop` start, pause, and
  stop playback (stopping releases the prepared state); assigning `time` or
  `frame` seeks and reports completion through `seekCompleted`. `isLooping`,
  `playbackSpeed` (accepted only when `canSetPlaybackSpeed` is true),
  `skipOnDrop`, `waitForFirstFrame`, `playOnAwake`, and `StepForward` apply.
* **Events:** `prepareCompleted` fires when preparation finishes, `started`
  when playback begins, `frameReady` for each new frame when
  `sendFrameReadyEvents` is enabled, `loopPointReached` when playback reaches
  the end of the source (looping or not), `frameDropped` when a frame is
  skipped, `clockResyncOccurred` when playback resynchronizes to its time
  reference, and `errorReceived` on failure. An error message names the source
  and the reason.
* **Render modes:** Camera Far Plane and Camera Near Plane (with target camera
  and alpha), Render Texture, Material Override (with renderer and texture
  property), and API Only (frames available through `VideoPlayer.texture`),
  together with aspect ratio modes.
* **Audio output modes:** None, Audio Source (per track, with track enablement
  and volume), Direct (with per-track volume and mute), and API Only (samples
  available through an audio sample provider).
* **Time reference:** the time update mode (DSP time, game time, or unscaled
  game time) and time reference (free run, internal time, or external time
  through `externalReferenceTime`) determine whether playback follows game
  time or runs independently. With game time, `Time.timeScale` 0 pauses
  playback; with unscaled game time or DSP time it does not. Application pause
  pauses playback.
* **Codecs and containers:** for each target, the product reports which video
  codecs and containers it plays (and, for transcoded clips, which transcode
  targets apply). A clip or URL whose codec or container the target cannot
  play is reported at import, at build, and at runtime with the source, codec,
  container, and target named, and `errorReceived` fires; it never plays as a
  silent black frame. Unsupported formats are Platform-limited, not
  Unsupported, when another supported target plays them.

Video state (source, prepared state, playing state, time, frame, frame count,
frame rate, dropped frames, render target, and audio routing) is inspectable
in the editor and through automation.

## Device and platform behavior

Input, audio, and video capabilities are reported for the active platform and
connected hardware. Connection, disconnection, permission denial, focus
changes, output device changes, mobile interruptions, and application
backgrounding have defined, inspectable results.

Unavailable vibration, haptics, microphones, motion sensors, on-screen
keyboards, video decoders, or audio outputs return explicit status. Platform
capability is not inferred merely because a corresponding API exists.

## Accessibility outcomes

The feature set enables games to provide remappable controls, keyboard-only and
controller-only navigation, scalable text and interfaces, captions and
subtitles, alternatives to audio-only cues, reduced-motion choices, and
alternatives to color-only cues. The accessibility contract itself is owned by
ACC-001 in
[Accessibility, localization, and persistence](accessibility-localization-persistence.md).

Playtest scenarios can validate focus order, device-only operation, high-scale
layouts, captions, localization expansion, safe areas, and interruption
recovery.

## Acceptance outcomes

1. The same UI flow can be completed with every control scheme the game's input
   actions define, through both `StandaloneInputModule` and
   `InputSystemUIInputModule`. (UI-001, INP-001, INP-002)
2. RectTransform rectangles and `CanvasScaler` scale factors equal the values
   computed by the rules in this document across the resolutions and aspects a
   scenario lists, safe areas, and each scaler mode, and a layout cycle is
   reported with the participating objects. (UI-001)
3. Pointer events reach UI graphics through `GraphicRaycaster` and scene
   objects through `PhysicsRaycaster` and `Physics2DRaycaster` in the stated
   result order, and an object that received no event reports why. (UI-001)
4. A `Button.onClick` persistent listener and a dynamically added listener both
   run on activation, as defined by RUN-008. (UI-001)
5. An `Image` in Filled radial 360 mode at fill amount 0.25 covers one quarter
   of its rectangle from the fill origin, a sliced image with
   `pixelsPerUnitMultiplier` 2 draws its borders at half size, an `Outline`
   effect adds its outline geometry, `alphaHitTestMinimumThreshold` ignores
   transparent texels, and `LayoutRebuilder.ForceRebuildLayoutImmediate` makes
   updated sizes readable in the same frame. (UI-001)
6. `EventSystem.IsPointerOverGameObject` returns true for the mouse and for a
   touch identifier over UI, and a `ScrollRect` with Elastic movement returns
   within its bounds within the elasticity time after release. (UI-001)
7. An `OnGUI` script with `GUILayout` controls, a custom `GUISkin`, and two
   scripts at different `GUI.depth` values draws in depth order, delivers input
   to the lower depth first, and returns the stated control values. (UI-002)
8. TextMeshPro text with fallback fonts, sprite assets, rich-text tags, and
   ellipsis overflow renders with a `textInfo` that matches the drawn glyphs
   and truncates with an ellipsis at the rectangle edge. (TXT-001)
9. Missing glyphs are visible diagnostics that name the character, font,
   fallback chain, locale, and text object, and fallback selection follows the
   stated search order on every run and target. (TXT-001)
10. Right-to-left text with the TextMeshPro option reverses each line, no script
    shaping or bidirectional reordering is applied, and the AUT-006 API
    reference lists that limitation. (TXT-001)
11. A legacy UI `Text` using Best Fit selects the largest size that fits and
    populates its dynamic font texture, and
    `Font.CreateDynamicFontFromOSFont` is reported Platform-limited on the
    browser target. (TXT-001)
12. Frame-based legacy input transitions are true only during the frame of the
    transition, `GetAxis` smooths by sensitivity and gravity while `GetAxisRaw`
    does not, and legacy `Input` APIs throw `InvalidOperationException` when
    the project uses the Input System only. (INP-001)
13. Interactive rebinding excludes reserved controls, saved overrides restore
    on the next run, and stale or conflicting bindings are reported explicitly.
    (INP-002)
14. `Gamepad.current` and `Keyboard.current` report `wasPressedThisFrame` in the
    same input update in which the bound action performs, an `OnScreenStick`
    drives a 2D vector action, and a project-registered custom processor
    alters the action value. (INP-002)
15. Two local players join through `PlayerInputManager` with distinct devices,
    and device disconnection and reconnection neither duplicate nor lose an
    action. (INP-002)
16. Every inaudible AudioSource has an inspectable state and routing reason.
    (AUD-001)
17. Captured output level follows the rolloff curve as the source and listener
    move, and a reverb zone applies fully inside its minimum distance and not
    at all beyond its maximum. (AUD-001)
18. A mixer snapshot transition and an exposed parameter change alter the
    captured output level over the given transition time, and filter
    components and an `OnAudioFilterRead` script process audio in component
    order. (AUD-001)
19. Two clips scheduled with `PlayScheduled` at adjacent sample boundaries
    render with no gap or overlap in audio captured through AUT-002, and
    `AudioClip.GetData` and `SetData` round-trip samples on a Decompress On
    Load clip. (AUD-001)
20. `AudioSource.PlayClipAtPoint` plays a clip at a position and its
    temporary object is destroyed when playback ends, and an
    `AudioPlayableOutput` plays through its `AudioSource` with the source's
    mixer group applied. (AUD-001)
21. Microphone capture records samples when permission is granted and reports
    an explicit denied status otherwise. (AUD-001)
22. A `VideoPlayer` prepares, plays, pauses, seeks, loops, and fires its events
    at the stated points in each render mode and audio output mode, and a clip
    with a codec the target cannot play reports the codec, container, and
    target through `errorReceived` and diagnostics. (MED-001)
23. UI, text, input, audio, and video behavior agrees between editor play,
    headless sessions, and supported players, except for device capabilities
    reported as Platform-limited. (UI-001, UI-002, TXT-001, INP-001, INP-002,
    AUD-001, MED-001)
24. For a project using UI, text, input, audio, and video members whose unitas
    behavior differs from Unity 2022.3, each difference appears in the AUT-006
    API reference and in the project compatibility report. (UI-001, UI-002,
    TXT-001, INP-001, INP-002, AUD-001, MED-001)
