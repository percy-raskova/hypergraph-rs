# Phase 9 — CLI (`hypergraph-rs-cli`)

**Goal:** Implement the clap-derive command-line binary `hypergraph-rs` per spec §8 —
the 8 subcommands (`inspect`, `convert`, `validate`, `render`, `matrix`, `run`,
`generate`, `selftest`), the exit-code contract (0/1/2/3, spec §8.4), `--output -`
stdout streaming (spec §8.3 principle 2), and a smoke test for every subcommand driven
through `assert_cmd`. The CLI is a **thin wrapper** (spec §8.3 principle 1): every
command calls already-built, already-cited `hypergraph-rs` core functions from Phases
0–6. No serialization or graph logic is re-implemented here.

**Scope guard (owner decision):** NO release/distribution tasks. Spec §8.5 mentions
`cargo-dist` / pre-built GitHub-release binaries — those are **explicitly out of scope**.
This phase builds `--release` and smoke-tests only. No `cargo publish`, no `cargo-dist`,
no binary uploads.

**Depends on (must be landed first):** Phases 0–6 in order. Concretely this phase calls,
all through the default feature set of `hypergraph-rs`:

- `readwrite` (default): `read_json`/`write_json`, `read_edgelist`/`write_edgelist`,
  `read_hif`/`write_hif`, `read_incidence_matrix`/`write_incidence_matrix`
  (Phase 4 Tasks 13–17).
- `linalg` (unconditional core): `incidence_matrix`, `adjacency_matrix`
  (Phase 3 Tasks 1–2).
- `algorithms` (default): `connected_components`, `is_connected`,
  `number_connected_components`, `katz_centrality`, `clique_eigenvector_centrality`,
  `clustering_coefficient`, `density` (Phase 3 Tasks 5–10).
- `stats` (default): `node_degree`, `edge_size`, `Stat<T>` (Phase 4 Tasks 10–11).
- `generators` (default): `empty_hypergraph`, `trivial_hypergraph`,
  `complete_hypergraph`, `random_hypergraph`, `fast_random_hypergraph`,
  `uniform_erdos_renyi_hypergraph`, `star_clique`, `ring_lattice`, `sunflower`,
  `PType`, `GeneratorError` (Phase 4 Tasks 3–8).
- `layout` + `viz` (default): `LayoutStrategy`, `BipartiteAlign`,
  `Hypergraph<Value,Value,M>::render_to_svg` / `render_to_json` (Phase 5 Tasks 8–10).

The CLI keeps the **default** feature set of `hypergraph-rs` (which per spec §10.1 =
`["algorithms", "generators", "stats", "readwrite", "layout", "viz"]`). It does NOT need
the non-default `convert`/`dynamics`/`communities` modules — file-format conversion is
served entirely by `readwrite`.

---

## Global constraints (carried from Phases 0–6)

- **No `unsafe`.** `#![forbid(unsafe_code)]` at the crate root (`main.rs`).
- **All loops statically bounded.** Every loop in this crate iterates a finite collection
  (node/edge id vectors, matrix rows, a fixed selftest check list) — no `loop {}`, no
  unbounded `while`.
- **No function longer than ~100 lines.** Command handlers are split so each stays within
  one printed page.
- **Explicit error handling.** No `.unwrap()`/`.expect()` on fallible I/O or parse paths in
  non-test code; every fallible path returns `Result<_, AppError>`. (`.unwrap()` is allowed
  only in `#[test]` code and on infallible operations like writing to an already-sized
  `String`.)
- **Strict typing.** Explicit return types on every function; clap arg enums derive
  `ValueEnum`.
- **Clippy clean** at `-D warnings`, `cargo fmt` clean.
- **Commit after each task** with the trailer
  `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- **Do NOT run any cargo/build command yourself while authoring** — the run commands below
  are for EXECUTION time. A separate build workflow owns the machine.

## Deliberate deviations / interpretations (declared up front)

1. **`validate` semantics are NOT defined by XGI** (XGI has no CLI). Spec §8.2 shows only
   the success line `OK: 247 nodes, 18 hyperedges, no semantic violations.` We define
   validation as: **non-strict** = the file parses into a hypergraph (any parse failure →
   exit 1). **strict** = additionally no empty edges, no duplicate edges, and no isolated
   nodes; any such finding → exit 2. This mirrors the core's own notions
   (`EdgeView::empty`, `EdgeView::duplicates`, `NodeView::isolates` — Phase 2 Tasks 7–8).
   (Owner ruling 2026-07-19: APPROVED as drafted; this CLI-invented semantics is the binding
   definition.)

2. **`render --width/--height` are accepted but currently advisory (no-op).** The spec §8.1
   `Render` subcommand lists `width`/`height` as `Option<f32>`, but the landed
   `render_to_svg(strategy)` (Phase 5 Task 10) auto-sizes the SVG `viewBox` from the layout
   bounding box and takes no size arguments. The CLI keeps the flags for spec-surface parity
   but prints a one-line note to stderr when they are supplied and does not resize. A future
   core `render_to_svg_sized` would honor them. (Owner ruling 2026-07-19: APPROVED as
   advisory no-ops with the stderr note; no core API change this phase.)

3. **`matrix` sparse output is derived from the dense matrix.** The CLI emits sparse triplets
   by scanning the dense `Array2<f64>` returned by `linalg::incidence_matrix` /
   `adjacency_matrix` for nonzeros — it does NOT call `incidence_matrix_sparse` (avoids a
   `sprs` dependency in the CLI and keeps a single code path). Output values are identical.
   (Owner ruling 2026-07-19: approved.)

4. **`generate` parameters are a JSON object** passed via `--params`. Each generator names
   the fields it reads (documented in `generate.rs`). Unknown generator or missing/invalid
   param → exit 1 (application-level usage error routed through `AppError::General`). The
   implemented generator set is a representative 9-generator subset (empty, trivial, complete,
   random, fast-random, uniform-erdos-renyi, star-clique, ring-lattice, sunflower); more can be
   added later. (Owner ruling 2026-07-19: approved for v0.1.0.)

5. **`run` algorithm hyperparameters are fixed defaults this phase** — no per-algorithm tuning
   flags. `katz` uses cutoff = 3; `clique-eigenvector` uses tol = 1e-6, max_iter = 100. These
   constants are stated in the `run` subcommand `--help` text and are the same values the spec
   examples imply. Adding tuning flags is deferred. (Owner ruling 2026-07-19: keep fixed
   defaults, no extra flags this phase.)

6. **`selftest` is an in-process invariant battery, NOT the spec's conformance suite.** Spec
   §8.2's example shows `Running 47 conformance tests (fast mode)... PASS: 47/47`, implying
   selftest runs the full conformance oracle. It does not: the real conformance gate is the
   Phase-7 pytest suite at the workspace-root `tests/` tree, which `selftest` deliberately does
   NOT duplicate. Instead `selftest` runs a fixed 5-check (`fast`) / 7-check (`full`) smoke of
   the built binary against its own core, printing `Running N conformance checks` with N = 5 or
   7. The printed count and semantics therefore intentionally differ from the spec §8.2
   illustrative example. (Owner ruling 2026-07-19: add this deviation, per review D4.)

7. **Exit-code contract follows spec §8.4 exactly; clap-level usage errors exit 2.** Exit 1 is
   reserved for "file not found, parse error" (spec §8.4) plus application-level usage errors
   routed through `AppError::General` (unknown algorithm/generator, invalid `--params`). CLI
   *parse* errors caught inside `Cli::parse()` (unknown flag, missing required arg, missing
   subcommand) are handled by clap BEFORE dispatch and exit with clap's default code 2 — the
   same code this CLI assigns to a `validate --strict` semantic failure. This collision is
   accepted and documented (owner ruling 2026-07-19, review D3 option (a)); CI callers that need
   to distinguish a malformed invocation from a validation finding should inspect stderr, not
   rely on exit 2 alone.

---

## File structure added by Phase 9

```
hypergraph-rs/crates/hypergraph-rs-cli/
├── Cargo.toml                       # MODIFY: deps + dev-deps + features
├── src/
│   ├── main.rs                      # REWRITE: parse → dispatch → exit code
│   ├── cli.rs                       # NEW: clap Parser/Subcommand + arg enums
│   ├── error.rs                     # NEW: AppError + exit_code()
│   ├── io.rs                        # NEW: read_hypergraph, write_string_output, write_via_path
│   └── commands/
│       ├── mod.rs                   # NEW: dispatch(command)
│       ├── inspect.rs               # NEW (Task 3)
│       ├── convert.rs               # NEW (Task 4)
│       ├── validate.rs             # NEW (Task 5)
│       ├── matrix.rs                # NEW (Task 6)
│       ├── run.rs                   # NEW (Task 7)
│       ├── generate.rs              # NEW (Task 8)
│       ├── render.rs                # NEW (Task 9)
│       └── selftest.rs             # NEW (Task 10)
└── tests/
    ├── common/mod.rs                # NEW: fixture helpers (shared across test bins)
    ├── fixtures/community.json      # NEW: valid xgi-data JSON fixture
    ├── test_cli_skeleton.rs         # NEW (Task 1)
    ├── test_inspect.rs              # NEW (Task 3)
    ├── test_convert.rs              # NEW (Task 4)
    ├── test_validate.rs             # NEW (Task 5)
    ├── test_matrix.rs               # NEW (Task 6)
    ├── test_run.rs                  # NEW (Task 7)
    ├── test_generate.rs             # NEW (Task 8)
    ├── test_render.rs               # NEW (Task 9)
    └── test_selftest.rs             # NEW (Task 10)
```

Integration tests stay under `crates/hypergraph-rs-cli/tests/` (per the workspace
convention: Rust integration tests live in `crates/<crate>/tests/`; the Python conformance
suite of Phase 7 is a separate, workspace-root `tests/` tree).

---

## Task 1: CLI crate deps + the full clap command surface

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs-cli/Cargo.toml`
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/src/cli.rs`
- Rewrite: `hypergraph-rs/crates/hypergraph-rs-cli/src/main.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/tests/common/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/tests/fixtures/community.json`
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/tests/test_cli_skeleton.rs`

**Interfaces:** the `Cli` parser and `Command` subcommand enum (spec §8.1, verbatim
subcommand set), plus the seven `ValueEnum` argument enums. This task wires the surface and
the smoke harness; command handlers are empty stubs returning `Ok(())` until Tasks 3–10.

- [ ] **Step 1: Update `Cargo.toml`.** Replace the crate's dependency block:

