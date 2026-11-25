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
controller, and touch on the targets that offer each input mode, and through
authorized automation on every target.

This is an original, redistributable unitas sample. Its story, art,
translations, and audio are unitas content, its fonts are redistributable
fonts with recorded licences, and it is not derived from any Unity sample. It proves Canvas and
RectTransform layout, controls, focus, text, font fallback, localization,
accessibility, and persistent preferences.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns this game's acceptance under SMP-001.

## Targets and Input Modes

* **Targets:** macOS, Windows, and Linux desktop players; iOS and Android
  players, run automatically on the iOS Simulator and the Android emulator and
  optionally on physical devices; a browser player (Unity's WebGL build
  target); the editor's play mode; and the headless editor.
* **Render pipeline:** URP 14.x (Forward) for the 3D desk scene behind the
  interface.
* **Input modes:** pointer (mouse), keyboard, gamepad, and touch, all through
  one Input System 1.x action asset and `InputSystemUIInputModule`; and
  authorized automation through the editor's MCP interface on every target. Touch is offered on the
  mobile targets and on desktop and browser hosts that report a touchscreen.
  The capability report states which modes each target offers, and a mode a
  target does not offer is reported not applicable, never as passed.

## Game Design

### Core loop

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

### Shifts

| Shift | Letters | New elements | Medal thresholds (Bronze / Silver / Gold) |
|---|---|---|---|
| 1. Quiet Harbor | 6 | districts, postage, the rate card | 300 / 600 / 900 |
| 2. Festival Night | 6 | handling rules, a second page of the rate card, letters in right-to-left scripts | 400 / 800 / 1,200 |
| 3. Storm Mail | 6 | Keep Dry and Night Boat rules, two letters arriving at once, recipient names in four scripts | 500 / 1,000 / 1,500 |
| 0. Lantern Isles (unlock) | 6 | letters in Lanternese, the pseudo-locale, and mixed scripts on one slip | 600 / 1,200 / 1,800 |

The district guide is a scroll view of nine district cards, and the rate card
is a scroll view of rules in the selected language. Both are open at any time
from the desk.

### Scoring and failure

* A slip earns 20 points per correct field, 100 for a fully correct slip, plus
  a speed bonus of 2 points per second under 40 seconds from opening the
  letter to Send. The total is multiplied by the current streak multiplier.
* A wrong district returns the letter 60 game minutes later with a red
  "Returned" stamp and costs 50 points. A returned letter that is routed
  correctly the second time earns half points.
* When the clock reaches 06:00 with letters unsent, the shift ends as "Shift
  missed". The player keeps the score but earns no medal. Retry restarts the
  shift with every slip, streak, and score reset.

### What makes it fun

* **Hook.** Within 10 seconds a letter rattles onto the desk under a
  flickering lamp. The first letter needs only a district and a postage
  value; stamping it lands with a heavy thunk, the ink spreads, and the letter
  whooshes into the tube with "+100" and "Streak x1". The second letter, by
  second 30, is written in Japanese, and the desk's Translation Lens appears.
* **Mastery curve.** A new player reads every rule and finishes shift 1 in
  about 7 minutes. A skilled player learns the rate card (2 stamps per 100 g,
  plus 1 for Express and 2 for Night Boat), recognizes districts by their
  stamp shapes and script, navigates the slip without the pointer, and
  finishes shift 1 in under 4 minutes with a x3 streak.
* **Risk and reward.** Holding the Translation Lens shows the sender's note
  in the selected language but costs 15 game minutes, or 15 points with
  relaxed timing. The Archivist hint fills
  one correct field for 30 points. Sending without checking the guide is fast
  and earns the speed bonus, but a wrong district comes back as a returned
  letter and breaks the streak.
* **Variety and escalation.** Each shift adds the elements in the shifts
  table. Shift 3 delivers two letters at once, so the player switches between
  two open slips. Shift 0 mixes scripts on one slip and uses Lanternese, whose
  expanded text tests both the player and the layout.
* **Juice.** Every control answers within one frame with a 0.1 s color tint
  and a soft click. The stamp shakes the slip by 4 pixels and leaves an ink
  splash rotated by a random angle between -8 and 8 degrees from a seeded
  generator. The streak counter pops to 1.3 scale for 0.15 s on each increase.
  A returned letter flops onto the desk with a paper thud. The clock ticks
  louder in the last 30 game minutes. Reduced motion replaces every movement
  with an instant change and keeps the sounds.
