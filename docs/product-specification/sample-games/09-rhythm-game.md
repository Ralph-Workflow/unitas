# Sample 9: Rhythm Game ("Beatline")

## Overview

Notes slide down four lanes toward a judgment line, and the player presses
each lane's key on the beat. Accurate presses build a combo, fill a Fever
meter, and raise the score. The game has two original songs, each with Easy,
Normal, and Hard charts, behind a music-synchronized video background. A song
lasts about two minutes, and a session of several charts takes 10 to 20
minutes.

This is an original, redistributable unitas sample. Its songs, charts, video,
art, and sounds are made for unitas and are distinct from any official Unity
sample or commercial rhythm game. It is one of the demonstrations of
sample-accurate audio timing, input judged against the audio clock, and the
same game on the Web target and on desktop players; the acceptance of those
features is owned by the domain specifications it cites.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns the acceptance of this game under SMP-001.

## Targets and Input Modes

### Targets

* **Automated:** macOS, Windows, and Linux desktop players, and the Web target
  ([Web target](../web-target.md)).
* **Capability-reported:** gamepads on the Web target, which depend on the
  browser exposing the Gamepad API; output latency, which depends on the
  target's report.
* **Editor play mode:** yes.
* **Headless editor:** yes.

### Render pipelines

URP 14.x, Forward rendering path.

### Input modes and devices

Keyboard and gamepad, through an Input System 1.x action asset with Keyboard
and Gamepad control schemes, in Dynamic update mode. Menus also accept the
pointer. On the Web target, audio starts only after the first user gesture, and
`vSyncCount` is ignored, so `targetFrameRate` paces frames up to the display
refresh.

Headless runs use synthetic devices with timestamped events, marked synthetic;
haptic and feel checkpoints need physical hardware and are otherwise
unevaluable.

## Game Design

### Core loop and flow

1. On the Web target, the first screen reads "Click or press any key to start"
   until audio is permitted. The title screen then shows Play, Calibrate,
   Settings, and Quit (Quit is absent on the Web target).
2. Song select lists Tidewalk (112 BPM, 2:04) and Copper Sky (148 BPM, 1:52),
   each with Easy, Normal, and Hard, the best score, grade, and badges.
3. The song is scheduled with `PlayScheduled` to start 1.5 seconds after
   selection. Song time is derived from `AudioSettings.dspTime`. Notes appear
   at the top of the lanes and reach the judgment line exactly on their beat.
4. Pressing a lane's key judges the nearest unjudged note in that lane. A note
   not pressed within 100 ms after its time is a Miss. Holds also need the key
   held until their end, and each held beat adds 50 points.
5. After the last note, the results screen shows score, grade, judgment
   counts, best combo, mean timing offset in milliseconds, the counts of early
   and late presses, and badges.
6. Results offer Retry, Song Select, and Quit to Title. Retry reschedules the
   song from the start with every transient value reset.

### Rules and mechanics

* **Judgment.** Each press is judged at its input event time placed on the
  audio clock, minus the input offset, compared with the note time plus the
  audio offset. The frame time of the press does not matter.
* **Windows.** Perfect within ±33 ms (300 points), Great within ±66 ms (200),
  Good within ±100 ms (100), otherwise Miss. A press more than 100 ms before
  every note in its lane judges nothing. Assist Timing widens the windows to
  ±50, ±99, and ±150 ms.
* **Note motion.** Each note's position is a function of song time and note
  speed, not of accumulated frame time.
* **Fever.** Perfect adds 2% to the Fever meter and Great 1%. At 100%, Fever can
  start on the next downbeat and lasts 16 beats. During Fever, scores double,
  bloom intensity rises from 0.3 to 1.0, and the lanes turn gold. A Miss ends
  Fever early.
* **Calibration.** Audio offset: 16 clicks at 120 BPM are scheduled with
  `PlayScheduled`. The player taps on clicks 5 to 16. The median tap offset
  becomes the audio offset, accepted from -250 to +250 ms. When the
  interquartile range of the taps exceeds 40 ms, the screen shows "Taps too
  uneven, try again". Input offset: a marker flashes 16 times at 120 BPM with
  no sound, and the player taps on each flash. The median becomes the input
  offset, from -250 to +250 ms.
