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
as declared in the [index](INDEX.md), so an extension never removes
accessibility from the default inspector presentation used in its place or from
unrelated editor functions.

## Accessibility features available to games

The engine enables projects to offer remappable controls, keyboard-only and
controller-only paths, text and UI scaling, captions, subtitles, speaker cues,
contrast-conscious presentation, alternatives to color-only information,
reduced motion, and configurable timing where the game exposes it. The input,
UI, text, and audio building blocks for these are owned by
[UI, audio, and input](ui-audio-input.md).

Unity 2020.3 through 2022.3 offers no player screen-reader API. Any
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

When the project uses a listed Localization package version:

* Locales (with identifier, code, fallback metadata, and custom formatter),
  string tables, asset tables, shared table data, and table collections retain
  their entries, keys, key identifiers, and metadata.
* `LocalizedString`, `LocalizedAsset<T>`, and the Localize String Event and
  Localize Asset Event components resolve their table reference and entry
  reference and update their targets when the selected locale changes.
* Smart Strings resolve variables (persistent variables, global variables
  groups, and arguments), plural-sensitive content, gender and choice
  selection, conditional formatting, lists, and nested localized strings as
  in the package.
* Locale-aware number, currency, date, and time formatting follows the
  selected locale's culture or custom formatter.
* Startup locale selectors (command line, system locale, specific locale, and
  player preference selectors) run in their declared order, and the result is
  distinguishable from the device locale and the project default.
* `LocalizationSettings.SelectedLocale`, `SelectedLocaleChanged`, the
  initialization operation, table preloading, and asynchronous table and asset
  loading behave as in the package. Loading through Addressables follows
  RUN-006 in [Scripting runtime](scripting-runtime.md).
* Fallback locales apply in their declared chain, and the missing-translation
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

Right-to-left and complex-script support is declared per language and feature
by the project, and the declared support state for each locale is reported.
The presence of Unicode characters alone is not treated as full language
support, and text rendering for a locale is bounded by TXT-001.

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

As a player, I can create, update, load, enumerate, and delete save data when the
game exposes those actions. A save identifies its game or profile, declared
version, timestamp or play context when used, and slot or identity.

As a developer, I can distinguish:

* absent data from corrupt data;
* new-game state from a failed load;
* current-version data from older supported data;
* recoverable data from incompatible data;
* local data from cloud or externally synchronized data;
* a user cancellation from an I/O or permission failure.

For each target, the declared storage contract identifies when a save is
committed and whether the prior valid save survives interruption, quota
exhaustion, and process termination. A save operation does not report success
until durable state is available under that contract. An interrupted or
out-of-space save does not replace a valid prior save with an apparently
successful corrupt one.

## Save evolution and migration

When a project declares support for older save versions, loading preserves the
meaning of supported player progress and settings. A migration identifies input
version, resulting version, outcome, and any data that cannot retain meaning.

Migration can be playtested using representative older saves. A failed
migration leaves the original data available where the platform's storage
contract permits it and does not masquerade as a new game.

## Cloud and conflict behavior

When synchronized storage is supported, the game distinguishes local and remote
versions, offline changes, upload or download failure, quota, authentication,
and conflict. A conflict exposes enough information for the game's declared
policy or user choice; it never silently treats the newest timestamp as correct
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
   formatting produce the package's output for each declared locale. (LOC-001)
5. Previewing a pseudo-localized or right-to-left locale in the editor or
   through automation reports missing translations, missing assets, overflow,
   and the declared script support state without changing saved content.
   (LOC-001)
6. `PlayerPrefs` values survive a player restart in Unity's per-target
   location, and clearing preferences does not remove save data. (SAV-001)
7. Interrupted saving never appears successful and does not silently destroy a
   prior valid save. (SAV-001)
8. Older supported saves migrate with declared results; corrupt, incompatible,
   absent, and failed data remain distinct outcomes. (SAV-001)
