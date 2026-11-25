# Sample 9: Rhythm Game

## Overview

Notes slide down four lanes toward a judgment line, and the player presses
each lane's key on the beat. Accurate presses build a combo, fill a Fever
meter, and raise the score. The game has two original songs, each with Easy,
Normal, and Hard charts, behind a music-synchronized video background. A song
lasts about two minutes, and a session of several charts takes 10 to 20
minutes.

This is an original, redistributable unitas sample. Its songs, charts, video,
art, and sounds are made for unitas and are distinct from any official Unity
sample or commercial rhythm game. It is the suite's proof of sample-accurate
audio timing, input judged against the audio clock, and the same game on a
browser target and on desktop players.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns the acceptance of this game under SMP-001.

## Targets and Input Modes

* Targets: macOS, Windows, and Linux desktop players, a browser
  (WebGL-class) player, plus the editor's play mode and the headless editor.
* Render pipeline: URP 14.x, Forward rendering path.
* Input modes: keyboard and gamepad, through an Input System 1.x action asset
  with Keyboard and Gamepad control schemes, in Dynamic update mode. Menus also
  accept the pointer.
* On the browser target, audio starts only after the first user gesture, and
  `vSyncCount` is ignored, so `targetFrameRate` paces frames. Gamepad
  checkpoints need a physical gamepad. Without one they are not evaluable,
  never passed. Headless runs use synthetic devices with timestamped events,
  marked synthetic.

## Game Design

### Core loop

1. On the browser, the first screen reads "Click or press any key to start"
   until audio is permitted. The title screen then shows Play, Calibrate,
   Settings, and Quit (Quit is absent on the browser).
2. Song select lists Tidewalk (112 BPM, 2:04) and Copper Sky (148 BPM, 1:52),
   each with Easy, Normal, and Hard, the best score, grade, and badges.
3. The song is scheduled to start 1.5 seconds after selection. Notes appear
   at the top of the lanes and reach the judgment line exactly on their beat.
4. Pressing a lane's key judges the nearest unjudged note in that lane. A note
   not pressed within 100 ms after its time is a Miss. Holds also need the key
   held until their end, and each held beat adds 50 points.
5. After the last note, the results screen shows score, grade, judgment
   counts, best combo, mean timing offset in milliseconds, and badges.
6. Results offer Retry, Song Select, and Quit to Title. Retry reschedules the
   song from the start with every transient value reset.

### Controls

| Action | Keyboard | Gamepad | Mouse |
|---|---|---|---|
| Lanes 1 to 4 | D, F, J, K | D-pad left, D-pad down, South, East (or LT, LB, RB, RT) | — |
| Fever | Space | North button | — |
| Pause | Escape or P | Start | — |
| Menus: navigate, confirm, back | Arrows, Enter, Escape | D-pad, South, East | Pointer, left click |
| Calibration tap | Space | South button | Left click |

### Judgment and scoring

* Each press is judged at its input event time placed on the audio clock,
  minus the input offset, compared with the note time plus the audio offset.
  The frame time of the press does not matter.
* Windows: Perfect within ±33 ms (300 points), Great within ±66 ms (200),
  Good within ±100 ms (100), otherwise Miss. A press more than 100 ms before
  every note in its lane judges nothing.
* The combo counts consecutive non-Miss judgments. The multiplier is x1 below
  10, x2 from 10, x3 from 30, and x4 from 50.
* Grades: S at 95% of the chart's maximum score or more, A at 90%, B at 80%, C
  at 70%, and D below. Full Combo means no Miss, and All Perfect means only
  Perfect.

### Charts

Charts are JSON text assets with version, BPM, start offset in milliseconds,
and notes as beat, lane, and hold length in beats. Note counts:

| Song | Easy | Normal | Hard |
|---|---|---|---|
| Tidewalk | 180 | 340 | 520 |
| Copper Sky | 220 | 410 | 690 |

Easy uses quarter notes, Normal adds eighths and holds, and Hard adds
sixteenth-note streams and two-lane chords.

### What makes it fun

* **Hook.** Tidewalk opens with eight beats of beat cues, then quarter notes on
  the downbeat. On Normal, the combo reaches 10 at about 12 seconds, which
  turns on x2 with a lane flash. The first Fever is ready at about 25 seconds.