* **Video.** The game's `VideoPlayer` plays the song's WebM file from its URL in
  Camera Far Plane mode with audio output None, using External time with
  `externalReferenceTime` set to song time each frame. A video that cannot play
  falls back to the static background, and play continues.
* **Signals.** Each song's `PlayableDirector` runs in DSP clock mode from the
  song's start; section signals change lane colors and trigger camera kicks.
* **Rebinding.** Rebinding a lane excludes the other lanes' keys.

### Levels and progression

Charts are JSON text assets with version, BPM, start offset in milliseconds,
and notes as beat, lane, and hold length in beats. Note counts:

| Song | Easy | Normal | Hard |
|---|---|---|---|
| Tidewalk | 180 | 340 | 520 |
| Copper Sky | 220 | 410 | 690 |

Easy uses quarter notes, Normal adds eighths and holds, and Hard adds
sixteenth-note streams and two-lane chords. Hard charts unlock after an A on
that song's Normal, or from Settings.

### Scoring, failure, and edge rules

* The combo counts consecutive non-Miss judgments. The multiplier is x1 below
  10, x2 from 10, x3 from 30, and x4 from 50.
* Grades: S at 95% of the chart's maximum score or more, A at 90%, B at 80%, C
  at 70%, and D below. Full Combo means no Miss, and All Perfect means only
  Perfect. The Metronome badge is earned when the mean offset rounds to 0 ms
  (absolute mean offset below 0.5 ms). Records set with Assist Timing are
  flagged Assist.
* There is no fail state mid-song. A run with zero non-Miss judgments still
  reaches the results screen with grade D.
* A stalled frame never shifts the music or the judgments.
* A damaged `highscores.json` is kept as `highscores.json.corrupt`, the game
  shows "Scores could not be read", and a new record writes a valid file.

### Controls

| Action | Keyboard | Gamepad | Mouse |
|---|---|---|---|
| Lanes 1 to 4 | D, F, J, K | D-pad left, D-pad down, South, East (or LT, LB, RB, RT) | — |
| Fever | Space | North button | — |
| Pause | Escape or P | Start | — |
| Menus: navigate, confirm, back | Arrows, Enter, Escape | D-pad, South, East | Pointer, left click |
| Calibration tap | Space | South button | Left click |

### Pause, focus, and lifecycle

Pause stops the song and video, freezes notes, and moves the mixer to the
Paused snapshot (Music to -80 dB in 0.1 s). Resume runs a 3-beat visual
countdown, then schedules the song at the paused sample. No note is judged
during the countdown. Losing focus, or a hidden Web target tab, pauses the game
within one frame.

### Settings

Master, music, and effects volume, hit tick on or off, note speed (1.0 to 6.0),
background (Video, Static, Off), frame-rate cap (30, 60, 144, 240, uncapped),
vertical sync on desktop, Assist Timing, beat cues, high-contrast notes,
reduced motion, lane rebinding, Hard unlocks, and the two offsets. They persist
through `PlayerPrefs`.

### What makes it fun

* **Hook.** Tidewalk opens with eight beats of beat cues, then quarter notes on
  the downbeat. On Normal, the combo reaches 10 at about 12 seconds, which
  turns on x2 with a lane flash. The first Fever is ready at about 25 seconds.
* **Mastery curve.** A new player passes Easy with a C. A skilled player holds
  x4 for whole sections, reads Hard chords, and keeps the mean offset within
  10 ms. The early and late counts tell the player which way to correct.
* **Risk and reward.** Saving Fever for the dense chorus pays more, but a Miss
  ends it.
* **Variety and escalation.** Each song builds from verse to chorus with rising
  density. Copper Sky's bridge switches to off-beat patterns.
* **Game feel.**
  * The lane key glows on every press.
  * Hit sparks match the judgment: gold for Perfect, blue for Great, white for
    Good, 12 to 24 particles each.
  * The judgment text pops from 120% to 100% over 0.1 s.
  * The judgment line pulses on every beat.
  * A shockwave plays on every 50-combo milestone.
  * The camera kicks on chorus signals.
  * An optional hit tick sounds on each press.
