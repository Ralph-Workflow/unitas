# Accessibility, Localization, and Persistence

## Scope

This specification defines editor accessibility, game accessibility features,
localized content behavior, player settings, save data, and recovery from data
problems.

Localization behavior provided by the Localization package (`com.unity.localization`)
is covered for the 1.x versions listed in the supported package matrix
(CMP-002, owned by the [Compatibility contract](compatibility-contract.md)).
Fonts, glyphs, fallback, and text layout are owned by TXT-001 in
[UI, audio, and input](ui-audio-input.md).

Every capability in this document, including accessibility inspection, locale
selection and preview, localization diagnostics, and inspection of preferences
and save data, is available to authorized agents through automation as defined
in [Automation and MCP](automation-and-mcp.md), in the visual editor, and in
headless sessions.

### Compatibility tiers

The tiers below are recorded in the [Feature catalog](feature-catalog.md) and
defined in the [Compatibility contract](compatibility-contract.md); the
baseline is Unity 2022.3 LTS:

| ID | Tier |
|---|---|
| LOC-001 | Familiar |
| ACC-001 | Native |
| SAV-001 | Semantic |

* **ACC-001 (Native):** editor and game accessibility are unitas-defined.
* **LOC-001 (Familiar):** Localization package content imports with its
  identity, and public types and members keep their Unity 2022.3 names and
  signatures; the behavior is the one this document states, and every
  difference from Unity 2022.3 is listed in the API reference owned by AUT-006
  in [Automation and MCP](automation-and-mcp.md) and in the project
  compatibility report.
* **SAV-001 (Semantic):** `PlayerPrefs` and the serializers used for save data
  behave as documented for Unity 2022.3 LTS in ordinary use; a found difference
  is reported as Partially supported, naming it.

## Architecture

The domain spans the UI and text subsystems, the platform layer, and scripting, layered as in
[Engine architecture](architecture.md).

* **Accessibility tree.** Each engine instance derives one accessibility tree from its UI
  (uGUI, TextMeshPro, and UI Toolkit panels) through change tracking: a node updates only when
  its control's label, value, state, focus, or visibility changes. Script overrides and
  announcements write into the tree. Per-target adapters in the platform layer expose it to
  UI Automation, the Apple accessibility interfaces, the Android accessibility framework,
  AT-SPI, and the browser's accessibility tree. Platform requests arrive on platform threads
  and are answered from the tree's last published state, never by calling into scripts.
* **Editor accessibility.** The editor, being a unitas project, uses the same tree and
  adapters. Extension controls running in a worker contribute through the element trees the
  worker publishes for agents, so drawing, agent operation, and accessibility share one
  description of each control.
* **Platform preferences.** Screen-reader presence, reduced motion, high contrast, text scale,
  and caption preferences are read by the platform layer and delivered to scripts as
  main-thread notifications at a frame boundary.
* **Localization.** The Localization package compiles and runs as ordinary package code. Below
  scripting the engine supplies culture and plural data for every locale on every target,
  including ahead-of-time and browser targets; shaping and bidirectional layout through the
  text subsystem; and asynchronous loading of cooked tables and assets. A locale change is a
  main-thread event; layout re-runs only for text whose content changed. Editor locale preview
  runs in the worker and writes no project content, and localization diagnostics (overflow,
  clipping, missing glyphs, missing shaping data) come from the text subsystem's own layout
  results.
* **Storage roots.** Project assets, project settings, editor preferences, player preferences,
  and game data each live in a distinct storage root with one owner; clearing one root never
  touches another. Each isolated play session has its own player-preference and
  persistent-data roots.
* **Preferences and files.** `PlayerPrefs` is an in-memory store on the main thread backed by
  each target's Unity location. Engine-initiated I/O is asynchronous and never blocks the main
  thread; synchronous script APIs (`PlayerPrefs.Save`, `System.IO` writes) block only their
  calling thread, only as long as their contract requires. Durability (flush to stable storage
  and atomic replace) is implemented once per target in the platform layer. On the browser
  target the persistent-data directory is a filesystem whose closed files are copied to
  persistent browser storage asynchronously, with each file's pending state tracked, and atomic
  replace is journaled over that storage.
