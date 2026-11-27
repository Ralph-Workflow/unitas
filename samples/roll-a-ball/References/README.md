# Approved visual references

These BMPs are the approved visual references for the roll-a-ball sample
game's S8 checkpoints. Each scenario names its reference with the `reference`
key and the allowed fraction of differing pixels with `tolerance`; the samples
lane compares every fresh capture against the file and fails the step when the
difference exceeds the tolerance.

| Reference | Scenario | Checkpoint | Approved tolerance |
|---|---|---|---|
| `s8_title_checkpoint.bmp` | `s8_title_checkpoint.scenario.json` | `title-screen` | 0.005 |
| `s8_arena_countdown.bmp` | `s8_arena_countdown.scenario.json` | `arena-at-countdown` | 0.005 |
| `s8_arena_x5_checkpoint.bmp` | `s8_arena_x5_checkpoint.scenario.json` | `arena-at-x5` | 0.005 |
| `s8_results_gold_checkpoint.bmp` | `s8_results_gold_checkpoint.scenario.json` | `results-gold-medal` | 0.005 |

## How the references were produced

Each file is the capture the unitas `wgpu` render graph produced offscreen for
that scenario's `render_capture` checkpoint (256x256, `Main Camera`, the
background colour the scenario declares), taken from the scenario run itself on
macOS Apple Silicon — not a Unity screenshot and not a software rasterizer.

The captures are reproducible. The native renderer used to order its draw list
by `HashMap` iteration order, which is seeded per process, so two runs of the
same scene could disagree on which of two coincident surfaces won the depth
test and the arena materials swapped between runs.
`crates/unitas-runtime/src/runtime.rs` now sorts the renderer handles before it
builds the instance list, and three consecutive runs of every scenario produce
byte-identical BMPs.

## What the 2026-11-26 re-approval changed

The four references were re-approved on 2026-11-26 when headless capture moved
to the `wgpu` render graph (`RenderGraph` in `crates/unitas-render`, executed
offscreen through `GpuFramePresenter::render_offscreen` and composited with the
loaded scene's uGUI canvas via `Runtime::composite_canvas_image` in
`crates/unitas-runtime/src/runtime.rs`): the capture shows the sample's HUD and
its title panel rendered through the product's single `wgpu` render graph.
Against the prior CPU-rasterizer references, the four render-graph captures
differed by 3328/65536 pixels (5.08 %) for `s8_title_checkpoint` and
`s8_arena_countdown`, 3475/65536 pixels (5.30 %) for `s8_arena_x5_checkpoint`,
and 3314/65536 pixels (5.06 %) for `s8_results_gold_checkpoint`; the approved
files are the new render-graph captures, taken from the scenario runs on this
host.

The four references were re-approved again on 2026-11-26 because the engine now
renders a real-time directional shadow map from the project's quality level
(`ProjectSettings/QualitySettings.asset`, `Standalone` level `High`:
`shadows: 2`, `shadowResolution: 2`, `shadowDistance: 50`). Against the prior
unshadowed `wgpu` render-graph references, the four directional-shadow captures
differed by 5970/65536 pixels (9.11 %) for `s8_title_checkpoint` and
`s8_arena_countdown`, 6236/65536 pixels (9.52 %) for `s8_arena_x5_checkpoint`,
and 5974/65536 pixels (9.12 %) for `s8_results_gold_checkpoint`. Visual
inspection of all four regenerated captures confirms that the arena casters'
shadows (cast by the arena walls, the pickups, and the player ball under the
scene's directional light) visibly darken the arena floor compared to the lit
floor pixels.

The re-approval keeps the declared tolerance of 0.005 unchanged. It widens
nothing: the approved files are the exact captures the lane produces today, and
the lane still fails when a capture differs from them by more than 0.5 % of its
pixels.

One property of the references changed with it. The canvas text is shaped and
drawn through the system font stack (`unitas_render::glyphs`), so the
letterforms depend on the fonts of the machine that produced the capture. Two
runs on one machine are byte-identical, but a machine with a different font set
draws different glyphs and its capture no longer matches the approved file
within 0.005. The arena pixels stay reproducible everywhere; the HUD text does
not. A reference-free approval, an embedded font, or a per-platform approved
reference is what removes that dependence.

## What the 2026-11-27 re-approval changed

The four references were re-approved on 2026-11-27 when the scene path started
carrying the **authored material** to the draw call. Before this, the engine
drew every renderer with its flat `_Color` only, so the sample's own textures
(`Pickup.png`, `PickupNormal.png`) were silently dropped on the way from the
`.mat` file to the pixel. The asset crate now parses `_BumpMap`,
`_EmissionMap`, `_Metallic`, `_Glossiness` and `_EmissionColor`, the scene
assembly resolves those texture guids to paths, the runtime decodes them into
the renderer's material maps (keeping the meshes' UVs and tangents), and the
presenter draws through the render crate's material path with cached mesh
buffers and cached material bind groups. The same re-approval picked up the
sample's new authored content (`Marble.mat` with a normal map, `Floor.mat`
with a tiled albedo, and `Pickup.mat`'s emission map and emission colour) and
the **engine-bundled** Noto Sans face the HUD text is rasterized from, so the
letterforms no longer depend on the fonts of the machine that produced the
capture — the machine dependence the 2026-11-26 re-approval gave up is closed.

