# Playable Samples and Compatibility Benchmarks

## Product Outcome

unitas includes complete playable games and focused compatibility projects that
demonstrate the engine's declared behavior. A sample is not accepted because it
opens, imports, compiles, or reaches a title screen. It has objective gameplay
scenarios that succeed in editor play mode, headless through MCP, and on
declared player targets. Agent authoring benchmarks show that an agent can
change a sample through MCP alone.

Compatibility tiers of the features this specification owns, as recorded in
the [Feature catalog](../feature-catalog.md) and defined in the
[Compatibility contract](../compatibility-contract.md): SMP-001 Native, CMP-001
Native. The playable samples are original unitas games; their acceptance is
defined by their own scenarios and unitas-approved references, not by
comparison with output from Unity. Only the compatibility benchmarks compare
with Unity 2022.3 LTS, and only for Exact-tier and Semantic-tier behavior.

Every sample provides controls, gameplay objective, expected presentation,
feature coverage, supported targets, accessibility information, known
limitations, and repeatable acceptance scenarios. Samples use redistributable
content with clear licensing.

## Sample-Level Requirements

Each playable sample includes:

* A complete start, gameplay, completion or failure, restart, pause, and exit
  experience.
* At least one automated happy-path scenario and one failure or recovery
  scenario.
* Manual playtest guidance for behavior that requires human judgment.
* unitas-approved reference images, as defined by TST-002 in
  [Diagnostics, profiling, and playtesting](../diagnostics-profiling-playtesting.md),
  for stable visual checkpoints; visual checkpoints of Familiar-tier and
  Native-tier features are judged against these references, never against
  Unity screenshots.
* Performance expectations stated separately for representative target classes.
* A compatibility report that identifies every feature the sample exercises.
* A saved or reproducible state for reaching important test situations quickly.
* Keyboard accessibility and at least one additional suitable input mode where
  the target supports it.

Sample performance expectations are project-declared expectations as defined
under Performance validation (TST-002) in
[Diagnostics, profiling, and playtesting](../diagnostics-profiling-playtesting.md);
they are distinct from, and never presented as, the engine budgets of PRF-002.

## Headless and Agent Execution

Every acceptance scenario of every sample and benchmark runs in two ways: by a
person in the editor or a player, and headless through MCP by an authorized
agent or a continuous-integration invocation, as defined in
[Automation and MCP](../automation-and-mcp.md) and by BLD-002 in
[Builds, platforms, XR, and networking](../build-platforms-xr-networking.md). Both
ways use the same scenario definition and produce the same pass or fail outcome
and the same checkpoint evidence, including visual checkpoints captured without
a display. A scenario that requires human judgment states that requirement, and
its headless run reports the checkpoint as awaiting human review rather than as
passed.

"Headless through MCP" always means the editor. An agent, or a
continuous-integration job, connects to the headless editor's MCP interface,
opens the sample project, and works on it the way a developer does. It builds
and edits the game, starts play sessions in the editor's worker process,
injects input, reads state, captures evidence, and stops play. The game itself,
the engine, and every packaged player contain no MCP. Scenarios in a packaged
player run from replayed input and the player's own logs and captures, and the
editor collects their results.

Every scenario result records the sample revision, the unitas version, the
target, and whether it ran headless.

## The Sample Suite

The suite is the set of games below. Each game has its own detailed
requirement and feature document in this directory. That document owns the
game's design, content, required engine behavior, scenarios, and acceptance
outcomes. The engine is complete only when every game in the suite is
complete. The order is the order in which the suite is proven. Every game is
required, and no game is optional because of its position.