* **Serializers.** Save serializers run on the hosted .NET runtime; on ahead-of-time targets the
  build supplies the metadata and code they need
  ([Builds, platforms, XR, and networking](build-platforms-xr-networking.md)).
* **Everywhere.** Editor play mode, headless play sessions, and packaged players run the same
  accessibility, localization, and storage code; only the storage locations differ, as owned by
  [Scripting runtime](scripting-runtime.md).

## Accessible editor experience

Every editor workflow is keyboard operable and exposes accessible names,
roles, values, focus, selection, expanded state, validation, progress, and
status to the host platform's accessibility interface. Information does not
depend on color, pointer hover, spatial layout, animation, or a screenshot
alone.

As a developer, I can:

* navigate project content, hierarchy, inspector, console, build status, and
  play controls without a pointing device;
* identify current focus and selection;
* scale editor text and controls while preserving essential content;
* use high-contrast presentation and non-color status indicators;
* pause or reduce nonessential editor motion;
* inspect the same state through structured automation.

Commands contributed by custom editor extensions appear as keyboard-reachable
commands with accessible names. Custom inspectors, property drawers, and
editor windows drawn by extensions (RUN-010 in
[Scripting runtime](scripting-runtime.md)) take part in the same keyboard
focus order as the rest of the editor. Their IMGUI and UI Toolkit controls
expose accessible names taken from their labels, along with their roles,
values, and states. A control that an extension draws without a label is
reported, naming the extension and the control. An extension never removes
accessibility from unrelated editor functions.

## Accessibility features available to games

The engine enables projects to offer remappable controls, keyboard-only and
controller-only paths, text and UI scaling, captions, subtitles, speaker cues,
contrast-conscious presentation, alternatives to color-only information,
reduced motion, and configurable timing where the game exposes it. The input,
UI, text, and audio building blocks for these are owned by
[UI, audio, and input](ui-audio-input.md).

Unity 2022.3 LTS offers no player screen-reader API. unitas provides one as
the player accessibility extension. It is identified as a unitas extension and
is not represented as Unity-compatible behavior:

* In a running player, uGUI, TextMeshPro, and UI Toolkit controls are exposed
  to the platform screen reader on every target that has one: Narrator and
  other UI Automation clients on Windows, VoiceOver on macOS and iOS, TalkBack
  on Android, Orca on Linux, and the browser's screen reader on the browser
  target. Each control exposes an accessible name taken from its visible
  label, a role (button, toggle, slider, dropdown, text field, list, tab,
  dialog, or image), its value, its state (selected, checked, disabled, or
  expanded), and its focus. The screen reader's focus follows the selected
  control of the event system and the focused element of UI Toolkit panels.
* Project scripts can set or override a control's name, description, role,
  and value, mark content as decorative so it is not announced, and announce
  a message through the screen reader, for example when a score changes.
* Game code can read whether a screen reader is running and the player's
  platform preferences for reduced motion, high contrast, text scale, and
  captions, and is notified when they change. A target that does not report a
  preference returns an explicit not-reported state, not a default value.
* The project setting **Player accessibility extension** is on by default. A
  project that provides its own narration turns it off, and then the player
  exposes no controls to the screen reader, as in Unity 2022.3.
* A control that has no name from its label or from a script is reported in
  diagnostics and in accessibility playtest results, naming the control.

Accessibility choices that a game stores through `PlayerPrefs` or file access
persist across sessions like any other preference, and a playtest scenario
observes them after a relaunch. The platform accessibility preferences above
are readable from the first frame, so a game can offer its essential
accessibility choices before visually or physically demanding content
begins.

Caption and subtitle behavior can represent text, speaker, timing, duration,
sound cues, and readability choices. Muting audio does not remove textual
alternatives that the game provides.

## Accessibility playtesting

Scenarios can evaluate keyboard-only, controller-only, touch-only, high-scale
UI, captions, reduced motion, contrast, color-independent cues, focus order,
navigation, time limits, and interruption recovery.

Scenarios can also drive the player accessibility extension: they read the
names, roles, values, and states the screen reader receives, move screen-reader
focus, and record announcements.

