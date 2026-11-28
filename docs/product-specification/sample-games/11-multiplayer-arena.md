# Sample 11: Multiplayer Arena ("Core Rush")

## Overview

Core Rush is a team arena game for two to eight players. Two teams of hover
drones fight over glowing energy cores in a compact arena. A player grabs a
core, carries or passes it across the arena, and drops it on the team's
capture pad, while opponents stun the carrier with a short-range pulse. A match
is best of three rounds of 3 minutes, so it takes 8 to 11 minutes.

Players host a session, join one by address or from the local network list, or
join a dedicated server running on Linux. The game uses Netcode for GameObjects
within the version line that CMP-002 in the
[Compatibility contract](../compatibility-contract.md) lists, with Unity
Transport as its transport dependency. It uses no proprietary online service:
there are no accounts, relays, matchmaking, or cloud storage.

This is an original, redistributable unitas sample. Its rules, arena, drones,
art, and audio are original unitas content. It is one of the demonstrations of
networked play, dedicated servers, and multi-instance testing; the acceptance
of those features is owned by the domain specifications it cites.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns this game's acceptance under SMP-001.

## Targets and Input Modes

### Targets

* **Automated:** macOS, Windows, and Linux desktop players, each able to host
  or join; a Linux x86-64 dedicated server player.
* **Capability-reported:** gamepad rumble, which depends on the gamepad's
  motors.
* **Editor play mode:** yes.
* **Headless editor:** yes, including several isolated play sessions in one
  agent session.

### Render pipelines

URP 14.x with the Forward rendering path. The dedicated server renders
nothing.

### Input modes and devices

Keyboard and mouse, and gamepad, through Input System 1.x actions (INP-002).

Headless runs use synthetic devices marked synthetic; haptic and feel
checkpoints need physical hardware and are otherwise unevaluable.

## Game Design

### Core loop and flow

1. The title screen shows Host, Join, Settings, Records, and Quit.
2. Host opens a session with a server name, an optional password, a maximum of
   2 to 8 players, and a "visible on local network" toggle. Join lists sessions
   found on the local network and accepts an address (IPv4, IPv6, or host name)
   and port, default 7777.
3. The lobby shows two team columns. Each row shows the player's name, ready
   state, and round-trip time, with a crown on the host. Players pick a team,
   edit their name (1 to 16 letters, digits, or spaces), and toggle Ready. The
   host's Start button is enabled when there are at least two players, at least
   one per team, and everyone is ready. A dedicated server starts a 10-second
   countdown when those conditions hold.
4. Each round opens with a 3-second countdown. A core then drops from the
   ceiling at the center. Drones fight to carry cores to their own pad.
5. A round ends after 3 minutes or when a team reaches 5 points.
6. The results panel shows the winner, per-player stats (deposits, steals,
   parries, passes caught), and medals. Rematch keeps the teams and returns to
   the lobby. Leave returns to the title screen.
7. Quit on the title exits the player.

### Rules and mechanics

* **Movement.** Drones hover at 8 m/s and face the aim direction.
* **Dash.** A burst to 14 m/s for 0.25 s. Cooldown 2 s.
* **Pulse.** After a visible 0.3 s wind-up, the pulse stuns every enemy drone
  within 4 m for 1.5 s. Cooldown 4 s. A stunned carrier drops its core.
* **Parry.** Pulsing within 0.2 s before an enemy pulse lands reflects the stun
  back onto the attacker.
* **Slam.** A pulse fired within 0.3 s after a dash ends reaches 5 m instead of
  4 m.
* **Carrying.** A drone picks up a free core by touching it. A carrier moves 20
  percent slower and cannot dash.
* **Pass.** The carrier throws the core in a 12 m arc. A teammate who touches it
  in flight catches it. An opponent who touches it intercepts it.
* **Overcharge.** A carrier that stays inside the center ring for 3 s charges the
  core. A charged core is worth 2 points and shows on every player's radar.
* **Cores.** A new core spawns 8 s after each deposit, with at most two in play.
  In the last 30 s of a round, a Surge drops two extra cores.
* **Server authority.** Only the server moves drones and cores. Each network
  tick, a client sends an unreliable RPC that carries its last three input
  frames, each with a sequence number, so a single lost packet loses no input.
  Pulse, dash, and throw presses travel inside these frames. Pulse hits come
  from `Physics.OverlapSphereNonAlloc` on the server; pad and ring triggers run
  on the server only. A request whose sequence number the server has already
  handled has no effect and is logged as ignored. A throw from a drone that is
  no longer carrying is rejected as stale, and the thrower alone receives
  "Throw rejected: core no longer carried."
