# User Interface, Text, Audio, Input, and Video

## Scope

This specification defines player-facing user interfaces, immediate-mode
runtime GUI, text and fonts, input devices and actions, audio playback,
spatialization, mixing, capture, video playback, and the diagnostics used to
validate them.

These are desired end-state requirements, not claims about currently shipped
capability. The project's Unity release, from Unity 2020.3 LTS through Unity
2022.3 LTS, is the behavioral baseline, with Unity 2022.3 LTS as the principal
reference, as defined in the [Compatibility contract](compatibility-contract.md).
uGUI (`com.unity.ugui`) 1.0.x, TextMeshPro 3.0.x, the Input System 1.x, and
other package features are covered only for the package families and version lines
listed in the supported package matrix (CMP-002, owned by the
[Compatibility contract](compatibility-contract.md)). In this document,
"within scope" means those listed packages and versions for the project's
baseline editor release together with the built-in modules described here.
UI Toolkit runtime panels and documents are recognized and reported as
Unsupported, as listed in the [index](INDEX.md).

Localized content is owned by LOC-001 in
[Accessibility, localization, and persistence](accessibility-localization-persistence.md).
Persistent `UnityEvent` wiring, `SendMessage`, and delayed invocation are owned
by RUN-008 in [Scripting runtime](scripting-runtime.md).

Every capability in this document, including inspection of UI, input, audio,
and video state and the injection of synthetic input, is available to
authorized agents through automation as defined in
[Automation and MCP](automation-and-mcp.md), in the visual editor, and in
headless sessions.

## Canvas and layout

The runtime user-interface feature supports screen-space overlay,
screen-space camera, and world-space canvases. RectTransform content exposes
anchors, pivot, position, size, offsets, rotation, scale, and hierarchy
relationships.

### Required behavior

* Layout responds to parent size, reference resolution, aspect ratio, screen
  orientation, safe area, and content changes with the same resulting
  RectTransform values as Unity.
* `CanvasScaler` Constant Pixel Size, Scale With Screen Size (with match width
  or height and expand and shrink modes), and Constant Physical Size modes
  produce Unity's scale factor for the same screen and DPI.
* Anchors and pivots retain their Unity geometric meaning.
* Horizontal, vertical, and grid layout groups, `ContentSizeFitter`,
  `LayoutElement`, and `AspectRatioFitter` compute Unity's layout for the same
  content, including child control, child force expand, and preferred,
  minimum, and flexible sizes.
* Nested layout components reach a stable result or report a cycle or conflict
  that names the participating RectTransforms.
* Screen-space overlay, camera-associated, and world-space interfaces remain
  distinct and inspectable, including canvas sort order, override sorting,
  sorting layer, target display, and pixel-perfect settings.
* `Mask`, `RectMask2D`, and `CanvasGroup` (alpha, interactable, blocks
  raycasts, and ignore parent groups) affect drawing and pointer eligibility
  consistently.
* UI scale and readable size choices apply without corrupting saved layout.

## Controls and event behavior

Supported controls include images, raw images, text, buttons, toggles, toggle
groups, sliders, scrollbars, scroll rects, dropdowns, input fields, masks, and
other `Selectable` controls from uGUI and TextMeshPro within scope.

A control has visible, interactable, highlighted, focused, selected, pressed,
and disabled states, and input fields additionally have a validation state.
Selectable transitions (color tint, sprite swap, and animation) present those
states as in Unity. Pointer, touch, keyboard, gamepad, and automation can
interact through the same control behavior.

Events include pointer enter and exit, down, up, click, initialize-potential-
drag, begin drag, drag, end drag, drop, scroll, select, deselect, update
selected, move, submit, cancel, and value changes. An event identifies its
target and propagation behavior: it is delivered through the
`IEventSystemHandler` interfaces (`IPointerClickHandler`, `IDragHandler`, and
the others in uGUI) to the first handler found walking up from the raycast
target, as `ExecuteEvents` does in Unity. A control cannot report a successful
activation when effective pointer hit-testing routes that input to another
element; visibility alone does not imply input blocking.

### Graphics and mesh effects

