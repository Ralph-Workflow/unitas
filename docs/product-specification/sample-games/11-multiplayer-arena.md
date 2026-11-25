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
art, and audio are original unitas content.

Compatibility tiers follow the [Feature catalog](../feature-catalog.md). This
document owns this game's acceptance under SMP-001.

## Targets and Input Modes

* **Targets:** macOS, Windows, and Linux desktop players, each able to host or
  join; a Linux x86-64 dedicated server player; the editor's play mode; and the
  headless editor.
* **Render pipeline:** URP 14.x with the Forward rendering path. The dedicated
  server renders nothing.
* **Input modes:** keyboard and mouse, and gamepad, through Input System 1.x
  actions (INP-002).

## Game Design

### Core loop

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
5. A round ends after 3 minutes or when a team reaches 5 points. The team with
   more points wins the round, and a tied round goes to the team that scored
   last. The first team to win two rounds wins the match.
6. The results panel shows the winner, per-player stats (deposits, steals,
   parries, passes caught), and medals. Rematch keeps the teams and returns to
   the lobby. Leave returns to the title screen.

### Mechanics

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

### Round modifiers

Round 1 is always Standard. Rounds 2 and 3 each draw a different modifier from
the server's seeded random generator. Every participant sees the modifier
banner at the same countdown.

| Modifier | Effect |
|---|---|
| Low Gravity | Thrown cores fly 18 m, and pulses knock drones back 3 m |
| Overcharge | Every core starts charged |
| Blackout | Arena lights drop to 10 percent; drones are visible by their trails and carriers by their glow |
| Magnet Pads | Pads pull a carrier within 6 m toward them at 3 m/s |

### What makes it fun

* **Hook.** Five seconds after round 1 starts, a core drops into a light beam at
  the center with a 3-second countdown. Every drone races for it, and the first
  pulse fight starts within 15 seconds of the match.
* **Mastery curve.**
  * New players chase the core and pulse at once.
  * Skilled players parry, open with a dash into a slam, pass ahead of a
    teammate, and overcharge only when the center is clear.
  * The results panel shows parries, slam hits, and passes caught, so improvement
    is visible.
* **Risk and reward.**
  * Overcharging doubles a core's value but reveals the carrier on every radar.
  * Passing escapes a stun but risks an interception.
  * A pulse leaves the attacker without its stun for 4 s, so a missed pulse
    invites a counterattack.
* **Escalation.** Round modifiers change tactics each round. The Surge doubles
  the cores in the final 30 seconds, and a tied deciding round goes to the team
  that scored last.
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
* **Replay.**
  * End-of-match medals: MVP (most points), Interceptor (most steals), Wall
    (most parries), and Playmaker (most passes caught).
  * Local records of best deposits in a match, best parries, and total wins.
  * Drone trails unlock at 5, 15, and 30 matches played. Unlocks are stored
    locally, with no online service.
  * Rematch keeps teams together for a quick series.

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

### Pause and focus

A networked match cannot pause. The menu overlays the arena while the match
continues, and offers Settings, Leave, and, for the host, End Match. Losing
window focus releases all held input, and the drone stops.

### Settings

* master, music, effects, and announcer volume
* fullscreen or windowed mode, resolution, vertical sync, and quality level
* camera shake and hit-stop on or off
* team colors: Standard or Color-blind safe
* player name
* in development builds only, a simulated network profile (see below)

Settings, the player name, records, unlocks, and a player token (a random
identifier created on first launch) persist through `PlayerPrefs`.

### Session rules

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

### Connection outcomes

Each outcome has its own message and log category:

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

### Simulated network conditions

Development builds offer four profiles, selected in Settings or with the
`-netsim` argument. Each profile sets Unity Transport's network simulator to
the delay, jitter, packet loss, and packet duplication below. Jitter larger
than the send interval also reorders packets.

| Profile | Delay | Jitter | Loss | Duplication |
|---|---|---|---|---|
| LAN | 0 ms | 0 ms | 0 percent | 0 percent |
| Broadband | 60 ms | 10 ms | 1 percent | 0.5 percent |
| Poor | 150 ms | 40 ms | 5 percent | 2 percent |
| Severe | 300 ms | 80 ms | 15 percent | 5 percent |