* **Mastery curve.** A new player passes Easy with a C. A skilled player holds
  x4 for whole sections, reads Hard chords, and keeps the mean offset within
  10 ms. The results screen shows how many presses were early and how many
  late, so the player knows which way to correct.
* **Risk and reward.** Perfect adds 2% to the Fever meter and Great 1%. At 100%,
  Fever can start on the next downbeat and lasts 16 beats. During Fever, scores
  double and the lanes turn gold, and a Miss ends Fever early. Saving Fever
  for the dense chorus pays more, but risks losing it.
* **Variety and escalation.** Each song builds from verse to chorus with rising
  density. Copper Sky's bridge switches to off-beat patterns. Hard charts
  unlock after an A on that song's Normal.
* **Juice.**
  * The lane key glows on every press.
  * Hit sparks match the judgment: gold for Perfect, blue for Great, white for
    Good.
  * The judgment text pops from 120% to 100% over 0.1 s.
  * The judgment line pulses on every beat.
  * A shockwave plays on every 50-combo milestone.
  * The camera kicks on chorus signals.
  * An optional hit tick sounds on each press.
* **Replay reasons.** Records per chart: best score, grade, best combo, and
  Full Combo and All Perfect badges. Hard unlocks, and a 0 ms mean offset
  earns the Metronome badge.

### Calibration

* Audio offset: 16 clicks at 120 BPM are scheduled with `PlayScheduled`. The
  player taps on clicks 5 to 16. The median tap offset becomes the audio
  offset, accepted from -250 to +250 ms. When the interquartile range of the
  taps exceeds 40 ms, the screen shows "Taps too uneven, try again".
* Input offset: a marker flashes 16 times at 120 BPM with no sound, and the
  player taps on each flash. The median becomes the input offset, from -250 to
  +250 ms.

### Pause and settings

Pause stops the song and video, freezes notes, and moves the mixer to the
Paused snapshot. Resume runs a 3-beat visual countdown, then resumes the song
from the paused sample. No note is judged during the countdown. Losing focus,
or a hidden browser tab, pauses the game. Settings are master, music, and
effects volume, hit tick on or off, note speed (1.0 to 6.0), background
(Video, Static, Off), frame-rate cap (30, 60, 144, 240, uncapped), vertical
sync on desktop, Assist Timing, beat cues, high-contrast notes, reduced
motion, lane rebinding, and the two offsets. They persist through
`PlayerPrefs`.

### Failure and edge rules

* There is no fail state mid-song. A run with zero non-Miss judgments still
  reaches the results screen with grade D.
* A stalled frame never shifts the music or the judgments.
* A video that cannot play falls back to the static background, and play
  continues.

## Content

| Asset | Source format | What matters on import |
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
| `ProjectSettings/` | Unity YAML | DSP buffer size Best Latency, sample rate 48 kHz, browser template |

Packages: URP 14.x, Timeline 1.x, Input System 1.x, uGUI 1.0.x, and
TextMeshPro 3.0.x.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001, PRJ-003, PRJ-007 | Open in place, stable identities, audio and browser settings | S1 |
| PRJ-004 | Streamed Vorbis songs, decompressed WAV ticks, per-target load types | S1, S14 |
| SER-001, SER-002 | Scenes, prefabs, serialized chart and offset values | S1, S2 |
| RUN-003, RUN-004 | Lifecycle and focus messages, `AudioSettings.dspTime`, unscaled time | S2, S11 |
| RUN-006 | Video files read from `StreamingAssets` | S9 |
| RUN-009 | `JsonUtility` charts and high scores under `Application.persistentDataPath` | S2, S15 |
| AUD-001 | `PlayScheduled`, DSP time, gapless resume, mixer snapshots, latency report | S2, S7, S11 |
| INP-002 | Timestamped presses, input offset, rebinding, two control schemes | S3, S8 |
| MED-001 | `VideoPlayer` from a URL on external time, sync, error fallback | S9 |
| CIN-001 | Timeline signals and lane-color tracks on the DSP clock | S10 |
| REN-003, REN-004 | Bloom on Fever, hit-spark particle bursts | S4, S10 |
| REN-007 | Note travel at 30 to 240 frames per second, browser pacing | S6, S12 |
| UI-001, TXT-001 | Menus, results, TextMeshPro score, combo, and judgment text | S2, S4 |
| PLT-001 | Browser audio unlock, tab hiding, storage; desktop players | S13, S16 |
| SAV-001 | Settings and offsets in `PlayerPrefs`, high scores with corruption handling | S15 |
| ACC-001 | Assist Timing, beat cues without sound, high contrast, keyboard only | S16 |
| BLD-001 | Desktop and browser players with build reports | S16 |
| BLD-002, TST-002, AUT-002 | Headless runs, timestamped input replay, audio and visual captures | all |

