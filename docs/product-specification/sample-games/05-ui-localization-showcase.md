# Sample 5: UI and Localization Showcase ("Night Desk")

## Overview

Night Desk is a short puzzle game played entirely through its interface. The
player is the night clerk at the Lanternport post office, a harbor city with
nine districts that each write in their own language. Letters arrive on the
desk one at a time. The player reads each letter, checks the rate card and
the district guide, fills in the routing slip, stamps it, and sends it down
the pneumatic tube before the shift clock runs out. Perfect slips build a
streak, mistakes come back as returned letters, and every perfect letter adds
a stamp to the player's album. A shift of six letters takes 5 to 8 minutes;
the three shifts take 15 to 25 minutes.

The menus, forms, scroll views, modals, and live language switching are the
gameplay itself. The same play-through is completed with pointer, keyboard,
controller, and touch on the targets that offer each input mode. Automation
means headless control of editor play sessions through the editor's MCP
interface, and replayed input in packaged players.

This is an original, redistributable unitas sample. Its story, art,
translations, and audio are unitas content, its fonts are redistributable
fonts with recorded licences, and it is not derived from any Unity sample. It
is one of the demonstrations of Canvas and RectTransform layout, controls,
focus, text, font fallback, localization, accessibility, and persistent
preferences; the acceptance of those features is owned by the domain
specifications it cites.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns this game's acceptance under SMP-001.

## Targets and Input Modes

### Targets

* **Automated:** macOS, Windows, and Linux desktop players; iOS and Android
  players on the iOS Simulator and the Android emulator; the Web target
  ([Web target](../web-target.md)).
* **Capability-reported:** iOS and Android physical devices (optional); touch
  on desktop and Web target hosts, offered only where the host reports a
  touchscreen.
* **Editor play mode:** yes.
* **Headless editor:** yes.

### Render pipelines

URP 14.x (Forward) for the 3D desk scene behind the interface.

### Input modes and devices

Pointer (mouse), keyboard, gamepad, and touch, all through one Input System 1.x
action asset and `InputSystemUIInputModule`. Touch is offered on the mobile
targets and on desktop and Web target hosts that report a touchscreen. The
capability report states which modes each target offers, and a mode a target
does not offer is reported unevaluable on that target, never as passed.

Headless runs use synthetic devices marked synthetic; haptic and feel
checkpoints need physical hardware and are otherwise unevaluable.

## Game Design

### Core loop and flow

1. The title shows Start Shift, Continue (when a shift is in progress), Stamp
   Album, Settings, and Quit. A first-run panel offers language, text size,
   high contrast, reduced motion, and relaxed timing before play begins.
2. A shift starts at 22:00 on the desk clock. One game minute passes per real
   second, so a timed shift lasts 8 real minutes and ends at 06:00. The HUD
   shows the clock, letters left, score, and streak.
3. A letter slides onto the desk. Opening it shows the letter modal: the
   sender's note in the district's language, the address card, and the
   parcel weight.
4. The player fills the routing slip on the desk's world-space canvas:
   * District: a dropdown of the nine districts.
   * Postage: a slider from 1 to 9 stamps, priced by the rate card.
   * Service: a toggle group of Standard, Express, and Night Boat.
   * Handling: toggles for Fragile and Keep Dry.
   * Recipient: an input field, typed in the recipient's own script.
5. Pressing and holding Stamp for 0.6 s opens the supervisor modal, "Send
   this letter?", with Send and Check Again. Send scores the slip and drops the
   letter into the tube.
6. The shift ends when every letter is sent or the clock reaches 06:00. The
   shift report shows score, medal, perfect letters, returned letters, and new
   stamps, with Next Shift, Replay Shift, and Quit to Title.
7. Quit on the title exits the desktop player; it is hidden on mobile and Web
   target players.

### Rules and mechanics

* The district guide is a scroll view of nine district cards, and the rate card
  is a scroll view of rules in the selected language. Both are open at any time
  from the desk.
* The rate card: 2 stamps per 100 g, plus 1 for Express and 2 for Night Boat.
* The recipient field accepts up to 24 characters and validates against the
  letter's expected script, showing an invalid-script state otherwise.