* `Image` supports Simple, Sliced, Tiled, and Filled types, with fill method
  (horizontal, vertical, radial 90, radial 180, and radial 360), fill origin,
  fill amount, and clockwise direction, `preserveAspect`, fill center, and
  `pixelsPerUnitMultiplier`, producing Unity's geometry.
* `Image.alphaHitTestMinimumThreshold` restricts pointer hits to texels at or
  above the threshold; it requires a readable sprite texture, and a
  non-readable texture produces Unity's compatible error naming it.
* `RawImage` draws its texture with its `uvRect`.
* `Shadow`, `Outline`, and `PositionAsUV1` effects, and project subclasses of
  `BaseMeshEffect` or implementations of `IMeshModifier`, modify the generated
  UI mesh in component order as in Unity.

### Rebuild timing and scrolling

* `Canvas.willRenderCanvases` fires before canvases render each frame, and
  layout and graphic rebuilds run at Unity's point in that sequence.
  `Canvas.ForceUpdateCanvases` and
  `LayoutRebuilder.ForceRebuildLayoutImmediate` complete pending rebuilds
  before returning, so RectTransform values read afterwards reflect them.
* `ScrollRect` movement types (unrestricted, elastic, and clamped),
  elasticity, inertia, deceleration rate, scroll sensitivity, viewport, and
  scrollbar visibility modes behave as in Unity.
* `EventSystem.IsPointerOverGameObject`, with no argument for the mouse and
  with a pointer or touch identifier, returns whether that pointer is over an
  object that a raycaster hits, as in Unity for the active input module.

Control callbacks such as `Button.onClick`, `Toggle.onValueChanged`, and
`Slider.onValueChanged` are `UnityEvent`s; their persistent and dynamic
listeners behave as defined by RUN-008 in
[Scripting runtime](scripting-runtime.md).

## Event system, raycasters, and input modules

* `EventSystem.current` resolves as in Unity when several event systems are
  enabled, and more than one enabled event system produces Unity's warning
  and a diagnostic naming each one. The first selected object, drag
  threshold, and `sendNavigationEvents` retain their meaning.
* `GraphicRaycaster` hits UI graphics whose raycast target is enabled,
  honoring blocking objects, blocking mask, ignore reversed graphics, canvas
  groups, and masks. `PhysicsRaycaster` and `Physics2DRaycaster` hit 3D and 2D
  colliders through the camera with their event mask, so scene objects
  receive the same pointer events as UI. Results from multiple raycasters are
  ordered as in Unity by sort order, depth, and distance.
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
Directional navigation follows explicit links when supplied; Automatic,
Horizontal, and Vertical navigation modes select the neighbor that Unity's
`Selectable.FindSelectable` selects for the same geometry. Disabled, hidden,
or non-interactable controls are skipped.

Changing input device does not strand focus. Modal content constrains focus to
its intended scope. Opening and closing a modal restores the prior selection
when that control still exists and is interactable. The editor and automation
can show why a control did or did not receive navigation or pointer input.

## Immediate-mode runtime GUI

Runtime IMGUI through `OnGUI` is Supported in play mode, headless sessions, and
players:

* `OnGUI` runs once per GUI event on every enabled `MonoBehaviour` that
  defines it, with a Layout event before each other event and a Repaint event
  each frame, as in Unity. `Event.current` reports the event type
  (`MouseDown`, `MouseUp`, `MouseDrag`, `MouseMove`, `KeyDown`, `KeyUp`,
  `ScrollWheel`, `Repaint`, `Layout`, `Used`, and the other runtime types),
  mouse position, button, key code, character, and modifiers; `Event.Use`
  stops further handling.
* `GUI` controls (`Label`, `Box`, `Button`, `RepeatButton`, `Toggle`,
  `TextField`, `TextArea`, `PasswordField`, `HorizontalSlider`,
  `VerticalSlider`, `HorizontalScrollbar`, `VerticalScrollbar`, `Toolbar`,
  `SelectionGrid`, `Window`, `DragWindow`, `BeginGroup`, and `BeginScrollView`)
  and their `GUILayout` counterparts with layout options produce Unity's
  results and return values.
* `GUILayout` computes positions in the Layout pass and reuses them in later
  passes; a control sequence that differs between passes produces Unity's
  compatible error naming the script.
