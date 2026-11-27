# unitas

> **Work in progress.** unitas is in early development. It is not ready for
> production use, it does not yet open real existing game projects end to end,
> and its APIs, file formats, and behavior change without notice. Nothing here
> is released.

unitas is an open-source, commercial-quality game engine, editor, and player. A
Unity 2022.3 LTS project opens in place and runs unchanged, and a person can make,
play, and ship a game with it on desktop, the web, and later mobile, XR, and consoles.
An agent can do the same work through the editor's Model Context Protocol (MCP)
interface. MCP belongs to the editor only. The engine and the games built with it
contain none.

## Status

The [product specification](docs/product-specification/INDEX.md) describes the
finished product. Its present-tense statements are requirements, not claims about
what exists today. The real state of the product is the code on `main` and the
results of its gates and lanes.

## Repository layout

| Path | Contents |
| --- | --- |
| `crates/` | Rust workspace: engine, runtime, build, and editor-ABI crates |
| `bridge/` | C# scripting API, bridge host, and their tests |
| `editor/` | The editor and its MCP interface |
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