* **Replay reasons.** Consecutive perfect slips raise the streak multiplier to
  x2 after two and x3 after four, and a mistake resets it. Each shift awards a
  medal at the thresholds above and records the best score and best time.
  Each perfect letter adds its stamp to the Stamp Album, a grid of 24 stamps,
  one per letter. Gold on shifts 1 to
  3 unlocks shift 0. Records set with relaxed timing carry a marker.

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

### Pause and exit

Pause sets `Time.timeScale` to zero, stops the clock, hides the letter text
behind a pause panel (Resume, Settings, Quit to Title), and keeps menu sounds
playing. Quit to Title saves the shift in progress. Quit on the title exits
the desktop player; it is hidden on mobile and browser targets. Losing focus
pauses the game, and it stays paused until the player resumes.

### Settings

Language (the nine locales, plus Lanternese once unlocked); text size 100,
125, 150, or 200 percent; high contrast; reduced motion; relaxed timing (no
clock); master, music, and effects volume; control bindings; and Clear
Preferences. Every setting persists through `PlayerPrefs` and applies
immediately, without reloading the scene.

## Content

| Asset | Source format | What matters on import |
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
| `ProjectSettings/` | Unity YAML | default locale, player resolution, orientation (portrait and landscape on mobile) |

The `A11y` table holds the accessible names of icon-only controls. Fonts and
emoji sheets record their licences in `Licenses/content-licenses.json`.

Packages: uGUI 1.0.x, TextMeshPro 3.0.x, Localization 1.x (with the
Addressables 1.x line it depends on), Input System 1.x, and URP 14.x.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001 | The project opens in place with no conversion | S1 |
| PRJ-003 | Table, locale, font asset, and prefab identities stay stable | S1 |
| PRJ-004 | Font sources, sprite sheets, models, WAV and OGG | S1 |
| PRJ-006 | Localization, Addressables, uGUI, TextMeshPro, and Input System resolve | S1 |
| SER-002, SER-003 | Desk scene, nested modal prefabs, district card variants | S1, S2 |
| RUN-003, RUN-008 | Lifecycle, `UnityEvent` control callbacks, focus messages | S2, S8 |
| RUN-009 | Progress file under `Application.persistentDataPath` | S11 |
| UI-001 | Canvas, RectTransform, layout groups, content size fitters, scroll views, masks, input fields, dropdowns, sliders, toggles, modals, world-space canvas, focus, navigation | S2, S3, S5, S7, S8, S10 |
| UI-002 | The immediate-mode debug panel | S14 |
| TXT-001 | TextMeshPro, fallback chains, complex-script content, emoji sprite asset, rich-text tags, missing-glyph diagnostics | S5, S7, S14 |
| LOC-001 | String and asset tables, Smart Strings with plurals and variables, runtime locale switching, right-to-left locales, pseudo-localization, missing-translation behavior | S4, S5, S6, S7, S14 |
| ACC-001 | Accessible names and roles, focus order, text scaling, high contrast, reduced motion, relaxed timing | S3, S7, S9 |
| SAV-001 | Preferences in `PlayerPrefs`, progress file, recovery from bad values | S11, S13 |
| INP-002 | Pointer, keyboard, gamepad, and touch schemes; rebinding; device changes | S3, S8 |
| PLT-001 | Safe areas, orientation, mobile and browser lifecycle | S10, S15 |
| AUD-001 | UI sounds, music, mixer groups, browser audio activation | S2, S12 |
| DIA-001 | Missing-translation, missing-glyph, and layout diagnostics with navigation | S14 |
| BLD-001 | Desktop, mobile, and browser players with build reports | S15 |
| BLD-002 | Headless runs of every scenario | all |
| TST-002 | Scenarios, checkpoints, approved references, input replay | all |
| AUT-002 | Agents play, inspect UI and locale state, and capture frames | all |

## Engine Behavior This Game Depends On

### Scripting and runtime

* Control callbacks (`Button.onClick`, `Toggle.onValueChanged`,
  `Slider.onValueChanged`, `TMP_Dropdown.onValueChanged`, and
  `TMP_InputField.onEndEdit`) fire once per change with persistent listeners
  as serialized.