* The default skin, `GUISkin` assets, `GUIStyle` states, `GUI.color`,
  `GUI.backgroundColor`, `GUI.contentColor`, `GUI.enabled`, `GUI.changed`,
  `GUI.matrix`, control naming with `GUI.SetNextControlName` and
  `GUI.FocusControl`, and `GUIUtility.hotControl` and `keyboardControl`
  behave as in Unity.
* `GUI.depth` orders drawing and input between scripts: lower depth draws on
  top and receives input first. IMGUI draws after all cameras in screen
  coordinates with the origin at the top left.
* IMGUI in editor extensions (custom inspectors and editor windows) is outside
  this feature and is reported as listed in the [index](INDEX.md).

## Text and fonts

Runtime text supports Unicode content, font assets, fallback fonts,
size and auto-sizing, alignment, wrapping, overflow, spacing, style, supported
rich text, and resolution-aware rendering for the legacy text components
(UI `Text` and `TextMesh`) and for TextMeshPro 3.0.x.

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
* Legacy UI `Text` Best Fit chooses the size Unity chooses between its
  minimum and maximum, and `Font.RequestCharactersInTexture`,
  `Font.textureRebuilt`, and `Font.GetCharacterInfo` behave as in Unity.

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
  `TMP_TextUtilities` returns the same link as in Unity.
* TMP shaders (distance field, mobile, bitmap, and sprite) render with
  their material properties (face, outline, underlay, glow, and bevel) as
  shader content governed by [Rendering and animation](rendering-and-animation.md).
* A TMP feature outside 3.0.x, or a TMP resource missing from the project
  (for example the TMP Essential Resources), is reported with the affected
  text objects.

### Glyphs, fallback, and scripts

Missing glyphs identify the character, the font, the fallback chain searched,
the locale where one is active, and the affected text object. Fallback is
visible and deterministic: the same content, font assets, and settings select
the same fallback font on every run and target. Text does not silently
disappear, drop unsupported characters, or substitute them without diagnostic
state; the missing-glyph character is drawn as Unity draws it.

Layout uses the displayed glyph metrics and updates when content, font,
scaling, or locale changes. Text layout responds to scaling, long strings,
line breaks, mixed scripts, bidirectional content, and dynamic values within
the language support stated for each locale under LOC-001.

Text rendering produces what the Unity text system in use produces for the
same content: right-to-left presentation through TextMeshPro's right-to-left
option, and no implicit script shaping or bidirectional reordering that
Unity's text systems do not perform. Basic character display is not
represented as complete complex-script support; per-locale support states are
reported as defined by LOC-001.

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

Supported `UnityEngine.Input` behavior covers keyboard keys, `GetKey`,
`GetKeyDown`, and `GetKeyUp` transitions, `anyKey`, `inputString`, mouse
buttons, pointer position and delta, scrolling, named buttons, named axes from
the Input Manager settings, raw axes, touches, `simulateMouseWithTouches`,
multi-touch, acceleration, gyroscope, compass, and location where the device
provides them.

Frame transitions are true only for the eligible frame. Named axes respect
configured sources, dead zone, sensitivity, gravity, inversion, snapping, and joystick number.
Focus loss, cursor lock (`Cursor.lockState`), cursor visibility, and device
disconnection have defined outcomes. Touch identity remains stable from begin
through end or cancel.

When the project's active input handling setting is Input System only, legacy
`Input` APIs throw Unity's `InvalidOperationException`; when it is Both, legacy
and action-based input observe the same devices.

## Action-based input