## Engine Behavior This Game Depends On

### Scripting and runtime

* `AudioSettings.dspTime` advances with output samples, independent of frame
  rate and `Time.timeScale`. Song time is derived from it.
* `OnApplicationFocus` and `OnApplicationPause` arrive on focus loss and on a
  hidden browser tab, and the game pauses within one frame.
* `JsonUtility.FromJson` parses charts and high scores with culture-invariant
  numbers. A malformed file raises the exception with its parse position.

### Project and content

* Song clips import as Streaming Vorbis. During play, residency shows them
  streaming, never fully decoded in memory. On the browser target, the build
  report states each clip's effective load type and compression. A setting the
  target does not honor is reported as Platform-limited.
* The WebM files copy unprocessed into each player and resolve through
  `Application.streamingAssetsPath`.

### Audio

* `PlayScheduled(start)` starts the song at the output sample for `start`.
  Captured audio shows the first sample within one buffer of the scheduled
  time.
* Resume schedules the song at the paused sample. Captured audio has no gap
  longer than one DSP buffer and no repeated samples beyond one buffer.
* The calibration clicks play exactly 0.5 s apart in captured audio.
* The Paused snapshot lowers Music to -80 dB in 0.1 s. `AudioSettings`
  reports the DSP buffer size and sample rate, and the latency diagnostics
  report output latency where the target reports it.

### Input

* Each press carries its input event timestamp, so two presses in one frame
  are judged at their own times.
* In the browser player, keyboard and gamepad events arrive with timestamps on
  the same timebase.
* Rebinding a lane through `PerformInteractiveRebinding` excludes the other
  lanes' keys, and the overrides persist.

### Video and cinematics

* The `VideoPlayer` plays the song's WebM file from its URL in Camera Far
  Plane mode with audio output None. It uses External time, with
  `externalReferenceTime` set to song time each frame.
* The shown frame stays within one video frame (33 ms) of song time. Pause,
  resume, and seek keep that bound within 0.2 s.
* A missing or undecodable file raises `errorReceived` with the source and
  reason. The game shows the static background.
* Each song's `PlayableDirector` runs in DSP clock mode from the song's start.
  Every signal fires once, within one frame of its song time. Pausing and
  resuming fires no signal twice.

### Rendering

* Each note's position is a function of song time and note speed, not of
  accumulated frame time. At any frame rate, a note drawn at song time T sits
  within 1 pixel of its position at T at 60 frames per second.
* Between audio clock updates, note motion stays smooth: no frame moves a note
  backward, and the song time drawn stays within 5 ms of the audio clock.
* Fever raises bloom intensity from 0.3 to 1.0. Hit sparks emit 12 to 24
  particles per judgment.

### UI and text

* TextMeshPro score, combo (with a `<size>` tag on the multiplier), and
  judgment text update on the frame of each judgment, crisp from 1280 by 720
  to 3840 by 2160.

### Player, build, and automation

* The browser player reports loading progress and whether audio is
  permitted. It never schedules a song while audio is suspended.
* Its persistent storage keeps high scores across page reloads, and a quota
  failure is reported to the game.
* Agents inject timestamped presses, read song time, judgments, combo, Fever,
  and video time, and capture audio and frames through MCP.

## Presentation and Performance

* Visual checkpoints, each with a unitas-approved reference: the title, song
  select, play at Tidewalk's first chorus, Fever, the 50-combo shockwave, the
  results screen, the calibration screen, and the static fallback background.
* Audio checkpoints: song start within one buffer of the scheduled time,
  calibration clicks 0.5 s apart, a gapless resume, and the Paused snapshot.
