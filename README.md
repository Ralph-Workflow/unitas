# unitas

> **Work in progress.** unitas is in early development. It is not ready for
> production use, it does not yet open real existing game projects end to end,
> and its APIs, file formats, and behavior change without notice. Nothing here
> is released.

unitas is an open-source game engine, runtime, and build product that is
C#-first and built for AI agents and the Model Context Protocol (MCP). The goal
is that an existing C# game project opens in place, and that an authorized
agent, a developer, or both together can inspect it, change it, run it, test
it, debug it, and build it without converting it first.

## Status

The [product specification](docs/product-specification/INDEX.md) describes the
finished product. Its present-tense statements are requirements, not claims
about what exists today.

What the repository contains today:

* **Asset import.** Parsing of YAML-serialized scenes, prefabs, materials,
  `.meta` files and GUIDs, and project settings (`crates/unitas-asset`).
* **Scene graph and transforms** (`crates/unitas-core`).
* **Runtime.** Script lifecycle ticks, coroutines, multi-scene loading, legacy
  input, application lifecycle, and persistent preferences
  (`crates/unitas-runtime`).
* **Physics.** Rigidbodies; box, sphere, and capsule colliders; collision and
  trigger callbacks; raycasts and other queries; and a kinematic character
  controller (`crates/unitas-physics`).
* **Animation.** Animation clips, curves, and animator controllers
  (`crates/unitas-animation`).
* **Audio.** Scene audio playback of WAV clips (`crates/unitas-audio`).
* **Rendering.** A headless software renderer for scenes and their materials
  (`crates/unitas-render`).
* **Builds.** Build configuration, content bundles, build reports, and a
  `unitas-player` binary that launches a bundle and steps it headlessly
  (`crates/unitas-build`).
* **C# bridge.** A C# scripting API that runs component scripts against the
  Rust runtime, plus a declarative playtest scenario runner (`bridge/`).
* **Editor host and MCP automation.** A headless C# editor host that serves
  the Model Context Protocol over stdio and HTTP, registers project, scene,
  audit, play-session, capture, and native Rust tools, and binds to the Rust
  editor ABI (`editor/`, `crates/unitas-editor-abi`).

Not built yet: the visual editor, opening an existing project directly,
platform targets beyond the macOS player, and most of the
[feature catalog](docs/product-specification/feature-catalog.md).

## Repository layout

| Path | Contents |
| --- | --- |
| `crates/` | Rust workspace: engine, runtime, build, and editor-ABI crates |
| `bridge/` | C# scripting API, bridge host, and their tests |
| `editor/` | C# editor host: MCP tools, play sessions, and the headless CLI |
| `docs/product-specification/` | What the finished product does, feature by feature |
| `docs/ralph-workflow-policy/` | Quality policy that every change must satisfy |
| `tools/` | Verification gates and the CLA check, in standard-library Python |

## Building and verifying

You need:

* a Rust toolchain with `cargo`, `clippy`, and `rustfmt`
* the .NET 8 SDK (the Makefile looks for it in `~/.dotnet`)
* Python 3.11 or later, with `ruff`, `mypy`, `pytest`, and `detect-secrets`

Run every gate:

```sh
make verify
```

That runs the policy self-check (`policy-validate`), the bypass audit
(`bypass-audit`), the unsafe-zone gate (`zone-gate`), the ban-catalog gate
(`banned-constructs`), the comment audit (`comment-audit`), the dependency gate
(`deps`), the secret scan (`secret-scan`), the linting and format checks
(`ruff check`, `ruff format --check`, `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `dotnet-format`),
the type checks (`mypy`, `dotnet-build`, and the Rust type check that
`cargo clippy --workspace --all-targets -- -D warnings` performs while it
compiles every workspace crate and target), and the test suites (`pytest`,
`cargo test`, `dotnet-test`), all within a fixed time budget. The C#
test step loads the runtime as a `.dylib`, so the full gate currently runs on
macOS only.

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) and
[AGENTS.md](AGENTS.md) first. Every contributor signs the
[Contributor License Agreement](CLA.md) once, on their first pull request.

## License

Copyright (C) 2026 Ralph Workflow and the unitas contributors.

unitas is dual licensed. It is available under the
[GNU General Public License version 3](LICENSE) and under a separate commercial
license from Ralph Workflow. unitas is provided without any warranty, as set out
in the license. For commercial licensing, contact
hello@ralphworkflow.com.

## Trademarks and affiliation

unitas is an independent open-source project. It is not affiliated with,
endorsed by, sponsored by, or otherwise connected to any other game engine, its
vendor, or its products. Any third-party product names, logos, and trademarks
that appear in this repository's code or documentation belong to their
respective owners and are used only to describe compatibility. Such use does
not imply any relationship with, or endorsement by, those owners. See
[TRADEMARKS.md](TRADEMARKS.md) for details.

unitas is written independently. Contributions must not include code, assets,
or documentation copied or decompiled from proprietary engines, editors, or
their tools. The rules are in [CONTRIBUTING.md](CONTRIBUTING.md#independent-implementation).
