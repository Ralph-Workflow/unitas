# Contributing to unitas

Thank you for helping build unitas. Before you open a pull request:

1. Read `AGENTS.md` and the policy files it points to under
   `docs/ralph-workflow-policy/`.
2. Run `make verify` and make sure every gate passes.
3. Keep pull requests small and focused, with tests for every behavior change.

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