* **Server invariants.** A team's score equals the points of the deposits
  credited to it; a core has at most one carrier; a deposit identifier scores
  once.
* **Logging.** Every line the game writes starts with the participant tag
  `[<role> c<clientId> "<name>"]`.
* **Discovery.** Local discovery uses UDP broadcast on port 47778 within the
  local subnet. Hosts and servers that are visible on the local network answer
  within 1 second. The game opens no network connection to any other
  destination.

### Levels and progression

Round 1 is always Standard. Rounds 2 and 3 each draw a different modifier from
the server's seeded random generator. Every participant sees the modifier
banner at the same countdown.

| Modifier | Effect |
|---|---|
| Low Gravity | Thrown cores fly 18 m, and pulses knock drones back 3 m |
| Overcharge | Every core starts charged |
| Blackout | Arena lights drop to 10 percent; drones are visible by their trails and carriers by their glow |
| Magnet Pads | Pads pull a carrier within 6 m toward them at 3 m/s |

End-of-match medals: MVP (most points), Interceptor (most steals), Wall (most
parries), and Playmaker (most passes caught). Drone trails unlock at 5, 15, and
30 matches played.

### Scoring, failure, and edge rules

* The team with more points wins the round. A round tied above 0 to 0 goes to
  the team that scored last. A round tied at 0 to 0 is replayed with the same
  modifier. The first team to win two rounds wins the match; round 3, when it
  is played, is the deciding round.
* Connection approval checks the payload's game version, password, and player
  token before any player object spawns.
* Late joining is allowed until 30 seconds before the deciding round ends. A late
  joiner goes to the team with fewer players.
* A player who disconnects keeps a reserved slot for 60 seconds. Reconnecting
  with the same player token within that time restores the team, name, and
  personal stats. After 60 seconds the slot is freed.
* When the host leaves, the session ends for every client.
* Play beyond the local network connects directly to an address the client can
  reach. The game has no relay or NAT traversal service.
* Each connection outcome has its own message and log category:

  | Outcome | Message |
  |---|---|
  | Connected | "Joined <server name>" |
  | Refused | "The host refused the connection: <reason>" |
  | Unreachable | "No server answered at <address>:<port>" (after 10 s) |
  | Full | "Session full (<n> of <n>)" |
  | Authentication failure | "Wrong password" |
  | Version mismatch | "Version mismatch: server <version>, you <version>" |
  | Timeout | "Connection lost: no data for 10 seconds" |
  | Host departure | "The host left the session" or "Server closed" |
  | Deliberate disconnection | "You left the session" |

* **Simulated network conditions.** Development builds offer four profiles,
  selected in Settings or with the `-netsim` argument. Each profile sets Unity
  Transport's network simulator to the delay, jitter, packet loss, and packet
  duplication below. Jitter larger than the send interval also reorders
  packets. A participant under any profile other than LAN shows a "SIMULATED
  NETWORK: <profile>" banner, and its log, scenario results, and playtest
  records mark the condition.

  | Profile | Delay | Jitter | Loss | Duplication |
  |---|---|---|---|---|
  | LAN | 0 ms | 0 ms | 0 percent | 0 percent |
  | Broadband | 60 ms | 10 ms | 1 percent | 0.5 percent |
  | Poor | 150 ms | 40 ms | 5 percent | 2 percent |
  | Severe | 300 ms | 80 ms | 15 percent | 5 percent |

### Controls

| Action | Keyboard and mouse | Gamepad |
|---|---|---|
| Move | WASD | Left stick |
| Aim | Pointer | Right stick |
| Pulse | Left click | Right trigger |
| Dash | Space or right click | Left trigger |
| Throw core | E | West button |
| Scoreboard | Tab (hold) | Select (hold) |
| Menu | Escape | Start |
| Navigate menus | Arrow keys, Tab | D-pad, left stick |
| Confirm, back | Enter, Escape | South button, East button |

### Pause, focus, and lifecycle

A networked match cannot pause. The menu overlays the arena while the match
continues, and offers Settings, Leave, and, for the host, End Match. Losing
window focus releases all held input, and the drone stops. The dedicated server
is listening within 3 seconds of launch; on SIGTERM it sends "Server closed" to
every client and exits with status 0 within 2 seconds; when the port is in use,
it logs the port and exits with a nonzero status.

### Settings