Actions express gameplay intent independently of a specific device. Action
assets of the Input System 1.x contain action maps, actions (Value, Button, and
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

* Action phases (Disabled, Waiting, Started, Performed, and Canceled), values,
  and callbacks reflect control changes as in the Input System, including
  initial state checks and control disambiguation.
* `PlayerInput` notification behaviors (send messages, broadcast messages,
  invoke Unity events, and invoke C# events), action map switching, and
  control scheme auto-switching behave as in the package. `PlayerInputManager`
  join behaviors, maximum player count, and split-screen layouts assign
  devices and viewports as in the package.
* Device-level access through `Keyboard.current`, `Mouse.current`,
  `Gamepad.current`, `Touchscreen.current`, `Pen.current`, and
  `InputSystem.devices`, and control-level `isPressed`,
  `wasPressedThisFrame`, `wasReleasedThisFrame`, and `ReadValue`, report the
  same values and frame transitions as in the package. `EnhancedTouch`
  (`Touch.activeTouches` and finger tracking) is Supported once enabled.
* On-screen controls (`OnScreenStick` and `OnScreenButton`) feed their bound
  control paths as virtual devices.
* The Input System settings asset (update mode, background behavior, default
  dead zones, default press point and tap and hold times, supported devices,
  and filtering of noisy controls) applies as in the package.
* Custom processors, interactions, composites, and device layouts registered
  by project code (`InputSystem.RegisterProcessor`, `RegisterInteraction`,
  `RegisterBindingComposite`, and `RegisterLayout`) participate in binding
  resolution as built-in ones do. A binding that names an unregistered
  processor, interaction, composite, or layout is reported with the action
  and binding.
* The generated C# wrapper class for an action asset exposes the same maps and
  actions as the asset.
* The input update mode (dynamic update, fixed update, or manual) determines
  when actions observe new values.
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
when the clip ends, as in Unity; the temporary source is visible in voice
inspection while it plays.

Playback state distinguishes scheduled, loading, playing, paused, virtualized,
stopped, completed, and failed. Seeking and time reporting use the clip's
length in samples and the target's decoder capability. Loop boundaries are
gapless for clips whose load type and encoding allow seamless looping on the target.

Destroying or disabling an owner, changing scenes, pausing the application
(`AudioListener.pause`, `ignoreListenerPause`, and application pause),
changing `Time.timeScale`, and changing global volume have explicit effects
according to the compatible API and project settings.

### Clip loading

* Clip load types Decompress On Load, Compressed In Memory, and Streaming
  retain their Unity meaning, together with Preload Audio Data and Load In
  Background. `AudioClip.loadState`, `LoadAudioData`, and `UnloadAudioData`
  report and change residency as in Unity.
* `AudioClip.GetData` returns the clip's samples for clips that Unity allows
  to be read (uncompressed or Decompress On Load clips), and `SetData` writes
  samples that later playback renders. Reading a clip whose load type does not
  permit it fails with Unity's compatible error naming the clip.
* `AudioClip.Create`, including streamed clips with PCM reader and position
  callbacks, produces playable clips with the requested length, channels, and
  frequency.
* Encodings or sample rates that a target cannot play are reported at import
  or build with the clip and target named.

## Spatial audio and listeners

Three-dimensional audio responds to listener and source position, distance
attenuation (logarithmic, linear, and custom rolloff curves), spatial blend,
source direction, Doppler behavior, spread, and reverb zone mix.
Two-dimensional audio remains independent of scene distance.

Spatializer and ambisonic decoder plugins selected in the project's audio
settings are native plugins assessed as defined by CMP-003 in the
[Compatibility contract](compatibility-contract.md); the selected plugin and
its compatibility state are reported. Audio import formats are owned by
[Projects and assets](project-and-assets.md).

`AudioReverbZone` components apply their preset or custom room parameters
between minimum and maximum distance, and overlapping zones blend as in Unity.

The runtime identifies the active listener state. Missing listeners, conflicting
listeners, inaudible range, muted routing, zero volume, paused state, unsupported
output, and voice limits are distinct reasons for silence.

## Mixing and routing

`AudioMixer` assets retain groups, the group hierarchy, sends and receives,
effect chains, ducking, solo, mute, bypass, and views.

* Groups control volume, mute, solo, and effects. The built-in mixer effects
  (attenuation, lowpass, highpass, simple lowpass and highpass, echo, flange,
  distortion, normalize, parametric EQ, pitch shifter, chorus, compressor,
  SFX reverb, send, receive, and duck volume) process audio as in Unity.
* Snapshots store parameter states; `AudioMixerSnapshot.TransitionTo` and
  `AudioMixer.TransitionToSnapshots` with weights transition over the given
  time.
* Exposed parameters are readable and writable by name through `SetFloat`,
  `GetFloat`, and `ClearFloat`; an unknown parameter name returns false as in
  Unity and is reported in diagnostics.
* Playable audio follows the same routing: an `AudioPlayableOutput` plays
  through its target `AudioSource`, whose output mixer group then applies,
  and an `AudioClipPlayable` honors its clip's load type and its own loop
  setting. Playable graph behavior is owned by ANI-002 in
  [Rendering and animation](rendering-and-animation.md).
* Mixer effects provided by native audio plugins are project native plugins,
  assessed as defined by CMP-003 in the
  [Compatibility contract](compatibility-contract.md).
* Voice count (`AudioSettings` real and virtual voice limits), priority, and
  resource limits determine which sources are virtualized or rejected, as in
  Unity, and expose which sources were affected.

The developer and authorized agents can monitor active voices, levels, routing,
clipping, latency where the platform reports it, and the object that owns each
source. Preview audio is distinguished from runtime audio.

## Audio filters and custom processing

* Filter components (`AudioLowPassFilter`, `AudioHighPassFilter`,
  `AudioEchoFilter`, `AudioDistortionFilter`, `AudioReverbFilter`, and
  `AudioChorusFilter`) on an `AudioSource` or `AudioListener` process audio in
  component order as in Unity.
* `OnAudioFilterRead(float[] data, int channels)` receives interleaved samples
  at the output sample rate in its component position in the filter chain, on
  the audio thread as in Unity. A script on a GameObject without an
  `AudioSource` acts as a generator. Unity APIs called from this callback are
  governed by the main-thread rules of RUN-004 in
  [Scripting runtime](scripting-runtime.md). An exception in the callback is
  reported with the script and does not stop other audio.
* `AudioSource.GetOutputData`, `GetSpectrumData`, and the `AudioListener`
  equivalents return the same windowed data as in Unity for the same signal.

## Audio timing and scheduling

* `AudioSettings.dspTime` advances with the number of samples output by the
  audio system, independent of `Time.timeScale` and frame rate.
  `AudioSettings.outputSampleRate`, `GetDSPBufferSize`, `GetConfiguration`,
  `Reset`, and `OnAudioConfigurationChanged` behave as in Unity; a device
  change raises the configuration-changed callback.
* `AudioSource.PlayScheduled(dspTime)` starts playback at the output sample
  corresponding to that time, and `SetScheduledStartTime` and
  `SetScheduledEndTime` adjust a scheduled source. Two clips scheduled back to
  back at exact sample boundaries play with no gap and no overlap. A time
  already in the past starts playback at the next audio buffer.
* In headless sessions, audio renders to a capturable output at the project's
  audio output sample rate so scheduling and mixing results can be verified as defined in
  [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).

## Microphone capture

* `Microphone.devices`, `Microphone.Start` (device, loop, length, and
  frequency), `End`, `IsRecording`, `GetPosition`, and `GetDeviceCaps` behave
  as in Unity, and the recorded `AudioClip` contains the captured samples.
* Capture requires the target's permission: the project requests it with
  `Application.RequestUserAuthorization` or the Android microphone
  permission, and a denied, revoked, or not-yet-granted permission returns an
  explicit status and an empty device list or failed start rather than
  silence presented as recording.
* Targets where Unity does not offer the `Microphone` class (the browser
  target) report microphone capture as Platform-limited.

## Video playback

`VideoPlayer` plays video from a `VideoClip` asset or a URL (local file,
StreamingAssets path, or network address), as in Unity.

* **Control:** `Prepare`, `isPrepared`, `Play`, `Pause`, `Stop`, `time` and
  `frame` seeking, `isLooping`, `playbackSpeed` (with `canSetPlaybackSpeed`),
  `skipOnDrop`, `waitForFirstFrame`, `playOnAwake`, and step forward behave as
  in Unity. Seeking to a time or frame reports completion through
  `seekCompleted`.
* **Events:** `prepareCompleted`, `started`, `frameReady` (when
  `sendFrameReadyEvents` is enabled), `loopPointReached`, `frameDropped`,
  `clockResyncOccurred`, and `errorReceived` fire at the points Unity defines.
  An error message names the source and the reason.
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
  time or runs independently. Pausing the application and changing
  `Time.timeScale` affect playback according to these settings.
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
2. Layout matches Unity's RectTransform results across the resolutions and
   aspects a scenario lists, safe areas, and `CanvasScaler` modes, and a layout
   cycle is reported with the participating objects. (UI-001)
3. Pointer events reach UI graphics through `GraphicRaycaster` and scene
   objects through `PhysicsRaycaster` and `Physics2DRaycaster`, and an object
   that received no event reports why. (UI-001)
4. A `Button.onClick` persistent listener and a dynamically added listener both
   run on activation, as defined by RUN-008. (UI-001)
5. An `Image` in Filled radial 360 mode at fill amount 0.25, a sliced image
   with `pixelsPerUnitMultiplier` 2, and an `Outline` effect produce Unity's
   geometry; `alphaHitTestMinimumThreshold` ignores transparent texels; and
   `LayoutRebuilder.ForceRebuildLayoutImmediate` makes updated sizes readable
   in the same frame. (UI-001)
6. `EventSystem.IsPointerOverGameObject` returns true for the mouse and for a
   touch identifier over UI, and a `ScrollRect` with elastic movement and
   inertia settles at Unity's position. (UI-001)
7. An `OnGUI` script with `GUILayout` controls, a custom `GUISkin`, and two
   scripts at different `GUI.depth` values draws, orders input, and returns
   control values as in Unity. (UI-002)
8. TextMeshPro text with fallback fonts, sprite assets, rich-text tags, and
   ellipsis overflow renders with Unity's layout and `textInfo`. (TXT-001)
9. Missing glyphs are visible diagnostics that name the character, font,
   fallback chain, locale, and text object, and fallback selection is the
   same on every run and target. (TXT-001)
10. Mixed-script and right-to-left text render as the Unity text system in use
    renders them, and no complex-script support beyond that is claimed.
    (TXT-001)
11. A legacy UI `Text` using Best Fit and a dynamic TrueType font selects
    Unity's size and populates its glyph texture, and
    `Font.CreateDynamicFontFromOSFont` is reported Platform-limited on the
    browser target. (TXT-001)
12. Frame-based legacy input transitions occur only on their eligible frames,
    and legacy `Input` APIs throw Unity's exception when the project uses the
    Input System only. (INP-001)
13. Interactive rebinding excludes reserved controls, saved overrides restore
    on the next run, and stale or conflicting bindings are reported explicitly.
    (INP-002)
14. `Gamepad.current` and `Keyboard.current` report `wasPressedThisFrame` on
    the same frame as the bound action performs, an `OnScreenStick` drives a
    2D vector action, and a project-registered custom processor alters the
    action value. (INP-002)
15. Two local players join through `PlayerInputManager` with distinct devices,
    and device disconnection and reconnection neither duplicate nor lose an
    action. (INP-002)
16. Every inaudible AudioSource has an inspectable state and routing reason.
    (AUD-001)
17. Spatial audio responds consistently to source and listener transforms and
    to reverb zones. (AUD-001)
18. A mixer snapshot transition and an exposed parameter change alter the
    captured output level over the given transition time, and filter
    components and an `OnAudioFilterRead` script process audio in component
    order. (AUD-001)
19. Two clips scheduled with `PlayScheduled` at adjacent sample boundaries
    render with no gap or overlap in captured headless output, and
    `AudioClip.GetData` and `SetData` round-trip samples on a Decompress On Load
    clip. (AUD-001)
20. `AudioSource.PlayClipAtPoint` plays a clip at a position and its
    temporary object is destroyed when playback ends, and an
    `AudioPlayableOutput` plays through its `AudioSource` with the source's
    mixer group applied. (AUD-001)
21. Microphone capture records samples when permission is granted and reports
    an explicit denied status otherwise. (AUD-001)
22. A `VideoPlayer` prepares, plays, pauses, seeks, loops, and fires its events
    in each render mode and audio output mode, and a clip with a codec the
    target cannot play reports the codec, container, and target through
    `errorReceived` and diagnostics. (MED-001)
23. UI, text, input, audio, and video behavior agrees between editor play,
    headless sessions, and supported players, except for device capabilities
    reported as Platform-limited. (UI-001, UI-002, TXT-001, INP-001, INP-002,
    AUD-001, MED-001)