* The shift clock uses scaled time and stops at `Time.timeScale = 0`; the
  stamp hold and modal fades use unscaled time.

### Project and content

* Every string table entry, asset table entry, and locale keeps its key
  identifier and metadata across reimport. District card variants apply their
  overrides to all nine cards.

### Layout and controls

* The overlay canvas uses Scale With Screen Size with a 1920 by 1080
  reference and a match of 0.5. Anchors and offsets place every panel by the
  rules UI-001 defines, recomputed when the size, aspect, orientation, safe
  area, text size, or locale changes.
* The routing slip's vertical layout group and content size fitters grow each
  row to its text. The district guide's grid uses two columns below 1200
  reference pixels of width and three above. Scroll views use `RectMask2D`,
  Elastic movement, and inertia. Circular portraits on district cards use
  `Mask` with a circle sprite.
* The world-space slip canvas on the desk receives pointer and touch events
  through its `GraphicRaycaster` and the desk camera.
* The letter and supervisor modals constrain focus to themselves and restore
  the prior selection on close. A `CanvasGroup` blocks raycasts to the desk
  while a modal is open.
* The recipient input field accepts up to 24 characters, shows IME
  composition text underlined before commit, and validates against the
  letter's expected script. On mobile it opens the on-screen keyboard.

### Text and localization

* Each locale's primary font comes from the `Desk Assets` table. Its fallback
  list covers the other scripts in a fixed order, then the global fallback
  list, and the selected font for any character is the same on every run and
  target.
* Rich-text tags `<b>`, `<i>`, `<color>`, `<size>`, `<link>`, and `<sprite>`
  render in letters. Emoji characters resolve to the emoji sprite asset.
* Smart Strings select plural forms by each language's rules (en and de: one,
  other; pl: one, few, many, other; ar: zero, one, two, few, many, other;
  ja and zh-Hans: other) and format scores and clock times by the selected
  locale's culture.
* Assigning `LocalizationSettings.SelectedLocale` refreshes every bound string
  and asset, on the next rendered frame for preloaded tables, raises
  `SelectedLocaleChanged` once, and leaves the open letter, slip values, focus, scroll positions, clock, and
  score unchanged.
* In ar and he the layout mirrors: slip labels move to the right, the reading
  order of focus runs right to left, and the text is shaped and reordered
  correctly:
  * Arabic letters take their joining forms.
  * Hebrew and Arabic runs mixed with Latin text, numbers, and punctuation
    follow Unicode bidirectional order.
  * Devanagari and Thai form their shaped clusters with correct line breaking.

  This holds for TextMeshPro, legacy `Text`, and UI Toolkit text. Every locale
  is fully supported.

### Accessibility

* Through the unitas player accessibility extension, which is identified as
  a unitas extension and not as Unity behavior, every control exposes a name
  from its visible label or the `A11y` table, a role (button, toggle, slider,
  dropdown, text field, tab, dialog), its value, and its state. Automation
  reads them on every target.
* Focus order follows reading order. Text size scales every text element.
  High contrast swaps the theme to contrast ratios of at least 7:1 for text.

### Audio, input, and player

* Clicks start within one frame of activation. In the browser no sound plays
  before the first user gesture.
* Changing input device does not strand focus, and the control hints switch
  to the most recently used device within one frame.
* Mobile players fit the root panel to `Screen.safeArea` and relayout on
  orientation change. The browser player reports loading, storage, and audio
  activation as PLT-001 defines.

### Editor and automation

* An agent can select a locale, read every visible string with its table, key,
  and resolved font, read accessible names and roles, read the focus path, and
  capture frames through the editor's MCP interface. Locale preview in the editor leaves saved
  content unchanged.

## Presentation and Performance

* Visual checkpoints with unitas-approved references, in en, de, ar, ja, and
  Lanternese at 100 and 200 percent text size: the title; the desk with an
  open letter; the routing slip; the district guide scrolled to its end; the
  supervisor modal; the shift report with a Gold medal; the Stamp Album; and
  the script sampler in Settings > About Languages, which shows one line in
  each of the nine locales plus emoji.
* Mobile checkpoints in portrait and landscape on a device profile with a
  display cutout, showing the safe-area margins.