* **Replay.** Records per chart (best score, grade, best combo), the Full Combo,
  All Perfect, and Metronome badges, and Hard unlocks.

## Content

### Assets

| Asset | Source format | Import settings that matter |
|---|---|---|
| `Boot.unity`, `Title.unity`, `SongSelect.unity`, `Play.unity`, `Calibrate.unity` | Unity YAML scene | prefab instances, references, Volume |
| Song audio | OGG Vorbis 44.1 kHz stereo | Streaming load type, Vorbis quality 70, Preload Audio Data off |
| Clicks, hit tick, UI sounds | WAV 44.1 kHz mono | Decompress On Load, Preload Audio Data on |
| `RhythmMixer.mixer` | Unity YAML audio mixer | groups Music, Ticks, UI; snapshots Play and Paused |
| Background videos | VP8 WebM, 1280 by 720, 30 frames per second, no audio, under `StreamingAssets/Video/` | URL source on every target |
| Static backgrounds | PNG, sRGB | fallback when video is off or fails |
| Charts | `.json` text assets | text equal to file content, version field |
| `Tidewalk.playable`, `CopperSky.playable` | Timeline assets | signal tracks at section changes, animation tracks for lane colors |
| Note, lane, and spark prefabs | Unity YAML prefabs | particle bursts per judgment |
| `RhythmControls.inputactions` | Input action asset | two control schemes, press interactions |
| Fonts | TrueType with TextMeshPro SDF assets | SDF atlas, rich-text tags for combo display |
| `ProjectSettings/` | Unity YAML | DSP buffer size Best Latency, sample rate 48 kHz, Web target template |

### Packages

URP 14.x, Timeline 1.x, Input System 1.x, uGUI 1.0.x, and TextMeshPro 3.0.x, at
the version lines CMP-002 lists. Packages are resolved from their registry when
the project opens and are never vendored in the sample repository.

### Save data

Settings and offsets in `PlayerPrefs`; `highscores.json` under
`Application.persistentDataPath`, with a format version.

### Licences and provenance

Every asset family's source, author, and licence is recorded in
`Licenses/content-licenses.json`. Unity packages' licence terms are recorded in
the build's licence report.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001 | Open in place | S1 |
| PRJ-003 | Stable identities | S1 |
| PRJ-004 | Streamed Vorbis songs, decompressed WAV ticks, per-target load types | S1, S14 |
| PRJ-007 | Audio and Web target settings | S1 |
| SER-001 | Serialized chart and offset values | S1, S2 |
| SER-002 | Scenes and prefabs | S1 |
| RUN-003 | Lifecycle and focus messages | S11 |
| RUN-004 | `AudioSettings.dspTime`, unscaled time | S2, S11 |
| RUN-006 | Video files read from `StreamingAssets` | S9 |
| RUN-009 | `JsonUtility` charts and high scores under `Application.persistentDataPath` | S2, S15 |
| AUD-001 | `PlayScheduled`, DSP time, gapless resume, mixer snapshots, latency report | S2, S7, S11 |
| INP-002 | Timestamped presses, input offset, rebinding, two control schemes | S3, S8 |
| MED-001 | `VideoPlayer` from a URL on external time, sync, error fallback | S9 |
| CIN-001 | Timeline signals and lane-color tracks on the DSP clock | S10 |
| REN-003 | Bloom on Fever | S4 |
| REN-004 | Hit-spark particle bursts | S10 |
| REN-007 | Note travel at 30 to 240 frames per second, Web target pacing | S6, S12 |
| UI-001 | Menus, results | S2, S4 |
| TXT-001 | TextMeshPro score, combo, and judgment text | S2, S4 |
| PLT-001 | Web target audio unlock, tab hiding, storage; desktop players | S13, S16 |
| SAV-001 | Settings and offsets in `PlayerPrefs`, high scores with corruption handling | S15 |
| ACC-001 | Assist Timing, beat cues without sound, high contrast, keyboard only | S16 |
| BLD-001 | Desktop and Web target players with build reports | S16 |
| BLD-002 | Headless runs | S1–S12, S14, S15 |
| TST-002 | Timestamped input replay, audio and visual captures | all |
| AUT-002 | Play control through the editor's MCP interface | S1–S12, S14, S15 |

