# Accessibility, Localization, and Persistence Product Specification

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

## Accessible editor experience

Core editor workflows are keyboard operable and expose accessible names,
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
commands with accessible names. Custom editor extension drawing is Unsupported
as listed in the [index](INDEX.md), so an extension never removes
accessibility from the default inspector presentation used in its place or from
unrelated editor functions.

## Accessibility features available to games

The engine enables projects to offer remappable controls, keyboard-only and
controller-only paths, text and UI scaling, captions, subtitles, speaker cues,
contrast-conscious presentation, alternatives to color-only information,
reduced motion, and configurable timing where the game exposes it. The input,
UI, text, and audio building blocks for these are owned by
[UI, audio, and input](ui-audio-input.md).

Unity 2022.3 LTS offers no player screen-reader API. Any
assistive-technology metadata that unitas offers to players is a unitas
extension, identified as such, and is not represented as Unity-compatible
behavior.

The player retains accessibility choices across sessions when the project
requests persistence. A first-run flow can expose essential accessibility
choices before visually or physically demanding content begins.

Caption and subtitle behavior can represent text, speaker, timing, duration,
sound cues, and readability choices. Muting audio does not remove textual
alternatives that the game provides.

## Accessibility playtesting

Scenarios can evaluate keyboard-only, controller-only, touch-only, high-scale
UI, captions, reduced motion, contrast, color-independent cues, focus order,
navigation, time limits, and interruption recovery.

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

A project that implements localization without the package (for example with
its own tables of strings) is ordinary project content; its behavior follows
from the scripts and assets it uses.

### Required behavior

As a player, I can select an available locale and see the change applied to
supported content. Locale selection persists when the project requests it and
remains distinguishable from device locale and project default.

Changing locale updates every bound localized string and asset without
reloading scenes or recreating unrelated game state. Changing locale does not
corrupt saved game state or confuse content identity with translated display
text.

A locale is right-to-left when its Locale asset metadata or the project's
localization settings mark it so. For each locale, the right-to-left state and
the support state of the text system that renders it, as defined by TXT-001 in
[UI, audio, and input](ui-audio-input.md), are reported. The presence of
Unicode characters alone is not treated as full language support.

Missing translations remain visibly diagnosable and do not silently masquerade
as translated content.

### Localization diagnostics

The editor and automation can preview content in a selected locale without
changing saved project content, and identify:

* missing or stale translations;
* missing localized assets;
* fallback use;
* truncated, clipped, overlapping, or overflowing text;
* missing glyphs and unsupported script behavior (as reported by TXT-001);
* content variables absent from or inconsistent with a translation;
* layouts that fail under right-to-left direction or expanded strings.

## Player preferences

Projects can store user choices such as volume, graphics, controls, locale,
accessibility, and gameplay preferences. `PlayerPrefs` (`SetInt`, `SetFloat`,
`SetString`, `GetInt`, `GetFloat`, `GetString`, `HasKey`, `DeleteKey`,
`DeleteAll`, and `Save`) behaves as in Unity and stores data in the same
per-user location that Unity uses for the target, company name, and product
name. Preference failures, such as exceeding the browser storage quota, are
reported to gameplay code as in Unity and in diagnostics.

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

For each target, unitas documents in its product knowledge (AUT-006 in
[Automation and MCP](automation-and-mcp.md)) when `PlayerPrefs.Save` and a file
write under `Application.persistentDataPath` become durable (including the
browser target's synchronized storage), whether a crash loses `PlayerPrefs`
changes not yet saved, and whether a prior file survives interruption, quota
exhaustion, and process termination during a write. Engine storage behavior
matches that documentation: a write that fails for lack of space or
permission reports the failure to the calling code, and the engine never
reports a write as complete before it is durable under that documentation.
A game that writes to a new file and replaces the prior one only after the
write completes keeps its prior valid save on every target whose
documentation says the replacement is atomic.

### Serializers for save data

* `JsonUtility` and Newtonsoft Json (`com.unity.nuget.newtonsoft-json` 3.x) are
  Supported on every scripting backend.
* `System.Xml.Serialization.XmlSerializer` is Supported, with the
  ahead-of-time compilation limits defined by BLD-003 in
  [Builds, platforms, XR, and networking](build-platforms-xr-networking.md).
* `BinaryFormatter` is Partially supported: it is unavailable on the browser
  target, and it is subject to the BLD-003 ahead-of-time compilation limits.
* A serialization that fails because of ahead-of-time compilation or managed
  code stripping names the type and member that could not be serialized.

## Save evolution and migration

When a project's load code accepts older save versions, loading preserves the
meaning of supported player progress and settings. A migration identifies input
version, resulting version, outcome, and any data that cannot retain meaning.

Migration can be playtested using representative older saves. A failed
migration leaves the original data available on targets whose documented
storage behavior keeps the prior file, and does not masquerade as a new game.

## Cloud and conflict behavior

When a game uses synchronized storage from a platform service, the game distinguishes local and remote
versions, offline changes, upload or download failure, quota, authentication,
and conflict. A conflict exposes enough information for the conflict policy
the game implements or for a user choice; it never silently treats the newest timestamp as correct
when ordering is unknown.

## Privacy and user control

Persistent data categories and their purpose are inspectable by the project.
The product does not label diagnostic, playtest, identity, or telemetry data as
ordinary preferences. Deleting player data reports what was removed, retained,
unavailable, or controlled by an external platform service.

## Acceptance stories

1. Core editor authoring, diagnostics, and play controls are usable by keyboard,
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
   and the reported right-to-left and text-system support state without
   changing saved content. (LOC-001)
6. `PlayerPrefs` values survive a player restart in Unity's per-target
   location, and clearing preferences does not remove save data. (SAV-001)
7. A playtest scenario that terminates the player during a save file write
   under `Application.persistentDataPath`, and one that exhausts storage
   quota, each observe the outcome that the target's product-knowledge
   documentation states, and a write-then-replace save keeps the prior valid
   save on targets documented as atomic. (SAV-001)
8. Older saves that a project's load code accepts migrate with reported input
   version, resulting version, and outcome; corrupt, incompatible, absent, and
   failed data remain distinct outcomes. (SAV-001)
9. For a project using Localization members whose unitas behavior differs from
   Unity 2022.3, each difference appears in the AUT-006 API reference and in the
   project compatibility report. (LOC-001)
10. Save data round-trips through `JsonUtility`, Newtonsoft Json, and
   `XmlSerializer` on each scripting backend; `BinaryFormatter` is reported
   Partially supported and unavailable on the browser target; and a type
   removed by stripping produces a diagnostic naming the type. (SAV-001)
11. Product knowledge available through automation states, for each target,
    when `PlayerPrefs.Save` and persistent-data file writes are durable and
    what a crash loses. (SAV-001)