* Project-declared expectations on the reference desktop class at 1080p: 144
  frames per second or more with video. On the browser at 60 frames per
  second, no frame over 33 ms during a song. These are the sample's own
  expectations, not the PRF-002 engine budgets.

## Acceptance Scenarios

Every scenario runs in editor play mode, in a packaged player, and headless
through MCP, unless it says otherwise. "Player" includes the desktop and
browser players.

* **S1. Open and inspect.** Precondition: no cache. Open the project.
  Expected: assets import without errors. Songs report Streaming Vorbis, and
  reopening yields identical identities. (PRJ-001, PRJ-003, PRJ-004, PRJ-007,
  SER-001, SER-002)
* **S2. Happy path.** Tidewalk Normal, offsets 0, synthetic presses exactly on
  every note. Expected: 340 Perfects, the maximum score, grade S, and the Full
  Combo and All Perfect badges. The record is saved, and captured audio starts
  within one buffer of the scheduled time. (AUD-001, RUN-004, UI-001, TXT-001,
  RUN-009)
* **S3. Timing windows.** Press notes at offsets of ±32, ±34, ±65, ±67, ±99,
  and ±101 ms, plus one press 150 ms early. Run at 30 and 240 frames per
  second. Expected: Perfect, Great, Great, Good, Good, and Miss for both
  signs. The early press judges nothing, and results are equal at both frame
  rates. (INP-002, AUD-001)
* **S4. Combo and Fever.** Tidewalk Normal with recorded input that misses
  note 45, fills the meter, starts Fever before the chorus, and misses once
  during it. Expected: the multiplier steps at 10, 30, and 50 and resets at
  the miss. Fever starts on the next downbeat, doubles scores, and ends at the
  miss. The score equals the recorded value. (UI-001, TXT-001, REN-003)
* **S5. Charts, grades, and unlocks.** Play Copper Sky Normal with recorded
  input that scores 91%, then Hard. Expected: grade A, Hard unlocks and
  persists, and the Hard chart shows 690 notes with chords judged per lane.
  (SAV-001, UI-001)
* **S6. Frame-rate independence.** Repeat S2 at 30, 60, 144, and 240 frames
  per second, capturing note positions at fixed song times. Expected: each
  note is within 1 pixel of the 60 frames-per-second position, no note moves
  backward, and the score is identical. (REN-007, RUN-004)
* **S7. Audio calibration.** Precondition: synthetic taps 80 ms after each
  click. Calibrate, then repeat S2 with presses 80 ms late. Expected: the
  offset reads 80 ± 2 ms, and every note is Perfect. Taps with an interquartile
  range of 60 ms show "Taps too uneven". (AUD-001, INP-002, SAV-001)
* **S8. Input calibration.** Synthetic taps 45 ms after each flash. Expected:
  the input offset reads 45 ± 2 ms, and S2 with presses 45 ms late scores all
  Perfect. (INP-002)
* **S9. Video sync and failure.** Play Tidewalk with video, pause at 40 s,
  resume, then delete the WebM file and replay. Expected: the shown frame stays
  within 33 ms of song time, and within that bound 0.2 s after resume. The
  missing file raises `errorReceived` naming it, the static background shows,
  and judgments are unaffected. (MED-001, RUN-006)
* **S10. Signals and beat effects.** Play Copper Sky and record signal times,
  then pause and resume across a signal. Expected: each section signal fires
  once within one frame of its song time, lane colors change on it, the line
  pulses on every beat, and no signal fires twice. (CIN-001, REN-004)
* **S11. Pause, focus, and resume.** Pause at 30 s for 10 s, then remove window
  focus at 60 s and restore it. Expected: each pause stops audio and notes.
  Resume runs the 3-beat countdown, and the song continues from the paused
  sample with no gap or repeat longer than one buffer. Later judgments stay
  aligned. (AUD-001, RUN-003, RUN-004)
* **S12. Frame stall.** Stall one frame for 250 ms during a dense section
  while synthetic presses continue on time. Expected: the music continues,
  notes jump to their correct positions on the next frame, presses during the
  stall are judged by their timestamps, and the score matches the unstalled
  run. (REN-007, INP-002, AUD-001)