## Engine Behavior This Game Depends On

Only engine and package behavior, each tied to a catalog ID. Game rules are
defined under Game Design and implemented by the sample's own scripts; the
engine contains no code specific to this game. Package behavior (Timeline,
Input System) comes from the package's own code at the pinned version line,
compiled and run unmodified (CMP-002). Agents reach the game only through the
editor's generic MCP object, property, method-invocation, input, and capture
commands, with no game-specific command.

### Scripting and runtime

* `AudioSettings.dspTime` advances with output samples, independent of frame
  rate and `Time.timeScale`. (RUN-004, AUD-001)
* `OnApplicationFocus` and `OnApplicationPause` arrive on focus loss and on a
  hidden Web target tab within one frame. (RUN-003, PLT-001)
* `JsonUtility.FromJson` parses with culture-invariant numbers; a malformed
  file raises the exception with its parse position. (RUN-009)

### Project and content

* Clips imported as Streaming Vorbis stream during play and are never fully
  decoded in memory, as residency shows. On the Web target, the build report
  states each clip's effective load type and compression; a setting the target
  does not honor is reported as Platform-limited. (PRJ-004, AUD-001)
* Files under `StreamingAssets` copy unprocessed into each player and resolve
  through `Application.streamingAssetsPath`. (RUN-006)

### Simulation

No requirements beyond the domain documents.

### Rendering and animation

* Frame rate caps through `targetFrameRate` and `vSyncCount` pace frames as
  REN-007 defines; on the Web target `vSyncCount` is ignored. (REN-007)
* Bloom intensity set from script and particle bursts render as REN-003 and
  REN-004 define. (REN-003, REN-004)

### Audio and video

* `PlayScheduled(start)` starts a clip at the output sample for `start`;
  captured audio shows the first sample within one buffer of the scheduled
  time. Two schedulings 0.5 s apart play exactly 0.5 s apart in captured audio.
  (AUD-001)
* Scheduling a clip to resume at a given sample leaves no gap longer than one
  DSP buffer and no repeated samples beyond one buffer. (AUD-001)
* Mixer snapshot transitions reach their levels in their stated time.
  `AudioSettings` reports the DSP buffer size and sample rate, and the latency
  diagnostics report output latency where the target reports it. (AUD-001)
* A `VideoPlayer` on External time shows the frame for `externalReferenceTime`
  within one video frame (33 ms), and within that bound 0.2 s after pause,
  resume, and seek. A missing or undecodable file raises `errorReceived` with
  the source and reason. (MED-001)
* A `PlayableDirector` in DSP clock mode fires each signal once, within one
  frame of its time; pausing and resuming fires no signal twice. (CIN-001)

### Input

* Each input event carries its timestamp, so two presses in one frame are
  distinguishable by time. On the Web target, keyboard and gamepad events carry
  timestamps on the same timebase. (INP-002)
* `PerformInteractiveRebinding` with excluded controls behaves as INP-002
  defines, and overrides persist. (INP-002)

### UI and text

* TextMeshPro text with a `<size>` tag updates on the frame it is set, and
  renders from its SDF atlas at every resolution from 1280 by 720 to 3840 by
  2160, matching the approved text checkpoints. (TXT-001)

### Player and build

* The Web target player reports loading progress and whether audio is
  permitted; while audio is suspended `dspTime` does not advance. (PLT-001,
  AUD-001)
* Its persistent storage keeps files across page reloads, and a quota failure is
  reported to the game. (PLT-001, SAV-001)

### Editor and automation

* Through the editor's generic MCP commands an agent injects timestamped
  presses, reads any component field (song time, judgments, combo, Fever, video
  time), and captures audio and frames. (AUT-002)

## Presentation and Performance

### Visual checkpoints