A result identifies the input method and accessibility settings used. Passing a
pointer flow is not represented as passing keyboard, controller, touch, or
assistive-technology flows.

## Text and fonts

Text and font behavior, including missing-glyph diagnostics, deterministic
fallback, mixed scripts, and bidirectional content, is owned by TXT-001 in
[UI, audio, and input](ui-audio-input.md).

## Localization

As a developer, I can provide localized strings and assets, define locales and
locale metadata, choose a locale at startup or runtime, preview locales in the
editor, and identify missing or stale translations.

### Localization package scope

When the project uses Localization 1.x:

* Locales (with identifier, code, fallback metadata, and custom formatter),
  string tables, asset tables, shared table data, and table collections retain
  their entries, keys, key identifiers, and metadata.
* `LocalizedString`, `LocalizedAsset<T>`, and the Localize String Event and
  Localize Asset Event components resolve their table reference and entry
  reference and update their targets when the selected locale changes.
* Smart Strings resolve variables (persistent variables, global variables
  groups, and arguments), plural-sensitive content, gender and choice
  selection, conditional formatting, lists, and nested localized strings.
  A plural form is selected by the plural rules of the selected locale's
  language, and a variable missing at format time produces a formatting error
  naming the entry and variable.
* Locale-aware number, currency, date, and time formatting follows the
  selected locale's culture or custom formatter.
* Startup locale selectors (command line, system locale, specific locale, and
  player preference selectors) run in the order listed in the Localization
  Settings until one returns an available locale, and the result is
  distinguishable from the device locale and the project default.
* Assigning `LocalizationSettings.SelectedLocale` raises
  `SelectedLocaleChanged` once and refreshes every bound `LocalizedString` and
  `LocalizedAsset`. `LocalizationSettings.InitializationOperation` completes
  after the selected locale is chosen and preloaded tables are loaded; tables
  marked for preload are loaded before it completes, and other tables and
  assets load asynchronously on first use, with their operations reporting
  completion or failure. Loading through Addressables follows RUN-006 in
  [Scripting runtime](scripting-runtime.md).
* Fallback locales apply in the order set by each Locale's fallback metadata,
  and the missing-translation
  behavior configured in the project (showing the missing message, an empty
  string, or the fallback) is honored and diagnosed.
* Pseudo-localization locales produce their expanded, accented, or bracketed
  output so layout expansion can be tested.
* Localized property variants (the Game Object Localizer and its tracked
  properties) apply per-locale values to component properties, including
  RectTransform size and position, fonts, and colors, when the locale changes.
* String table collections export to and import from CSV and XLIFF with their
  keys, key identifiers, comments, and metadata. An entry whose key is
  missing, duplicated, or conflicting in the imported file is reported with
  the file, row, and key. Google Sheets synchronization works when the project
  has the Google service account or credentials it needs.

A project that implements localization without the package (for example with
its own tables of strings) is ordinary project content; its behavior follows
from the scripts and assets it uses.

### Required behavior

As a player, I can select an available locale and see the change applied to
all localized content. Locale selection persists when the project requests it and
remains distinguishable from device locale and project default.

Changing locale updates every bound localized string and asset without
reloading scenes or recreating unrelated game state. Changing locale does not
corrupt saved game state or confuse content identity with translated display
text.

A locale is right-to-left when its Locale asset metadata or the project's
localization settings mark it so. Text in every locale, including Arabic,
Hebrew, Persian, Urdu, Indic, Thai, Lao, Khmer, and Myanmar locales, is shaped
and bidirectionally ordered as defined by TXT-001 in
[UI, audio, and input](ui-audio-input.md). For each locale, the report states
its right-to-left state and whether the project's fonts cover it: every
character in the locale's tables has a glyph in the font or its fallback
chain, and each font that renders a complex script has shaping data for it.
A locale whose content is covered is reported fully supported. A locale with
gaps is reported with each missing glyph or missing shaping data, and the font
and entry affected. When the project's Text layout mode is Unity per-line
reversal, the report states that its right-to-left and complex-script
locales rely on pre-shaped text.

Missing translations remain visibly diagnosable and do not silently masquerade
as translated content.

### Localization diagnostics