* The letter and supervisor modals constrain focus to themselves: the game's
  scripts keep the selected object inside the open modal and restore the prior
  selection on close. A `CanvasGroup` blocks raycasts to the desk while a modal
  is open.
* Holding the Translation Lens shows the sender's note in the selected language
  and costs 15 game minutes, or 15 points with relaxed timing. The Archivist
  hint fills one correct field for 30 points.
* The control hints switch to the most recently used device within one frame.
* The stamp's ink splash is rotated by a random angle between -8 and 8 degrees
  from a seeded generator.

### Levels and progression

| Shift | Letters | New elements | Medal thresholds (Bronze / Silver / Gold) |
|---|---|---|---|
| 1. Quiet Harbor | 6 | districts, postage, the rate card | 300 / 600 / 900 |
| 2. Festival Night | 6 | handling rules, a second page of the rate card, letters in right-to-left scripts | 400 / 800 / 1,200 |
| 3. Storm Mail | 6 | Keep Dry and Night Boat rules, two letters arriving at once, recipient names in four scripts | 500 / 1,000 / 1,500 |
| 0. Lantern Isles (unlock) | 6 | letters in Lanternese, the pseudo-locale, and mixed scripts on one slip | 600 / 1,200 / 1,800 |

A medal is awarded when the shift score is at or above its threshold. Each
perfect letter adds its stamp to the Stamp Album, a grid of 24 stamps, one per
letter. Gold on shifts 1 to 3 unlocks shift 0 and Lanternese. Records set with
relaxed timing carry a marker.

### Scoring, failure, and edge rules

* A slip earns 20 points per correct field; a fully correct slip has five
  correct fields and earns 100. A speed bonus of 2 points per second under 40
  seconds, from opening the letter to Send, is added. The total is multiplied
  by the current streak multiplier.
* Consecutive perfect slips raise the streak multiplier to x2 after two and x3
  after four, and a mistake resets it to x1.
* A wrong district returns the letter 60 game minutes later with a red
  "Returned" stamp and costs 50 points. A returned letter that is routed
  correctly the second time earns half points.
* When the clock reaches 06:00 with letters unsent, the shift ends as "Shift
  missed". The player keeps the score but earns no medal. Retry restarts the
  shift with every slip, streak, and score reset.
* An unknown stored locale code falls through the startup locale selectors to
  the system locale or en, and a notice says so. A corrupt progress file is
  reported as corrupt, not absent.

### Controls

| Action | Pointer | Keyboard | Gamepad | Touch |
|---|---|---|---|---|
| Move focus | — | Tab, Shift+Tab, arrow keys | D-pad, left stick | — |
| Activate | Left click | Enter, Space | South button | Tap |
| Back, close modal | Right click, close button | Escape | East button | Close button, system back on Android |
| Scroll | Wheel, drag scrollbar | Page Up, Page Down | Right stick | Drag, fling |
| Adjust slider | Drag | Left and Right arrows | D-pad left and right | Drag |
| Stamp (hold 0.6 s) | Hold left button | Hold S | Hold West button | Press and hold |
| Translation Lens (hold) | Hold right button on the letter | Hold L | Hold left trigger | Two-finger press on the letter |
| Open guide, rate card | Tab buttons | G, R | Left and right shoulder | Tab buttons |
| Pause | Pause button | P | Start | Pause button |
| Debug panel (development builds) | — | F1 | Hold Select (View) for 1 s | Three-finger tap |

Every gameplay keyboard and gamepad binding can be rebound.

### Pause, focus, and lifecycle

Pause sets `Time.timeScale` to zero, stops the clock, hides the letter text
behind a pause panel (Resume, Settings, Quit to Title), and keeps menu sounds
playing. Quit to Title saves the shift in progress. Losing focus pauses the
game, and it stays paused until the player resumes. On mobile, backgrounding
pauses the same way; on the Web target, a refresh reloads to Continue with the
saved shift.

### Settings

Language (the nine locales, plus Lanternese once unlocked); text size 100,
125, 150, or 200 percent; high contrast; reduced motion; relaxed timing (no
clock); master, music, and effects volume; control bindings; and Clear
Preferences, which leaves progress and the album intact. Every setting
persists through `PlayerPrefs` and applies immediately, without reloading the
scene.