A participant under any profile other than LAN shows a "SIMULATED NETWORK:
<profile>" banner. Its log, its scenario results, and its playtest records mark
the condition.

## Content

| Asset | Source format | What matters on import |
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

Packages: Netcode for GameObjects 1.x (with Unity Transport as its transport
dependency), URP 14.x, Input System 1.x, uGUI 1.0.x, TextMeshPro 3.0.x, and
Unity Test Framework 1.x.

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
| NET-001 | Host, client, and server modes; approval; `NetworkVariable` permissions; RPCs; ownership transfer; late join; reconnect; timeouts; version mismatch; simulated conditions | S2 to S16 |
| INP-002 | Input System actions for keyboard, mouse, and gamepad; gamepad rumble | S2, S15 |
| UI-001 | Title, host and join panels, lobby, scoreboard, results | S2, S11, S14 |
| TXT-001 | TextMeshPro names, messages, and results | S2, S11 |
| AUD-001 | Announcer stingers, 3D arena sounds, music | S15, S16 |
| REN-004 | Spark, burst, trail, and stun effects | S15 |
| SAV-001 | Player token, settings, records, and unlocks in `PlayerPrefs` | S9, S16 |
| PLT-001 | Linux dedicated server lifecycle and Platform-limited graphics calls | S3, S10 |
| BLD-001 | Desktop players and the Linux dedicated server player with build reports | S1 |
| BLD-002 | Headless builds and headless runs of every scenario | S1, S3, all |
| TST-002 | Multi-participant scenarios with rule, synchronization, and transport checkpoints | all, S12 |
| DIA-001 | Participant-attributed logs and network diagnostics | S12, S13 |
| PRF-001 | Network traffic and server tick profiling | S7 |
| AUT-002 | Play control, input injection, and evidence per play session | S13 |
| AUT-004 | Several isolated headless play sessions in one agent session | S13 |
| ACC-001 | Color-blind team colors with shapes, captions, shake and hit-stop toggles | S16 |

## Engine Behavior This Game Depends On

### Scripting and runtime

* `NetworkManager.StartHost`, `StartClient`, and `StartServer` start the three
  modes. A second start call returns `false` and is reported. `Shutdown`
  disconnects every client and despawns every object.
* The connection approval callback receives the client's payload and refuses a
  client with the reason the callback supplies. No player object spawns before
  approval.