The editor and automation can preview content in a selected locale without
changing saved project content, and identify:

* missing or stale translations;
* missing localized assets;
* fallback use;
* truncated, clipped, overlapping, or overflowing text;
* missing glyphs, and fonts that lack shaping data for a locale's script (as
  reported by TXT-001);
* content variables absent from or inconsistent with a translation;
* layouts that fail under right-to-left direction or expanded strings.

## Player preferences

Projects can store user choices such as volume, graphics, controls, locale,
accessibility, and gameplay preferences. `PlayerPrefs` (`SetInt`, `SetFloat`,
`SetString`, `GetInt`, `GetFloat`, `GetString`, `HasKey`, `DeleteKey`,
`DeleteAll`, and `Save`) behaves as in Unity and stores data in the same
per-user location that Unity uses for the target, company name, and product
name. On the browser target, `PlayerPrefs` has no fixed size limit and is
bounded only by the storage quota the browser grants the page. This is a unitas
extension to Unity 2022.3's 1 MB browser limit, and it is reported as such in
the compatibility report. Preference failures, such as exceeding that quota,
are reported to gameplay code as in Unity and in diagnostics.

Project assets, shared project settings, editor preferences, player preferences,
and game-progress data remain distinct. Clearing one category does not silently
delete another. File locations such as `Application.persistentDataPath` are
owned by RUN-009 in [Scripting runtime](scripting-runtime.md).

## Game saves

unitas provides no save API beyond `PlayerPrefs` and file access (RUN-009 in
[Scripting runtime](scripting-runtime.md)). The outcomes in this section and
the next are what the engine's storage behavior, diagnostics, and playtest
tools (TST-002 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md))
let a project observe and verify for the save system it implements.

As a player, I can create, update, load, enumerate, and delete save data when the
game exposes those actions. A save identifies its game or profile, the version
number the game writes into it, timestamp or play context when used, and slot or identity.

As a developer, I can distinguish:

* absent data from corrupt data;
* new-game state from a failed load;
* current-version data from older supported data;
* recoverable data from incompatible data;
* local data from cloud or externally synchronized data;
* a user cancellation from an I/O or permission failure.

Engine storage gives the same guarantees on every target:

* `PlayerPrefs.Save` returns after the preferences are durable. Changes not
  yet saved are written when the player quits normally and are lost if the
  process crashes first, as in Unity.
* A file written under `Application.persistentDataPath` is durable once the
  call that closes or flushes it returns. On the browser target, a closed file
  is copied to the page's persistent browser storage automatically, without
  project code, and is durable when that copy completes. Until then the file
  is reported as pending synchronization, and closing the page first finishes
  any pending copy that the browser allows.
* Replacing a file with a completed file through `File.Replace` is atomic on
  every target, including the browser target: after interruption, quota
  exhaustion, or process termination, either the prior file or the new one is
  present, never a mix or neither.
* A write that fails for lack of space or permission reports the failure to
  the calling code, and the engine never reports a write as complete or
  synchronized before it is durable.

A game that writes to a new file and replaces the prior one only after the
write completes keeps its prior valid save on every target. unitas states
these guarantees, and each target's storage location and quota, in its product
knowledge (AUT-006 in [Automation and MCP](automation-and-mcp.md)).

### Serializers for save data

* `JsonUtility` and Newtonsoft Json (`com.unity.nuget.newtonsoft-json` 3.x) are
  Supported on every scripting backend.
* `System.Xml.Serialization.XmlSerializer` and `BinaryFormatter` are
  Supported on every scripting backend and every target, including the
  browser target. On ahead-of-time backends (BLD-003 in
  [Builds, platforms, XR, and networking](build-platforms-xr-networking.md))
  they serialize and deserialize the same data as on just-in-time backends,
  for every type the project serializes with them. Current .NET no longer
  provides a working `BinaryFormatter`; unitas supplies and maintains a
  compatible implementation and does not depend on the base class library
  for it.
* A serialization that fails because the project's managed code stripping
  settings removed a type or member names that type and member and the
  preservation mechanisms that would retain it, as defined by BLD-003.

## Save evolution and migration