### What makes it fun

* **Hook.** Within 10 seconds a letter rattles onto the desk under a
  flickering lamp. The first letter needs only a district and a postage
  value; stamping it lands with a heavy thunk, the ink spreads, and the letter
  whooshes into the tube with "+100" and "Streak x1". The second letter, by
  second 30, is written in Japanese, and the desk's Translation Lens appears.
* **Mastery curve.** A new player reads every rule and finishes shift 1 in
  about 7 minutes. A skilled player learns the rate card, recognizes districts
  by their stamp shapes and script, navigates the slip without the pointer, and
  finishes shift 1 in under 4 minutes with a x3 streak.
* **Risk and reward.** The Translation Lens and the Archivist hint trade time
  or points for certainty. Sending without checking the guide is fast and earns
  the speed bonus, but a wrong district comes back as a returned letter and
  breaks the streak.
* **Variety and escalation.** Each shift adds the elements in the shifts
  table. Shift 3 delivers two letters at once, so the player switches between
  two open slips. Shift 0 mixes scripts on one slip and uses Lanternese, whose
  expanded text tests both the player and the layout.
* **Game feel.** Every control answers within one frame with a 0.1 s color
  tint and a soft click. The stamp shakes the slip by 4 pixels and leaves an
  ink splash. The streak counter pops to 1.3 scale for 0.15 s on each
  increase. A returned letter flops onto the desk with a paper thud. The clock
  ticks louder in the last 30 game minutes. Reduced motion replaces every
  movement with an instant change and keeps the sounds.
* **Replay.** Streak multipliers, medals per shift, best score and best time
  per shift, the Stamp Album, and the shift 0 unlock.

## Content

### Assets

| Asset | Source format | Import settings that matter |
|---|---|---|
| `Title.unity`, `Desk.unity` | Unity YAML scene | overlay, camera-space, and world-space canvases; event system; references |
| `LetterModal.prefab`, `ConfirmModal.prefab` | Unity YAML prefab | nested `Panel.prefab`, `CanvasGroup`, focus scope |
| `DistrictCard.prefab`, `StampCell.prefab` | prefab and variants | variants per district overriding stamp sprite and accent color |
| `RoutingSlip.prefab` | prefab | vertical layout group, content size fitters, TMP dropdown, input field, slider, toggle group |
| Desk, lamp, tube | FBX models, PNG textures | URP Lit materials, sRGB, mipmaps |
| UI sprites | PNG, nine-slice | 12-pixel borders, `pixelsPerUnitMultiplier` 1, packed in `UI.spriteatlasv2` |
| Fonts | OpenType sources for Latin, Hebrew, Arabic, Devanagari, Thai, Japanese, and Simplified Chinese | dynamic TMP SDF font assets, 2048 atlas, multi-atlas on for CJK |
| `Emoji.asset` | TMP sprite asset from a PNG sheet | 32 original emoji glyphs mapped to their Unicode code points |
| `Desk Strings`, `Letters`, `A11y` | Localization string table collections | keys, key identifiers, Smart String flags, comments |
| `Desk Assets` | Localization asset table collection | per-locale primary font asset, district stamp sprites, supervisor voice line |
| Locales | Locale assets for en, de, pl, ar, he, hi, th, ja, zh-Hans, and the Lanternese pseudo-locale | right-to-left metadata on ar and he, fallback to en |
| Sounds and music | WAV effects, OGG music, `Desk.mixer` | decompress-on-load clicks, streamed music, Master, Music, Effects, UI groups |
| `Controls.inputactions` | Input System action asset | UI and Desk maps; Keyboard, Gamepad, Touch, and Pointer schemes |
| `ProjectSettings/` | Unity YAML | default locale, player resolution, orientation (portrait and landscape on mobile), Text layout mode Unicode |

The `A11y` table holds the accessible names of icon-only controls. The game's
calls to the unitas accessibility extension are guarded by a scripting define
that only unitas sets, so the project compiles and runs in Unity 2022.3 LTS
without them.

### Packages

