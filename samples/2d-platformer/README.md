# Wick's Lantern (2D Platformer Sample)

`samples/2d-platformer` is the Unity `2022.3.62f1` project skeleton for *Wick's Lantern* (`docs/product-specification/sample-games/02-2d-platformer.md`). It exercises 2D project settings, orthographic camera assembly, 2D sprite importing, 2D physics simulation (`Rigidbody2D`, `CapsuleCollider2D`, `BoxCollider2D`, `CircleCollider2D`, `CompositeCollider2D`, `TilemapCollider2D`, `PlatformEffector2D`, `SurfaceEffector2D`), and `PlayerController2D` gameplay scripting in the Unitas runtime and headless editor host.

## Demonstrated in this project

- **Unity 2022.3 LTS Project Layout**:
  - `ProjectSettings/ProjectVersion.txt` pinned to `2022.3.62f1`.
  - `Packages/manifest.json` declaring the 2D package set (`com.unity.2d.sprite`, `com.unity.2d.tilemap`, `com.unity.2d.tilemap.extras`, `com.unity.2d.animation`, `com.unity.2d.pixel-perfect`, `com.unity.2d.psdimporter`, `com.unity.render-pipelines.universal`, `com.unity.cinemachine`, `com.unity.inputsystem`, `com.unity.ugui`, `com.unity.textmeshpro`, and 2D engine modules), all resolving to `Supported` in the compatibility matrix.
  - `ProjectSettings/Physics2DSettings.asset` (`gravity: [0, -9.81]`, `queriesHitTriggers: 1`), `TimeManager.asset` (`fixedTimestep: 0.02`), `TagManager.asset` (tags `MainCamera`, `Player`, `Ember`, `Checkpoint`, `Hazard`, `GoalBell`; 2D layers `Ground`, `Player`, `OneWay`, `Hazard`, `Pickup`, `Checkpoint`, `Beetle`; 7 sorting layers `Background`, `Parallax`, `Tiles`, `Props`, `Characters`, `Foreground`, `Effects`), and `EditorBuildSettings.asset` (`Assets/Scenes/Rootway.unity`).
- **Assets and Scene (`Assets/Scenes/Rootway.unity`)**:
  - Pixel-art PNG sprites under `Assets/Sprites/` (`Wick.png`, `GroundTiles.png`, `Ember.png`, `OneWayPlatform.png`) configured with `textureType: 8` (Sprite), `filterMode: 0` (Point), `wrapMode: 1` (Clamp), `mipmapEnabled: 0`, and `spritePixelsToUnits: 16`.
  - `Main Camera` with orthographic projection (`orthographic size: 5.625` for 180 vertical pixels at 16 PPU).
  - `Wick` player entity with dynamic `Rigidbody2D` (`m_GravityScale: 3`, `m_Interpolate: 1`, `m_CollisionDetection: 1`, `m_Constraints: 4` frozen Z rotation), vertical `CapsuleCollider2D` (`0.75 x 1.375`), `SpriteRenderer`, and `PlayerController2D` `MonoBehaviour`.
  - `Ground` with static `Rigidbody2D`, `CompositeCollider2D`, `BoxCollider2D` (`m_UsedByComposite: 1`), `Tilemap`, `TilemapCollider2D` (`m_UsedByComposite: 1`), and `SpriteRenderer`.
  - `OneWayBridge` on layer `OneWay` with `BoxCollider2D` (`m_UsedByEffector: 1`) and `PlatformEffector2D` (`m_UseOneWay: 1`, `m_SurfaceArc: 200`).
  - `ConveyorLedge` with `BoxCollider2D` (`m_UsedByEffector: 1`) and `SurfaceEffector2D` (`m_Speed: 3`).
  - `Ember_1` and `Ember_2` trigger pickups (`CircleCollider2D`), `Checkpoint_1` trigger (`BoxCollider2D`), `ThornHazard` trigger (`BoxCollider2D`), and `GoalBell` trigger (`BoxCollider2D`).
- **Gameplay Script (`Assets/Scripts/PlayerController2D.cs`)**:
  - Horizontal acceleration to 6 units/s, variable jump height (3.5 units apex, 40% early-release velocity cut), 0.1 s coyote time, 0.1 s jump buffer, 18 units/s fall speed cap, `Physics2D.BoxCast` ground check with `ContactFilter2D` normal filtering (`45..135` degrees), ember collection with 3 s chain multiplier (`x1` to `x4`), checkpoint lighting, hazard heart loss with simulated-off respawn pause, and goal bell completion.
- **Playtest Scenario (`Scenarios/movement_smoke.json`)**:
  - Deterministic headless scenario verifying scene load, ground settling, horizontal run, ember pickup, jump onto the one-way bridge, checkpoint activation, and final transform position.
