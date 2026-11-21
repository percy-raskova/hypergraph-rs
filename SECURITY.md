# Security policy

## Supported versions

Fixes are made on `main` and released in the next version. Only the latest
released version is supported.

## Reporting a vulnerability

Please do not open a public issue for a security problem.

Report it privately through GitHub: on this repository's **Security** tab,
choose **Report a vulnerability**. That opens a private advisory that only
the maintainers can see.

Say what is affected, how to reproduce it, and what an attacker could do. You
will get an answer within a week. Once a fix is released, the advisory is
published and you are credited unless you ask not to be.

## What is checked automatically

- `cargo-deny` checks every dependency against the RustSec advisory database,
  the license allow list and the allowed sources, on every change and weekly
  (`deny.toml`, `.github/workflows/supply-chain.yml`).
- CodeQL analyses the Rust, the Python and the workflows
  (`.github/workflows/codeql.yml`).
- Dependabot raises security updates at once and version updates weekly
  (`.github/dependabot.yml`).
- Every workflow action is pinned to a full commit.