* Audio checkpoint: the stamp thunk within one frame of the hold completing.
* Project-declared expectations: 60 frames per second on the declared
  reference desktop, mobile, and browser classes; a locale switch completes
  within 0.5 s with preloaded tables; no layout rebuild loop. These are the
  sample's own expectations, not the PRF-002 engine budgets.

## Acceptance Scenarios

Every scenario runs in editor play mode, in a packaged player on each declared
target, and headless through the editor's MCP interface, unless it says otherwise.

* **S1. Open and inspect.** No import cache exists. Open the project.
  * Expected: all assets import with no errors; every package is reported
    Supported with its tier; every table, locale, and font asset keeps its
    identity on reopening.
  * (PRJ-001, PRJ-003, PRJ-004, PRJ-006, SER-002, SER-003, LOC-001)
* **S2. Happy path.** With the pointer, in en, play shift 1 with replayed input
  that routes all six letters correctly.
  * Expected: each slip scores 100 plus its speed bonus times the streak
    multiplier, the streak reaches x3 on the fifth letter, the shift report
    shows Gold, and six stamps enter the album.
  * (UI-001, TXT-001, RUN-008, AUD-001, SER-003)
* **S3. Same flow in every input mode.** Repeat S2 with keyboard only,
  gamepad only, touch only, and automation only, on each target that offers
  the mode.
  * Expected: the same score and report every time; focus is visible at every
    step for keyboard and gamepad; each result names its input mode and
    settings; a mode a target lacks is reported not applicable.
  * (UI-001, INP-002, ACC-001)
* **S4. Locale switch mid-letter.** In shift 2, open a letter, fill three
  fields, scroll the guide halfway, then switch through all nine locales.
  * Expected: every bound string and the stamp sprites change; the slip
    values, focus, scroll position, clock, and score are unchanged;
    `SelectedLocaleChanged` fires once per switch; no scene reloads.
  * (LOC-001, UI-001)
* **S5. Right-to-left and complex scripts.** Play one letter in ar and one in
  he, then open the script sampler in each locale.
  * Expected: the layout mirrors and focus runs right to left; every
    character renders with no missing glyph through the fallback chain; the
    chosen font per character matches the recorded chain on every target;
    emoji render from the sprite asset; the locale report states the
    right-to-left state; Arabic joining forms, Hebrew and Arabic mixed-direction
    runs, and Devanagari and Thai clusters match their approved reference
    captures; ar, he, hi, and th are reported fully supported.
  * (LOC-001, TXT-001, UI-001)
* **S6. Plurals and variables.** For 0, 1, 2, 3, 5, 11, 22, and 101 letters
  left, read the HUD line in every locale.
  * Expected: each line uses the plural form its language rules select, the
    numbers use the locale's format, and the clock shows the locale's time
    format. A test fixture that removes the `{district}` variable produces a
    formatting error naming the entry and variable.
  * (LOC-001)
* **S7. Long strings and text size.** From the profile with shift 0
  unlocked, in de and in Lanternese at 200 percent text size, visit every
  screen.
  * Expected: no text is truncated, clipped, or overlapping; long rows wrap and
    their layout groups grow; scroll views appear where content exceeds the
    viewport; the localization diagnostics report no overflow.
  * (UI-001, LOC-001, ACC-001, TXT-001)
* **S8. Controls, input fields, and modals.** Type a Japanese recipient name
  through an IME on desktop and through the on-screen keyboard on mobile;
  exceed 24 characters; drag the slider; open the dropdown with the gamepad;
  cancel the supervisor modal; unplug the gamepad while a modal is open.
  * Expected: composition text is shown underlined until commit; the 25th
    character is rejected; the invalid-script state shows on a Latin entry;
    focus stays inside the modal and returns to Stamp after cancel; unplugging
    leaves focus on the modal's current control. Targets whose IME support
    is limited report that state instead of passing.
  * (UI-001, TXT-001, INP-002, RUN-008)
* **S9. Accessibility settings.** Enable high contrast, reduced motion,
  relaxed timing, and 150 percent text, then walk every control with the
  keyboard.
  * Expected: every control reports a non-empty name, the right role, and its
    value; focus order matches reading order; text contrast is at least 7:1;
    no element moves or animates; the clock is hidden and records carry the
    relaxed marker.
  * (ACC-001, UI-001)