* `NetworkVariable` writes outside their write permission raise an exception and
  send nothing. Owner-read variables (each player's private stats) never reach
  other clients. `OnValueChanged` runs on each receiving participant with the
  previous and new values.
* `[ServerRpc(RequireOwnership = true)]` calls from a non-owner are rejected and
  reported, naming the RPC and the caller. Reliable RPCs on one object arrive in
  send order. `ClientRpcParams` delivers private notices to one client only.
* `ChangeOwnership` and `RemoveOwnership` deliver `OnLostOwnership` and
  `OnGainedOwnership` to the right participants.
* `JsonUtility` parses `server.json` and the approval payload. A malformed file
  is reported with its path, and the server exits with a nonzero status.

### Project and content

* Every prefab in `NetworkPrefabs.asset` spawns with the same `NetworkObjectId`
  on every participant.
* The team variants apply their material override on every participant.

### Simulation

* Only the server moves drones and cores. Each network tick, a client sends an
  unreliable RPC that carries its last three input frames, each with a sequence
  number, so a single lost packet loses no input. Pulse, dash, and throw
  presses travel inside these frames. `NetworkTransform` interpolates on
  clients.
* Pulse hits come from `Physics.OverlapSphereNonAlloc` on the server. Pad and
  ring triggers run on the server only.
* Game rules hold on the server under every network profile:
  * a team's score equals the points of the deposits credited to it
  * a core has at most one carrier
  * a deposit identifier scores once
  * a request with a sequence number the server has already handled has no
    effect

### Networking and transport

* `UnityTransport` connects to the configured address and port. The 10-second
  disconnect timeout produces the Timeout outcome, and a client that cannot
  connect within 10 seconds gets the Unreachable outcome.
* A late-joining client loads the network scenes, then receives every spawned
  object with its current `NetworkVariable` values and owner.
* Unity Transport's network simulator applies the profile's delay, jitter,
  loss, and duplication to every packet the participant sends and receives. It
  is active only in development builds, and the condition is marked on every
  affected participant.
* Local discovery uses UDP broadcast on port 47778 within the local subnet.
  Hosts and servers that are visible on the local network answer within 1
  second. The game opens no network connection to any other destination.

### Rendering and animation

* Clients render the arena, drones, and effects. The dedicated server renders
  nothing, and a graphics API call on it is reported as Platform-limited.

### Audio

* Stingers play on every client within one frame of receiving the score change.
  The dedicated server produces no audio output.

### Input

* Keyboard, mouse, and gamepad produce the same actions. Rumble returns an
  explicit status on gamepads without motors.

### UI and text

* Every menu, lobby control, and outcome message works with keyboard, gamepad,
  and pointer, and the focus is always visible.

### Player and build

* The Linux dedicated server player runs with `-batchmode -nographics`, defines
  `UNITY_SERVER`, and applies server-build asset stripping. It accepts `-port`,
  `-config`, `-maxPlayers`, `-password`, `-netsim` (development builds only), and
  `-logFile`.
* The server is listening within 3 seconds of launch. On SIGTERM it sends
  "Server closed" to every client and exits with status 0 within 2 seconds. When
  the port is in use, it logs the port and exits with a nonzero status.
* Each player writes its log to the target's `Player.log` location, or to the
  `-logFile` path. Every line the game writes starts with the participant tag
  `[<role> c<clientId> "<name>"]`.

### Editor and automation

* One agent session can start several isolated play sessions in the headless
  editor and connect them to each other and to a development server player over
  the loopback interface. Each play session keeps its own identity, input,
  time, and logs.
* Diagnostics attribute each log entry, exception, and network event to its play
  session or player connection.

## Presentation and Performance

* **Visual checkpoints**, each with a unitas-approved reference:
  * the lobby with eight players and ready states
  * the round-start core drop
  * a deposit burst
  * the Blackout modifier
  * the results panel with medals
  * the simulated-network banner
* **Audio checkpoints:** the deposit stinger starts within one frame of the
  client receiving the score change. Arena sounds pan toward their sources.
* **Project-declared expectations.** These are the sample's own expectations,
  not the PRF-002 engine budgets.
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

## Acceptance Scenarios

Every scenario runs in editor play mode, in packaged players, and headless
through the editor's MCP interface, unless it says otherwise. A scenario with several participants
names each one. Its checkpoints are tagged as **rule** (evaluated on the
server's authoritative state), **synchronization** (a remote observation on a
named client within the profile's bound), or **transport** (a connection
outcome or timeout).

* **S1. Open and build.** Precondition: no import cache exists. Open the
  project. From one headless command-line invocation, build desktop players
  for each desktop target and a Linux dedicated server player.
  * Expected: all assets import with no errors. Netcode for GameObjects is
    reported with its tier and state. Each build report lists scenes, assets,
    sizes, and the build identifier. The server build defines `UNITY_SERVER` and
    reports its stripped assets.
  * (PRJ-001, PRJ-007, SER-003, BLD-001, BLD-002)
* **S2. Hosted match.** One host and three clients, as local instances. Each
  sets a name, picks a team, and readies. The host starts the match. Recorded
  input plays three rounds.
  * Expected: Start is enabled only once every condition holds. Every
    participant shows the same scores, rounds, modifiers, and winner at every
    round end (synchronization). The final score matches the credited deposits
    (rule). The results panel lists the same stats on every participant.
  * (NET-001, SER-002, SER-003, UI-001, TXT-001, INP-002, RUN-004)
* **S3. Dedicated server match.** Launch the Linux dedicated server with
  `-batchmode -nographics` and a `server.json`, then connect four clients and
  play two rounds.
  * Expected: the server is listening within 3 seconds and opens no window. The
    countdown starts when the lobby conditions hold. The match completes as in
    S2. Calling the debug "Render frame" command on the server reports a
    Platform-limited graphics call. A malformed `server.json` makes the server
    exit nonzero with the file path in its log.
  * (NET-001, BLD-002, PLT-001, RUN-009, PRJ-007)
* **S4. Late join.** Precondition: a match in round 2 with a charged core
  carried by Blue 2. A fourth client joins.
  * Expected: the joiner loads `Lobby` and `Arena`, then receives every drone and
    core with its current position, score, modifier, timer, and owner, before its
    own drone spawns. It joins the team with fewer players. `OnValueChanged`
    does not fire for values it never saw change.
  * (NET-001, SER-002, RUN-003)
* **S5. Ownership and passes.** Blue 1 picks up a core, passes it to Blue 2, and
  Red 1 intercepts the next pass. Red 1 is then stunned while carrying.
  * Expected: ownership moves Blue 1, server, Blue 2, server, Red 1, server.
    `OnGainedOwnership` and `OnLostOwnership` fire once each on the right
    participants. A throw request from a non-owner is rejected, naming the RPC
    and the caller. The stunned carrier drops the core where it was hit.
  * (NET-001, PHY-001, RUN-003)
* **S6. Permissions.** From a client, trigger the debug actions "Write team
  score" and "Spawn core." Read another player's private stats.
  * Expected: the score write raises an exception and sends nothing. The spawn
    raises an exception naming the object. Private stats never reach a
    non-owner. Each failure is logged under the calling participant.
  * (NET-001, DIA-001)
* **S7. Latency, loss, and jitter.** Repeat S2 under the Broadband, Poor, and
  Severe profiles applied to every client. Capture a network profile of the
  server.
  * Expected: every rule checkpoint passes under every profile. Synchronization
    checkpoints pass within each profile's bound. Every client shows the banner,
    and every result records the profile. The capture shows traffic per client
    and tick time, and meets the bandwidth expectation.
  * (NET-001, TST-002, PRF-001)
* **S8. Duplicates and stale state.** Under the Severe profile, one client
  pulses five times and dashes five times. Then a carrier throws the core 50 ms
  after the server has already stunned it.
  * Expected: each press arrives in up to three redundant input frames, plus
    any duplicated or reordered packets. Each pulse stuns once and each dash
    fires once. Every repeated or out-of-order sequence number is logged as
    ignored.
    The late throw is rejected as stale. The thrower alone receives "Throw
    rejected: core no longer carried," and its view reverts within one round
    trip plus 100 ms.
  * (NET-001)
* **S9. Disconnect and reconnect.** Terminate a client process during a round.
  Restart it and reconnect after 30 seconds. Repeat with a 70-second gap. Then
  press Leave on another client.
  * Expected: the server reports the Timeout outcome for the terminated client
    within 10 seconds, and every other client shows it leaving. The 30-second
    reconnect restores team, name, and stats. The 70-second reconnect joins as a
    new player with zero stats. Leave shows "You left the session" on that
    client only.
  * (NET-001, SAV-001)
* **S10. Host departure and server shutdown.** Quit the host mid-round. In a
  second run, send SIGTERM to the dedicated server.
  * Expected: every client shows "The host left the session" within 10 seconds
    in the first run and "Server closed" within 2 seconds in the second, then
    returns to the title screen. No exception is logged. The server exits with
    status 0.
  * (NET-001, PLT-001)
* **S11. Connection outcomes.** Against a server of version 1.4.0 with a
  password and eight players: join from a 1.3.0 client, join as a ninth player,
  join with a wrong password, join an address with no server, and join with an
  approval payload that is not valid JSON.
  * Expected: the client sees, in order, the Version mismatch message naming
    both versions, Full, Authentication failure, Unreachable after 10 seconds,
    and Refused with the parse reason. Each outcome has its own log category,
    and no player object spawns for any of them.
  * (NET-001, UI-001, TXT-001, RUN-009)
* **S12. Failure classification.** Run S2 three times: with the debug switch
  "Double points" on the server, with all traffic to one client dropped for 15
  seconds, and with a 4-second delay applied to one client under the Poor
  profile bound.
  * Expected: the first run fails on a rule checkpoint. The second fails on a
    transport checkpoint (Timeout). The third fails on a synchronization
    checkpoint while every rule checkpoint passes. Each result names the first
    failed checkpoint and the participant, keeps later evidence, and shows each
    participant's log lines around the failure under its tag.
  * (NET-001, TST-002, DIA-001)
* **S13. Local multi-instance testing through the editor's MCP interface.** Headless only. One agent
  session starts a development Linux server player and four isolated play
  sessions in the headless editor, connects them over the loopback interface,
  injects input into each, and stops one client mid-match.
  * Expected: each play session reports its own identity, time, and logs.
    Input sent to one never reaches another. Stopping one does not stop the
    others, and the server reports that client's departure. Every log entry and
    capture is attributed to its participant.
  * (AUT-004, AUT-002, DIA-001, NET-001, TST-002)
* **S14. Local discovery and join by address.** Start two hosts and one
  dedicated server on the local network, one hidden from local discovery.
  Open Join on a fourth machine. Then join the hidden host by IPv4, by host name,
  and by IPv6 address.
  * Expected: the two visible sessions appear within 2 seconds with name,
    players, version, and round-trip time. An entry disappears 5 seconds after
    its session ends. Each address form connects. An invalid address is rejected
    in the field before connecting. A packet capture shows traffic only to the
    local subnet broadcast and the addresses used.
  * (NET-001, UI-001)
* **S15. Combat feel.** On the LAN profile, Red 1 pulses at Blue 1 three times:
  once with Blue 1 pulsing 0.15 s before impact, once 0.25 s before, and once
  without a response. Then Red 1 dashes and pulses 0.2 s after the dash ends at a
  target 4.6 m away.
  * Expected: the 0.15 s response parries and stuns Red 1. The 0.25 s response
    does not parry. The unanswered pulse stuns Blue 1 for 1.5 s. The slam reaches
    the 4.6 m target. Each hit shows the hit-stop, sparks, camera shake, and
    rumble on gamepads with motors. Local effects appear on the input frame, and
    the outcome matches the server's.
  * (NET-001, PHY-001, REN-004, INP-002, RUN-004, AUD-001)
* **S16. Round modifiers, Surge, and medals.** Play a match on server seed 42,
  and enter the last 30 seconds of round 3. Finish the match with recorded
  input.
  * Expected: rounds 2 and 3 use the modifiers that seed 42 selects, and every
    participant shows each banner at the same countdown. Each modifier's effect
    matches its table. The Surge drops two cores at 0:30. Medals go to the
    players with the most points, steals, parries, and passes caught. Records and
    match counts update, and the first trail unlocks after the fifth match.
  * (NET-001, RUN-009, SAV-001, AUD-001, ACC-001)

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

The first change request adds a Shrinking Arena round modifier. The arena wall
moves inward 1 m every 10 seconds, and drones outside it are stunned. It comes
with a scenario proving that every client sees the same boundary within the
synchronization bound.

A second change request adds spectators. A late joiner can choose to watch
without a drone, and spectators never appear in team counts. It comes with a
test of the lobby conditions with spectators present.

## Accessibility

Every menu and lobby action can be performed with the keyboard. Menus expose
accessibility names. The color-blind team colors are blue and yellow, and team
badges add a circle or a triangle, so color is never the only cue. Announcer
lines have captions. Camera shake and hit-stop can be turned off.

## Saved and Reproducible States

Matches are not saved. Development server builds reach test situations with the
`-debugState` argument:

* `round3_tied`: round 3, rounds 1 to 1, score 4 to 4, 20 seconds left.
* `carried_center`: round 2, a charged core carried by Blue 2 in the center ring.
* `lobby_seven`: a lobby with seven ready players and one open slot.
* `surge`: round 1 at 0:31 remaining.

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
* Late joiners and reconnecting players receive current state, and ownership
  of cores moves between participants with the right callbacks. (SMP-001,
  NET-001)
* Game rules hold under simulated latency, loss, and jitter, duplicate requests,
  and stale requests, and every affected participant marks the simulated
  condition. (SMP-001, NET-001, TST-002)
* Version mismatch, full session, wrong password, unreachable endpoint, timeout,
  host departure, and deliberate disconnection each produce a distinct outcome.
  (SMP-001, NET-001, UI-001)
* One agent session runs a dedicated server and several isolated headless
  play sessions, each with its own identity and attributed logs. (SMP-001,
  AUT-004, TST-002, DIA-001)
* The game finds sessions on the local network or by address and uses no
  proprietary online service. (SMP-001, NET-001)
* Parries, slams, round modifiers, the Surge, and medals behave as specified on
  every participant. (SMP-001, NET-001, RUN-009)
