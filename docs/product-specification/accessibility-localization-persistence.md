# Accessibility, Localization, and Persistence Product Specification

## Scope

This specification defines editor accessibility, game accessibility features,
localized content behavior, player settings, save data, and recovery from data
problems.

## Accessible editor experience

Core editor workflows are keyboard operable and expose meaningful accessible
names, roles, values, focus, selection, expanded state, validation, progress,
and status. Information does not depend on color, pointer hover, spatial layout,
animation, or a screenshot alone.

As a developer, I can:

* navigate project content, hierarchy, inspector, console, build status, and
  play controls without a pointing device;
* identify current focus and selection;
* scale editor text and controls while preserving essential content;
* use high-contrast presentation and non-color status indicators;
* pause or reduce nonessential editor motion;
* inspect the same state through structured automation.

Custom editor extensions expose equivalent accessible state to the extent of
their declared support. An inaccessible extension does not erase accessibility
of unrelated editor functions.

## Accessibility features available to games

The engine enables projects to offer remappable controls, keyboard-only and
controller-only paths, text and UI scaling, captions, subtitles, speaker cues,
contrast-conscious presentation, alternatives to color-only information,
reduced motion, configurable timing where game design permits it, and supported
assistive-technology metadata.

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

Runtime text supports declared font assets, Unicode content, fallback fonts,
alignment, wrapping, overflow, rich text within scope, sizing, and resolution-
aware presentation.

Missing glyphs identify the font, character, locale, and affected text where
available. Fallback is visible and deterministic. Text does not silently drop
unsupported characters or substitute them without diagnostic state.

Text layout responds predictably to scaling, long strings, line breaks, mixed
scripts, bidirectional content, and dynamic values within declared language
support. Basic character display is not represented as complete complex-script
support.

## Localization

When the project's supported localization package and version supplies the
feature, as a developer I can define localized strings and assets, locale metadata,
fallback rules, and supported plural, variable, gender, number, date, and time
behavior within the declared scope.

As a player, I can select an available locale and see the change applied to
supported content. Locale selection persists when requested and remains
distinguishable from device locale and project default.

The editor can preview content in a selected locale and identify:

* missing or stale translations;
* missing localized assets;
* fallback use;
* truncated, clipped, overlapping, or overflowing text;
* missing glyphs and unsupported script behavior;
* content variables absent from or inconsistent with a translation;
* layouts that fail under right-to-left direction or expanded strings.

Changing locale does not corrupt saved game state or confuse content identity
with translated display text.

## Player preferences

Projects can store user choices such as volume, graphics, controls, locale,
accessibility, and gameplay preferences. Preferences expose success and failure
to gameplay code and use platform-appropriate per-user storage.

Project assets, shared project settings, editor preferences, player preferences,
and game-progress data remain distinct. Clearing one category does not silently
delete another.

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
migration leaves the original data available where the platform permits and
does not masquerade as a new game.

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

1. Core editor authoring, diagnostics, and play controls are usable by keyboard
   and expose readable accessible state.
2. A game can offer captions, scalable UI, remappable controls, reduced motion,
   and non-color-only cues and can playtest each configuration explicitly.
3. Switching locale updates supported strings and assets, reports fallback and
   layout failures, and preserves game-state identity.
4. Interrupted saving never appears successful and does not silently destroy a
   prior valid save.
5. Older supported saves migrate with declared results; corrupt, incompatible,
   absent, and failed data remain distinct outcomes.