Each has a unitas-approved reference: the title, song select, play at
Tidewalk's first chorus, Fever, the 50-combo shockwave, the results screen, the
calibration screen, and the static fallback background.

### Audio checkpoints

Song start within one buffer of the scheduled time, calibration clicks 0.5 s
apart, a gapless resume, and the Paused snapshot.

### Project-declared expectations

On the reference desktop class at 1080p: 144 frames per second or more with
video. On the declared reference browser class at 60 frames per second: no
frame over 33 ms during a song. At any frame rate, a note drawn at song time T
sits within 1 pixel of its position at T at 60 frames per second, no frame
moves a note backward, and the drawn song time stays within 5 ms of the audio
clock. Expectations are evaluated only on the declared class where one is
named; other runs record the measurement and report the expectation as
unevaluable (TST-002). These are the sample's own expectations, not the PRF-002
engine budgets.

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
* **Expected:** assets import without errors. Songs report Streaming Vorbis, and
  reopening yields identical identities.
* **Demonstrates:** PRJ-001, PRJ-003, PRJ-004, PRJ-007, SER-001, SER-002

### S2. Happy path

* **Runs in:** editor play mode; headless editor; desktop packaged players; Web
  target player.
* **Preconditions:** offsets 0.
* **Steps:** play Tidewalk Normal with synthetic presses exactly on every note.
* **Expected:** 340 Perfects, the maximum score, grade S, and the Full Combo and
  All Perfect badges. The record is saved, and captured audio starts within one
  buffer of the scheduled time.
* **Demonstrates:** AUD-001, RUN-004, UI-001, TXT-001, RUN-009

### S3. Timing windows

* **Runs in:** headless editor; desktop packaged players at 30 and 240 frames
  per second; Web target player at 30 frames per second and at the display
  refresh.
* **Preconditions:** offsets 0.
* **Steps:** press notes at offsets of ±32, ±34, ±65, ±67, ±99, and ±101 ms,
  plus one press 150 ms early.
* **Expected:** Perfect, Great, Great, Good, Good, and Miss for both signs. The
  early press judges nothing, and results are equal at every frame rate.
* **Demonstrates:** INP-002, AUD-001

### S4. Combo and Fever

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** offsets 0.
* **Steps:** play Tidewalk Normal with recorded input that misses note 45, fills
  the meter, starts Fever before the chorus, and misses once during it.
* **Expected:** the multiplier steps at 10, 30, and 50 and resets at the miss.
  Fever starts on the next downbeat, doubles scores, raises bloom to 1.0, and
  ends at the miss. The score equals the recorded value.
* **Demonstrates:** UI-001, TXT-001, REN-003

### S5. Charts, grades, and unlocks

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** a fresh profile.
* **Steps:** play Copper Sky Normal with recorded input that scores 91%, then
  Hard.
* **Expected:** grade A, Hard unlocks and persists, and the Hard chart shows 690
  notes with chords judged per lane.
* **Demonstrates:** SAV-001, UI-001

### S6. Frame-rate independence

* **Runs in:** headless editor; desktop packaged players at 30, 60, 144, and 240
  frames per second; Web target player at 30 and 60 frames per second.
* **Preconditions:** offsets 0.
* **Steps:** repeat S2, capturing note positions at fixed song times.
* **Expected:** each note is within 1 pixel of the 60 frames-per-second
  position, no note moves backward, and the score is identical.
* **Demonstrates:** REN-007, RUN-004

### S7. Audio calibration

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** synthetic taps 80 ms after each click.
* **Steps:** calibrate, then repeat S2 with presses 80 ms late. Then calibrate
  with taps whose interquartile range is 60 ms.
* **Expected:** the offset reads 80 ± 2 ms, and every note is Perfect. The
  uneven taps show "Taps too uneven".
* **Demonstrates:** AUD-001, INP-002, SAV-001

### S8. Input calibration

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** synthetic taps 45 ms after each flash.
* **Steps:** calibrate the input offset, then repeat S2 with presses 45 ms late.
* **Expected:** the input offset reads 45 ± 2 ms, and S2 scores all Perfect.
* **Demonstrates:** INP-002