* master, music, effects, and announcer volume
* announcer captions on or off
* fullscreen or windowed mode, resolution, vertical sync, and quality level
* camera shake and hit-stop on or off
* team colors: Standard or Color-blind safe
* player name
* in development builds only, a simulated network profile

Settings, the player name, records, unlocks, and a player token (a random
identifier created on first launch) persist through `PlayerPrefs`.

### What makes it fun

* **Hook.** Five seconds after round 1 starts, a core drops into a light beam at
  the center with a 3-second countdown. Every drone races for it, and the first
  pulse fight starts within 15 seconds of the match.
* **Mastery curve.** New players chase the core and pulse at once. Skilled
  players parry, open with a dash into a slam, pass ahead of a teammate, and
  overcharge only when the center is clear. The results panel shows parries,
  slam hits, and passes caught, so improvement is visible.
* **Risk and reward.** Overcharging doubles a core's value but reveals the
  carrier on every radar. Passing escapes a stun but risks an interception. A
  pulse leaves the attacker without its stun for 4 s, so a missed pulse invites
  a counterattack.
* **Variety and escalation.** Round modifiers change tactics each round. The
  Surge doubles the cores in the final 30 seconds.
* **Game feel.**
  * The local drone shows its own dash and pulse effects on the frame of input.
    The server's outcome confirms them.
  * A pulse hit triggers a 60 ms hit-stop on the hit drones' visuals and a
    camera shake.
  * The game plays a 150 ms rumble on gamepads that expose motors.
  * Stunned drones spin with spark particles.
  * The dash leaves a trail.
  * A deposit fires a burst of team-colored particles and plays an announcer
    stinger ("Blue scores!").
  * The score pops on change, and the kill feed lists steals and parries.
* **Replay.** Medals, local records of best deposits in a match, best parries,
  and total wins, trail unlocks stored locally, and Rematch for a quick series.

## Content

### Assets

| Asset | Source format | Import settings that matter |
|---|---|---|
| `Title.unity`, `Lobby.unity`, `Arena.unity` | Unity YAML scene | object identity; `Lobby` and `Arena` are network scenes |
| `NetworkManager.prefab` | Unity YAML prefab | `NetworkManager` with tick rate 30, connection approval on, and scene management on; `UnityTransport` with port 7777 and 10 s timeouts |
| `Drone.prefab` with Red and Blue variants | prefab and prefab variants | `NetworkObject`, `NetworkTransform` (server authority, interpolation), team material override |
| `Core.prefab`, `CapturePad.prefab`, `CenterRing.prefab` | Unity YAML prefab | `NetworkObject`, `Rigidbody`, sphere and box triggers |
| `NetworkPrefabs.asset` | network prefab list | references to every spawnable prefab |
| Drone, core, and arena models | FBX | mesh, material slots, scale factor |
| Textures | PNG, sRGB, mipmaps; normal maps | sRGB, normal-map mode, compression |
| Effects | Particle System prefabs for sparks, deposit bursts, trails, and stun spin | bursts, color over lifetime |
| Sounds and announcer lines | WAV | decompress-on-load, 3D settings for arena sounds |
| Music | OGG, streaming | streaming load type, looping |
| Fonts | TrueType font with a TextMeshPro SDF font asset | SDF atlas, fallback font |
| `ArenaRules.asset` | ScriptableObject | round length, target score, pulse, dash, and core values |
| `NetworkProfiles.asset` | ScriptableObject | the four simulated network profiles |
| `server.json` | JSON beside the server executable or given by `-config` | server name, port, maximum players, password, round length, local visibility |
| `ProjectSettings/` | Unity YAML | `Application.version` "1.4.0", Dedicated Server target settings, layers, fixed time step, quality levels |

### Packages