uGUI 1.0.x, TextMeshPro 3.0.x, Localization 1.x (with the Addressables 1.x
line it depends on), Input System 1.x, and URP 14.x, at the version lines
CMP-002 lists. Packages are resolved from their registry when the project
opens and are never vendored in the sample repository.

### Save data

Preferences in `PlayerPrefs`; shift progress, records, and the album in a
versioned progress file under `Application.persistentDataPath`.

### Licences and provenance

Every asset family's source, author, and licence, fonts and emoji sheets
included, is recorded in `Licenses/content-licenses.json`. Unity packages'
licence terms are recorded in the build's licence report.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001 | The project opens in place with no conversion | S1 |
| PRJ-003 | Table, locale, font asset, and prefab identities stay stable | S1 |
| PRJ-004 | Font sources, sprite sheets, models, WAV and OGG | S1 |
| PRJ-006 | Localization, Addressables, uGUI, TextMeshPro, and Input System resolve | S1 |
| SER-002 | Desk scene | S1, S2 |
| SER-003 | Nested modal prefabs, district card variants | S1, S2 |
| RUN-003 | Lifecycle and focus messages | S2, S8 |
| RUN-008 | `UnityEvent` control callbacks | S2, S8 |
| RUN-009 | Progress file under `Application.persistentDataPath` | S11 |
| UI-001 | Canvas, RectTransform, layout groups, content size fitters, scroll views, masks, input fields, dropdowns, sliders, toggles, modals, world-space canvas, focus, navigation | S2, S3, S5, S7, S8, S10 |
| UI-002 | The immediate-mode debug panel | S14 |
| TXT-001 | TextMeshPro, fallback chains, complex-script content, emoji sprite asset, rich-text tags, missing-glyph diagnostics | S5, S7, S14 |
| LOC-001 | String and asset tables, Smart Strings with plurals and variables, runtime locale switching, right-to-left locales, pseudo-localization, missing-translation behavior | S4, S5, S6, S7, S14 |
| ACC-001 | Accessible names and roles, focus order, text scaling, high contrast, reduced motion, relaxed timing | S3, S7, S9 |
| SAV-001 | Preferences in `PlayerPrefs`, progress file, recovery from bad values | S11, S13 |
| INP-002 | Pointer, keyboard, gamepad, and touch schemes; rebinding; device changes | S3, S8 |
| PLT-001 | Safe areas, orientation, mobile and Web target lifecycle | S10, S15 |
| AUD-001 | UI sounds, music, mixer groups, Web target audio activation | S2, S12 |
| DIA-001 | Missing-translation, missing-glyph, and layout diagnostics with navigation | S14 |
| BLD-001 | Desktop, mobile, and Web target players with build reports | S15 |
| BLD-002 | Headless runs of every scenario that lists the headless editor | S1–S9, S11–S14 |
| TST-002 | Scenarios, checkpoints, approved references, input replay | all |
| AUT-002 | Agents play, inspect UI and locale state, and capture frames | S1–S9, S11–S14 |

## Engine Behavior This Game Depends On

Only engine and package behavior, each tied to a catalog ID. Game rules are
defined under Game Design and implemented by the sample's own scripts; the
engine contains no code specific to this game. Package behavior (uGUI,
TextMeshPro, Localization, Input System) comes from the package's own code at
the pinned version line, compiled and run unmodified (CMP-002). Agents reach
the game only through the editor's generic MCP object, property,
method-invocation, input, and capture commands, with no game-specific command.

### Scripting and runtime

* Control callbacks (`Button.onClick`, `Toggle.onValueChanged`,
  `Slider.onValueChanged`, `TMP_Dropdown.onValueChanged`, and
  `TMP_InputField.onEndEdit`) fire once per change with persistent listeners as
  serialized. (RUN-008)
* Scaled time stops at `Time.timeScale = 0`; unscaled time keeps running.
  (RUN-004)

### Project and content

* Every string table entry, asset table entry, and locale keeps its key
  identifier and metadata across reimport. Prefab variants apply their
  overrides to every instance. (PRJ-003, SER-003, LOC-001)

### Simulation

No requirements beyond the domain documents.

### Rendering and animation

* The 3D desk scene renders with URP 14.x Forward behind the canvases. (REN-006)

### Audio and video