### S9. Video sync and failure

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** background set to Video.
* **Steps:** play Tidewalk with video, pause at 40 s, resume, then remove the
  WebM file (a harness action on the player's content folder) and replay.
* **Expected:** the shown frame stays within 33 ms of song time, and within that
  bound 0.2 s after resume. The missing file raises `errorReceived` naming it,
  the static background shows, and judgments are unaffected.
* **Demonstrates:** MED-001, RUN-006

### S10. Signals and beat effects

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** none.
* **Steps:** play Copper Sky and record signal times, then pause and resume
  across a signal.
* **Expected:** each section signal fires once within one frame of its song
  time, lane colors change on it, the line pulses on every beat, hit sparks emit
  12 to 24 particles each, and no signal fires twice.
* **Demonstrates:** CIN-001, REN-004

### S11. Pause, focus, and resume

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** Tidewalk Normal in play.
* **Steps:** pause at 30 s for 10 s; then at 60 s remove and restore window
  focus, through a focus event in the replay stream (headless and editor) or a
  harness action that changes OS window or tab focus (packaged players).
* **Expected:** each pause stops audio and notes. Resume runs the 3-beat
  countdown, and the song continues from the paused sample with no gap or
  repeat longer than one buffer. Later judgments stay aligned.
* **Demonstrates:** AUD-001, RUN-003, RUN-004

### S12. Frame stall

* **Runs in:** editor play mode; headless editor; packaged players.
* **Preconditions:** the saved state "Copper Sky Hard at the start of the
  bridge".
* **Steps:** stall one frame for 250 ms during a dense section (a
  development-build debug command that blocks the main thread for that frame)
  while synthetic presses continue on time.
* **Expected:** the music continues, notes jump to their correct positions on
  the next frame, presses during the stall are judged by their timestamps, and
  the score matches the unstalled run.
* **Demonstrates:** REN-007, INP-002, AUD-001

### S13. Web target audio unlock

* **Runs in:** Web target player only, under the browser test runner.
* **Preconditions:** a fresh page load.
* **Steps:** load the page and wait 10 s, then press a key; then hide the tab (a
  harness action in the browser test runner).
* **Expected:** the start prompt shows, the game reports audio not permitted and
  schedules nothing, and `dspTime` does not drive song time. After the key,
  audio is permitted, and S2 passes. Hiding the tab pauses the song.
* **Demonstrates:** PLT-001, AUD-001

### S14. Streamed audio

* **Runs in:** headless editor; packaged players.
* **Preconditions:** none.
* **Steps:** play both songs back to back with residency inspection.
* **Expected:** song clips stream and are never fully decoded in memory, ticks
  are resident PCM, and the Web target build report states each clip's
  effective load type.
* **Demonstrates:** PRJ-004, AUD-001

### S15. High scores

* **Runs in:** packaged players; headless editor with a play-session restart.
* **Preconditions:** a fresh profile.
* **Steps:** set records on two charts, restart the player (a harness action
  that restarts the player process or reloads the page, or, headless, a new
  play session with the same storage root), then truncate `highscores.json` (a
  harness action on the storage root) and restart again.
* **Expected:** records persist. The damaged file is kept as
  `highscores.json.corrupt`, the game shows "Scores could not be read", and a
  new record writes a valid file.
* **Demonstrates:** SAV-001, RUN-009, PLT-001

### S16. Accessibility and package

* **Runs in:** headless editor (build), desktop packaged players, and the Web
  target player.
* **Preconditions:** none.
* **Steps:** build desktop and Web target players. With Assist Timing, beat
  cues, and music muted, play Tidewalk Easy on keyboard only with presses at
  ±90 ms.
* **Expected:** presses judge as Great, because the windows widen to ±50, ±99,
  and ±150 ms. The record is flagged Assist, and beat cues pulse on every beat
  without sound. Each build report lists scenes, sizes, and the build
  identifier.
* **Demonstrates:** ACC-001, BLD-001, PLT-001

## Manual Playtest Guidance

A person judges the following, and a headless run reports these checkpoints as
awaiting human review, never as passed:

* Whether presses feel locked to the music after calibration.
* Whether the first 30 seconds of Tidewalk pull a new player into the beat.
* Whether Fever timing feels like a real decision.
* Whether Hard charts feel hard but fair, with no unreadable patterns.
* Whether hit feedback and the video background add energy without hiding
  notes.
* Whether a near-miss of a grade makes the player want to retry.

## Agent Authoring Change Requests

### CR1. Flick notes

* **Request:** add a third note type, the Flick, which needs a press and release
  within 80 ms, appears in Copper Sky Hard, and has its own spark and sound.
* **Acceptance scenario:** in Copper Sky Hard with synthetic input, every flick
  released 70 ms after its press is a hit and every flick released at 90 ms is
  a Miss; all other judgments are unchanged.
* **Test:** a test that judges a flick released at 70 ms as a hit and at 90 ms
  as a Miss.

### CR2. Practice mode

* **Request:** add a practice mode that loops a chosen 8-bar section at 75%
  speed.
* **Acceptance scenario:** looping Copper Sky's bridge at 75% speed with
  synthetic presses on every note judges every note Perfect on each loop.
* **Test:** a test that checks the loop boundaries and the scaled note times.

## Accessibility

* Every action works with the keyboard alone, and lanes can be rebound (S16).
* Assist Timing widens every window by half and marks records Assist (S16).
* Beat cues show a pulsing ring on the judgment line and a 1-2-3-4 beat bar, so
  the game can be played without sound (S16).
* Judgments differ by text and shape, not by color alone (S2).
* High-contrast notes and note speed help readability (S6 with high-contrast
  notes on).
* Reduced motion turns off the camera kick, shockwave, and video (S10 with
  reduced motion on).
* Hard unlocks can be granted in Settings (S5).

## Saved and Reproducible States

| State | How it is reached | Used by |
|---|---|---|
| Tidewalk Normal at 1:10 with a 49 combo and a full Fever meter | development-build start argument | S4 |
| Copper Sky Hard at the start of the bridge | development-build start argument | S12 |
| Calibration complete with audio offset 80 ms and input offset 45 ms | save file | S7, S8 |
| Records file with an A on Copper Sky Normal and Hard unlocked | save file | S5 |

## Known Limitations and Unsupported Conditions

* Output latency comes from the target's own report. Where a target reports
  none, the calibration offset alone corrects it.
* The Web target player has no Quit action, and its frame pacing follows
  `targetFrameRate` within the display refresh.

Any behavior that this document requires and that is not demonstrated is
reported with its real compatibility state.

## Acceptance Outcomes

* The rhythm game plays both songs on every chart in editor, desktop players,
  and the Web target player, starting each song within one audio buffer of its
  scheduled DSP time. (SMP-001, AUD-001, RUN-004, PLT-001)
* Presses are judged by their timestamps against the audio clock, so the same
  input gets the same judgments at every tested frame rate and through a 250 ms
  stall. (SMP-001, INP-002, REN-007)
* Audio and input calibration measure synthetic offsets within 2 ms and
  re-center judging. (SMP-001, AUD-001, INP-002)
* Combo multipliers, Fever, grades, badges, and Hard unlocks follow their
  stated rules and persist. (SMP-001, UI-001, TXT-001, SAV-001)
* The video background stays within one frame of the music and falls back to
  a static image on error. (SMP-001, MED-001)
* Timeline signals fire once per crossing on the DSP clock and drive
  beat-synchronized effects. (SMP-001, CIN-001)
* Songs stream as compressed audio, and the Web target build reports each
  clip's effective load type. (SMP-001, PRJ-004, AUD-001)
* The Web target player waits for the first gesture to start audio, pauses when
  hidden, and keeps high scores across reloads. (SMP-001, PLT-001, SAV-001)
* Assist Timing widens every window and marks the record, and beat cues let
  the game be played without sound. (SMP-001, ACC-001)
* Each desktop and Web target player is packaged with a build report and
  passes the replayed happy path. (SMP-001, BLD-001)