Netcode for GameObjects 1.x (with Unity Transport as its transport dependency),
URP 14.x, Input System 1.x, uGUI 1.0.x, TextMeshPro 3.0.x, and Unity Test
Framework 1.x, at the version lines CMP-002 lists. Each package resolves to
unitas's own implementation of that version line
([Packages](../architecture.md#packages)); no Unity-published package code is
vendored in the sample repository, fetched, or compiled.

### Save data

Matches are not saved. Settings, the player name, records, unlocks, and the
player token are in `PlayerPrefs`.

### Licences and provenance

Every asset family's source, author, and licence is recorded in
`Licenses/content-licenses.json`. Unity packages' licence terms are recorded in
the build's licence report.

## Feature Coverage

| ID | Behavior this game exercises | Scenarios |
|---|---|---|
| PRJ-001 | The project opens in place with no conversion | S1 |
| PRJ-007 | Dedicated Server target settings, version, layers, time step | S1, S3 |
| SER-002 | Network scenes loaded by the server and followed by clients | S2, S4 |
| SER-003 | Team prefab variants of the networked drone | S1, S2 |
| RUN-003 | `OnNetworkSpawn`, `OnNetworkDespawn`, ownership callbacks, `OnApplicationFocus` | S4, S5 |
| RUN-004 | Round timers, countdowns, cooldowns | S2, S15 |
| RUN-009 | `JsonUtility` for `server.json` and approval payloads, seeded `UnityEngine.Random` for modifiers, log and file locations | S3, S11, S16 |
| PHY-001 | Core rigidbody throws, pad and ring triggers, pulse overlap queries | S5, S15 |
| NET-001 | Host, client, and server modes; approval; `NetworkVariable` permissions; RPCs; ownership transfer; late join; reconnect; timeouts; version mismatch; simulated conditions | S2–S16 |
| INP-002 | Input System actions for keyboard, mouse, and gamepad; gamepad rumble | S2, S15 |
| UI-001 | Title, host and join panels, lobby, scoreboard, results | S2, S11, S14 |
| TXT-001 | TextMeshPro names, messages, and results | S2, S11 |
| AUD-001 | Announcer stingers, 3D arena sounds, music | S15, S16 |
| REN-004 | Spark, burst, trail, and stun effects | S15 |
| SAV-001 | Player token, settings, records, and unlocks in `PlayerPrefs` | S9, S16 |
| PLT-001 | Linux dedicated server lifecycle and Platform-limited graphics calls | S3, S10 |
| BLD-001 | Desktop players and the Linux dedicated server player with build reports | S1 |
| BLD-002 | Headless builds and headless runs | S1, S3, S13 |
| TST-002 | Multi-participant scenarios with rule, synchronization, and transport checkpoints | all |
| DIA-001 | Participant-attributed logs and network diagnostics | S6, S12, S13 |
| PRF-001 | Network traffic and server tick profiling | S7 |
| AUT-002 | Play control, input injection, and evidence per play session | S13 |
| AUT-004 | Several isolated headless play sessions in one agent session | S13 |
| ACC-001 | Color-blind team colors with shapes, captions, shake and hit-stop toggles | S16 |

## Engine Behavior This Game Depends On

Only engine and package behavior, each tied to a catalog ID. Game rules are
defined under Game Design and implemented by the sample's own scripts; the
engine contains no code specific to this game. Package behavior (Netcode for
GameObjects, Unity Transport, Input System) comes from unitas's own
implementation of each package's public API at the pinned version line
([Packages](../architecture.md#packages), CMP-002); the items below state that
package behavior as observed and the engine surface it calls. Agents reach the
game only through the editor's generic MCP object, property, method-invocation,
input, capture, and play-session commands, with no game-specific command.

### Scripting and runtime

* `NetworkManager.StartHost`, `StartClient`, and `StartServer` start the three
  modes. A second start call returns `false` and is reported. `Shutdown`
  disconnects every client and despawns every object. (NET-001)
* The connection approval callback receives the client's payload and refuses a
  client with the reason the callback supplies. No player object spawns before
  approval. (NET-001)
* `NetworkVariable` writes outside their write permission raise an exception and
  send nothing. Owner-read variables never reach other clients. `OnValueChanged`
  runs on each receiving participant with the previous and new values. (NET-001)
* `[ServerRpc(RequireOwnership = true)]` calls from a non-owner are rejected and
  reported, naming the RPC and the caller. Reliable RPCs on one object arrive in
  send order. `ClientRpcParams` delivers to the named clients only. (NET-001)
* `ChangeOwnership` and `RemoveOwnership` deliver `OnLostOwnership` and
  `OnGainedOwnership` to the right participants. (NET-001, RUN-003)
* `JsonUtility` reports a malformed document with its parse position.
  (RUN-009)

### Project and content

* Every prefab in a network prefab list spawns with the same `NetworkObjectId`
  on every participant, and prefab variants apply their overrides on every
  participant. (NET-001, SER-003)

### Simulation

* `Physics.OverlapSphereNonAlloc` and triggers behave identically on a
  dedicated server, which has no rendering. (PHY-001)
* `NetworkTransform` with server authority interpolates on clients. (NET-001)

### Rendering and animation

* Clients render as REN-004 defines. The dedicated server renders nothing, and a
  graphics operation that needs a GPU on it is reported as Platform-limited
  once, naming the API. (PLT-001)

### Audio and video

* A one-shot started from script plays within one frame. 3D sources pan toward
  their positions. The dedicated server produces no audio output. (AUD-001)

### Input

* Keyboard, mouse, and gamepad bindings produce the same actions. Rumble returns
  an explicit status on gamepads without motors. (INP-002)

### UI and text

* Menus and lobby controls work with keyboard, gamepad, and pointer, with the
  selected element's visual state always shown. (UI-001, TXT-001)

### Player and build

* The Linux dedicated server player behaves as if launched with `-batchmode
  -nographics`, defines `UNITY_SERVER`, and applies server-build asset
  stripping. It passes `-port`, `-config`, `-maxPlayers`, `-password`, `-netsim`
  (development builds only), and `-logFile` to the game through the command
  line. (BLD-001, PLT-001)
* Each player writes its log to the target's `Player.log` location, or to the
  `-logFile` path. SIGTERM is delivered to the game as an application quit.
  (PLT-001)
* `UnityTransport` connects to the configured address and port; its disconnect
  timeout produces a timeout disconnect reason. A late-joining client loads the
  network scenes, then receives every spawned object with its current
  `NetworkVariable` values and owner. (NET-001)
* Unity Transport's network simulator applies the configured delay, jitter,
  loss, and duplication to every packet the participant sends and receives. It
  is present only in development builds. (NET-001)

### Editor and automation

* One agent session starts several isolated play sessions in the headless
  editor and connects them to each other and to a development server player
  over the loopback interface. Each play session keeps its own identity, input,
  time, storage root, and logs. (AUT-004, AUT-002)
* Diagnostics attribute each log entry, exception, and network event to its
  play session or player connection. (DIA-001)

### Networking

* No requirements beyond those above and the domain documents.

## Presentation and Performance

### Visual checkpoints

Each has a unitas-approved reference:

* the lobby with eight players and ready states
* the round-start core drop
* a deposit burst
* the Blackout modifier
* the results panel with medals
* the simulated-network banner

### Audio checkpoints

The deposit stinger starts within one frame of the client receiving the score
change. Arena sounds pan toward their sources.

### Project-declared expectations

* Clients run at 60 frames per second or more at 1080p on the declared
  reference desktop class.
* The server holds 30 ticks per second with eight clients, using no more than
  5 ms per tick, on the declared reference server class.
* Each client receives at most 48 kbit/s with eight players.
* Server memory stays under 400 MB.
* On the LAN profile, joining a lobby takes under 2 seconds, and a late join
  becomes playable in under 3 seconds.
* A server state change is observed on every client within 0.2 s (LAN), 0.5 s
  (Broadband), 1.0 s (Poor), and 2.5 s (Severe).

Expectations tied to a declared class are evaluated only on that class; other
runs record the measurement and report the expectation as unevaluable
(TST-002). These are the sample's own expectations, not the PRF-002 engine
budgets.

### PRF-002 measurements

None.

## Acceptance Scenarios

Outcomes use the TST-002 vocabulary: passed, failed, skipped, unevaluable, and
awaiting human review. Each perturbation names its mechanism: a TST-002
replay-stream event, a harness action outside the player, or a
development-build debug command implemented in the sample's scripts. A scenario
with several participants names each one. Its checkpoints are tagged as
**rule** (evaluated on the server's authoritative state), **synchronization**
(a remote observation on a named client within the profile's bound), or
**transport** (a connection outcome or timeout).

### S1. Open and build

* **Runs in:** headless editor.
* **Preconditions:** no import cache exists.
* **Steps:** open the project. From one headless command-line invocation, build
  desktop players for each desktop target and a Linux dedicated server player.
* **Expected:** all assets import with no errors. Netcode for GameObjects is
  reported with its tier and state. Each build report lists scenes, assets,
  sizes, and the build identifier. The server build defines `UNITY_SERVER` and
  reports its stripped assets.
* **Demonstrates:** PRJ-001, PRJ-007, SER-003, BLD-001, BLD-002

### S2. Hosted match

* **Runs in:** desktop packaged players as local instances; isolated headless
  play sessions.
* **Preconditions:** one host and three clients.
* **Steps:** each sets a name, picks a team, and readies. The host starts the
  match. Recorded input plays three rounds.
* **Expected:** Start is enabled only once every condition holds. Every
  participant shows the same scores, rounds, modifiers, and winner at every
  round end (synchronization). The final score matches the credited deposits
  (rule). The results panel lists the same stats on every participant.
* **Demonstrates:** NET-001, SER-002, SER-003, UI-001, TXT-001, INP-002, RUN-004

### S3. Dedicated server match

* **Runs in:** the Linux dedicated server player with desktop packaged clients.
* **Preconditions:** a valid `server.json`.
* **Steps:** launch the server, then connect four clients and play two rounds.
  Invoke the development-build debug command "Render frame" on the server. In a
  second run, launch the server with a malformed `server.json`.
* **Expected:** the server is listening within 3 seconds and opens no window.
  The countdown starts when the lobby conditions hold. The match completes as in
  S2. The debug command reports a Platform-limited graphics call. The malformed
  file makes the server exit nonzero with the file path in its log.
* **Demonstrates:** NET-001, BLD-002, PLT-001, RUN-009, PRJ-007

### S4. Late join

* **Runs in:** desktop packaged players; isolated headless play sessions.
* **Preconditions:** the development state `carried_center`.
* **Steps:** a fourth client joins.
* **Expected:** the joiner loads `Lobby` and `Arena`, then receives every drone
  and core with its current position, score, modifier, timer, and owner, before
  its own drone spawns. It joins the team with fewer players. `OnValueChanged`
  does not fire for values it never saw change.
* **Demonstrates:** NET-001, SER-002, RUN-003

### S5. Ownership and passes

* **Runs in:** desktop packaged players; isolated headless play sessions.
* **Preconditions:** a match in round 1.
* **Steps:** Blue 1 picks up a core, passes it to Blue 2, and Red 1 intercepts
  the next pass. Red 1 is then stunned while carrying. A non-owner sends a
  throw request (a development-build debug command).
* **Expected:** ownership moves Blue 1, server, Blue 2, server, Red 1, server.
  `OnGainedOwnership` and `OnLostOwnership` fire once each on the right
  participants. The non-owner's throw request is rejected, naming the RPC and
  the caller. The stunned carrier drops the core where it was hit.
* **Demonstrates:** NET-001, PHY-001, RUN-003

### S6. Permissions

* **Runs in:** desktop packaged players; isolated headless play sessions.
* **Preconditions:** a match in round 1, development builds.
* **Steps:** from a client, trigger the development-build debug commands "Write
  team score" and "Spawn core." Read another player's private stats.
* **Expected:** the score write raises an exception and sends nothing. The spawn
  raises an exception naming the object. Private stats never reach a non-owner.
  Each failure is logged under the calling participant.
* **Demonstrates:** NET-001, DIA-001

### S7. Latency, loss, and jitter

* **Runs in:** development desktop packaged players; isolated headless play
  sessions.
* **Preconditions:** one host and three clients.
* **Steps:** repeat S2 under the Broadband, Poor, and Severe profiles applied to
  every client. Capture a network profile of the server.
* **Expected:** every rule checkpoint passes under every profile.
  Synchronization checkpoints pass within each profile's bound. Every client
  shows the banner, and every result records the profile. The capture shows
  traffic per client and tick time; the bandwidth expectation is evaluated on
  the declared class.
* **Demonstrates:** NET-001, TST-002, PRF-001

### S8. Duplicates and stale state

* **Runs in:** development desktop packaged players; isolated headless play
  sessions.
* **Preconditions:** the Severe profile on one client.
* **Steps:** that client pulses five times and dashes five times. Then a carrier
  throws the core 50 ms after the server has already stunned it.
* **Expected:** each press arrives in up to three redundant input frames, plus
  any duplicated or reordered packets. Each pulse stuns once and each dash fires
  once. Every repeated or out-of-order sequence number is logged as ignored. The
  late throw is rejected as stale. The thrower alone receives "Throw rejected:
  core no longer carried," and its view reverts within one round trip plus
  100 ms.
* **Demonstrates:** NET-001

### S9. Disconnect and reconnect

* **Runs in:** desktop packaged players.
* **Preconditions:** a match in round 1 with four players.
* **Steps:** kill a client process during a round (a harness action that
  terminates the process). Restart it and reconnect after 30 seconds. Repeat
  with a 70-second gap. Then press Leave on another client.
* **Expected:** the server reports the Timeout outcome for the terminated client
  within 10 seconds, and every other client shows it leaving. The 30-second
  reconnect restores team, name, and stats. The 70-second reconnect joins as a
  new player with zero stats. Leave shows "You left the session" on that client
  only.
* **Demonstrates:** NET-001, SAV-001

### S10. Host departure and server shutdown

* **Runs in:** desktop packaged players; the Linux dedicated server player.
* **Preconditions:** a match in round 1.
* **Steps:** quit the host mid-round. In a second run, send SIGTERM to the
  dedicated server (a harness action).
* **Expected:** every client shows "The host left the session" within 10
  seconds in the first run and "Server closed" within 2 seconds in the second,
  then returns to the title screen. No exception is logged. The server exits
  with status 0.
* **Demonstrates:** NET-001, PLT-001

### S11. Connection outcomes

* **Runs in:** desktop packaged players; isolated headless play sessions.
* **Preconditions:** a server of version 1.4.0 with a password and eight players;
  a client build of version 1.3.0.
* **Steps:** join from the 1.3.0 client, join as a ninth player, join with a
  wrong password, join an address with no server, and join with an approval
  payload that is not valid JSON (a development-build debug command).
* **Expected:** the client sees, in order, the Version mismatch message naming
  both versions, Full, Authentication failure, Unreachable after 10 seconds, and
  Refused with the parse reason. Each outcome has its own log category, and no
  player object spawns for any of them.
* **Demonstrates:** NET-001, UI-001, TXT-001, RUN-009

### S12. Failure classification

* **Runs in:** development desktop packaged players; isolated headless play
  sessions.
* **Preconditions:** one host and three clients.
* **Steps:** run S2 three times: with the development-build debug command
  "Double points" on the server; with all traffic to one client dropped for 15
  seconds (the network simulator set to 100 percent loss on that client); and
  with a 4-second delay applied to one client under the Poor profile bound.
* **Expected:** the first run fails on a rule checkpoint. The second fails on a
  transport checkpoint (Timeout). The third fails on a synchronization checkpoint
  while every rule checkpoint passes. Each result names the first failed
  checkpoint and the participant, keeps later evidence, and shows each
  participant's log lines around the failure under its tag.
* **Demonstrates:** NET-001, TST-002, DIA-001

### S13. Local multi-instance testing through the editor's MCP interface

* **Runs in:** headless editor only.
* **Preconditions:** a development Linux server player build.
* **Steps:** one agent session starts the server player and four isolated play
  sessions, connects them over the loopback interface, injects input into each,
  and stops one client mid-match.
* **Expected:** each play session reports its own identity, time, and logs.
  Input sent to one never reaches another. Stopping one does not stop the
  others, and the server reports that client's departure. Every log entry and
  capture is attributed to its participant.
* **Demonstrates:** AUT-004, AUT-002, DIA-001, NET-001, TST-002

### S14. Local discovery and join by address

* **Runs in:** desktop packaged players on one host, each bound to its own
  loopback alias or network namespace on a shared virtual subnet that carries
  broadcast, IPv4, and IPv6 (a harness action that creates the subnet).
* **Preconditions:** two hosts and one dedicated server on the virtual subnet,
  one host hidden from local discovery; a fourth client instance on the same
  subnet; a host name that resolves to the hidden host on that subnet.
* **Steps:** open Join on the fourth instance. Then join the hidden host by
  IPv4, by host name, and by IPv6 address. Enter an invalid address.
* **Expected:** the two visible sessions appear within 2 seconds with name,
  players, version, and round-trip time. An entry disappears 5 seconds after its
  session ends. Each address form connects. The invalid address is rejected in
  the field before connecting. A packet capture on the virtual subnet shows
  traffic only to the subnet broadcast and the addresses used.
* **Demonstrates:** NET-001, UI-001

### S15. Combat feel

* **Runs in:** desktop packaged players; isolated headless play sessions. With
  synthetic devices the rumble feel checkpoint is unevaluable.
* **Preconditions:** the LAN profile; Red 1 and Blue 1 facing each other.
* **Steps:** Red 1 pulses at Blue 1 three times: once with Blue 1 pulsing 0.15 s
  before impact, once 0.25 s before, and once without a response. Then Red 1
  dashes and pulses 0.2 s after the dash ends at a target 4.6 m away.
* **Expected:** the 0.15 s response parries and stuns Red 1. The 0.25 s response
  does not parry. The unanswered pulse stuns Blue 1 for 1.5 s. The slam reaches
  the 4.6 m target. Each hit shows the hit-stop, sparks, camera shake, and a
  rumble command on gamepads with motors. Local effects appear on the input
  frame, and the outcome matches the server's.
* **Demonstrates:** NET-001, PHY-001, REN-004, INP-002, RUN-004, AUD-001

### S16. Round modifiers, Surge, and medals

* **Runs in:** desktop packaged players; isolated headless play sessions.
* **Preconditions:** server seed 42; color-blind team colors and captions on;
  camera shake and hit-stop off on one client.
* **Steps:** play a match, enter the last 30 seconds of round 3 (the development
  state `surge` for the Surge check), and finish the match with recorded input.
  Then play four more matches.
* **Expected:** rounds 2 and 3 use the modifiers that seed 42 selects, and every
  participant shows each banner at the same countdown. Each modifier's effect
  matches its table. The Surge drops two cores at 0:30. Medals go to the players
  with the most points, steals, parries, and passes caught. Records and match
  counts update, and the first trail unlocks after the fifth match. Team badges
  show a circle or a triangle, announcer lines show captions, and the client
  with shake and hit-stop off shows neither.
* **Demonstrates:** NET-001, RUN-009, SAV-001, AUD-001, ACC-001

## Manual Playtest Guidance

A person judges the following, and a headless run reports these checkpoints as
awaiting human review, never as passed:

* Whether the opening core drop creates an immediate scramble.
* Whether pulses and parries feel crisp on LAN and still fair on Broadband.
* Whether passing and interception produce exciting plays.
* Whether each round modifier changes how the team plays.
* Whether a losing team still feels it can come back in the Surge.
* Whether the hit-stop, shake, and stingers feel satisfying without being
  tiring.
* Whether the lobby and outcome messages are clear to a first-time player.

## Agent Authoring Change Requests

### CR1. Shrinking Arena

* **Request:** add a Shrinking Arena round modifier. The arena wall moves inward
  1 m every 10 seconds, and drones outside it are stunned.
* **Acceptance scenario:** under every network profile, every client sees the
  same boundary within the synchronization bound, and a drone outside the wall
  is stunned on the server.
* **Test:** a test of the boundary radius over time.

### CR2. Spectators

* **Request:** add spectators. A late joiner can choose to watch without a drone,
  and spectators never appear in team counts.
* **Acceptance scenario:** with two spectators joined, the lobby's Start
  conditions and late-join team assignment ignore them, and they receive the
  same state as players.
* **Test:** a test of the lobby conditions with spectators present.

## Accessibility

* Every menu and lobby action can be performed with the keyboard (S2).
* Menus expose accessibility names (S2).
* The color-blind team colors are blue and yellow, and team badges add a circle
  or a triangle, so color is never the only cue (S16).
* Announcer lines have captions (S16).
* Camera shake and hit-stop can be turned off (S16).

## Saved and Reproducible States

| State | How it is reached | Used by |
|---|---|---|
| `round3_tied`: round 3, rounds 1 to 1, score 4 to 4, 20 seconds left | development-build `-debugState` argument | S2 |
| `carried_center`: round 2, a charged core carried by Blue 2 in the center ring | development-build `-debugState` argument | S4 |
| `lobby_seven`: a lobby with seven ready players and one open slot | development-build `-debugState` argument | S11 |
| `surge`: round 1 at 0:31 remaining | development-build `-debugState` argument | S16 |

## Known Limitations and Unsupported Conditions

* Gamepad rumble plays only on gamepads that have motors.
* Local discovery finds only sessions on the same subnet, because routers do not
  forward its broadcast.

Any behavior this document requires that is not demonstrated is reported with
its real compatibility state.

## Acceptance Outcomes

* The multiplayer arena distinguishes game-rule failures from transport and
  synchronization failures and attributes logs to participants. (SMP-001,
  NET-001)
* Host, client, and Linux dedicated server sessions play complete matches with
  the same scores and winner on every participant. (SMP-001, NET-001, BLD-002,
  PLT-001)
* Late joiners and reconnecting players receive current state, and ownership of
  cores moves between participants with the right callbacks. (SMP-001,
  NET-001)
* Game rules hold under simulated latency, loss, and jitter, duplicate requests,
  and stale requests, and every affected participant marks the simulated
  condition. (SMP-001, NET-001, TST-002)
* Version mismatch, full session, wrong password, unreachable endpoint, timeout,
  host departure, and deliberate disconnection each produce a distinct outcome.
  (SMP-001, NET-001, UI-001)
* One agent session runs a dedicated server and several isolated headless play
  sessions, each with its own identity and attributed logs. (SMP-001, AUT-004,
  TST-002, DIA-001)
* The game finds sessions on the local network or by address and uses no
  proprietary online service. (SMP-001, NET-001)
* Parries, slams, round modifiers, the Surge, and medals behave as specified on
  every participant. (SMP-001, NET-001, RUN-009)