* One-shots start within one frame of their call. On the Web target no sound
  plays before the first user gesture. (AUD-001, PLT-001)

### Input

* `InputSystemUIInputModule` routes pointer, keyboard, gamepad, and touch to the
  event system; a device change leaves the current selection unchanged; the
  most recently used device is reported within one frame. (INP-002, UI-001)
* A text field shows IME composition text underlined before commit and opens
  the on-screen keyboard on mobile targets. (TXT-001, INP-002)

### UI and text

* A canvas with Scale With Screen Size (1920 by 1080 reference, match 0.5)
  places panels by anchors and offsets as UI-001 defines, and recomputes layout
  when the size, aspect, orientation, safe area, text size, or locale changes.
  (UI-001)
* Vertical layout groups and content size fitters grow rows to their text; a
  grid layout's column count changes with the width the script sets; scroll
  views with `RectMask2D`, Elastic movement, and inertia, and `Mask` with a
  circle sprite, behave as UI-001 defines. (UI-001)
* A world-space canvas receives pointer and touch events through its
  `GraphicRaycaster` and event camera; a `CanvasGroup` with `blocksRaycasts`
  off stops raycasts to its children. (UI-001)
* A TMP font asset's fallback list is searched in order, then the global
  fallback list, and the selected font for any character is the same on every
  run and target. Rich-text tags `<b>`, `<i>`, `<color>`, `<size>`, `<link>`,
  and `<sprite>` render, and emoji characters resolve to the sprite asset.
  (TXT-001)
* With the project's Text layout mode set to Unicode, TextMeshPro shapes Arabic
  joining forms, orders Hebrew and Arabic runs mixed with Latin text, numbers,
  and punctuation by the Unicode bidirectional algorithm, and forms Devanagari
  and Thai clusters with correct line breaking (TXT-001). Opened in Unity
  2022.3 LTS, the same text is unshaped; that is a listed difference.
* Smart Strings select plural forms by each language's rules (en and de: one,
  other; pl: one, few, many, other; ar: zero, one, two, few, many, other; ja
  and zh-Hans: other) and format numbers and times by the selected locale's
  culture. Assigning `LocalizationSettings.SelectedLocale` refreshes every bound
  string and asset on the next rendered frame for preloaded tables and raises
  `SelectedLocaleChanged` once. (LOC-001)
* Through the unitas player accessibility extension, which is identified as a
  unitas extension and not as Unity behavior, every control exposes a name, a
  role (button, toggle, slider, dropdown, text field, tab, dialog), its value,
  and its state. Automation reads them in editor play sessions and the platform
  accessibility interface reads them in packaged players. (ACC-001)

### Player and build

* Mobile players report `Screen.safeArea` and orientation changes; Web target
  players report loading, storage, and audio activation as PLT-001 defines.
  (PLT-001)
* Each build report lists scenes, tables, fonts, sizes, and the build
  identifier. (BLD-001)

### Editor and automation

* Through the editor's generic MCP commands an agent selects a locale, reads
  every visible string with its table, key, and resolved font, reads accessible
  names and roles, reads the focus path, and captures frames. Locale preview in
  the editor leaves saved content unchanged. (AUT-002, LOC-001)

## Presentation and Performance

### Visual checkpoints

Each has a unitas-approved reference, in en, de, ar, ja, and Lanternese at 100
and 200 percent text size: the title; the desk with an open letter; the routing
slip; the district guide scrolled to its end; the supervisor modal; the shift
report with a Gold medal; the Stamp Album; and the script sampler in Settings >
About Languages, which shows one line in each of the nine locales plus emoji.
Mobile checkpoints in portrait and landscape on a device profile with a display
cutout show the safe-area margins.

### Audio checkpoints

The stamp thunk within one frame of the hold completing.

### Project-declared expectations

60 frames per second on the declared reference desktop, mobile, and browser
classes; a locale switch completes within 0.5 s with preloaded tables; no frame
in which a layout rebuild is requested by the rebuild it triggers.
Expectations are evaluated only on the declared class; other runs record the
measurement and report the expectation as unevaluable (TST-002). These are the
sample's own expectations, not the PRF-002 engine budgets.

### PRF-002 measurements

None.

## Acceptance Scenarios