| Order | Game | Document | Principal feature families |
|---|---|---|---|
| 1 | Roll-a-Ball-style arena ("Marble Rush") | [01-roll-a-ball.md](01-roll-a-ball.md) | RUN-001, RUN-003, RUN-004, PHY-001, INP-001, INP-002, REN-002, REN-004, UI-001, AUD-001, SAV-001, BLD-001 |
| 2 | Two-dimensional platformer | [02-2d-platformer.md](02-2d-platformer.md) | REN-005, PHY-002, ANI-001, SAV-001, INP-002, UI-001 |
| 3 | Third-person adventure | [03-third-person-adventure.md](03-third-person-adventure.md) | ANI-001, ANI-002, CIN-001, PHY-003, NAV-001, ENV-001, REN-003, MED-001, LOC-001, SAV-001 |
| 4 | First-person interaction | [04-first-person-interaction.md](04-first-person-interaction.md) | INP-002, PHY-001, PHY-004, REN-001, REN-003, REN-006, AUD-001 |
| 5 | UI and localization showcase | [05-ui-localization-showcase.md](05-ui-localization-showcase.md) | UI-001, TXT-001, LOC-001, ACC-001, SAV-001 |
| 6 | Kart racer | [06-kart-racer.md](06-kart-racer.md) | PHY-004, CIN-001, REN-001, REN-003, AUD-001, RUN-006, BLD-005, INP-002 |
| 7 | Arena shooter | [07-arena-shooter.md](07-arena-shooter.md) | REN-004, REN-006, UI-002, RUN-009, RUN-010, RUN-001 |
| 8 | Strategy skirmish | [08-strategy-skirmish.md](08-strategy-skirmish.md) | RUN-011, NAV-001, ENV-001, REN-001, REN-006, PRF-002 |
| 9 | Rhythm game | [09-rhythm-game.md](09-rhythm-game.md) | AUD-001, MED-001, INP-002, RUN-004, PLT-001 |
| 10 | Mobile game | [10-mobile-game.md](10-mobile-game.md) | PLT-001, INP-002, SAV-001, BLD-004 |
| 11 | Multiplayer arena | [11-multiplayer-arena.md](11-multiplayer-arena.md) | NET-001, RUN-009, BLD-002 |
| 12 | XR interaction room | [12-xr-interaction-room.md](12-xr-interaction-room.md) | XR-001, INP-002, AUD-001, UI-001 |

Every sample is a game a person wants to play for its own sake, and not only a
feature demonstration. Each one has a hook, a mastery curve, meaningful
decisions, escalation, strong game feel, and reasons to replay. A sample that
exercises its features but is not fun to play is incomplete.