```toml
[package]
name = "hypergraph-rs-cli"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true
description = "CLI for hypergraph-rs"

[[bin]]
name = "hypergraph-rs"
path = "src/main.rs"

[dependencies]
# Default features of the core crate cover everything the CLI needs
# (algorithms, generators, stats, readwrite, layout, viz).
hypergraph-rs = { path = "../hypergraph-rs" }
clap = { version = "4", features = ["derive"] }
serde_json.workspace = true
ndarray.workspace = true
indexmap.workspace = true
tempfile = "3"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

Notes: `ndarray.workspace = true` is needed to name/index the `Array2<f64>` returned by
`linalg` (Task 6). `indexmap.workspace = true` is needed because `run.rs` (Task 7) names
`IndexMap<String, f64>` in a public helper signature — the centrality/clustering algorithms
return `IndexMap` (indexmap 2 is already a workspace dependency). `tempfile` (runtime dep)
backs `--output -` streaming for the path-based `readwrite` writers (Task 4).
`assert_cmd`/`predicates` are the smoke harness;
if the reviewer prefers zero new deps, every `Command::cargo_bin("hypergraph-rs")` call can
be rewritten with `std::process::Command::new(env!("CARGO_BIN_EXE_hypergraph-rs"))` and the
`predicate::str::contains(..)` assertions with plain `String::from_utf8` + `assert!` —
noted once here, not repeated per task.

- [ ] **Step 2: Write the failing smoke test.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/tests/test_cli_skeleton.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("hypergraph-rs").expect("binary builds")
}

#[test]
fn version_flag_prints_version() {
    bin()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("hypergraph-rs"));
}

#[test]
fn help_lists_all_eight_subcommands() {
    let assert = bin().arg("--help").assert().success();
    let out = assert.get_output();
    let text = String::from_utf8_lossy(&out.stdout);
    for sub in [
        "inspect", "convert", "validate", "render", "matrix", "run", "generate", "selftest",
    ] {
        assert!(text.contains(sub), "help missing subcommand: {sub}");
    }
}

#[test]
fn each_subcommand_has_help() {
    for sub in [
        "inspect", "convert", "validate", "render", "matrix", "run", "generate", "selftest",
    ] {
        bin()
            .args([sub, "--help"])
            .assert()
            .success();
    }
}

#[test]
fn no_subcommand_is_an_error() {
    // clap exits non-zero (code 2) when a required subcommand is missing.
    bin().assert().failure();
}
```

- [ ] **Step 3: Verify it fails.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_cli_skeleton`
Expected: FAIL to compile / fail assertions (the stub `main.rs` prints a placeholder and
has no subcommands).

- [ ] **Step 4: Create the fixture.** Write a valid xgi-data JSON (schema:
`convert/hypergraph_dict.py`, cited via Phase 4 Task 13) to
`hypergraph-rs/crates/hypergraph-rs-cli/tests/fixtures/community.json`:

```json
{
  "type": "hypergraph",
  "hypergraph-data": { "name": "community" },
  "node-data": { "1": {}, "2": {}, "3": {}, "4": {}, "5": {} },
  "edge-data": { "e0": { "weight": 2 }, "e1": {}, "e2": {} },
  "edge-dict": {
    "e0": ["1", "2", "3"],
    "e1": ["2", "3"],
    "e2": ["3", "4", "5"]
  }
}
```

This parses (via `from_hypergraph_dict`) to 5 nodes, 3 hyperedges — used by every
command's smoke test.

- [ ] **Step 5: Shared test helper.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/tests/common/mod.rs`:

```rust
//! Shared helpers for the CLI integration-test binaries.
//! (Each `tests/*.rs` is a separate binary; `mod common;` pulls this in.)

use std::path::PathBuf;

/// Absolute path to a checked-in fixture under `tests/fixtures/`.
pub fn fixture(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push(name);
    p
}
```

- [ ] **Step 6: Write `cli.rs`.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/src/cli.rs`:

```rust
//! The clap-derive command surface (spec §8.1).

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// XGI-compatible hypergraph toolkit (Rust).
#[derive(Parser, Debug)]
#[command(name = "hypergraph-rs", version, about = "XGI-compatible hypergraph toolkit (Rust)")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// The eight subcommands (spec §8.1).
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Inspect a hypergraph file (stats summary).
    Inspect {
        input: PathBuf,
        #[arg(long, default_value = "text")]
        format: InspectFormat,
    },
    /// Convert between formats (json, edgelist, hif, incidence-matrix).
    Convert {
        input: PathBuf,
        #[arg(long, value_enum)]
        from: Format,
        #[arg(long, value_enum)]
        to: Format,
        #[arg(long, default_value = "-")]
        output: String,
    },
    /// Validate a hypergraph file against XGI semantics.
    Validate {
        input: PathBuf,
        #[arg(long)]
        strict: bool,
    },
    /// Render a hypergraph to SVG or scene-graph JSON.
    Render {
        input: PathBuf,
        #[arg(long, default_value = "output.svg")]
        output: PathBuf,
        #[arg(long, default_value = "hyperedge-aware")]
        layout: LayoutStrategyArg,
        #[arg(long, default_value = "svg")]
        format: RenderFormat,
        #[arg(long)]
        width: Option<f32>,
        #[arg(long)]
        height: Option<f32>,
    },
    /// Compute the incidence or adjacency matrix.
    Matrix {
        input: PathBuf,
        #[arg(long, value_enum, default_value = "incidence")]
        kind: MatrixKind,
        #[arg(long, default_value = "dense")]
        format: MatrixFormat,
        #[arg(long, default_value = "-")]
        output: String,
    },
    /// Run an algorithm (centrality, components, etc.) and print results.
    ///
    /// Fixed hyperparameters this version: katz cutoff = 3;
    /// clique-eigenvector tol = 1e-6, max_iter = 100. No per-algorithm tuning flags.
    Run {
        input: PathBuf,
        /// Algorithm name: components, is-connected, num-components, density,
        /// degree, clustering, katz, clique-eigenvector.
        #[arg(long)]
        algorithm: String,
        #[arg(long)]
        node: Option<String>,
    },
    /// Generate a hypergraph from a named generator.
    Generate {
        #[arg(long)]
        generator: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long)]
        params: Option<String>,
    },
    /// Self-test: run a subset of the conformance suite.
    Selftest {
        #[arg(long, default_value = "fast")]
        mode: SelftestMode,
    },
}

/// Output format for `inspect`.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum InspectFormat {
    Text,
    Json,
}

/// File formats for `convert` (`--from` / `--to`).
#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum Format {
    Json,
    Edgelist,
    Hif,
    IncidenceMatrix,
}

/// Layout strategy for `render`.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum LayoutStrategyArg {
    HyperedgeAware,
    Spring,
    Bipartite,
    Radial,
}

/// Output format for `render`.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum RenderFormat {
    Svg,
    Json,
}

/// Matrix kind for `matrix`.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum MatrixKind {
    Incidence,
    Adjacency,
}

/// Matrix serialization for `matrix`.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum MatrixFormat {
    Dense,
    Sparse,
}

/// Mode for `selftest`.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum SelftestMode {
    Fast,
    Full,
}
```

Note: clap's `ValueEnum` derives kebab-case value names by default, so
`IncidenceMatrix` → `incidence-matrix`, `HyperedgeAware` → `hyperedge-aware` — matching
the spec's example CLI strings. The `default_value = "…"` strings parse through the enum.

- [ ] **Step 7: Rewrite `main.rs` with stub dispatch.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/src/main.rs`:

```rust
//! `hypergraph-rs` CLI entry point (spec §8).
#![forbid(unsafe_code)]

mod cli;
mod commands;
mod error;
mod io;

use clap::Parser;

use crate::cli::Cli;

fn main() {
    let cli = Cli::parse();
    let code = match commands::dispatch(cli.command) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            e.exit_code()
        }
    };
    std::process::exit(code);
}
```

- [ ] **Step 8: Minimal `error.rs`, `io.rs`, `commands/mod.rs` so it compiles.** These are
fleshed out in Task 2; for now write compiling minimal versions.

Write to `hypergraph-rs/crates/hypergraph-rs-cli/src/error.rs`:

```rust
//! CLI error type and exit-code contract (spec §8.4).

use std::fmt;

/// A CLI error, carrying the process exit code it maps to.
#[derive(Debug)]
pub enum AppError {
    /// Generic error — spec §8.4 exit 1: "file not found, parse error" plus
    /// application-level usage errors (unknown algorithm/generator, invalid
    /// `--params`). NOTE: clap-level parse/usage errors (unknown flag, missing
    /// required arg, missing subcommand) never reach this type — clap handles
    /// them inside `Cli::parse()` and exits with its own default code 2.
    General(String),
    /// Validation failure (`validate`) — exit 2.
    Validation(String),
    /// Conformance failure (`selftest`) — exit 3.
    Conformance(String),
}

impl AppError {
    /// The process exit code for this error (spec §8.4).
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::General(_) => 1,
            AppError::Validation(_) => 2,
            AppError::Conformance(_) => 3,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::General(m) | AppError::Validation(m) | AppError::Conformance(m) => {
                write!(f, "{m}")
            }
        }
    }
}
```

Write to `hypergraph-rs/crates/hypergraph-rs-cli/src/io.rs`:

```rust
//! I/O helpers: read a hypergraph, write output (file or `-` = stdout).

// Filled in by Task 2.
```

Write to `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/mod.rs`:

```rust
//! Subcommand dispatch. Handlers land in Tasks 3–10.

use crate::cli::Command;
use crate::error::AppError;

/// Route a parsed command to its handler.
pub fn dispatch(command: Command) -> Result<(), AppError> {
    match command {
        Command::Inspect { .. }
        | Command::Convert { .. }
        | Command::Validate { .. }
        | Command::Render { .. }
        | Command::Matrix { .. }
        | Command::Run { .. }
        | Command::Generate { .. }
        | Command::Selftest { .. } => Ok(()),
    }
}
```

- [ ] **Step 9: Verify it passes.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_cli_skeleton`
Expected: PASS — 4 tests. (Handlers are no-ops; the smoke test only checks the parse
surface.)

- [ ] **Step 10: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs-cli): clap command surface + smoke harness (8 subcommands)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 2: Error mapping + I/O helpers (exit codes, `--output -` streaming)

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs-cli/src/error.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs-cli/src/io.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs-cli/src/main.rs` (add `#[cfg(test)]` unit mod
  is NOT needed — tests go in `io.rs`/`error.rs` inline test modules)

**Interfaces:**
- `AppError::General/Validation/Conformance` + `From<hypergraph_rs::readwrite::ReadWriteError>`
  and `From<std::io::Error>` (both → `General`, exit 1) — spec §8.4: "file not found, parse
  error" are generic (1).
- `io::read_hypergraph(input: &Path, format: Format) -> Result<Hypergraph, AppError>`.
- `io::write_string_output(output: &str, content: &str) -> Result<(), AppError>`.
- `io::write_via_path<F>(output: &str, writer: F) -> Result<(), AppError>` where
  `F: FnOnce(&Path) -> Result<(), AppError>` — routes real paths directly and `-` through a
  tempfile streamed to stdout, so the core's path-based `readwrite` writers are reused
  verbatim (no re-implemented serialization).

- [ ] **Step 1: Write the failing unit tests.** Append to
`hypergraph-rs/crates/hypergraph-rs-cli/src/io.rs` a test module (written now, implemented
next step):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Format;

    #[test]
    fn read_missing_file_is_general_error() {
        let err = read_hypergraph(std::path::Path::new("/no/such/file.json"), Format::Json)
            .unwrap_err();
        assert_eq!(err.exit_code(), 1);
    }

    #[test]
    fn write_string_output_to_file_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt");
        write_string_output(path.to_str().unwrap(), "hello\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello\n");
    }

    #[test]
    fn write_via_path_dash_uses_tempfile_and_does_not_write_named_file() {
        // `-` must not create a file literally named "-".
        let _ = std::fs::remove_file("-");
        write_via_path("-", |p| {
            std::fs::write(p, b"x").map_err(AppError::from)
        })
        .unwrap();
        assert!(!std::path::Path::new("-").exists());
    }
}
```

- [ ] **Step 2: Verify it fails.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --lib`
Expected: FAIL to compile (`read_hypergraph`, `write_string_output`, `write_via_path` do
not exist yet).

- [ ] **Step 3: Implement the `From` conversions in `error.rs`.** Append to
`hypergraph-rs/crates/hypergraph-rs-cli/src/error.rs`:

```rust
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::General(e.to_string())
    }
}

impl From<hypergraph_rs::readwrite::ReadWriteError> for AppError {
    fn from(e: hypergraph_rs::readwrite::ReadWriteError) -> Self {
        // Spec §8.4: parse / file errors are the generic (exit-1) class.
        AppError::General(e.to_string())
    }
}
```

- [ ] **Step 4: Implement `io.rs`.** Replace the placeholder body of
`hypergraph-rs/crates/hypergraph-rs-cli/src/io.rs` (keep the `#[cfg(test)] mod tests` from
Step 1 at the bottom):

```rust
//! I/O helpers: read a hypergraph, write output (file or `-` = stdout).

use std::io::Write;
use std::path::Path;

use hypergraph_rs::readwrite::edgelist::read_edgelist;
use hypergraph_rs::readwrite::hif::read_hif;
use hypergraph_rs::readwrite::incidence::read_incidence_matrix;
use hypergraph_rs::readwrite::json::read_json;
use hypergraph_rs::Hypergraph;

use crate::cli::Format;
use crate::error::AppError;

/// Read a hypergraph from `input`, dispatching on the declared `format`.
/// All four readers are the already-cited Phase-4 `readwrite` functions.
pub fn read_hypergraph(input: &Path, format: Format) -> Result<Hypergraph, AppError> {
    let h = match format {
        Format::Json => read_json(input)?,
        // `#` comment prefix + auto-detected delimiter (XGI edgelist default).
        Format::Edgelist => read_edgelist(input, None, "#")?,
        Format::Hif => read_hif(input)?,
        Format::IncidenceMatrix => read_incidence_matrix(input, None)?,
    };
    Ok(h)
}

/// Write string `content` to `output`, or to stdout when `output == "-"`
/// (spec §8.3 principle 2 — streaming for piping).
pub fn write_string_output(output: &str, content: &str) -> Result<(), AppError> {
    if output == "-" {
        let mut out = std::io::stdout().lock();
        out.write_all(content.as_bytes())?;
        out.flush()?;
    } else {
        std::fs::write(output, content)?;
    }
    Ok(())
}

/// Run a path-based writer against `output`. For a real path the writer targets
/// it directly; for `-` the writer targets a tempfile whose contents are then
/// streamed to stdout. This reuses the core `readwrite` path writers verbatim.
pub fn write_via_path<F>(output: &str, writer: F) -> Result<(), AppError>
where
    F: FnOnce(&Path) -> Result<(), AppError>,
{
    if output == "-" {
        let tmp = tempfile::Builder::new()
            .prefix("hypergraph-rs-")
            .tempfile()?;
        writer(tmp.path())?;
        let content = std::fs::read_to_string(tmp.path())?;
        let mut out = std::io::stdout().lock();
        out.write_all(content.as_bytes())?;
        out.flush()?;
    } else {
        writer(Path::new(output))?;
    }
    Ok(())
}
```

**Ground-truth check before relying on the `use` paths above.** The exact public module
paths for the readwrite readers must match what Phase 4 landed. Phase 4 Task 1 declares
`pub mod json; pub mod edgelist; pub mod incidence; pub mod hif;` inside
`readwrite/mod.rs`, and Tasks 13–17 define `read_json`, `read_edgelist`,
`read_incidence_matrix`, `read_hif` in those modules. Before this step, grep to confirm the
re-export shape actually landed (the plan may also re-export them at `readwrite::` top
level):

```bash
rg -n "pub use|pub fn read_(json|edgelist|hif|incidence)" \
  hypergraph-rs/crates/hypergraph-rs/src/readwrite/
```

If the functions are re-exported at the `readwrite` root (e.g.
`pub use json::{read_json, write_json};`), prefer `use hypergraph_rs::readwrite::read_json;`
etc. If only the submodule form exists, use the `readwrite::json::read_json` form as
written. Adjust the `use` lines to the landed shape; do not invent a path.

- [ ] **Step 5: Verify it passes.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --lib`
Expected: PASS — 3 unit tests.

- [ ] **Step 6: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs-cli): AppError exit codes + I/O helpers (stdout streaming)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 3: `inspect`

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/inspect.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/tests/test_inspect.rs`

**Interfaces:** `inspect::run(input: &Path, format: InspectFormat) -> Result<(), AppError>`.

**Behavior (spec §8.2 example):** read the JSON hypergraph, print a stats summary. Fields:
`Nodes` (`num_nodes`), `Hyperedges` (`num_edges`), `Density`
(`algorithms::properties::density(h, None, None, false)` — cited Phase 3 Task 5,
`properties.py:349-427`), and `Membership stats` = per-edge member counts
(min/max/mean/median of `stats::edge_size(h, None)` — cited Phase 4 Task 11,
`edgestats.py`). Membership = "how many members each hyperedge has" (the spec example's
`max=87` for 18 hyperedges can only be an edge size, not a node membership count). `--format
json` emits the same numbers as a JSON object for machine consumption.

- [ ] **Step 1: Write the failing test.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/tests/test_inspect.rs`:

```rust
mod common;

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("hypergraph-rs").unwrap()
}

#[test]
fn inspect_text_reports_counts() {
    bin()
        .args(["inspect", common::fixture("community.json").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nodes:").and(predicate::str::contains("5")))
        .stdout(predicate::str::contains("Hyperedges:").and(predicate::str::contains("3")));
}

#[test]
fn inspect_json_is_valid_json_with_counts() {
    let assert = bin()
        .args([
            "inspect",
            common::fixture("community.json").to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: serde_json::Value = serde_json::from_str(out.trim()).expect("valid json");
    assert_eq!(v["num_nodes"], 5);
    assert_eq!(v["num_edges"], 3);
}

#[test]
fn inspect_missing_file_exits_1() {
    bin()
        .args(["inspect", "/no/such/file.json"])
        .assert()
        .code(1);
}
```

- [ ] **Step 2: Verify it fails.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_inspect`
Expected: FAIL (handler is a no-op; prints nothing).

- [ ] **Step 3: Implement `inspect.rs`.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/src/commands/inspect.rs`:

```rust
//! `inspect` — print a stats summary of a hypergraph file (spec §8.2).

use std::path::Path;

use hypergraph_rs::algorithms::properties::density;
use hypergraph_rs::readwrite::json::read_json;
use hypergraph_rs::stats::edge_size;
use hypergraph_rs::Hypergraph;

use crate::cli::InspectFormat;
use crate::error::AppError;

/// Summary of edge-size (membership) distribution.
struct SizeStats {
    min: usize,
    max: usize,
    mean: f64,
    median: f64,
}

/// Compute min/max/mean/median of per-edge member counts.
fn size_stats(h: &Hypergraph) -> Option<SizeStats> {
    let mut sizes = edge_size(h, None).aslist();
    if sizes.is_empty() {
        return None;
    }
    sizes.sort_unstable();
    let n = sizes.len();
    let sum: usize = sizes.iter().sum();
    let mean = sum as f64 / n as f64;
    let median = if n % 2 == 1 {
        sizes[n / 2] as f64
    } else {
        (sizes[n / 2 - 1] + sizes[n / 2]) as f64 / 2.0
    };
    Some(SizeStats {
        min: sizes[0],
        max: sizes[n - 1],
        mean,
        median,
    })
}

/// `inspect` handler.
pub fn run(input: &Path, format: InspectFormat) -> Result<(), AppError> {
    let h = read_json(input)?;
    let num_nodes = h.num_nodes();
    let num_edges = h.num_edges();
    // density is undefined for an empty hypergraph (it panics on 0 nodes);
    // report 0.0 for the empty case rather than aborting.
    let density_val = if num_nodes == 0 {
        0.0
    } else {
        density(&h, None, None, false)
    };
    let stats = size_stats(&h);

    match format {
        InspectFormat::Text => {
            let name = h
                .graph_attr("name")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| input.display().to_string());
            println!("Hypergraph: {name}");
            println!("  Nodes:           {num_nodes}");
            println!("  Hyperedges:      {num_edges}");
            println!("  Density:         {density_val:.2}");
            match stats {
                Some(s) => println!(
                    "  Membership stats: min={}, max={}, mean={:.1}, median={}",
                    s.min, s.max, s.mean, s.median
                ),
                None => println!("  Membership stats: (no hyperedges)"),
            }
        }
        InspectFormat::Json => {
            let mut obj = serde_json::Map::new();
            obj.insert("num_nodes".into(), num_nodes.into());
            obj.insert("num_edges".into(), num_edges.into());
            obj.insert(
                "density".into(),
                serde_json::Number::from_f64(density_val)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null),
            );
            if let Some(s) = stats {
                obj.insert("membership_min".into(), s.min.into());
                obj.insert("membership_max".into(), s.max.into());
                obj.insert(
                    "membership_mean".into(),
                    serde_json::Number::from_f64(s.mean)
                        .map(serde_json::Value::Number)
                        .unwrap_or(serde_json::Value::Null),
                );
                obj.insert(
                    "membership_median".into(),
                    serde_json::Number::from_f64(s.median)
                        .map(serde_json::Value::Number)
                        .unwrap_or(serde_json::Value::Null),
                );
            }
            println!("{}", serde_json::Value::Object(obj));
        }
    }
    Ok(())
}
```

**Ground-truth check:** `density` is imported from the **submodule** path
`algorithms::properties::density` — Phase 3's `algorithms/mod.rs` declares only `pub mod`
submodules (`pub mod properties; pub mod connected; pub mod centrality; pub mod clustering;
...`) and does NOT re-export their functions at the `algorithms` root, so a root-level
`use hypergraph_rs::algorithms::density;` would fail with E0432. `density` lives in
`properties` (Phase 3 Task 5, `properties.py:349-427`). `stats::edge_size` IS re-exported at
the `stats` root (Phase 4 Task 11 defines it in `stats::edgestats`). Grep to confirm before
relying on the `use` lines:

```bash
rg -n "pub use|pub fn density|pub fn edge_size" \
  hypergraph-rs/crates/hypergraph-rs/src/algorithms/ \
  hypergraph-rs/crates/hypergraph-rs/src/stats/
```

If `edge_size` is only at `stats::edgestats::edge_size` (no root re-export landed), use that
path; keep the `algorithms::properties::density` submodule path unless a root re-export was
added. Adjust to the landed shape.

- [ ] **Step 4: Wire into `dispatch`.** Edit
`hypergraph-rs/crates/hypergraph-rs-cli/src/commands/mod.rs` — add `pub mod inspect;` at the
top and replace the `Inspect { .. }` arm:

```rust
        Command::Inspect { input, format } => inspect::run(&input, format),
```

(Leave the other seven arms in the combined `Ok(())` fallthrough for now; each subsequent
task peels one off.)

- [ ] **Step 5: Verify it passes.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_inspect`
Expected: PASS — 3 tests.

- [ ] **Step 6: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs-cli): inspect subcommand (text + json summary)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 4: `convert`

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/convert.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/tests/test_convert.rs`

**Interfaces:** `convert::run(input, from, to, output) -> Result<(), AppError>`.

**Behavior:** read `input` via the `from` reader (`io::read_hypergraph`), then write via the
`to` format's path-based `readwrite` writer routed through `io::write_via_path` (so `-`
streams to stdout). Writers (all Phase 4, cited): `write_json`, `write_edgelist(_, _, ' ')`,
`write_hif`, `write_incidence_matrix(_, _, ' ')`. The edgelist/incidence writers use a space
delimiter (XGI default; `write_edgelist` default delimiter is a single space).

- [ ] **Step 1: Write the failing test.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/tests/test_convert.rs`:

```rust
mod common;

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("hypergraph-rs").unwrap()
}

#[test]
fn convert_json_to_edgelist_stdout() {
    // 3 edges → 3 non-empty lines on stdout.
    let assert = bin()
        .args([
            "convert",
            common::fixture("community.json").to_str().unwrap(),
            "--from",
            "json",
            "--to",
            "edgelist",
            "--output",
            "-",
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 3, "expected 3 edgelist lines, got {out:?}");
}

#[test]
fn convert_json_to_json_file_round_trips_counts() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.json");
    bin()
        .args([
            "convert",
            common::fixture("community.json").to_str().unwrap(),
            "--from",
            "json",
            "--to",
            "json",
            "--output",
            out.to_str().unwrap(),
        ])
        .assert()
        .success();
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(v["type"], "hypergraph");
    assert_eq!(v["edge-dict"].as_object().unwrap().len(), 3);
}

#[test]
fn convert_missing_input_exits_1() {
    bin()
        .args([
            "convert", "/no/such.json", "--from", "json", "--to", "hif", "--output", "-",
        ])
        .assert()
        .code(1);
}
```

- [ ] **Step 2: Verify it fails.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_convert`
Expected: FAIL.

- [ ] **Step 3: Implement `convert.rs`.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/src/commands/convert.rs`:

```rust
//! `convert` — read one format, write another (spec §8.1).

use std::path::Path;

use hypergraph_rs::readwrite::edgelist::write_edgelist;
use hypergraph_rs::readwrite::hif::write_hif;
use hypergraph_rs::readwrite::incidence::write_incidence_matrix;
use hypergraph_rs::readwrite::json::write_json;

use crate::cli::Format;
use crate::error::AppError;
use crate::io::{read_hypergraph, write_via_path};

/// `convert` handler.
pub fn run(input: &Path, from: Format, to: Format, output: &str) -> Result<(), AppError> {
    let h = read_hypergraph(input, from)?;
    write_via_path(output, |path| match to {
        Format::Json => write_json(&h, path).map_err(AppError::from),
        Format::Edgelist => write_edgelist(&h, path, ' ').map_err(AppError::from),
        Format::Hif => write_hif(&h, path).map_err(AppError::from),
        Format::IncidenceMatrix => {
            write_incidence_matrix(&h, path, ' ').map_err(AppError::from)
        }
    })
}
```

**Ground-truth check:** confirm the writer paths/signatures match Phase 4 (Tasks 14/16/17
define `write_edgelist(h, path, delimiter: char)`, `write_incidence_matrix(h, path,
delimiter: char)`, `write_hif(h, path)`, `write_json(h, path)`). Grep:

```bash
rg -n "pub fn write_(json|edgelist|hif|incidence_matrix)" \
  hypergraph-rs/crates/hypergraph-rs/src/readwrite/
```

Adjust `use` paths to the landed re-export shape if the functions are re-exported at
`readwrite::` root.

- [ ] **Step 4: Wire dispatch.** Add `pub mod convert;` and replace the `Convert` arm:

```rust
        Command::Convert { input, from, to, output } => convert::run(&input, from, to, &output),
```

- [ ] **Step 5: Verify it passes.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_convert`
Expected: PASS — 3 tests.

- [ ] **Step 6: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs-cli): convert subcommand (json/edgelist/hif/incidence)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 5: `validate`

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/validate.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/tests/test_validate.rs`

**Interfaces:** `validate::run(input: &Path, strict: bool) -> Result<(), AppError>`.

**Behavior (deviation 1, declared above):** parse the file (parse failure → exit 1 via
`AppError::General`). Non-strict: any successful parse is valid → print the success line
(spec §8.2) and exit 0. Strict: additionally check for empty edges
(`EdgeView::empty`), duplicate edges (`EdgeView::duplicates`), and isolated nodes
(`NodeView::isolates`) — all cited Phase 2 Tasks 7–8. If any are found, print the findings
to stderr and return `AppError::Validation` (exit 2).

- [ ] **Step 1: Write the failing test.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/tests/test_validate.rs`:

```rust
mod common;

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("hypergraph-rs").unwrap()
}

#[test]
fn validate_clean_fixture_ok() {
    bin()
        .args(["validate", common::fixture("community.json").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK:"));
}

#[test]
fn validate_strict_clean_fixture_ok() {
    // community.json has no empty/duplicate edges and no isolated nodes.
    bin()
        .args([
            "validate",
            common::fixture("community.json").to_str().unwrap(),
            "--strict",
        ])
        .assert()
        .success();
}

#[test]
fn validate_strict_isolated_node_exits_2() {
    // A node "99" that belongs to no edge → isolated → strict failure (exit 2).
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("iso.json");
    std::fs::write(
        &p,
        r#"{"type":"hypergraph","hypergraph-data":{},
            "node-data":{"1":{},"2":{},"99":{}},
            "edge-data":{"e0":{}},
            "edge-dict":{"e0":["1","2"]}}"#,
    )
    .unwrap();
    bin()
        .args(["validate", p.to_str().unwrap(), "--strict"])
        .assert()
        .code(2);
}

#[test]
fn validate_parse_error_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("bad.json");
    std::fs::write(&p, "{ not json").unwrap();
    bin().args(["validate", p.to_str().unwrap()]).assert().code(1);
}
```

- [ ] **Step 2: Verify it fails.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_validate`
Expected: FAIL.

- [ ] **Step 3: Implement `validate.rs`.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/src/commands/validate.rs`:

```rust
//! `validate` — check a hypergraph file against XGI-style semantics (spec §8.2).
//!
//! Non-strict validation succeeds for any file that parses. Strict validation
//! additionally rejects empty edges, duplicate edges, and isolated nodes.

use std::path::Path;

use hypergraph_rs::readwrite::json::read_json;

use crate::error::AppError;

/// `validate` handler.
pub fn run(input: &Path, strict: bool) -> Result<(), AppError> {
    let h = read_json(input)?; // parse failure → AppError::General (exit 1)
    let num_nodes = h.num_nodes();
    let num_edges = h.num_edges();

    if strict {
        let empty = h.edges().empty().ids().to_vec();
        let dups = h.edges().duplicates().ids().to_vec();
        let isolates = h.nodes().isolates().ids().to_vec();
        let mut findings: Vec<String> = Vec::new();
        if !empty.is_empty() {
            findings.push(format!("{} empty edge(s): {:?}", empty.len(), empty));
        }
        if !dups.is_empty() {
            findings.push(format!("{} duplicate edge(s): {:?}", dups.len(), dups));
        }
        if !isolates.is_empty() {
            findings.push(format!(
                "{} isolated node(s): {:?}",
                isolates.len(),
                isolates
            ));
        }
        if !findings.is_empty() {
            for f in &findings {
                eprintln!("VIOLATION: {f}");
            }
            return Err(AppError::Validation(format!(
                "{} semantic violation class(es)",
                findings.len()
            )));
        }
    }

    println!("OK: {num_nodes} nodes, {num_edges} hyperedges, no semantic violations.");
    Ok(())
}
```

**Ground-truth check:** the view accessors `h.edges()` / `h.nodes()` and the filters
`.empty()`, `.duplicates()`, `.isolates()`, and `.ids()` are Phase 2 Tasks 7–8
(`NodeView::isolates`, `EdgeView::empty`, `EdgeView::duplicates`, `.ids() -> &[String]`).
Grep to confirm the method names before relying on them:

```bash
rg -n "pub fn (nodes|edges|isolates|duplicates|empty|ids)\b" \
  hypergraph-rs/crates/hypergraph-rs/src/core/
```

If a filter has a different name (e.g. `singletons` instead of `empty` for zero-member
edges — note that in the bipartite core an "empty" edge has zero members while a "singleton"
has exactly one), adjust to the landed name and update the finding label accordingly. Since
the core silently creates empty edges (declared deviation, Phase 0–1), `empty` is the
correct 0-member check; keep the grep honest.

- [ ] **Step 4: Wire dispatch.** Add `pub mod validate;` and replace the `Validate` arm:

```rust
        Command::Validate { input, strict } => validate::run(&input, strict),
```

- [ ] **Step 5: Verify it passes.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_validate`
Expected: PASS — 4 tests.

- [ ] **Step 6: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs-cli): validate subcommand (strict = exit 2)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 6: `matrix`

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/matrix.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/tests/test_matrix.rs`

**Interfaces:** `matrix::run(input, kind, format, output) -> Result<(), AppError>`.

**Behavior:** read JSON, compute `linalg::incidence_matrix(&h, None)` (rows=nodes,
cols=edges — Phase 3 Task 1) or `linalg::adjacency_matrix(&h, None, 1, false)` (node×node,
`s=1`, unweighted — Phase 3 Task 2). `--format dense` → comma-separated numeric rows.
`--format sparse` → one `row_id,col_id,value` line per nonzero (row/col ids are the returned
node/edge id vectors, in insertion order). Route through `io::write_string_output` (`-` =
stdout). Values print as integers when whole, else as `{:.6}` (incidence/unweighted
adjacency are 0/1 integers; the general path stays exact).

- [ ] **Step 1: Write the failing test.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/tests/test_matrix.rs`:

```rust
mod common;

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("hypergraph-rs").unwrap()
}

#[test]
fn incidence_dense_has_row_per_node() {
    // 5 nodes → 5 rows; each row has 3 comma-separated cells (3 edges).
    let assert = bin()
        .args([
            "matrix",
            common::fixture("community.json").to_str().unwrap(),
            "--kind",
            "incidence",
            "--format",
            "dense",
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let rows: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(rows.len(), 5, "expected 5 node rows, got {out:?}");
    assert_eq!(rows[0].split(',').count(), 3, "expected 3 edge columns");
}

#[test]
fn incidence_sparse_lists_nonzeros() {
    // Total memberships in the fixture: |e0|=3 + |e1|=2 + |e2|=3 = 8 nonzeros.
    let assert = bin()
        .args([
            "matrix",
            common::fixture("community.json").to_str().unwrap(),
            "--format",
            "sparse",
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 8, "expected 8 sparse entries, got {out:?}");
}

#[test]
fn adjacency_dense_is_square_over_nodes() {
    let assert = bin()
        .args([
            "matrix",
            common::fixture("community.json").to_str().unwrap(),
            "--kind",
            "adjacency",
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let rows: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].split(',').count(), 5, "adjacency is node×node");
}
```

- [ ] **Step 2: Verify it fails.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_matrix`
Expected: FAIL.

- [ ] **Step 3: Implement `matrix.rs`.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/src/commands/matrix.rs`:

```rust
//! `matrix` — compute the incidence or adjacency matrix (spec §8.1).

use std::path::Path;

use hypergraph_rs::linalg::{adjacency_matrix, incidence_matrix};
use hypergraph_rs::readwrite::json::read_json;
use ndarray::Array2;

use crate::cli::{MatrixFormat, MatrixKind};
use crate::error::AppError;
use crate::io::write_string_output;

/// Format one f64 as an integer when whole, else with 6 decimals.
fn fmt_val(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v:.6}")
    }
}

/// Dense CSV: one row per matrix row, comma-separated values.
fn dense(m: &Array2<f64>) -> String {
    let mut out = String::new();
    for row in m.rows() {
        let cells: Vec<String> = row.iter().map(|&v| fmt_val(v)).collect();
        out.push_str(&cells.join(","));
        out.push('\n');
    }
    out
}

/// Sparse triplets: `row_id,col_id,value` per nonzero, insertion order.
fn sparse(m: &Array2<f64>, row_ids: &[String], col_ids: &[String]) -> String {
    let mut out = String::new();
    let (nr, nc) = (m.nrows(), m.ncols());
    // Bounded: exactly nr*nc cells scanned.
    for i in 0..nr {
        for j in 0..nc {
            let v = m[[i, j]];
            if v != 0.0 {
                out.push_str(&format!("{},{},{}\n", row_ids[i], col_ids[j], fmt_val(v)));
            }
        }
    }
    out
}

/// `matrix` handler.
pub fn run(
    input: &Path,
    kind: MatrixKind,
    format: MatrixFormat,
    output: &str,
) -> Result<(), AppError> {
    let h = read_json(input)?;
    // (matrix, row_ids, col_ids)
    let (m, row_ids, col_ids) = match kind {
        MatrixKind::Incidence => {
            let (m, rows, cols) = incidence_matrix(&h, None);
            (m, rows, cols)
        }
        MatrixKind::Adjacency => {
            // s=1, unweighted; adjacency is square over nodes → col_ids == row_ids.
            let (m, rows) = adjacency_matrix(&h, None, 1, false);
            let cols = rows.clone();
            (m, rows, cols)
        }
    };
    let content = match format {
        MatrixFormat::Dense => dense(&m),
        MatrixFormat::Sparse => sparse(&m, &row_ids, &col_ids),
    };
    write_string_output(output, &content)
}
```

**Ground-truth check:** `linalg::incidence_matrix(&h, None) -> (Array2<f64>, Vec<String>,
Vec<String>)` and `linalg::adjacency_matrix(&h, None, 1, false) -> (Array2<f64>,
Vec<String>)` are Phase 3 Tasks 1–2. Confirm the re-export path and arity:

```bash
rg -n "pub fn (incidence_matrix|adjacency_matrix)\b|pub use" \
  hypergraph-rs/crates/hypergraph-rs/src/linalg/
```

- [ ] **Step 4: Wire dispatch.** Add `pub mod matrix;` and replace the `Matrix` arm:

```rust
        Command::Matrix { input, kind, format, output } => {
            matrix::run(&input, kind, format, &output)
        }
```

- [ ] **Step 5: Verify it passes.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_matrix`
Expected: PASS — 3 tests.

- [ ] **Step 6: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs-cli): matrix subcommand (incidence/adjacency, dense/sparse)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 7: `run`

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/run.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/tests/test_run.rs`

**Interfaces:** `run::run(input, algorithm, node) -> Result<(), AppError>`.

**Behavior:** read JSON, dispatch on the `--algorithm` string to a landed
`algorithms`/`stats` function, print results (one `id<TAB>value` per line for
per-node results, in insertion order; components one-per-line; scalars bare). Supported
names (unknown → `AppError::General`, exit 1):

| `--algorithm` | core call (cited; submodule path per review D1) |
|---|---|
| `components`, `connected-components` | `algorithms::connected::connected_components` (P3 T6) |
| `is-connected` | `algorithms::connected::is_connected` (P3 T6) |
| `num-components` | `algorithms::connected::number_connected_components` (P3 T6) |
| `density` | `algorithms::properties::density(h, None, None, false)` (P3 T5) |
| `degree` | `stats::node_degree(h, None)` (P4 T10) |
| `clustering` | `algorithms::clustering::clustering_coefficient` (P3 T8) |
| `katz` | `algorithms::centrality::katz_centrality(h, 3)` (P3 T10) |
| `clique-eigenvector`, `eigenvector` | `algorithms::centrality::clique_eigenvector_centrality(h, 1e-6, 100)` (P3 T10) |

`--node <id>` restricts a per-node result to that single node (error if absent).

- [ ] **Step 1: Write the failing test.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/tests/test_run.rs`:

```rust
mod common;

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("hypergraph-rs").unwrap()
}

#[test]
fn run_is_connected_prints_bool() {
    // community.json is a connected hypergraph (e0∩e1={2,3}, e1∩e2={3}).
    bin()
        .args([
            "run",
            common::fixture("community.json").to_str().unwrap(),
            "--algorithm",
            "is-connected",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("true"));
}

#[test]
fn run_degree_lists_all_nodes() {
    let assert = bin()
        .args([
            "run",
            common::fixture("community.json").to_str().unwrap(),
            "--algorithm",
            "degree",
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 5, "one line per node");
}

#[test]
fn run_degree_single_node() {
    // node "3" is in all three edges → degree 3.
    bin()
        .args([
            "run",
            common::fixture("community.json").to_str().unwrap(),
            "--algorithm",
            "degree",
            "--node",
            "3",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("3"));
}

#[test]
fn run_unknown_algorithm_exits_1() {
    bin()
        .args([
            "run",
            common::fixture("community.json").to_str().unwrap(),
            "--algorithm",
            "bogus",
        ])
        .assert()
        .code(1);
}
```

- [ ] **Step 2: Verify it fails.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_run`
Expected: FAIL.

- [ ] **Step 3: Implement `run.rs`.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/src/commands/run.rs`:

```rust
//! `run` — execute a named algorithm and print results (spec §8.1).

use std::path::Path;

use indexmap::IndexMap;

// `algorithms/mod.rs` re-exports nothing at its root (only `pub mod` submodules),
// so each function is imported from its concrete submodule path (review D1).
use hypergraph_rs::algorithms::centrality::{clique_eigenvector_centrality, katz_centrality};
use hypergraph_rs::algorithms::clustering::clustering_coefficient;
use hypergraph_rs::algorithms::connected::{
    connected_components, is_connected, number_connected_components,
};
use hypergraph_rs::algorithms::properties::density;
use hypergraph_rs::readwrite::json::read_json;
use hypergraph_rs::stats::node_degree;
use hypergraph_rs::Hypergraph;

use crate::error::AppError;

/// Print a per-node f64 map (optionally filtered to one node).
fn print_node_f64(map: &IndexMap<String, f64>, node: Option<&str>) -> Result<(), AppError> {
    match node {
        Some(n) => {
            let v = map
                .get(n)
                .ok_or_else(|| AppError::General(format!("node {n} not found")))?;
            println!("{n}\t{v}");
        }
        None => {
            for (id, v) in map {
                println!("{id}\t{v}");
            }
        }
    }
    Ok(())
}

/// `run` handler.
pub fn run(input: &Path, algorithm: &str, node: Option<String>) -> Result<(), AppError> {
    let h: Hypergraph = read_json(input)?;
    let node = node.as_deref();
    match algorithm {
        "components" | "connected-components" => {
            for comp in connected_components(&h) {
                println!("{}", comp.join(" "));
            }
        }
        "is-connected" => println!("{}", is_connected(&h)),
        "num-components" => println!("{}", number_connected_components(&h)),
        "density" => println!("{:.6}", density(&h, None, None, false)),
        "degree" => {
            let stat = node_degree(&h, None);
            match node {
                Some(n) => {
                    let v = stat
                        .get(n)
                        .ok_or_else(|| AppError::General(format!("node {n} not found")))?;
                    println!("{n}\t{v}");
                }
                None => {
                    for (id, v) in stat.asdict() {
                        println!("{id}\t{v}");
                    }
                }
            }
        }
        "clustering" => print_node_f64(&clustering_coefficient(&h), node)?,
        "katz" => print_node_f64(&katz_centrality(&h, 3), node)?,
        "clique-eigenvector" | "eigenvector" => {
            print_node_f64(&clique_eigenvector_centrality(&h, 1e-6, 100), node)?
        }
        other => {
            return Err(AppError::General(format!(
                "unknown algorithm '{other}' (try: components, is-connected, num-components, \
                 density, degree, clustering, katz, clique-eigenvector)"
            )))
        }
    }
    Ok(())
}
```

**Ground-truth check:** the `algorithms` functions are imported from their **submodule**
paths (`algorithms::connected::*`, `algorithms::centrality::*`, `algorithms::clustering::*`,
`algorithms::properties::density`) because Phase 3's `algorithms/mod.rs` declares only
`pub mod` submodules with NO root re-exports (review D1 — a root-level
`use hypergraph_rs::algorithms::density;` fails with E0432). Confirm every algorithm
signature matches Phase 3 (`connected_components -> Vec<Vec<String>>`, `is_connected -> bool`,
`number_connected_components -> usize`, `density(h, Option, Option, bool) -> f64`,
`clustering_coefficient(h) -> IndexMap<String,f64>`, `katz_centrality(h, cutoff: usize) ->
IndexMap<String,f64>`, `clique_eigenvector_centrality(h, tol: f64, max_iter: usize) ->
IndexMap<String,f64>`) and `stats::node_degree(h, Option<usize>) -> Stat<usize>` with
`.asdict()`/`.get()` (Phase 4). Grep for the submodule each function lives in, and confirm
`indexmap` is already a direct CLI dep (added to `[dependencies]` in Task 1 Step 1 — needed
to name `IndexMap` in `print_node_f64`'s signature):

```bash
rg -n "pub fn (katz_centrality|clique_eigenvector_centrality|clustering_coefficient|connected_components|is_connected|number_connected_components)\b" \
  hypergraph-rs/crates/hypergraph-rs/src/algorithms/
rg -n '^indexmap' hypergraph-rs/crates/hypergraph-rs-cli/Cargo.toml
```

If a function lives in a differently-named submodule than assumed, adjust the `use` path to
the landed shape. `indexmap.workspace = true` is already declared in Task 1 (indexmap 2 is a
workspace dependency); no Cargo change is needed in this task.

- [ ] **Step 4: Wire dispatch.** Add `pub mod run;` and replace the `Run` arm:

```rust
        Command::Run { input, algorithm, node } => run::run(&input, &algorithm, node),
```

- [ ] **Step 5: Verify it passes.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_run`
Expected: PASS — 4 tests.

- [ ] **Step 6: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs-cli): run subcommand (components/centrality/degree dispatch)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 8: `generate`

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/generate.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/tests/test_generate.rs`

**Interfaces:** `generate::run(generator, output, seed, params) -> Result<(), AppError>`.

**Behavior:** dispatch on `--generator`, parse `--params` (a JSON object; default `{}`),
call the landed Phase-4 generator, `write_json` the result to `--output`. Deterministic
generators consume `--seed`. Implemented set (deviation 4; unknown/invalid → exit 1):

| `--generator` | params | core call (cited, Phase 4) |
|---|---|---|
| `empty` | — | `empty_hypergraph()` (T3) |
| `trivial` | `n` | `trivial_hypergraph(n)` (T3) |
| `complete` | `n`, `order?`, `max_order?`, `include_singletons?` | `complete_hypergraph(...)` (T3) |
| `random` | `n`, `ps:[f64]`, `order:[usize]?` | `random_hypergraph(n,&ps,order,seed)` (T6) |
| `fast-random` | `n`, `ps:[f64]`, `order:[usize]?` | `fast_random_hypergraph(n,&ps,order,seed)` (T6) |
| `uniform-erdos-renyi` | `n`, `m`, `p`, `p_type?`(prob\|degree), `multiedges?` | `uniform_erdos_renyi_hypergraph(...)` (T8) |
| `star-clique` | `n_star`, `n_clique`, `d_max` | `star_clique(...)` (T5) |
| `ring-lattice` | `n`, `d`, `k`, `l` | `ring_lattice(...)` (T4) |
| `sunflower` | `l`, `c`, `m` | `sunflower(...)` (T5) |

- [ ] **Step 1: Write the failing test.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/tests/test_generate.rs`:

```rust
mod common;

use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("hypergraph-rs").unwrap()
}

fn read_counts(path: &std::path::Path) -> (usize, usize) {
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    (
        v["node-data"].as_object().unwrap().len(),
        v["edge-dict"].as_object().unwrap().len(),
    )
}

#[test]
fn generate_trivial_writes_n_nodes() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("t.json");
    bin()
        .args([
            "generate", "--generator", "trivial", "--params", r#"{"n":7}"#, "--output",
        ])
        .arg(&out)
        .assert()
        .success();
    let (nodes, edges) = read_counts(&out);
    assert_eq!(nodes, 7);
    assert_eq!(edges, 0);
}

#[test]
fn generate_random_is_seed_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.json");
    let b = dir.path().join("b.json");
    for out in [&a, &b] {
        bin()
            .args([
                "generate",
                "--generator",
                "random",
                "--seed",
                "42",
                "--params",
                r#"{"n":10,"ps":[0.5,0.2]}"#,
                "--output",
            ])
            .arg(out)
            .assert()
            .success();
    }
    assert_eq!(
        std::fs::read_to_string(&a).unwrap(),
        std::fs::read_to_string(&b).unwrap(),
        "same seed must produce byte-identical output (III.7 parity)"
    );
}

#[test]
fn generate_unknown_generator_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("x.json");
    bin()
        .args(["generate", "--generator", "nope", "--output"])
        .arg(&out)
        .assert()
        .code(1);
}
```

- [ ] **Step 2: Verify it fails.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_generate`
Expected: FAIL.

- [ ] **Step 3: Implement `generate.rs`.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/src/commands/generate.rs`:

```rust
//! `generate` — build a hypergraph from a named generator (spec §8.1).

use std::path::Path;

use serde_json::Value;

use hypergraph_rs::generators::{
    complete_hypergraph, empty_hypergraph, fast_random_hypergraph, random_hypergraph,
    ring_lattice, star_clique, sunflower, trivial_hypergraph, uniform_erdos_renyi_hypergraph,
    PType,
};
use hypergraph_rs::readwrite::json::write_json;
use hypergraph_rs::Hypergraph;

use crate::error::AppError;

/// Missing/invalid-parameter error (exit 1).
fn param_err(msg: impl Into<String>) -> AppError {
    AppError::General(msg.into())
}

/// Read a required usize field from the params object.
fn req_usize(p: &Value, key: &str) -> Result<usize, AppError> {
    p.get(key)
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .ok_or_else(|| param_err(format!("missing/invalid usize param '{key}'")))
}

/// Read a required f64 field.
fn req_f64(p: &Value, key: &str) -> Result<f64, AppError> {
    p.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| param_err(format!("missing/invalid f64 param '{key}'")))
}

/// Read an optional usize field.
fn opt_usize(p: &Value, key: &str) -> Option<usize> {
    p.get(key).and_then(Value::as_u64).map(|v| v as usize)
}

/// Read a required `[f64]` field.
fn req_f64_vec(p: &Value, key: &str) -> Result<Vec<f64>, AppError> {
    let arr = p
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| param_err(format!("missing/invalid array param '{key}'")))?;
    arr.iter()
        .map(|v| v.as_f64().ok_or_else(|| param_err(format!("'{key}' must be f64"))))
        .collect()
}

/// Read an optional `[usize]` field.
fn opt_usize_vec(p: &Value, key: &str) -> Option<Vec<usize>> {
    p.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_u64().map(|n| n as usize)).collect())
}

/// Build the hypergraph named by `generator` from `params`/`seed`.
fn build(generator: &str, p: &Value, seed: Option<u64>) -> Result<Hypergraph, AppError> {
    let map_gen = |r: Result<Hypergraph, hypergraph_rs::generators::GeneratorError>| {
        r.map_err(|e| param_err(e.to_string()))
    };
    match generator {
        "empty" => Ok(empty_hypergraph()),
        "trivial" => Ok(trivial_hypergraph(req_usize(p, "n")?)),
        "complete" => map_gen(complete_hypergraph(
            req_usize(p, "n")?,
            opt_usize(p, "order"),
            opt_usize(p, "max_order"),
            p.get("include_singletons").and_then(Value::as_bool).unwrap_or(false),
        )),
        "random" => {
            let n = req_usize(p, "n")?;
            let ps = req_f64_vec(p, "ps")?;
            let order = opt_usize_vec(p, "order");
            map_gen(random_hypergraph(n, &ps, order.as_deref(), seed))
        }
        "fast-random" => {
            let n = req_usize(p, "n")?;
            let ps = req_f64_vec(p, "ps")?;
            let order = opt_usize_vec(p, "order");
            map_gen(fast_random_hypergraph(n, &ps, order.as_deref(), seed))
        }
        "uniform-erdos-renyi" => {
            let ptype = match p.get("p_type").and_then(Value::as_str).unwrap_or("prob") {
                "prob" => PType::Prob,
                "degree" => PType::Degree,
                other => return Err(param_err(format!("bad p_type '{other}'"))),
            };
            map_gen(uniform_erdos_renyi_hypergraph(
                req_usize(p, "n")?,
                req_usize(p, "m")?,
                req_f64(p, "p")?,
                ptype,
                p.get("multiedges").and_then(Value::as_bool).unwrap_or(false),
                seed,
            ))
        }
        "star-clique" => map_gen(star_clique(
            req_usize(p, "n_star")?,
            req_usize(p, "n_clique")?,
            req_usize(p, "d_max")?,
        )),
        "ring-lattice" => map_gen(ring_lattice(
            req_usize(p, "n")?,
            req_usize(p, "d")?,
            req_usize(p, "k")?,
            req_usize(p, "l")?,
        )),
        "sunflower" => map_gen(sunflower(
            req_usize(p, "l")?,
            req_usize(p, "c")?,
            req_usize(p, "m")?,
        )),
        other => Err(param_err(format!("unknown generator '{other}'"))),
    }
}

/// `generate` handler.
pub fn run(
    generator: &str,
    output: &Path,
    seed: Option<u64>,
    params: Option<String>,
) -> Result<(), AppError> {
    let p: Value = match params {
        Some(s) => serde_json::from_str(&s)
            .map_err(|e| param_err(format!("--params is not valid JSON: {e}")))?,
        None => Value::Object(serde_json::Map::new()),
    };
    let h = build(generator, &p, seed)?;
    write_json(&h, output)?;
    Ok(())
}
```

**Ground-truth check:** confirm generator signatures/arities against Phase 4 (Tasks 3–8) —
in particular `random_hypergraph(n, ps: &[f64], order: Option<&[usize]>, seed: Option<u64>)`
and `uniform_erdos_renyi_hypergraph(n, m, p, PType, multiedges, seed)` — and that
`GeneratorError` and `PType` are re-exported at `generators::`. Grep:

```bash
rg -n "pub fn (complete_hypergraph|random_hypergraph|fast_random_hypergraph|uniform_erdos_renyi_hypergraph|star_clique|ring_lattice|sunflower|trivial_hypergraph|empty_hypergraph)\b|pub enum (PType|GeneratorError)|pub use" \
  hypergraph-rs/crates/hypergraph-rs/src/generators/
```

Adjust `order.as_deref()` / signatures to the landed shape. Note `random_hypergraph` takes
`order: Option<&[usize]>` (a slice), which `opt_usize_vec(..).as_deref()` supplies.

- [ ] **Step 4: Wire dispatch.** Add `pub mod generate;` and replace the `Generate` arm:

```rust
        Command::Generate { generator, output, seed, params } => {
            generate::run(&generator, &output, seed, params)
        }
```

- [ ] **Step 5: Verify it passes.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_generate`
Expected: PASS — 3 tests.

- [ ] **Step 6: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs-cli): generate subcommand (9 seeded generators, JSON params)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 9: `render`

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/render.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/tests/test_render.rs`

**Interfaces:** `render::run(input, output, layout, format, width, height) -> Result<(),
AppError>`.

**Behavior:** read JSON, map `--layout` → `LayoutStrategy` (Phase 5 Task 1 enum), call
`h.render_to_svg(strategy)` or `h.render_to_json(strategy)` (Phase 5 Tasks 9–10 — on the
concrete `Hypergraph<Value,Value,M>` impl), write the resulting string to `--output` (a
path; `-` streams to stdout via `write_string_output`). Layout → strategy mapping uses fixed
deterministic defaults (spring/hyperedge-aware seed at `DEFAULT_LAYOUT_SEED = 0`, Phase 5):

- `hyperedge-aware` → `HyperedgeAware { iterations: 100, halo_padding: 10.0 }`
- `spring` → `Spring { iterations: 100, seed: Some(0) }`
- `bipartite` → `Bipartite { align: BipartiteAlign::Vertical }`
- `radial` → `Radial { rings: 4 }`

`--width`/`--height` (deviation 2): if either is supplied, print a one-line stderr note that
sizing is auto-derived from the layout bounding box and the flags are currently advisory;
do not fail.

- [ ] **Step 1: Write the failing test.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/tests/test_render.rs`:

```rust
mod common;

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("hypergraph-rs").unwrap()
}

#[test]
fn render_svg_to_file() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("g.svg");
    bin()
        .args(["render", common::fixture("community.json").to_str().unwrap(), "--output"])
        .arg(&out)
        .assert()
        .success();
    let svg = std::fs::read_to_string(&out).unwrap();
    assert!(svg.contains("<svg"), "expected SVG markup");
}

#[test]
fn render_json_scene_graph_to_stdout() {
    let assert = bin()
        .args([
            "render",
            common::fixture("community.json").to_str().unwrap(),
            "--format",
            "json",
            "--layout",
            "spring",
            "--output",
            "-",
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: serde_json::Value = serde_json::from_str(out.trim()).expect("valid scene json");
    assert!(v.get("nodes").is_some(), "scene graph has nodes");
}

#[test]
fn render_width_height_are_advisory_not_fatal() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("g.svg");
    bin()
        .args([
            "render",
            common::fixture("community.json").to_str().unwrap(),
            "--width",
            "800",
            "--height",
            "600",
            "--output",
        ])
        .arg(&out)
        .assert()
        .success()
        .stderr(predicate::str::contains("advisory"));
}
```

- [ ] **Step 2: Verify it fails.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_render`
Expected: FAIL.

- [ ] **Step 3: Implement `render.rs`.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/src/commands/render.rs`:

```rust
//! `render` — lay out a hypergraph and emit SVG or scene-graph JSON (spec §8.1).

use std::path::Path;

use hypergraph_rs::layout::{BipartiteAlign, LayoutStrategy};
use hypergraph_rs::readwrite::json::read_json;
use hypergraph_rs::Hypergraph;

use crate::cli::{LayoutStrategyArg, RenderFormat};
use crate::error::AppError;
use crate::io::write_string_output;

/// Map the CLI layout arg to a concrete, deterministic `LayoutStrategy`.
fn strategy_for(arg: LayoutStrategyArg) -> LayoutStrategy {
    match arg {
        LayoutStrategyArg::HyperedgeAware => LayoutStrategy::HyperedgeAware {
            iterations: 100,
            halo_padding: 10.0,
        },
        LayoutStrategyArg::Spring => LayoutStrategy::Spring {
            iterations: 100,
            seed: Some(0),
        },
        LayoutStrategyArg::Bipartite => LayoutStrategy::Bipartite {
            align: BipartiteAlign::Vertical,
        },
        LayoutStrategyArg::Radial => LayoutStrategy::Radial { rings: 4 },
    }
}

/// `render` handler.
pub fn run(
    input: &Path,
    output: &Path,
    layout: LayoutStrategyArg,
    format: RenderFormat,
    width: Option<f32>,
    height: Option<f32>,
) -> Result<(), AppError> {
    if width.is_some() || height.is_some() {
        eprintln!(
            "note: --width/--height are advisory; SVG sizing is auto-derived from the \
             layout bounding box (render_to_svg takes no size args)."
        );
    }
    let h: Hypergraph = read_json(input)?;
    let strategy = strategy_for(layout);
    let content = match format {
        RenderFormat::Svg => h.render_to_svg(strategy),
        RenderFormat::Json => h.render_to_json(strategy),
    };
    let out_str = output
        .to_str()
        .ok_or_else(|| AppError::General("output path is not valid UTF-8".into()))?;
    write_string_output(out_str, &content)
}
```

**Ground-truth check:** confirm `LayoutStrategy` variants + field names (`Spring { iterations,
seed }`, `HyperedgeAware { iterations, halo_padding }`, `Bipartite { align }`, `Radial {
rings }`), `BipartiteAlign::Vertical`, and that `render_to_svg`/`render_to_json` are methods
on `impl<M> Hypergraph<serde_json::Value, serde_json::Value, M>` (Phase 5 Tasks 8–10). Grep:

```bash
rg -n "pub enum LayoutStrategy|pub enum BipartiteAlign|pub fn render_to_(svg|json)" \
  hypergraph-rs/crates/hypergraph-rs/src/layout/ \
  hypergraph-rs/crates/hypergraph-rs/src/viz/
```

If a field name differs (e.g. the enum evolved), fix `strategy_for` to the landed shape.

- [ ] **Step 4: Wire dispatch.** Add `pub mod render;` and replace the `Render` arm:

```rust
        Command::Render { input, output, layout, format, width, height } => {
            render::run(&input, &output, layout, format, width, height)
        }
```

- [ ] **Step 5: Verify it passes.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_render`
Expected: PASS — 3 tests.

- [ ] **Step 6: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs-cli): render subcommand (SVG + scene-graph JSON)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 10: `selftest`

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/selftest.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs-cli/src/commands/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs-cli/tests/test_selftest.rs`

**Interfaces:** `selftest::run(mode: SelftestMode) -> Result<(), AppError>`.

**Behavior:** run a fixed, in-process battery of core-invariant checks (no Python, no
external files) — this is a fast smoke of the built binary against its own core, NOT the
Phase-7 pytest conformance gate (that lives in the workspace-root `tests/`). Each check is a
`(name, || -> bool)` closure; count passes; print `PASS: k/n` on success (exit 0) or list
failures and return `AppError::Conformance` (exit 3, spec §8.4). `fast` runs the base set;
`full` runs the base set plus additional generator/layout determinism checks. The checks
exercise: construct+count, JSON round-trip, edgelist round-trip via a tempfile, incidence
matrix shape, connectivity, a seeded generator's determinism, and a render smoke.

- [ ] **Step 1: Write the failing test.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/tests/test_selftest.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("hypergraph-rs").unwrap()
}

#[test]
fn selftest_fast_passes() {
    bin()
        .args(["selftest", "--mode", "fast"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS:"));
}

#[test]
fn selftest_full_passes() {
    bin()
        .args(["selftest", "--mode", "full"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS:"));
}

#[test]
fn selftest_default_mode_is_fast_and_passes() {
    bin().arg("selftest").assert().success();
}
```

- [ ] **Step 2: Verify it fails.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_selftest`
Expected: FAIL.

- [ ] **Step 3: Implement `selftest.rs`.** Write to
`hypergraph-rs/crates/hypergraph-rs-cli/src/commands/selftest.rs`:

```rust
//! `selftest` — an in-process battery of core-invariant checks (spec §8.2).
//!
//! This is a fast confidence smoke of the built binary against its own core,
//! not the Phase-7 pytest conformance gate. Exit 3 on any failure (spec §8.4).

use hypergraph_rs::algorithms::connected::is_connected;
use hypergraph_rs::generators::random_hypergraph;
use hypergraph_rs::layout::LayoutStrategy;
use hypergraph_rs::linalg::incidence_matrix;
use hypergraph_rs::readwrite::edgelist::{read_edgelist, write_edgelist};
use hypergraph_rs::readwrite::json::{from_hypergraph_dict, to_hypergraph_dict};
use hypergraph_rs::Hypergraph;

use crate::cli::SelftestMode;
use crate::error::AppError;

/// A small connected sample hypergraph (2 overlapping edges).
fn sample() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    let _ = h.add_edge(vec!["1".into(), "2".into(), "3".into()], Some("e0".into()), serde_json::Value::Null);
    let _ = h.add_edge(vec!["3".into(), "4".into()], Some("e1".into()), serde_json::Value::Null);
    h
}

fn check_construct() -> bool {
    let h = sample();
    h.num_nodes() == 4 && h.num_edges() == 2
}

fn check_json_round_trip() -> bool {
    let h = sample();
    match from_hypergraph_dict(&to_hypergraph_dict(&h)) {
        Ok(h2) => h2.num_nodes() == 4 && h2.num_edges() == 2,
        Err(_) => false,
    }
}

fn check_edgelist_round_trip() -> bool {
    let h = sample();
    let tmp = match tempfile::Builder::new().suffix(".el").tempfile() {
        Ok(t) => t,
        Err(_) => return false,
    };
    if write_edgelist(&h, tmp.path(), ' ').is_err() {
        return false;
    }
    match read_edgelist(tmp.path(), None, "#") {
        Ok(h2) => h2.num_edges() == 2,
        Err(_) => false,
    }
}

fn check_incidence_shape() -> bool {
    let h = sample();
    let (m, rows, cols) = incidence_matrix(&h, None);
    rows.len() == 4 && cols.len() == 2 && m.shape() == [4, 2]
}

fn check_connected() -> bool {
    is_connected(&sample())
}

fn check_generator_determinism() -> bool {
    let a = random_hypergraph(8, &[0.5, 0.2], None, Some(7));
    let b = random_hypergraph(8, &[0.5, 0.2], None, Some(7));
    match (a, b) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn check_render_smoke() -> bool {
    let h = sample();
    let svg = h.render_to_svg(LayoutStrategy::Spring { iterations: 20, seed: Some(0) });
    svg.contains("<svg")
}

/// `selftest` handler.
pub fn run(mode: SelftestMode) -> Result<(), AppError> {
    // Base battery (fast).
    let mut checks: Vec<(&str, fn() -> bool)> = vec![
        ("construct", check_construct),
        ("json_round_trip", check_json_round_trip),
        ("edgelist_round_trip", check_edgelist_round_trip),
        ("incidence_shape", check_incidence_shape),
        ("connected", check_connected),
    ];
    // Extended battery (full).
    if matches!(mode, SelftestMode::Full) {
        checks.push(("generator_determinism", check_generator_determinism));
        checks.push(("render_smoke", check_render_smoke));
    }

    let mode_label = match mode {
        SelftestMode::Fast => "fast",
        SelftestMode::Full => "full",
    };
    println!("Running {} conformance checks ({mode_label} mode)...", checks.len());

    let mut failures: Vec<&str> = Vec::new();
    // Bounded: iterates the fixed `checks` vec.
    for (name, f) in &checks {
        if !f() {
            failures.push(name);
        }
    }

    if failures.is_empty() {
        println!("PASS: {}/{}", checks.len(), checks.len());
        Ok(())
    } else {
        for name in &failures {
            eprintln!("FAIL: {name}");
        }
        Err(AppError::Conformance(format!(
            "{}/{} checks failed",
            failures.len(),
            checks.len()
        )))
    }
}
```

**Ground-truth check:** confirm `from_hypergraph_dict`/`to_hypergraph_dict` are in
`readwrite::json` (Phase 4 Task 13), `random_hypergraph` derives `PartialEq` on `Hypergraph`
(Phase 0–1 core `impl PartialEq` — verified present in the landed `core/hypergraph.rs`), and
`incidence_matrix(&h, None) -> (Array2, Vec<String>, Vec<String>)` shape. Grep:

```bash
rg -n "pub fn (to|from)_hypergraph_dict\b" hypergraph-rs/crates/hypergraph-rs/src/readwrite/
```

- [ ] **Step 4: Wire dispatch.** Add `pub mod selftest;` and replace the final `Selftest`
arm; the combined fallthrough now disappears (all eight arms are explicit):

```rust
//! Subcommand dispatch.

use crate::cli::Command;
use crate::error::AppError;

pub mod convert;
pub mod generate;
pub mod inspect;
pub mod matrix;
pub mod render;
pub mod run;
pub mod selftest;
pub mod validate;

/// Route a parsed command to its handler.
pub fn dispatch(command: Command) -> Result<(), AppError> {
    match command {
        Command::Inspect { input, format } => inspect::run(&input, format),
        Command::Convert { input, from, to, output } => convert::run(&input, from, to, &output),
        Command::Validate { input, strict } => validate::run(&input, strict),
        Command::Render { input, output, layout, format, width, height } => {
            render::run(&input, &output, layout, format, width, height)
        }
        Command::Matrix { input, kind, format, output } => {
            matrix::run(&input, kind, format, &output)
        }
        Command::Run { input, algorithm, node } => run::run(&input, &algorithm, node),
        Command::Generate { generator, output, seed, params } => {
            generate::run(&generator, &output, seed, params)
        }
        Command::Selftest { mode } => selftest::run(mode),
    }
}
```

- [ ] **Step 5: Verify it passes.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli --test test_selftest`
Expected: PASS — 3 tests.

- [ ] **Step 6: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs-cli): selftest subcommand (in-process check battery, exit 3)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 11: clippy + fmt + `--release` build + full smoke gate

**Files:** any files with clippy warnings.

This is the closing gate — no new behavior, just verification (no publishing, per the owner
constraint).

- [ ] **Step 1: Clippy the CLI crate.** Run:
`cd hypergraph-rs && cargo clippy -p hypergraph-rs-cli -- -D warnings`
Expected: clean. Likely lints and preferred fixes:
- `clippy::needless_range_loop` in `matrix::sparse` (the `for i in 0..nr { for j in 0..nc`
  double loop indexes `m[[i,j]]` **and** `row_ids[i]`/`col_ids[j]` — three arrays by shared
  indices, so an iterator adaptor does not apply). Add
  `#[allow(clippy::needless_range_loop)]` on `sparse` with a one-line justification.
- `clippy::uninlined_format_args` anywhere `format!("{}", x)` could be `format!("{x}")` —
  inline them.
- No `type_complexity` is expected (the CLI names no complex return types).

- [ ] **Step 2: Clippy the whole workspace.** Run:
`cd hypergraph-rs && cargo clippy --workspace -- -D warnings`
Expected: clean (Phases 0–6 already clean; this confirms the CLI added no workspace lint).

- [ ] **Step 3: fmt.** Run: `cd hypergraph-rs && cargo fmt --all`

- [ ] **Step 4: Full CLI test suite.** Run:
`cd hypergraph-rs && cargo test -p hypergraph-rs-cli`
Expected: PASS — all integration binaries: `test_cli_skeleton` (4), `test_inspect` (3),
`test_convert` (3), `test_validate` (4), `test_matrix` (3), `test_run` (4),
`test_generate` (3), `test_render` (3), `test_selftest` (3), plus the `--lib` unit tests (3)
— 33 tests total.

- [ ] **Step 5: Release build + a real end-to-end smoke through the release binary.** Build
optimized and drive one representative command per subcommand against the built artifact
(NOT `cargo run` — exercise the actual release binary path). Run:

```bash
cd hypergraph-rs
cargo build --release -p hypergraph-rs-cli
BIN=target/release/hypergraph-rs
FIX=crates/hypergraph-rs-cli/tests/fixtures/community.json
"$BIN" --version
"$BIN" inspect "$FIX"
"$BIN" validate "$FIX" --strict
"$BIN" convert "$FIX" --from json --to edgelist --output -
"$BIN" matrix "$FIX" --kind adjacency --format sparse
"$BIN" run "$FIX" --algorithm is-connected
"$BIN" generate --generator random --seed 1 --params '{"n":6,"ps":[0.4]}' --output /tmp/hg_gen.json
"$BIN" render "$FIX" --format json --output -
"$BIN" selftest --mode full
echo "exit-code spot checks:"
"$BIN" inspect /no/such.json; echo "  inspect-missing => $? (expect 1)"
"$BIN" validate /tmp/hg_gen.json --strict; echo "  validate => $? (0 or 2)"
"$BIN" selftest --mode fast; echo "  selftest => $? (expect 0)"
```

Expected: every command succeeds; the missing-file inspect prints exit `1`. (This is a smoke
of the release artifact only — no install, no packaging, no publish.)

- [ ] **Step 6: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "chore(hypergraph-rs-cli): clippy + fmt + release smoke gate

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Phase 9 Completion Summary

After all 11 tasks, `hypergraph-rs-cli` provides the `hypergraph-rs`
binary with the spec §8 command surface:

- **8 subcommands** (`inspect`, `convert`, `validate`, `render`, `matrix`, `run`,
  `generate`, `selftest`), each a thin wrapper over already-built, already-cited Phase-0–6
  core functions (readwrite / linalg / algorithms / stats / generators / layout+viz), no
  re-implemented serialization or graph logic.
- **Exit-code contract** (spec §8.4): `0` success, `1` generic (file not found / parse error
  / application-level usage errors via `AppError::General`), `2` validation failure
  (`validate --strict`), `3` conformance failure (`selftest`) — via
  `AppError::{General, Validation, Conformance}::exit_code()`. Note: clap-level parse/usage
  errors (unknown flag, missing arg/subcommand) are caught inside `Cli::parse()` and exit with
  clap's default code 2 (documented deviation 7), colliding with the validation code by design.
- **`--output -` stdout streaming** for `convert` (path-writers routed through a tempfile)
  and `matrix`/`render` (string writers), spec §8.3 principle 2.
- **Smoke tests** for every subcommand through `assert_cmd` + `predicates`, plus unit tests
  for the I/O helpers, plus a release-binary end-to-end gate. `#![forbid(unsafe_code)]`,
  clippy `-D warnings` clean, all loops statically bounded.

**Declared deviations:** (1) validate semantics defined by us (XGI has no CLI); (2) `render
--width/--height` advisory-only (core `render_to_svg` auto-sizes); (3) `matrix` sparse derived
from the dense matrix; (4) `generate` implements a representative 9-generator subset via JSON
`--params`; (5) `run` algorithm hyperparameters are fixed defaults (katz cutoff = 3,
clique-eigenvector tol = 1e-6 / max_iter = 100), no tuning flags; (6) `selftest` is an
in-process invariant battery (5/7 checks), NOT the spec's 47-test conformance suite (that is
the Phase-7 pytest tree); (7) clap-level usage errors exit 2, not 1.

**Explicitly out of scope (owner):** all release/distribution — no `cargo-dist`, no
`cargo publish`, no binary uploads. Build + smoke only.

**Next: Phase 10** (optional `@hypergraph-rs/react` package) or the Phase-7 Python
conformance gate, per spec §10.3.