Outcomes use the TST-002 vocabulary: passed, failed, skipped, unevaluable, and
awaiting human review. Each perturbation names its mechanism: a TST-002
replay-stream event, a harness action outside the player, or a
development-build debug command implemented in the sample's scripts.

### S1. Open and inspect

* **Runs in:** headless editor; editor play mode.
* **Preconditions:** no import cache exists.
* **Steps:** open the project; reopen it.
* **Expected:** all assets import with no errors; every package is reported
  Supported with its tier; every table, locale, and font asset keeps its
  identity on reopening.
* **Demonstrates:** PRJ-001, PRJ-003, PRJ-004, PRJ-006, SER-002, SER-003, LOC-001

### S2. Happy path

* **Runs in:** editor play mode; headless editor; packaged players on every
  automated target.
* **Preconditions:** a fresh profile, locale en.
* **Steps:** with the pointer, play shift 1 with replayed input that routes all
  six letters correctly.
* **Expected:** each slip scores 100 plus its speed bonus times the streak
  multiplier, the streak reaches x3 on the fifth letter, the shift report shows
  Gold, and six stamps enter the album.
* **Demonstrates:** UI-001, TXT-001, RUN-008, AUD-001, SER-003

### S3. Same flow in every input mode

* **Runs in:** editor play mode; headless editor; packaged players on each
  target that offers the mode.
* **Preconditions:** a fresh profile.
* **Steps:** repeat S2 with keyboard only, gamepad only, and touch only; and,
  in editor play sessions, through the editor's MCP interface only.
* **Expected:** the same score and report every time; focus is visible at every
  step for keyboard and gamepad; each result names its input mode and settings;
  a mode a target lacks is reported unevaluable.
* **Demonstrates:** UI-001, INP-002, ACC-001

### S4. Locale switch mid-letter

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the saved state "shift 2, letter 3 open".
* **Steps:** switch through all nine locales.
* **Expected:** every bound string and the stamp sprites change; the slip
  values, focus, scroll position, clock, and score are unchanged;
  `SelectedLocaleChanged` fires once per switch; no scene reloads.
* **Demonstrates:** LOC-001, UI-001

### S5. Right-to-left and complex scripts

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** a profile at shift 2.
* **Steps:** play one letter in ar and one in he, then open the script sampler
  in each locale.
* **Expected:** the layout mirrors (slip labels move to the right) and focus
  order runs right to left; every character renders with no missing glyph
  through the fallback chain; the chosen font per character matches the
  recorded chain on every target; emoji render from the sprite asset; the
  locale report states the right-to-left state; Arabic joining forms, Hebrew
  and Arabic mixed-direction runs, and Devanagari and Thai clusters match their
  approved reference captures; ar, he, hi, and th are reported fully supported.
* **Demonstrates:** LOC-001, TXT-001, UI-001

### S6. Plurals and variables

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** a development build or editor play session.
* **Steps:** for 0, 1, 2, 3, 5, 11, 22, and 101 letters left (set through a
  development-build debug command), read the HUD line in every locale. Load
  the committed test fixture that removes the `{district}` variable.
* **Expected:** each line uses the plural form its language rules select, the
  numbers use the locale's format, and the clock shows the locale's time
  format. The fixture produces a formatting error naming the entry and
  variable.
* **Demonstrates:** LOC-001

### S7. Long strings and text size

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the profile with shift 0 unlocked.
* **Steps:** in de and in Lanternese at 200 percent text size, visit every
  screen.
* **Expected:** no text is truncated, clipped, or overlapping; long rows wrap
  and their layout groups grow; scroll views appear where content exceeds the
  viewport; the localization diagnostics report no overflow.
* **Demonstrates:** UI-001, LOC-001, ACC-001, TXT-001

### S8. Controls, input fields, and modals

* **Runs in:** editor play mode; headless editor; packaged players on every
  automated target.
* **Preconditions:** a letter open in shift 3.
* **Steps:** type a Japanese recipient name through IME composition events in
  the replay stream (desktop, Web target, and headless) and through the
  on-screen keyboard on mobile; exceed 24 characters; drag the slider; open the
  dropdown with the gamepad; cancel the supervisor modal; remove the gamepad
  while a modal is open, through a device-removal event in the replay stream.
