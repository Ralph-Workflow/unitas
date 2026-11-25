# Contributing to unitas

Thank you for helping build unitas. Before you open a pull request:

1. Read `AGENTS.md` and the policy files it points to under
   `docs/ralph-workflow-policy/`.
2. Run `make verify` and make sure every gate passes. You need a Rust toolchain
   with `cargo`, `clippy`, and `rustfmt`, the .NET 8 SDK (in `~/.dotnet`), and
   Python 3.11 or later with `ruff`, `mypy`, `pytest`, and `detect-secrets`.
   `make verify` runs the policy self-check (`policy-validate`), the bypass
   audit (`bypass-audit`), the ban-catalog gate (`banned-constructs`), the
   comment audit (`comment-audit`), the dependency gate (`deps`), the secret
   scan (`secret-scan`), the linting and format checks (`ruff check`,
   `ruff format --check`, `cargo fmt --check`, `cargo clippy`,
   `dotnet-format`), the type checks (`mypy`, `cargo check`, `dotnet-build`),
   and the test suites (`pytest`, `cargo test`, `dotnet-test`).
3. Keep pull requests small and focused, with tests for every behavior change.

## Independent implementation

unitas reads and runs projects made for other engines, so it must be written
independently of them. Every contribution follows these rules:

* Work only from public documentation, published file formats, and the
  observed behavior of projects you have the right to use.
* Never copy code, comments, or documentation text from another engine, its
  editor, its packages, or its tools, even when that source is publicly
  viewable. Describe behavior in your own words.
* Never decompile, disassemble, or otherwise reverse engineer another
  engine's binaries, and never use leaked or confidential material or
  anything covered by a non-disclosure agreement.
* Test fixtures and sample content are written by hand, or come from content
  you own or that carries a license compatible with GPL-3.0 and the
  commercial license. Do not add asset-store content, bundled sample assets,
  or another vendor's logos, icons, or images.
* Name other products only to describe compatibility, as explained in
  [TRADEMARKS.md](TRADEMARKS.md). Never use another vendor's marks in the
  name or branding of anything unitas ships.

Say in the pull request where any third-party material came from and under
which license. If you are unsure whether something is allowed, ask before you
submit it.

## Contributor License Agreement

unitas is dual licensed under GPL-3.0 and a commercial license. Every
contribution must therefore be covered by the
[unitas Contributor License Agreement](CLA.md) before it is merged.

The `CLA` workflow checks every pull request:

* Everyone who must sign is listed: the person who opened the pull request,
  the GitHub account behind every commit author, and every
  `Co-authored-by:` trailer. Bot accounts and AI coding assistants are
  exempt; the human who submits their output signs for it.
* To sign, post this comment on the pull request:

  ```
  I have read the unitas Contributor License Agreement and I hereby sign it.
  ```

  You sign once. Later pull requests pass for you automatically.
* A commit whose author email is not linked to a GitHub account cannot be
  checked. Add the email to your GitHub account, or rewrite the commit with an
  email that is linked, then comment `recheck`.
* The workflow reports the result as the `cla` commit status and keeps one
  comment on the pull request up to date. `main` refuses to merge a pull
  request until that status passes.

Signatures are stored in `signatures.json` on the `cla-signatures` branch.
That branch is written only by the workflow and cannot be deleted or
force-pushed.
