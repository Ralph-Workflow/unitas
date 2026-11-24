# User Interface, Text, Audio, and Input

## Scope

This specification defines player-facing user interfaces, text and
localization, input devices and actions, audio playback and spatialization, and
the editor diagnostics used to validate them.

These are desired end-state requirements, not claims about currently shipped
capability. Stock Unity 2022.3 LTS defines the baseline. TextMeshPro, the Input
System, Localization, UI Toolkit, and other package features are covered only
for explicitly supported package families, versions, and feature subsets.

## Canvas and layout

The runtime user-interface feature supports screen-space and world-space
canvases. RectTransform content exposes anchors, pivot, position, size, offsets,
rotation, scale, and hierarchy relationships.

### Required behavior

* Layout responds predictably to parent size, reference resolution, aspect
  ratio, screen orientation, safe area, and content changes.
* Anchors and pivots retain their declared geometric meaning.
* Nested layout components reach a stable result or report a cycle or conflict.
* Screen-space overlay, camera-associated, and world-space interfaces remain
  distinct and inspectable.
* Clipping and masks affect drawing and pointer eligibility consistently.
* UI scale and readable size choices apply without corrupting saved layout.

## Controls and event behavior

Supported controls include images, text, buttons, toggles, toggle groups,
sliders, scrollbars, scrollable regions, dropdowns, input fields, masks, and
selectable controls within scope.

A control has visible, interactable, focused, selected, pressed, disabled, and
validation states where meaningful. Pointer, touch, keyboard, gamepad, and
automation can interact through the same declared control behavior.

Events include pointer enter and exit, press, release, click, drag, drop,
scroll, selection, deselection, navigation, submit, cancel, and value changes.
An event identifies its target and propagation behavior. A control cannot report
a successful activation when effective pointer hit-testing routes that input
to another element; visibility alone does not imply input blocking.

## Focus and navigation

The interface maintains one unambiguous selected control per applicable event
context. Directional navigation follows explicit links when supplied and a
predictable automatic relationship otherwise. Disabled, hidden, or
non-interactable controls are skipped.

Changing input device does not strand focus. Modal content constrains focus to
its intended scope. Opening and closing a modal restores a meaningful prior
selection when possible. The editor can show why a control did or did not
receive navigation or pointer input.

## Text and fonts

Runtime text supports Unicode content, font assets, fallback fonts, size,
alignment, wrapping, overflow, spacing, style, supported rich text, and
resolution-aware rendering.

Missing glyphs identify the character, font, and fallback result. Layout uses
the displayed glyph metrics and updates when content or locale changes. Text
does not silently disappear because a requested glyph is unavailable.

Input fields distinguish displayed text, placeholder, selection, caret,
composition, validation, password presentation, submission, cancellation, and
focus loss. Clipboard behavior and on-screen keyboard availability are stated
per platform.

## Localization

As a developer, I can provide localized strings and assets, choose a locale at
startup or runtime, preview locales in the editor, and identify missing or stale
translations.

Supported localization behavior includes variables, plural-sensitive content,
locale-aware formatting, fallback locales, and localized assets within scope.
Right-to-left and complex text support is declared per language and feature; the
presence of Unicode characters alone is not treated as full language support.

Changing locale updates eligible visible content without requiring recreation
of unrelated game state. Missing translations remain visibly diagnosable and do
not silently masquerade as translated content.

## Legacy input behavior

Supported UnityEngine.Input behavior covers keyboard keys, press and release
transitions, mouse buttons, pointer position and delta, scrolling, named
buttons, named axes, touches, and supported device motion.

Frame transitions are true only for the eligible frame. Named axes respect
configured sources, dead zone, sensitivity, gravity, inversion, and snapping.
Focus loss, cursor lock, cursor visibility, and device disconnection have
defined outcomes. Touch identity remains stable from begin through end or
cancel.

## Action-based input

Actions express gameplay intent independently of a specific device. An action
can have multiple bindings, composites, interactions, processors, control
schemes, and phases within the supported scope.

### User stories

* As a player, I can use keyboard and mouse, gamepad, touch, or supported XR
  controls according to the game's declared schemes.
* As a player, I can rebind controls and retain my choices.
* As a local multiplayer developer, I can assign distinct devices to players.
* As a tester, I can see which physical control caused an action.

### Required behavior

Action phases and values reflect control changes predictably. Device changes do
not duplicate or lose an eligible action. Binding conflicts are visible.
Rebinding can exclude reserved controls and supports cancellation. Saved
overrides identify the action and binding they affect and handle changed
projects explicitly.

Input diagnostics show connected devices, controls, current values, resolved
bindings, action phases, player assignment, focus, and the reason an action did
not fire. Automated input is marked as synthetic in playtest evidence.

## Audio clips and sources

AudioSource and AudioClip support one-shot and looping playback with clip,
volume, pitch, mute, pause, start time, priority, output destination, and spatial
behavior.

Playback state distinguishes scheduled, loading, playing, paused, virtualized,
stopped, completed, and failed. Seeking and time reporting use the clip's
declared duration and platform capability. Loop boundaries avoid unintended
gaps where the source and target support seamless looping.

Destroying or disabling an owner, changing scenes, pausing the application, and
changing global time have explicit effects according to the compatible API and
project settings.

## Spatial audio and listeners

Three-dimensional audio responds to listener and source position, distance
attenuation, spatial blend, source direction, Doppler behavior, spread, and
supported environmental effects. Two-dimensional audio remains independent of
scene distance.

The runtime identifies the active listener state. Missing listeners, conflicting
listeners, inaudible range, muted routing, zero volume, paused state, unsupported
output, and voice limits are distinct reasons for silence.

## Mixing and routing

Supported audio routing allows groups to control volume, mute, solo, and
declared effects. Changes can transition over time where the project specifies
it. Voice priority and resource limits have predictable outcomes and expose
which sources were virtualized or rejected.

The developer can monitor active voices, levels, routing, clipping, latency
where measurable, and the object that owns each source. Preview audio is
distinguished from runtime audio.

## Device and platform behavior

Input and audio capabilities are reported for the active platform and connected
hardware. Connection, disconnection, permission denial, focus changes, output
device changes, mobile interruptions, and application backgrounding have
defined, inspectable results.

Unavailable vibration, haptics, microphones, motion sensors, on-screen
keyboards, or audio outputs return explicit status. Platform capability is not
inferred merely because a corresponding API exists.

## Accessibility outcomes

The feature set enables games to provide remappable controls, keyboard-only and
controller-only navigation, scalable text and interfaces, captions and
subtitles, alternatives to audio-only cues, reduced-motion choices, and
alternatives to color-only cues.

Playtest scenarios can validate focus order, device-only operation, high-scale
layouts, captions, localization expansion, safe areas, and interruption
recovery.

## Acceptance outcomes

1. The same UI flow can be completed with every input scheme the game declares
   supported.
2. Layout remains usable across declared resolutions, aspects, safe areas, and
   locales.
3. Missing glyphs and translations are visible diagnostics, not silent absence.
4. Frame-based input transitions occur only on their eligible frames.
5. Rebinding persists and reports conflicts or stale bindings explicitly.
6. Every inaudible AudioSource has an inspectable state and routing reason.
7. Spatial audio responds consistently to source and listener transforms.
8. UI, input, and audio behavior agrees between editor play and supported
   players, subject to declared device differences.