* **Expected:** composition text is shown underlined until commit; the 25th
  character is rejected; the invalid-script state shows on a Latin entry; focus
  stays inside the modal and returns to Stamp after cancel; removing the
  gamepad leaves focus on the modal's current control.
* **Demonstrates:** UI-001, TXT-001, INP-002, RUN-008

### S9. Accessibility settings

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** a fresh profile.
* **Steps:** enable high contrast, reduced motion, relaxed timing, and 150
  percent text, then walk every control with the keyboard.
* **Expected:** every control reports a non-empty name, the right role, and its
  value; focus order matches reading order; text contrast is at least 7:1; no
  element moves or animates; the clock is hidden and records carry the relaxed
  marker.
* **Demonstrates:** ACC-001, UI-001

### S10. Safe areas and orientation

* **Runs in:** iOS Simulator and Android emulator players.
* **Preconditions:** a phone device profile with a display cutout; a letter
  open.
* **Steps:** rotate between portrait and landscape (a harness action on the
  simulator or emulator).
* **Expected:** no control lies outside `Screen.safeArea`; the desk switches
  between stacked and side-by-side layouts; the open letter and slip values
  survive the rotation.
* **Demonstrates:** PLT-001, UI-001

### S11. Preferences and recovery

* **Runs in:** packaged players; headless editor with a play-session restart.
* **Preconditions:** a fresh profile.
* **Steps:** change every setting, then quit and relaunch (a harness action
  that restarts the player, or, headless, a new play session with the same
  storage root). Then store the locale code `xx` in `PlayerPrefs` and corrupt
  the progress file (harness actions on the storage root), relaunch, and use
  Clear Preferences.
* **Expected:** every setting is restored; `xx` falls through to the system
  locale or en with a notice; the corrupt progress file is reported as corrupt,
  not absent; Clear Preferences leaves progress and the album intact.
* **Demonstrates:** SAV-001, LOC-001, RUN-009

### S12. Streak, hints, and returned letters

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** shift 2 started.
* **Steps:** send two perfect slips, use one Archivist hint, send a wrong
  district, then route the returned letter correctly.
* **Expected:** both perfect slips score at x1 and the counter shows x2 after
  the second; the hint costs 30 points and fills one correct field; the
  mistake resets the streak to x1; the wrong letter costs 50 and returns 60
  game minutes later stamped "Returned"; the second routing earns half points;
  the stamp thunk plays within one frame.
* **Demonstrates:** UI-001, AUD-001, LOC-001

### S13. Medals, album, and unlock

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** a fresh profile.
* **Steps:** replay Gold runs of shifts 1 to 3, then relaunch. Replay a shift 1
  run that scores 899.
* **Expected:** each shift awards Gold at its threshold, the best score and time
  are recorded, the album holds every perfect letter's stamp, shift 0 and
  Lanternese unlock, and all of it survives the relaunch. The 899 run awards
  Silver.
* **Demonstrates:** SAV-001, LOC-001, UI-001

### S14. Diagnostics and debug panel

* **Runs in:** editor play mode; headless editor; development packaged players.
* **Preconditions:** a development build.
* **Steps:** load the committed fixture that removes one de entry and the
  fixture that adds an unsupported character to a letter, then open the debug
  panel.
* **Expected:** the missing entry is reported with table, key, and locale, and
  the configured fallback to en is diagnosed as fallback use; the missing glyph
  is reported with character, font, fallback chain, locale, and text object,
  and drawn as the configured missing-glyph character; selecting each finding
  navigates to its entry; the panel shows locale, selected control, last
  device, and text size, and its buttons work with pointer, keyboard, and
  touch.
* **Demonstrates:** DIA-001, UI-002, LOC-001, TXT-001

### S15. Package and lifecycle

* **Runs in:** headless editor (build) and packaged players on every automated
  target.
* **Preconditions:** none.
* **Steps:** build players for every automated target. On mobile, background
  and foreground the app mid-letter (a harness action on the simulator or
  emulator). On the Web target, refresh the page mid-shift (a harness action in
  the browser test runner).