Three more references joined the set: `s8_quality_low.bmp`,
`s8_quality_medium.bmp` and `s8_quality_high.bmp`, the captures of
`Scenarios/s8_quality_sweep.scenario.json` at the three quality levels the
sample's `QualitySettings.asset` declares. The Low capture is the arena with
the shadows off, the High capture the arena with the directional shadow map,
so the set now shows the quality level changing the picture.

## What the 2026-11-27 quality-sweep re-approval changed

The three quality-sweep references were re-approved on 2026-11-27 because the
headless capture path did not apply the quality level the scenario asked for:
`Runtime::render_frame` took the shadow settings from the runtime's own quality
level, so the `set_quality` step changed nothing in the picture and the three
approved files were byte identical to each other and to
`s8_title_checkpoint.bmp`. `Runtime::render_frame` now applies the pending
display quality level to the runtime before it takes the shadow settings
(`unitas_runtime::presenter::apply_pending_display_quality`), so the Low
capture is the arena with the shadows off, the Medium capture the arena with
the hard shadow map, and the High capture the arena with the soft shadow map.

`s8_quality_low.bmp` and `s8_quality_medium.bmp` are the new captures and
differ from the files they replace by 5682/65536 pixels (8.67 %) and
5611/65536 pixels (8.56 %). `s8_quality_high.bmp` is unchanged (0/65536
differing pixels), because the project's `Standalone` default level is `High`
and the High capture is the arena the earlier references already show. Three
consecutive runs of the scenario produced byte identical captures at all three
levels (sha256 `547a9330…`, `509c92d1…`, `50c6440d…`).

The re-approval keeps the declared tolerance of 0.005 unchanged. It widens
nothing: the approved files are the exact captures the lane produces today,
and the lane still fails when a capture differs from them by more than 0.5 %
of its pixels.

## Why the tolerance is 0.005

`tolerance` is the fraction of pixels allowed to differ. The references are
byte-identical on the machine that produced them (0 differing pixels), so the
tolerance only absorbs minor GPU driver differences between processors. It is
bounded from both sides by measurement: the two captures of different
checkpoints differ by 502 of 65 536 pixels (0.77 %), so a tolerance above 0.5 %
would admit a changed scene, and the old per-process draw order swapped the
arena materials over 3.1 %-31.9 % of the pixels, so a tolerance below 0.5 %
keeps every material regression visible.

## Approval

Approved on 2026-11-25 by the development session that removed the draw-order
nondeterminism, against the presentation requirements of
`docs/product-specification/sample-games/01-roll-a-ball.md` (S8, TST-002).
Each reference is bound to exactly one scenario and one checkpoint.

Re-approved on 2026-11-26 by the development session that unified headless
capture on the `wgpu` render graph with the uGUI canvas composited into the
rendered frame, against the same requirements: the four files are the captures
the lane produces now on the `wgpu` render graph, with the HUD and the title
panel in the picture. The tolerance is unchanged.

Re-approved on 2026-11-26 by the development session that wired real-time
directional shadow-map rendering from the project's quality level
(`ProjectSettings/QualitySettings.asset`, `Standalone` level `High`:
`shadows: 2`, `shadowResolution: 2`, `shadowDistance: 50`), against the same
requirements: the four files are the directional-shadow captures the lane
produces now on the `wgpu` render graph, visually inspected to confirm the
arena casters' shadows darken the arena floor in all four captures (`5970/65536`
(`9.11 %`) for `s8_title_checkpoint` and `s8_arena_countdown`, `6236/65536`
(`9.52 %`) for `s8_arena_x5_checkpoint`, and `5974/65536` (`9.12 %`) for
`s8_results_gold_checkpoint` against the prior unshadowed references). The
tolerance `0.005` is unchanged.

Re-approved on 2026-11-27 by the development session that wired the scene's
authored materials into the render path and made the HUD text
machine-independent, against the same requirements: the four files are the
material captures the lane produces now on the `wgpu` render graph, with the
textured and normal-mapped pickups, the normal-mapped marble, the tiled floor
and the bundled-font HUD in the picture, and three new files
(`s8_quality_low.bmp`, `s8_quality_medium.bmp`, `s8_quality_high.bmp`) join
the set for the quality sweep. The tolerance `0.005` is unchanged, and the
approved files are byte-identical across three consecutive runs of every
scenario on the machine that produced them.

Re-approved on 2026-11-27 by the development session that made the headless
capture path apply the quality level the scenario asked for, against the same
requirements: `s8_quality_low.bmp` and `s8_quality_medium.bmp` are the
captures the lane produces now at the Low and Medium levels of the sample's
`QualitySettings.asset` (shadows off and the hard shadow map), and
`s8_quality_high.bmp` is unchanged because the project's default level is
`High`. The tolerance `0.005` is unchanged.
