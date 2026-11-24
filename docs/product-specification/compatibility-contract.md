# Compatibility Contract

## User Outcome

As the owner of a supported Unity project, I can determine before editing,
playing, or building which project behaviors unitas supports, which have stated
limits, and which cannot yet be represented. Compatibility information refers
to my actual assets, scenes, scripts, settings, packages, and targets.

## Compatibility Baseline

The compatibility baseline covers declared behavior from Unity 2020.3 LTS
through Unity 2022.3 LTS, with Unity 2022.3 LTS as the principal behavioral
reference. A project can contain older serialized forms or settings where those
releases supported migration to the baseline.

Compatibility includes, when the feature is declared supported:

* The meaning and identity of project content.
* Serialized values and object references.
* Public scripting behavior, including timing and failure behavior.
* Editor-visible state that affects saved project content.
* Runtime visuals, input, audio, simulation, application lifecycle, and state.
* Player and build behavior on the selected target.

Matching names, file extensions, public members, or screen layouts alone is not
compatibility.

## Compatibility States

Every assessed behavior has one state:

* **Supported:** all behavior named by the applicable unitas specification is
  available for the stated project and target conditions.
* **Partially supported:** a defined subset behaves as specified and every
  excluded behavior is named.
* **Unsupported:** the content or request is recognized, but its required
  behavior is unavailable.
* **Unknown:** unitas cannot classify the content or behavior reliably.
* **Platform-limited:** the behavior is supported generally but unavailable or
  materially different on the selected target.
* **Externally unavailable:** the behavior requires an entitlement, service,
  device, private SDK, account, or permission that is absent.

The editor and automation interfaces expose the same compatibility state.

## Project Compatibility Report

A report covers project version, packages, settings, scenes, prefabs, assets,
serialized types, scripts and APIs, shaders and materials, input, physics,
animation, UI, audio, platform services, and selected build targets.

Each finding includes:

* Stable identity and location of the affected content.
* Feature and behavior being assessed.
* Compatibility state.
* Observable consequence during editing, play, testing, or building.
* Conditions under which the finding applies.
* Related findings when one limitation causes multiple downstream effects.

A report distinguishes “not present in this project” from “present and
supported.” It never converts absence into a compatibility claim.

## Behavioral Equivalence

Equivalent behavior means a player, developer, test scenario, or supported
script observes the same specified outcome. Exact internal ordering is required
only when Unity exposes that order as a contract or project behavior depends on
it. Exact pixel, sample, floating-point, or timing identity is required only
when the applicable feature specification states a tolerance or exact result.

Permitted environmental variation never excuses a semantic difference such as
the wrong object receiving an event, a reference resolving to a different
asset, a save losing a value, a collision failing to occur, or a control
performing another action.

## Round-Trip Expectations

Supported project edits preserve Unity-readable identities and content where
the corresponding Unity format can express the result. Unknown data that can
be retained without misrepresentation remains intact. If an edit would require
discarding or rewriting unsupported meaning, the user sees the scope and
chooses whether to continue. If saving an unrelated edit cannot retain unknown
data, that save is blocked until the user explicitly chooses to discard the
identified data.

Opening and saving an unchanged asset does not create unexplained semantic
changes. Differences in non-semantic formatting are never presented as proof of
compatibility.

## Failure Behavior

A failed compatibility operation leaves unrelated content unchanged. It
identifies what was attempted, what content was affected, what remains usable,
and whether a retry is safe. Partial success is reported as partial success.

unitas never substitutes a plausible default, fallback shader, empty reference,
different API result, or ignored setting while reporting full support.

## Acceptance Stories

* Before relying on a release build, a project owner can identify every
  recognized, assessed unsupported behavior in the selected project and target;
  unknown or unassessed behavior remains explicit.
* A missing asset reference remains missing and identifies its original
  identity rather than resolving to a similarly named asset.
* A partially supported shader reports the omitted pass and visible effect.
* A console target without private SDK access is externally unavailable rather
  than reported as an engine failure or a supported build.
* Reassessment after a project change updates only affected findings and makes
  the report's project state clear.