* **Expected:** each player launches and completes S2 through replayed input;
  each build report lists scenes, tables, fonts, sizes, and the build
  identifier; the mobile app resumes paused with the letter intact; the Web
  target reloads to Continue with the saved shift.
* **Demonstrates:** BLD-001, PLT-001, SAV-001

## Manual Playtest Guidance

A person judges the following, and a headless run reports these checkpoints as
awaiting human review, never as passed:

* Whether the first two letters make the desk feel like a game, not a form.
* Whether learning the rate card and the district stamps feels like getting
  better.
* Whether the Translation Lens and Archivist hint are tempting but costly.
* Whether two letters at once in shift 3 feel tense but fair.
* Whether the stamp feels satisfying and a near-miss of Gold invites a replay.
* Whether translations read naturally to a speaker of each language, and
  whether the reported state for ar, hi, and th matches what a reader sees.
* Whether the platform screen reader, on targets where the unitas extension
  reaches it, announces names and roles usefully.

## Agent Authoring Change Requests

### CR1. Saltmarsh district

* **Request:** add a tenth district, Saltmarsh, with a new stamp, a district
  card, entries in every string and asset table for all nine locales, and two
  letters in shift 3 addressed to it.
* **Acceptance scenario:** both Saltmarsh letters route correctly in shift 3
  in en and in ar.
* **Test:** a test that checks every locale has every new entry.

### CR2. Postage Due

* **Request:** add a "Postage Due" toggle that doubles the postage when
  checked, with a Smart String that pluralizes the stamp count in every locale.
* **Acceptance scenario:** with the toggle checked, the slip's postage doubles,
  and the pl and ar stamp-count lines use the plural forms their rules select
  for 1, 2, 5, and 11 stamps.
* **Test:** a test of the doubled postage and of the plural forms.

## Accessibility

* The game is completable with keyboard only, controller only, touch only, and
  pointer only (S3).
* Every control has an accessible name and role (S9).
* Text size goes to 200 percent with no loss of content (S7).
* High contrast, reduced motion, and relaxed timing are offered on first run
  and persist (S9, S11).
* District identity is shown by stamp shape and name, never by color alone
  (S5).
* Every sound cue has a visible counterpart (S12).

## Saved and Reproducible States

| State | How it is reached | Used by |
|---|---|---|
| Shift 2, letter 3 open, three slip fields filled, guide scrolled halfway | save file | S4 |
| Shift 3 with two letters open at once | save file | S8 |
| A profile with Gold on shifts 1 to 3 and shift 0 unlocked | save file | S7 |
| Settings in ar at 200 percent text with high contrast on | save file | S5, S9 |

## Known Limitations and Unsupported Conditions

None. Touch steps on hosts without a touchscreen, and controller steps on
targets without a controller, are reported unevaluable on that target. Any
behavior this document requires that is not demonstrated is reported with its
real compatibility state, as a defect to fix.

## Acceptance Outcomes

* The UI and localization showcase completes the same flow with pointer,
  keyboard, controller, touch, and automation, including right-to-left
  layouts. (SMP-001, UI-001, TXT-001, LOC-001, ACC-001)
* Switching locale during a letter changes every bound string and asset and
  keeps every slip value, focus, scroll position, clock, and score. (SMP-001,
  LOC-001, UI-001)
* Plural forms, variables, and number and time formats follow each locale's
  rules, and pseudo-localized and long strings lay out without overflow at
  200 percent text size. (SMP-001, LOC-001, UI-001, ACC-001)
* Every character renders through a deterministic fallback chain or produces
  a missing-glyph diagnostic. Arabic, Hebrew, Devanagari, and Thai text is
  shaped and bidirectionally ordered to match its approved references.
  (SMP-001, TXT-001, LOC-001, DIA-001)
* Every control exposes a name and role through the identified unitas
  extension, and high contrast, reduced motion, relaxed timing, and text size
  persist across launches. (SMP-001, ACC-001, SAV-001)
* Streaks, hints, returned letters, medals, the album, and the shift 0 unlock
  follow their stated rules and persist. (SMP-001, SAV-001, UI-001)
* Mobile layouts respect safe areas through rotation, and every automated
  target is packaged with a build report and completes the replayed happy
  path. (SMP-001, PLT-001, BLD-001)