When a project's load code accepts older save versions, loading preserves the
meaning of supported player progress and settings. A migration identifies input
version, resulting version, outcome, and any data that cannot retain meaning.

Migration can be playtested using representative older saves. A failed
migration that writes its result to a new file leaves the original data
available on every target, and does not masquerade as a new game.

## Cloud and conflict behavior

Synchronized storage from platform services, including cloud saves, is owned
by Online and platform services in
[Builds, platforms, XR, and networking](build-platforms-xr-networking.md#online-and-platform-services).
The engine's part is local: saves written through `PlayerPrefs` or file access
stay intact when a service call fails, and each service failure reaches game
code as the service's SDK reports it and appears in diagnostics naming the
service and operation.

## Privacy and user control

The editor and its MCP interface list each storage root of a play session or a
development player (player preferences, persistent data, and cache) with its
location and size. Clearing one root reports what it removed, leaves the other
roots untouched, and names data it cannot remove because an external platform
service controls it. Diagnostic, playtest, identity, and telemetry data never
live in the player-preference or persistent-data roots.

## Acceptance outcomes

1. Editor authoring, diagnostics, and play controls, including custom
   inspectors and editor windows drawn by extensions, are usable by keyboard,
   expose readable accessible state, and present the same state through
   automation. (ACC-001)
2. A game can offer captions, scalable UI, remappable controls, reduced motion,
   and non-color-only cues and can playtest each configuration explicitly, with
   each result naming its input method and settings. (ACC-001)
3. Switching locale through `LocalizationSettings.SelectedLocale` updates bound
   strings and assets without reloading the scene, reports fallback and layout
   failures, and preserves game-state identity. (LOC-001)
4. Smart Strings with variables and plurals and locale-aware number and date
   formatting produce, for each locale the project defines, the plural form its
   language rules select and its culture's number and date format. (LOC-001)
5. Previewing a pseudo-localized or right-to-left locale in the editor or
   through automation reports missing translations, missing assets, overflow,
   the right-to-left state, and the locale's font coverage without changing
   saved content; ar, he, hi, and th locales whose fonts cover their content
   render shaped, bidirectionally ordered text and are reported fully
   supported. (LOC-001)
6. `PlayerPrefs` values survive a player restart in Unity's per-target
   location, and clearing preferences does not remove save data. (SAV-001)
7. A playtest scenario that terminates the player during a save file write
   under `Application.persistentDataPath`, and one that exhausts storage
   quota, each find the prior valid save intact after a write-then-replace
   save on every target, including the browser target, and the failed write
   is reported to the calling code. (SAV-001)
8. Older saves that a project's load code accepts migrate with reported input
   version, resulting version, and outcome; corrupt, incompatible, absent, and
   failed data remain distinct outcomes. (SAV-001)
9. For a project using Localization members whose unitas behavior differs from
   Unity 2022.3, each difference appears in the AUT-006 API reference and in the
   project compatibility report. (LOC-001)
10. Save data round-trips through `JsonUtility`, Newtonsoft Json,
   `XmlSerializer`, and `BinaryFormatter` on each scripting backend and every
   target, including the browser target, and a type removed by stripping
   produces a diagnostic naming the type. (SAV-001)
11. On every target, `PlayerPrefs` values saved with `PlayerPrefs.Save` and a
    closed persistent-data file survive a crash immediately afterwards (on the
    browser target, once its synchronization completes), and
    product knowledge available through automation states these guarantees
    and each target's storage quota. (SAV-001)
12. With the player accessibility extension on, the platform screen reader on
    Windows, macOS, iOS, Android, Linux, and the browser target announces each
    uGUI and UI Toolkit control's name, role, value, and state as focus moves,
    a script announcement is spoken, game code reads the platform's reduced
    motion and text scale preferences, and with the extension off no control
    is exposed. (ACC-001)
13. On the browser target, `PlayerPrefs` stores more than 1 MB up to the
    browser's quota, and a save file closed under
    `Application.persistentDataPath` is present after the page reloads.
    (SAV-001)
14. A string table collection exported to CSV and XLIFF and imported back
    keeps its keys, identifiers, and comments, and a Game Object Localizer
    changes a RectTransform width and a font when the locale changes.
    (LOC-001)