* **S13. Browser audio unlock.** Browser player only. Load the page and wait
  10 s, then press a key. Expected: the start prompt shows, the game reports
  audio not permitted and schedules nothing, and `dspTime` does not drive song
  time. After the key, audio is permitted, and S2 passes. Hiding the tab
  pauses the song. (PLT-001, AUD-001)
* **S14. Streamed audio.** Play both songs back to back with residency
  inspection. Expected: song clips stream and are never fully decoded in
  memory, ticks are resident PCM, and the browser build report states each
  clip's effective load type. (PRJ-004, AUD-001)
* **S15. High scores.** Set records on two charts, restart the player (reload
  the page on the browser), then corrupt `highscores.json`. Expected: records
  persist. The corrupt file is kept as `highscores.json.corrupt`, the game
  shows "Scores could not be read", and a new record writes a valid file.
  (SAV-001, RUN-009, PLT-001)
* **S16. Accessibility and package.** Build desktop and browser players.
  With Assist Timing, beat cues, and music muted, play Tidewalk Easy on
  keyboard only with presses at ±90 ms. Expected: presses judge as Great,
  because the windows widen to ±50, ±99, and ±150 ms. The record is flagged
  Assist, and beat cues pulse on every beat without sound. Each build report
  lists scenes, sizes, and the build identifier. (ACC-001, BLD-001, PLT-001)

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

Add a third note type, the Flick, which needs a press and release within 80 ms,
appears in Copper Sky Hard, and has its own spark and sound. Include a test
that judges a flick released at 70 ms as a hit and at 90 ms as a Miss.

A second change request adds a practice mode that loops a chosen 8-bar
section at 75% speed. Its scenario proves that judgments stay aligned at that
speed.

## Accessibility

Every action works with the keyboard alone, and lanes can be rebound. Assist
Timing widens every window by half and marks records Assist. Beat cues show a
pulsing ring on the judgment line and a 1-2-3-4 beat bar, so the game can be
played without sound. Judgments differ by text and shape, not by color alone.
High-contrast notes and note speed help readability. Reduced motion turns off
the camera kick, shockwave, and video. Hard unlocks can be granted in Settings.

## Saved and Reproducible States

* Tidewalk Normal at 1:10 with a 49 combo and a full Fever meter.
* Copper Sky Hard at the start of the bridge.
* Calibration complete with audio offset 80 ms and input offset 45 ms.
* Records file with an A on Copper Sky Normal and Hard unlocked.

## Known Limitations and Unsupported Conditions

* Output latency comes from the target's own report. Where a target reports
  none, the calibration offset alone corrects it.
* The browser player has no Quit action, and its frame pacing follows
  `targetFrameRate` within the display refresh.

Any behavior that this document requires and that is not demonstrated is
reported with its real compatibility state.

## Acceptance Outcomes

* The rhythm game plays both songs on every chart in editor, desktop players,
  and the browser player, starting each song within one audio buffer of its
  scheduled DSP time. (SMP-001, AUD-001, RUN-004, PLT-001)
* Presses are judged by their timestamps against the audio clock, so the same
  input gets the same judgments at 30 to 240 frames per second and through a
  250 ms stall. (SMP-001, INP-002, REN-007)
* Audio and input calibration measure synthetic offsets within 2 ms and
  re-center judging. (SMP-001, AUD-001, INP-002)
* Combo multipliers, Fever, grades, badges, and Hard unlocks follow their
  stated rules and persist. (SMP-001, UI-001, TXT-001, SAV-001)
* The video background stays within one frame of the music and falls back to
  a static image on error. (SMP-001, MED-001)
* Timeline signals fire once per crossing on the DSP clock and drive
  beat-synchronized effects. (SMP-001, CIN-001)
* Songs stream as compressed audio, and the browser build reports each clip's
  effective load type. (SMP-001, PRJ-004, AUD-001)
* The browser player waits for the first gesture to start audio, pauses when
  hidden, and keeps high scores across reloads. (SMP-001, PLT-001, SAV-001)
* Assist Timing widens every window and marks the record, and beat cues let
  the game be played without sound. (SMP-001, ACC-001)
* Each desktop and browser player is packaged with a build report and passes
  the replayed happy path. (SMP-001, BLD-001)