All samples are original unitas games. They are authored in Unity 2022.3 LTS
on-disk formats (scenes, prefabs, `.meta` files, `ProjectSettings/`,
`Packages/manifest.json`, and C# scripts) so that Unity 2022.3 LTS can open
them, and they use only redistributable content with recorded licences.

## Sample Document Structure

Every game document uses these sections, in this order:

1. **Overview.** The player's goal, session length, tone, and the claim that
   the game is original unitas content.
2. **Targets and input modes.** The declared player targets, the render
   pipelines used (Built-in, URP with version line), and every supported
   input mode.
3. **Game design.** The core loop, a controls table for every input mode,
   areas or levels, objectives, scoring or progression, the complete start,
   play, pause, completion, failure, restart, and exit flow, and the
   settings the player can change. It contains a **What makes it fun**
   subsection, which covers:
   * the hook: what makes the first 30 seconds compelling
   * the mastery curve: the skills that a better player shows
   * the risk-and-reward decisions
   * variety and escalation across the session
   * game feel and feedback ("juice": responsive controls, hit and collect
     feedback, camera, sound, and particles)
   * the reasons to replay: medals, records, ghosts, and unlocks

   Every element is concrete enough to test. Its scenarios observe the
   mechanics, and its manual playtest guidance judges whether it is fun.
4. **Content.** Scenes, prefabs and variants, models, textures, audio,
   fonts, video, and data assets, each with the source format that is
   imported and the import settings that matter. Also the Unity packages and
   their version lines from CMP-002.
5. **Feature coverage.** A table mapping each catalog identifier the game
   exercises to the concrete behavior it exercises and the scenarios that
   observe it.
6. **Engine behavior this game depends on.** The observable behavior
   required, grouped by area: scripting and runtime, project and content,
   simulation, rendering and animation, audio, input, UI and text, player
   and build, and editor and automation. Each item is specific enough to test.
7. **Presentation and performance.** The visual and audio checkpoints with
   unitas-approved references, and the project-declared performance
   expectations per target class. These expectations are kept separate from
   the PRF-002 engine budgets.
8. **Acceptance scenarios.** Numbered scenarios, each with preconditions,
   steps, expected observations, the catalog identifiers it demonstrates,
   and where it runs (editor play mode, player, headless through MCP). Each
   game has at least one happy-path scenario, at least one failure or
   recovery scenario, and scenarios for its listed edge cases.
9. **Manual playtest guidance.** The behavior that requires human judgment,
   and how a headless run reports it.
10. **Agent authoring change requests.** At least one change request with its
    acceptance scenario, for the Agent Authoring Benchmark.
11. **Accessibility.**
12. **Saved and reproducible states** for reaching important test
    situations quickly.
13. **Known limitations and unsupported conditions.**
14. **Acceptance outcomes.** Bullets, each naming the catalog identifiers it
    demonstrates.

## Feature Coverage of the Suite

Every catalog identifier maps to at least one game in the suite, through that
game's Feature coverage table, or to a benchmark below. The table below is
the suite-level index of that mapping. Each game document's own coverage table
is authoritative for that game.

| ID | Games (by suite order) | Workflows and benchmarks |
|---|---|---|
| PRJ-001 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 | — |
| PRJ-002 | — | W1 |
| PRJ-003 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 | — |
| PRJ-004 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 | — |
| PRJ-005 | — | W1 |
| PRJ-006 | 2, 5, 10, 12 | — |
| PRJ-007 | 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12 | — |
| PRJ-008 | — | W2 |
| SER-001 | 1, 2, 3, 4, 6, 7, 8, 9 | — |
| SER-002 | 1, 2, 3, 5, 6, 8, 9, 10, 11 | — |
| SER-003 | 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12 | — |
| SER-004 | — | W3 |
| SER-005 | 4, 8 | — |
| EDT-001 | 8 | — |
| EDT-002 | 7, 8 | — |
| EDT-003 | — | W4 |
| EDT-004 | 1, 2, 3, 4 | — |
| EDT-005 | 8 | W4 |
| EDT-006 | — | W4 |
| EDT-007 | — | W5 |
| EDT-008 | — | W7, agent authoring benchmark |
| RUN-001 | 1, 2, 3, 4, 6, 7, 8 | — |
| RUN-002 | 1, 2, 3, 4, 6, 7 | — |
| RUN-003 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 | — |
| RUN-004 | 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12 | — |
| RUN-005 | 1, 2, 3, 6, 8 | — |
| RUN-006 | 3, 6, 9 | — |
| RUN-007 | — | W6 |
| RUN-008 | 3, 4, 5, 7 | — |
| RUN-009 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 | — |
| RUN-010 | 7, 8 | — |
| RUN-011 | 8 | — |
| PHY-001 | 1, 3, 4, 6, 7, 8, 11, 12 | — |
| PHY-002 | 2 | — |
| PHY-003 | 3, 4 | — |
| PHY-004 | 4, 6 | — |
| NAV-001 | 3, 8 | — |
| ENV-001 | 3, 8 | — |
| REN-001 | 1, 2, 3, 4, 6, 8, 12 | — |
| REN-002 | 1, 3, 4, 6, 7, 8, 10, 12 | — |
| REN-003 | 1, 3, 4, 6, 7, 8, 9, 10, 12 | — |
| REN-004 | 1, 3, 4, 6, 7, 8, 9, 11, 12 | — |
| REN-005 | 2 | — |
| REN-006 | 2, 3, 4, 6, 7, 8 | — |
| REN-007 | 1, 2, 3, 4, 6, 8, 9, 10, 12 | — |
| ANI-001 | 2, 3 | — |
| ANI-002 | 3 | — |
| CIN-001 | 2, 3, 6, 7, 9 | — |
| UI-001 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 | — |
| UI-002 | 5, 7 | — |
| TXT-001 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 | — |
| INP-001 | 1 | — |
| INP-002 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 | — |
| AUD-001 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 | — |
| MED-001 | 3, 9 | — |
| BLD-001 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 | — |
| BLD-002 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 | — |
| BLD-003 | 10 | — |
| BLD-004 | 10 | — |
| BLD-005 | 3, 6 | — |
| PLT-001 | 2, 5, 9, 10, 11, 12 | — |
| XR-001 | 12 | — |
| NET-001 | 11 | — |
| LIC-001 | 10 | — |
| PRD-001 | — | W10 |
| PRD-002 | 10 | — |
| LOC-001 | 3, 5 | — |
| ACC-001 | 1, 3, 4, 5, 6, 7, 8, 9, 11, 12 | — |
| SAV-001 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 | — |
| DIA-001 | 5, 8, 11 | — |
| DIA-002 | — | W6 |
| DIA-003 | 10 | — |
| PRF-001 | 4, 8, 10, 11, 12 | — |
| PRF-002 | 7, 8 | — |
| TST-001 | — | W9 |
| TST-002 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 | — |
| TST-003 | 7 | W6 |
| SMP-001 | — | the suite |
| CMP-001 | — | compatibility benchmarks |
| CMP-002 | 4 | — |
| CMP-003 | — | third-party package benchmark |
| CMP-004 | — | compatibility benchmarks |
| CMP-005 | — | W10 |
| AUT-001 | — | W11, agent authoring benchmark |
| AUT-002 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 | — |
| AUT-003 | — | W7 |
| AUT-004 | 11 | — |
| AUT-005 | — | W6, agent authoring benchmark |
| AUT-006 | — | W8 |
| AUT-007 | 10 | W2 |
| AUT-008 | — | W8 |

## Development Workflows on the Suite

The games are also the projects on which people and agents prove the
development workflow. Each workflow below runs on the named game projects, in
the visual editor and headless through MCP, and its outcome is observed like
any other scenario.

* **W1. Browse and organize.** In the strategy skirmish project, search,
  label, preview, move, and delete assets. Inspect the dependencies and
  reverse dependencies of a unit prefab before deleting it. Every reference
  stays intact after the moves, and the delete warns about its dependents.
  (PRJ-002, PRJ-005)
* **W2. Trust.** Open the arena shooter project untrusted. No project code
  runs, and the editor states what is held back. Grant trust as the person,
  and the scripts compile and run. (PRJ-008, AUT-007)
* **W3. Broken references.** Delete a prefab that the third-person adventure's
  scenes reference. Every missing reference is shown at its owning object and
  property, survives save and reload, and resolves again when the prefab is
  restored. (SER-004)
* **W4. Scene authoring.** In the kart racer project, navigate a track in 3D,
  move props with handles and snapping, and edit a spline. Work with docked and
  floating views, shortcuts, drag and drop, and multi-object editing. Undo and
  redo every step. (EDT-003, EDT-006, EDT-005)
* **W5. Source control and merge.** Commit the 2D platformer to Git. Change the
  same level in two branches, one edited by a person and one by an agent, and
  merge them. The semantic merge combines the non-conflicting changes, and
  shows each conflict by object and property. (EDT-007)
* **W6. Code loop.** In the rhythm game project, add an assembly definition and
  a define. Write a script with a compile error, read the diagnostic, fix it,
  set a breakpoint from an external IDE through the debugger connection, step
  through the note judge, and run the project tests. Do this once as a person
  and once as an agent. (RUN-007, DIA-002, AUT-005, TST-003)
* **W7. Shared session.** A person and an agent edit the multiplayer arena's
  lobby scene at the same time. Each sees the other's changes immediately.
  A conflicting edit of the same property is resolved as AUT-003 defines. The
  agent's change set is reviewed, partially accepted, and partially reverted.
  (AUT-003, EDT-008)
* **W8. Audit and knowledge.** After W6 and W7, the audit log attributes every
  agent action to its session, with provenance. The agent answers questions
  about the engine from the product knowledge base, without other access.
  (AUT-008, AUT-006)
* **W9. Manual playtest.** A tester plays the first-person interaction build,
  records feedback with screenshots, and files it against the build. The
  feedback is linked to the scene, the time, and the build identifier.
  (TST-001)
* **W10. Install and support.** Install unitas from scratch on each desktop
  host and acquire the toolchains for the mobile game. Update to a newer
  release, and check the support status of the installed release line.
  (PRD-001, CMP-005)
* **W11. Parity.** Every operation in W1 to W10 that a person performs in the
  visual editor is also performed through MCP, with the same result.
  (AUT-001)

## Compatibility Benchmark Projects

Focused benchmark projects isolate behavior whose correctness is difficult to
infer from a complete game. Benchmarks of Exact-tier and Semantic-tier behavior
compare unitas with observations of Unity 2022.3 LTS; that set covers
serialization and references, prefabs and overrides, C# APIs and lifecycle
order, time and coroutines, scene loading, physics contacts and queries, asset
and resource loading, project tests, and save evolution.

Benchmarks of Familiar-tier and Native-tier behavior, including shader and
material behavior, animation transitions, UI layout and event routing, input
phases, audio timing, and target lifecycle, check the behavior that the owning
unitas specification defines against unitas-approved references. They are
never compared with Unity output, and a difference from Unity in them is a
listed difference, not a benchmark failure.

Each benchmark states its input project, tier, reference (Unity 2022.3 LTS for
Exact-tier and Semantic-tier benchmarks, the owning unitas specification and
its approved references otherwise), preconditions, observable expected
behavior, permitted variation, unsupported conditions, and result evidence. A
benchmark remains unchanged when unitas fails it; the result reports the
incompatibility rather than changing the expectation to match the engine. Each
benchmark result records the unitas version and, for Exact-tier and
Semantic-tier benchmarks, the Unity 2022.3 LTS release it was compared
against.

A third-party package benchmark is a project built from open-source Unity
packages, under permissive licences, of the kinds that real projects depend on:
tweening, dependency injection, serialization, state machines, and editor
tooling. Each package is assessed, compiled, and run through its own test suite
and a short scenario. Each finding names the package, the version, and the
member involved. (CMP-003)

An upstream Unity project is included only when its license permits
redistribution. Otherwise, the benchmark records acquisition prerequisites and
expected behavior without copying protected content. An original unitas sample
and an unmodified upstream compatibility benchmark remain explicitly distinct.

## Agent Authoring Benchmark

The Agent Authoring Benchmark demonstrates that an authorized agent using only
MCP can make a specified gameplay change to a sample and carry it through
compilation, testing, play, and developer review.

Each benchmark run starts from:

* a named playable sample at a recorded revision, with its existing acceptance
  scenarios passing;
* a written change request with an acceptance scenario that defines success;
* an agent session connected through MCP, as defined in
  [Automation and MCP](../automation-and-mcp.md), with no other access to the
  project and no human edits during the run.

The reference change request for the Roll-a-Ball-style sample is: add a new
pickup type that awards a different score value, has a distinct material and
collection sound, appears at specified positions in the arena, and shows a
score-feedback message in the UI when collected, together with a test that
checks the awarded score. Every other playable sample has at least one change
request of comparable scope, such as a new hazard in the two-dimensional
platformer or a new interactable in the first-person interaction sample.

During the run the agent inspects the project, creates or edits scripts,
assets, prefabs, and scene content, requests compilation and resolves its
diagnostics, adds and runs project tests (TST-003), runs the sample's scenarios
and the change request's scenario (TST-002), and plays the sample to capture
evidence. The developer then reviews the session's change set (EDT-008) in
[Editor and authoring](../editor-and-authoring.md) and accepts or rejects it.

A run succeeds only when every criterion is observed:

* the project compiles with no errors, and every warning introduced by the
  change is listed in the run result;
* the tests the agent added and the project's existing tests pass;
* the change request's acceptance scenario passes in editor play mode and in a
  player on at least one declared target;
* every acceptance scenario that passed at the starting revision still passes;
* the change set contains only changes attributed to the agent session, each
  shown as a semantic property-level difference for content and a text
  difference for scripts, with no unrelated serialized changes;
* the changed files remain Unity-readable under the Exact-tier round-trip
  expectations of the [Compatibility contract](../compatibility-contract.md), and,
  where Unity 2022.3 LTS is available, the changed project opens in it with the
  new scenes, prefabs, serialized values, and references intact;
* accepting the change set keeps the change, and rejecting it returns the
  project to its starting content.

Status of the benchmark: `make verify-benchmark` runs the agent session
against a copy of the sample, creates and imports the change request's distinct
material and binary collection sound through the `assets.import` MCP command,
and demonstrates the criteria it observes in editor play mode: the project
compiles with no errors, the test the agent added and the project's existing
tests pass, the change request's acceptance scenario passes, the scenarios that
passed at the starting revision still pass, the change set holds only the
changes the change request allows, and rejecting the change set returns the
copied project to its starting content. Two criteria are not observed by that
run and are recorded as not demonstrated rather than as passed: the acceptance
scenario is not run in a player on a declared target, because the run exercises
the sample in editor play mode and builds no player; and the changed project is
not opened in Unity 2022.3 LTS, because that editor is not present in the
environment the run executes in. The criteria above are unchanged: a run
succeeds only when every criterion is observed. A capability the agent needs
and the product does not provide is reported as a parity failure of AUT-001
naming the capability.

The run result records the sample revision, the change request, the unitas
version, the agent client as it identified itself, elapsed time, the number of
MCP requests, the diagnostics encountered, and the number of human
interventions, which is zero for a successful run. A failed run is classified as
a product failure when unitas lacked an MCP capability, returned an incorrect
result, or produced misleading diagnostics, and as an agent failure when the
product behaved as specified. A missing MCP capability is reported as a parity
failure of AUT-001 naming the capability.

## Acceptance Outcomes

* The sample suite is complete only when every game in the suite table meets
  every acceptance outcome of its own document on each of its declared
  targets. No game is optional, and the first game's completion is not
  completion of the suite. (SMP-001)
* The third-party package benchmark compiles and runs each listed package,
  and reports each incompatibility with the package, version, and member.
  (CMP-001, CMP-003)

* A new user can open, play, understand, alter, and build every sample for its
  declared targets. (SMP-001, BLD-001)
* Each feature claim in a sample maps to a catalog identifier and an observed
  scenario outcome. (SMP-001)
* Every acceptance scenario of every sample produces the same outcome and
  checkpoint evidence when a person runs it in the editor and when an agent runs
  it headless through MCP. (SMP-001, AUT-004, TST-002)
* A sample's performance report presents its project-declared expectations
  separately from the PRF-002 engine budgets and never labels them as engine
  budgets. (SMP-001, TST-002, PRF-002)
* A failing sample identifies the first failed behavior without erasing later
  evidence. (SMP-001, TST-002)
* Visual appeal never substitutes for behavioral coverage, and broad feature
  count never substitutes for a complete playable loop. (SMP-001)
* An agent using only MCP completes the Roll-a-Ball-style pickup change request,
  with compilation, tests, the new scenario, and all prior scenarios passing,
  and the developer accepts its change set; rejecting the same change set
  returns the project to its starting content. (SMP-001, AUT-005, TST-003,
  EDT-008)
* A failed Agent Authoring Benchmark run is classified as a product failure or
  an agent failure, and a missing MCP capability is reported by name. (SMP-001,
  AUT-001)
* A compatibility benchmark that unitas fails keeps its expectation unchanged,
  reports the incompatibility with its evidence, and records the unitas version
  and, for an Exact-tier or Semantic-tier benchmark, the Unity 2022.3 LTS
  release it was compared against. (CMP-001, CMP-004)
* A Semantic-tier benchmark of coroutine and lifecycle order compares unitas
  with Unity 2022.3 LTS observations, while a Familiar-tier benchmark of UI
  layout passes against its unitas-approved reference even where its layout
  differs from Unity's, and that difference is listed. (CMP-001, RUN-003,
  UI-001)
* A sample's visual checkpoint is judged against its unitas-approved reference
  image, and no sample acceptance scenario requires output identical to Unity.
  (SMP-001, TST-002)
* An upstream Unity project whose license does not permit redistribution is
  represented by acquisition prerequisites and expected behavior without copied
  content. (CMP-001)
* For each release, every generally available feature family in that release's
  feature catalog maps to at least one named sample scenario on a declared
  supported target; focused benchmarks cover boundary and failure behavior.
  (SMP-001, CMP-001, CMP-004)