* **S10. Safe areas and orientation.** Mobile targets only. On a phone profile
  with a display cutout, rotate between portrait and landscape during a
  letter.
  * Expected: no control lies outside `Screen.safeArea`; the desk switches
    between stacked and side-by-side layouts; the open letter and slip values
    survive the rotation.
  * (PLT-001, UI-001)
* **S11. Preferences and recovery.** Change every setting, quit, and
  relaunch. Then store the locale code `xx` in `PlayerPrefs`, and corrupt the
  progress file.
  * Expected: every setting is restored; `xx` falls through the startup
    locale selectors to the system locale or en, and a notice says so; the
    corrupt progress file is reported as corrupt, not absent; Clear
    Preferences leaves progress and the album intact.
  * (SAV-001, LOC-001, RUN-009)
* **S12. Streak, hints, and returned letters.** In shift 2, send two perfect
  slips, use one Archivist hint, send a wrong district, then route the
  returned letter correctly.
  * Expected: both perfect slips score at x1 and the counter shows x2 after
    the second; the hint costs 30 points and fills one correct field; the
    mistake resets the streak to x1; the wrong letter costs
    50 and returns 60 game minutes later stamped "Returned"; the second
    routing earns half points; the stamp thunk plays within one frame.
  * (UI-001, AUD-001, LOC-001)
* **S13. Medals, album, and unlock.** From a fresh profile, replay Gold runs of
  shifts 1 to 3, then relaunch.
  * Expected: each shift awards Gold at its threshold, the best score and time
    are recorded, the album holds every perfect letter's stamp, shift 0 and
    Lanternese unlock, and all of it survives the relaunch. A run 1 point
    below a threshold awards the lower medal.
  * (SAV-001, LOC-001, UI-001)
* **S14. Diagnostics and debug panel.** In a development build, load the
  fixture that removes one de entry and the fixture that adds an unsupported
  character to a letter, then open the debug panel.
  * Expected: the missing entry is reported with table, key, and locale, and
    the configured fallback to en is diagnosed as fallback use; the missing
    glyph is reported with character, font, fallback chain, locale, and text
    object, and drawn as the configured missing-glyph character; selecting
    each finding navigates to its entry; the panel shows locale, selected
    control, last device, and text size, and its buttons work with pointer,
    keyboard, and touch.
  * (DIA-001, UI-002, LOC-001, TXT-001)
* **S15. Package and lifecycle.** Build players for every declared target. On
  mobile, background and foreground the app mid-letter. In the browser,
  refresh the page mid-shift.
  * Expected: each player launches and completes S2 through replayed input,
    each build report lists scenes, tables, fonts, sizes, and the build
    identifier; the mobile app resumes paused with the letter intact; the
    browser reloads to Continue with the saved shift.
  * (BLD-001, PLT-001, SAV-001)

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

Add a tenth district, Saltmarsh, with a new stamp, a district card, entries in
every string and asset table for all nine locales, and two letters in shift 3
addressed to it. The request includes a test that checks every locale has
every new entry and a scenario that routes both letters correctly.

A second change request adds a "Postage Due" toggle that doubles the postage
when checked, with a Smart String that pluralizes the stamp count in every
locale and a scenario that checks pl and ar plural forms.

## Accessibility

The game is completable with keyboard only, controller only, touch only, and
pointer only. Every control has an accessible name and role. Text size goes to
200 percent with no loss of content. High contrast, reduced motion, and
relaxed timing are offered on first run and persist. District identity is
shown by stamp shape and name, never by color alone. Every sound cue has a
visible counterpart.

## Saved and Reproducible States

* Shift 2, letter 3 open, three slip fields filled, guide scrolled halfway.
* Shift 3 with two letters open at once.
* A profile with Gold on shifts 1 to 3 and shift 0 unlocked.
* Settings in ar at 200 percent text with high contrast on.

## Known Limitations and Unsupported Conditions

None that come from the engine. Touch scenarios on desktop targets without a
touch screen, and controller scenarios on targets without a controller, are
reported not applicable on that target. Any behavior this document requires
that is not demonstrated is reported with its real compatibility state, as a
defect to fix.

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
* Mobile layouts respect safe areas through rotation, and every declared
  target is packaged with a build report and completes the replayed happy
  path. (SMP-001, PLT-001, BLD-001)
