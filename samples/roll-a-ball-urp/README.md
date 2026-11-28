# Roll-a-Ball — URP variant ("Marble Rush URP")

The URP 14.x (Forward) twin of the Built-in sample at `samples/roll-a-ball`,
per the "Targets and Input Modes" section of
`docs/product-specification/sample-games/01-roll-a-ball.md`: the same game,
scenes, scripts, and input modes on the Universal Render Pipeline instead of
the Built-in Render Pipeline.

## What differs from the Built-in variant

- `Packages/manifest.json` adds `com.unity.render-pipelines.universal`
  `14.0.11` (URP 14.x, the Unity 2022.3 line).
- `Assets/Settings/RollABallURP.asset` is the
  `UniversalRenderPipelineAsset` (Forward renderer, HDR, MSAA 2x, SRP
  batcher); `Assets/Settings/RollABallRenderer.asset` is its
  `UniversalRendererData` (`m_RenderingMode: 0`, Forward).
- `Assets/Materials/*.mat` target the URP Lit shader
  (`933532a4fcc9baf4fa0491de14d08ed7`) with URP property names
  (`_BaseMap`, `_BaseColor`, `_Smoothness`); emission on `Pickup` and
  `BoostPad` keeps `_EMISSION`. Material GUIDs are unchanged, so the scene
  files are byte-identical to the Built-in variant.
- `ProjectSettings/QualitySettings.asset` points every quality level
  (Low/Medium/High) at the URP asset via `customRenderPipeline`.
- `ProjectSettings/GraphicsSettings.asset` sets `m_CustomRenderPipeline`
  to the URP asset (as a real 2022.3 URP project does).
- `ProjectSettings/ProjectSettings.asset` uses `productName: RollABallURP`
  and `com.unitas.rollaballurp`.

## Deliberate divergences (this phase)

- **Music source not mirrored.** The Built-in variant's new music source
  (per the specification's per-arena streaming music) is not copied here
  this phase; the URP variant keeps only the chimes
  (`BounceChime.ogg`, `CollectChime.wav`, `PickupChime.wav`). Add the
  music track when the Built-in source lands.
- **Captures pending.** `References/*.bmp` S8 checkpoints are produced by a
  later unit; the scenario files and this directory's README are already
  in place.
- **Generated state not mirrored.** `Library/` (compiled script assemblies)
  is regenerated on open, not copied.

Everything else — scenes, prefabs, scripts, textures, audio chimes,
scenarios, project settings — mirrors `samples/roll-a-ball` unchanged.
