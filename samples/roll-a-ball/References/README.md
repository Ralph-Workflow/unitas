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
