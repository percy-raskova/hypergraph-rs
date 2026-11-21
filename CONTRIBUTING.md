# Contributing

Thank you for helping. This guide says how to build, check and change the code.

## Set up

You need [rustup](https://rustup.rs) and [mise](https://mise.jdx.dev). `rust-toolchain.toml`
pins the toolchain (1.100.0), and rustup installs it the first time you run `cargo`. The
conformance gate also needs a Python 3.12 or newer virtual environment at `.venv`; the setup
is in the `xgi-conformance` job of `.github/workflows/ci.yml`.

## The checks

Run these before you open a pull request. CI runs the same mise tasks.

| Command | What it checks |
| --- | --- |
| `mise run rust:check` | format, clippy with warnings denied, tests and docs for the workspace; then clippy, tests and docs for the core crate with every feature on; then `mise run rust:features` |
| `mise run rust:features` | the core crate builds with no feature, and with each feature alone, without the defaults |
| `mise run rust:msrv` | the build at the minimum Rust version: the workspace, then the core crate with every feature on |
| `mise run py:gate` | the XGI conformance gate (needs `.venv` and a built extension: `.venv/bin/maturin develop`) |
| `cargo deny check` | advisories, licenses and sources |
| `cargo package -p hypergraph-rs --locked` | the core crate packs. Nothing is published |

Run one cargo command at a time: they share one build directory and lock it.

## The minimum Rust version

`rust-version` in `Cargo.toml` is 1.90. It holds for the workspace and for every feature of
the core crate, and `mise run rust:msrv` checks it. It is the lowest version the newest
dependencies allow. Dependencies are kept at their newest versions here, so the floor follows
them: when a newer dependency needs a newer Rust, raise `rust-version` in the same change. Do
not hold a dependency back to keep the floor.

## Tests

- Tests of the core crate are files directly under `crates/hypergraph-rs/tests/`, one file for
  each area (`test_member_data.rs`, `test_id_lookup.rs`, ...). Doc examples run as tests too.
- `crates/hypergraph-rs/tests/conformance/main.rs` replays XGI's recorded answers from
  `crates/hypergraph-rs/conformance/xgi_ground_truth.json`. Do not edit that file by hand.
  `mise run rust:fixtures` regenerates it from XGI 0.10.2. A change in it is news: find out why
  before you commit it.
- Write the test first and see it fail for the reason you expect.
- Nothing may depend on the order in which a `HashMap` is iterated. Listings follow insertion
  order.

## The XGI conformance gate

The Python package in `python/hypergraph_rs` must behave like XGI. `mise run py:gate` builds
no extension itself. It runs, against the extension in `.venv`:

1. the symbol audit, which fails when `tests/DEFERRED_SYMBOLS.md` is out of date
   (`mise run py:symbols` rewrites it);
2. the checks on the registers in `compat/` (`exclusions.toml`, `divergences.toml`,
   `partial_symbols.toml`) and on the manifest;
3. the ported XGI test suite in `tests/ported`.

A ported test that fails is put in one of the classes in `tests/EXCLUSIONS.md`: fix the
binding, fix the core, or record a reason in `compat/exclusions.toml`. Never record one to
hide a bug that can be fixed. XGI is pinned at 0.10.2 on purpose.

## Updating dependencies

Everything is kept at its newest version: the crates, the toolchain pinned in
`rust-toolchain.toml`, and the actions in `.github/workflows/`. Security fixes arrive as
ordinary releases, and a crate that is held back misses them.

Run `cargo update`. For a new major version, change the requirement in `Cargo.toml` first.
Then run every check above. The workspace uses resolver 2 on purpose: an update takes the
newest version of every crate, and if one needs a newer Rust than `rust-version`, the floor
check fails. Raise the floor then. A newer toolchain brings new lints; fix what they find.

Dependabot opens a pull request each week for the crates and for the actions.

Do not add a direct dependency on `petgraph`: it comes through `rustworkx-core`, and a second
copy would be a different, incompatible type.

## The license text in the package

`crates/hypergraph-rs/LICENSE` is a copy of the `LICENSE` at the root, so that the packed
crate carries the license text. If you change one, change the other.

## Commits

The history uses a short summary in the form `type(scope): what changed`, in the present tense
and with no full stop. The types in use are `feat`, `fix`, `perf`, `refactor`, `docs`, `test`,
`build`, `chore` and `ci`. The scope is the area, such as `core`, `deps` or `compat`. The body, when
there is one, says what changed and why, in plain sentences. Make one commit for each unit of
work, and make sure the checks pass for it.

## Reporting a security problem

Do not open a public issue. See `SECURITY.md`.
