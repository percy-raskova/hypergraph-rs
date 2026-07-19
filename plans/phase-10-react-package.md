# hypergraph-rs Phase 10 Implementation Plan — `@hypergraph-rs/react`

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the optional npm package `@hypergraph-rs/react` (spec §2, §7.4): a
**pure-TypeScript** React component library that renders a `SceneGraph` — the JSON
contract emitted by `hypergraph-rs::viz` (Phase 5) — as SVG, with **no WASM
dependency of its own** (it takes already-parsed JSON). The package ships a minimal
SVG renderer that handles every `HyperedgeGeometry` variant, TypeScript types that
mirror the serde wire format byte-for-byte, `package.json` with `react` as a peer
dependency, a `tsconfig`, and `vitest` component tests (jsdom). The packaging gate is
`npm pack --dry-run`. **NO npm publish** — publishing is an owner decision (spec §10.3
lists publish; this plan deliberately omits it).

**Position in the roadmap:** Phase 10 runs strictly AFTER Phases 0–9. It depends on
exactly ONE upstream artifact: the `SceneGraph` JSON shape defined by
`hypergraph-rs::viz` in Phase 5 (`crates/hypergraph-rs/src/viz/mod.rs` +
`crates/hypergraph-rs/src/layout/mod.rs`). It does **not** import the Rust core, the
WASM crate, or any cargo target — it is an independent Node/TypeScript sub-project
living at `npm/@hypergraph-rs/react/` inside the hypergraph-rs git repo. Spec §10.3
sizes this at 2–3 days.

**Tech stack (verified 2026-07-19):** node v25.9.0, npm 11.12.1. TypeScript ^5.7,
React (peer) `>=18`, dev React 19, `vitest` ^3 with `jsdom`, `@testing-library/react`
^16 (which requires `@testing-library/dom` ^10 as an explicit peer — declared as a
devDependency, not left to npm's auto-install-peers), `@vitejs/plugin-react` ^4. All version strings below are **install-time** ranges
resolved by `npm install`; this plan authors files only — it runs no npm/tsc/vitest
commands (a separate build workflow owns the machine).

---

## THE WIRE CONTRACT (the single source of truth this package consumes)

Everything this package renders is a `SceneGraph` JSON document. Its exact byte shape
is fixed by the serde derives on the Rust structs in the Phase 5 plan
(`plans/phase-5-layout-viz.md`). The TypeScript types in Task 1 are a **1-for-1
transcription** of that shape. The load-bearing facts:

1. **Struct field names are snake_case, verbatim.** No struct in `viz/mod.rs` or
   `layout/mod.rs` carries `#[serde(rename_all = ...)]` (phase-5 plan lines 405–468,
   383–388). Therefore the JSON keys are exactly the Rust field names:
   `bounding_box`, `member_ids`, `stroke_width`, `y_min`, `y_max`, `angle_range`,
   `control_points`.

2. **`HyperedgeGeometry` is externally tagged.** The enum has **no** `#[serde(tag=…)]`
   or `#[serde(untagged)]` attribute (phase-5 plan line 368:
   `#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]` — derives only).
   serde's **default** representation for an enum with struct variants is *external
   tagging* (serde.rs "Enum representations" — the default is a single-key object
   `{"VariantName": { …fields }}`). So the four variants serialize as:
   - `{"ConvexHull": {"points": [[x,y], …]}}`
   - `{"Spline": {"control_points": [[x,y], …]}}`
   - `{"Stripe": {"x": …, "y_min": …, "y_max": …}}`
   - `{"Arc": {"center": [x,y], "radius": …, "angle_range": [a0,a1]}}`

3. **Every Rust tuple becomes a JSON array.** serde serializes tuples as fixed-length
   sequences. So `(f64, f64)` → `[x, y]` and `bounding_box: (f64,f64,f64,f64)` →
   `[min_x, min_y, max_x, max_y]` (phase-5 plan line 466), and `angle_range`,
   `center` are 2-arrays.

4. **`attrs` and `metadata` are free-form JSON.** `attrs: serde_json::Value`
   (phase-5 plan lines 447, 457) → TS `unknown`; it may be `null` (the Rust `render`
   falls back to `serde_json::Value::Null` — phase-5 plan line 1767). `metadata:
   serde_json::Map<String, Value>` (line 467) → TS `Record<string, unknown>`.

5. **Style defaults** (phase-5 plan lines 412–438): `NodeStyle { fill "#1f77b4",
   stroke "#000000", stroke_width 1.0 }`; `HyperedgeStyle { fill "#ff7f0e", stroke
   "#ff7f0e", opacity 0.4 }`. These live in the JSON already (the Rust `render` writes
   them — lines 1774, 1790); the React layer reads them, it does not re-default them.

6. **Ordering:** `nodes` are insertion-ordered; `hyperedges` are edge-insertion
   ordered (phase-5 plan lines 1690–1695). The renderer preserves array order.

This is a **byte-level, language-agnostic contract** (per the repo's "contracts must
be language-agnostic to the byte" rule). Task 1 pins it with a JSON fixture that is a
hand-authored copy of what `serde_json::to_string(&scene)` emits, so a future change
to either side is caught by a failing test.

---

## Global Constraints

- **Pure TypeScript, no WASM.** This package imports nothing from
  `hypergraph-rs-wasm`, the Rust core, or any binary. Its only runtime input is a
  `SceneGraph` object (parsed JSON). (spec §7.4: "a separate pure-TypeScript package,
  no WASM dependency of its own — it just takes JSON".)
- **`react` is a peer dependency** (`>=18`), never a direct/bundled dependency — the
  consuming app owns the single React instance. `react-dom` is a **devDependency**
  only (needed to render in tests); consumers supply their own.
- **`strict` TypeScript**, `noUnusedLocals`/`noUnusedParameters` on, declaration
  emit on (`.d.ts` ships in the tarball). Every component has an explicit return type.
- **No `any`.** The geometry union narrows via the `in` operator on the variant key.
- **Bounded loops only.** The renderer's only loops are `Array.prototype.map` over
  `scene.nodes` and `scene.hyperedges` (bounded by the scene) and `.map` over a
  geometry's point arrays (bounded by that array). No unbounded `while`.
- **TDD:** every task is red → green → commit. Tests are `vitest` files colocated in
  `src/` as `*.test.ts` / `*.test.tsx`, excluded from the build tsconfig.
- **License:** BSD-3-Clause (matches the workspace).
- **Commit trailer** on every commit:

  ```
  Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
  ```

- **Commit scope:** `feat(react): …` / `test(react): …` / `chore(react): …`.
- **NO publishing.** No `npm publish`, no `wasm-pack publish`, no `gh release`. The
  packaging gate is `npm pack --dry-run` (lists the tarball, emits nothing).

---

## Directory layout produced by this phase

```
hypergraph-rs/
└── npm/
    └── @hypergraph-rs/
        └── react/
            ├── package.json
            ├── tsconfig.json                 # typecheck config (includes tests)
            ├── tsconfig.build.json           # emit config (excludes tests/fixtures)
            ├── vitest.config.ts
            ├── vitest.setup.ts
            ├── .gitignore
            ├── .npmignore
            ├── LICENSE                        # BSD-3-Clause (copied from workspace root)
            ├── README.md
            └── src/
                ├── index.ts                   # public barrel
                ├── types.ts                   # SceneGraph wire types
                ├── geometry.ts                # pure geometry → SVG-string helpers
                ├── NodeCircle.tsx             # one <circle> per node
                ├── HyperedgeShape.tsx         # one SVG element per geometry variant
                ├── HypergraphSVG.tsx          # composes the whole scene
                ├── __fixtures__/
                │   └── scene.fixture.json     # byte-copy of serde output (contract)
                ├── types.test.ts
                ├── geometry.test.ts
                ├── NodeCircle.test.tsx
                ├── HyperedgeShape.test.tsx
                └── HypergraphSVG.test.tsx
```

All paths below are relative to the repo root
`/home/user/projects/game/babylon/hypergraph-rs/`.

---

## Task 1: Package scaffold + wire types + contract fixture

**Files:**
- Create: `npm/@hypergraph-rs/react/package.json`
- Create: `npm/@hypergraph-rs/react/tsconfig.json`
- Create: `npm/@hypergraph-rs/react/tsconfig.build.json`
- Create: `npm/@hypergraph-rs/react/vitest.config.ts`
- Create: `npm/@hypergraph-rs/react/vitest.setup.ts`
- Create: `npm/@hypergraph-rs/react/.gitignore`
- Create: `npm/@hypergraph-rs/react/.npmignore`
- Create: `npm/@hypergraph-rs/react/LICENSE`
- Create: `npm/@hypergraph-rs/react/README.md`
- Create: `npm/@hypergraph-rs/react/src/types.ts`
- Create: `npm/@hypergraph-rs/react/src/__fixtures__/scene.fixture.json`
- Create: `npm/@hypergraph-rs/react/src/types.test.ts`

**Interfaces produced:** the TypeScript `SceneGraph` type family that every later task
consumes; the toolchain (npm + tsc + vitest) that every later task runs.

- [ ] **Step 1: Write the toolchain scaffold + contract fixture + failing test**

> **Step-order note (review D1):** the entire toolchain — `package.json`, both
> tsconfigs, the vitest config/setup, the ignore/LICENSE/README files — *and* the
> contract fixture *and* `types.test.ts` are all written in THIS step. Only
> `src/types.ts` is deferred: it is the Step 3 green step. This ordering matters — the
> Step 2 `npm install` needs `package.json` to already exist on disk, and the failing
> run needs the test present while `src/types.ts` (imported as `./types.js`) is absent.
> (Only Task 1 has this bootstrap ordering constraint; Tasks 2-6 inherit a working
> toolchain.)

The `types.test.ts` loads the fixture and asserts the exact wire shape — it fails
in Step 2 because `src/types.ts` does not exist yet.

Write to `npm/@hypergraph-rs/react/src/__fixtures__/scene.fixture.json` (a verbatim
copy of what `serde_json::to_string(&scene)` emits for a 3-node / 1-hyperedge
hyperedge-aware render — external-tagged geometry, array tuples, snake_case keys):

```json
{
  "nodes": [
    {
      "id": "a",
      "x": 0.0,
      "y": 0.0,
      "radius": 0.02,
      "attrs": { "role": "core" },
      "style": { "fill": "#1f77b4", "stroke": "#000000", "stroke_width": 1.0 }
    },
    {
      "id": "b",
      "x": 1.0,
      "y": 0.0,
      "radius": 0.02,
      "attrs": null,
      "style": { "fill": "#1f77b4", "stroke": "#000000", "stroke_width": 1.0 }
    },
    {
      "id": "c",
      "x": 0.5,
      "y": 1.0,
      "radius": 0.02,
      "attrs": null,
      "style": { "fill": "#1f77b4", "stroke": "#000000", "stroke_width": 1.0 }
    }
  ],
  "hyperedges": [
    {
      "id": "e1",
      "member_ids": ["a", "b", "c"],
      "geometry": {
        "ConvexHull": {
          "points": [[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]]
        }
      },
      "attrs": { "heat": 0.5 },
      "style": { "fill": "#ff7f0e", "stroke": "#ff7f0e", "opacity": 0.4 }
    }
  ],
  "bounding_box": [0.0, 0.0, 1.0, 1.0],
  "metadata": {}
}
```

Write to `npm/@hypergraph-rs/react/src/types.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import scene from "./__fixtures__/scene.fixture.json";
import type { SceneGraph, HyperedgeGeometry } from "./types.js";

describe("SceneGraph wire contract", () => {
  // `as unknown as` bridges the JSON-module inferred type (number[][], number[],
  // concrete attrs) to the tuple/`unknown` wire types without a TS2352 comparable-
  // relation error under strict + verbatimModuleSyntax (D4).
  const s = scene as unknown as SceneGraph;

  it("has snake_case top-level keys", () => {
    expect(Array.isArray(s.nodes)).toBe(true);
    expect(Array.isArray(s.hyperedges)).toBe(true);
    expect(Array.isArray(s.bounding_box)).toBe(true);
    expect(typeof s.metadata).toBe("object");
  });

  it("carries insertion-ordered nodes with style + attrs", () => {
    expect(s.nodes.map((n) => n.id)).toEqual(["a", "b", "c"]);
    expect(s.nodes[0].style.stroke_width).toBe(1);
    expect(s.nodes[0].attrs).toEqual({ role: "core" });
    expect(s.nodes[1].attrs).toBeNull();
  });

  it("encodes geometry as an externally-tagged variant object", () => {
    const geo: HyperedgeGeometry = s.hyperedges[0].geometry;
    expect("ConvexHull" in geo).toBe(true);
    if ("ConvexHull" in geo) {
      // tuples are JSON arrays
      expect(geo.ConvexHull.points[0]).toEqual([0, 0]);
      expect(geo.ConvexHull.points).toHaveLength(3);
    }
  });

  it("encodes bounding_box as a 4-array [min_x,min_y,max_x,max_y]", () => {
    expect(s.bounding_box).toEqual([0, 0, 1, 1]);
  });

  it("preserves member_ids order", () => {
    expect(s.hyperedges[0].member_ids).toEqual(["a", "b", "c"]);
  });
});
```

Now write the toolchain scaffold (still Step 1). All of these files must exist on disk
before the Step 2 `npm install`.

Write to `npm/@hypergraph-rs/react/package.json`:

```json
{
  "name": "@hypergraph-rs/react",
  "version": "0.1.0",
  "description": "Pure-TypeScript React SVG renderer for hypergraph-rs SceneGraph JSON.",
  "license": "BSD-3-Clause",
  "author": "Babylon Project",
  "type": "module",
  "sideEffects": false,
  "main": "./dist/index.js",
  "module": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "import": "./dist/index.js"
    }
  },
  "files": ["dist", "README.md", "LICENSE"],
  "scripts": {
    "build": "tsc -p tsconfig.build.json",
    "typecheck": "tsc -p tsconfig.json --noEmit",
    "test": "vitest run",
    "test:watch": "vitest",
    "prepack": "npm run build",
    "pack:check": "npm pack --dry-run"
  },
  "peerDependencies": {
    "react": ">=18"
  },
  "devDependencies": {
    "@testing-library/dom": "^10.4.0",
    "@testing-library/react": "^16.1.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "jsdom": "^26.0.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "typescript": "^5.7.0",
    "vitest": "^3.0.0"
  }
}
```

Write to `npm/@hypergraph-rs/react/tsconfig.json` (typecheck config — includes tests):

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "jsx": "react-jsx",
    "resolveJsonModule": true,
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "declaration": true,
    "noEmit": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "verbatimModuleSyntax": true
  },
  "include": ["src", "vitest.config.ts", "vitest.setup.ts"]
}
```

Write to `npm/@hypergraph-rs/react/tsconfig.build.json` (emit config — excludes tests
and fixtures so only library code ships):

```json
{
  "extends": "./tsconfig.json",
  "compilerOptions": {
    "noEmit": false,
    "outDir": "dist",
    "rootDir": "src",
    "declaration": true
  },
  "include": ["src"],
  "exclude": [
    "src/**/*.test.ts",
    "src/**/*.test.tsx",
    "src/__fixtures__"
  ]
}
```

Write to `npm/@hypergraph-rs/react/vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["./vitest.setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
```

Write to `npm/@hypergraph-rs/react/vitest.setup.ts`:

```ts
import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";

// Unmount React trees between tests so jsdom queries don't leak across cases.
afterEach(() => {
  cleanup();
});
```

Write to `npm/@hypergraph-rs/react/.gitignore`:

```
node_modules/
dist/
*.tgz
```

Write to `npm/@hypergraph-rs/react/.npmignore`:

```
src/
node_modules/
tsconfig*.json
vitest.config.ts
vitest.setup.ts
*.tgz
.gitignore
```

Write to `npm/@hypergraph-rs/react/LICENSE` (BSD-3-Clause — copy the exact text of the
workspace-root `hypergraph-rs/LICENSE` created in Phase 0; reproduced here for a
self-contained tarball):

```
BSD 3-Clause License

Copyright (c) 2026, Babylon Project

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its
   contributors may be used to endorse or promote products derived from
   this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

> **Copy byte-for-byte (ruling):** the workspace root `hypergraph-rs/LICENSE` (Phase 0)
> is the canonical copy. At execution time do NOT hand-type the text above — `cp` the
> workspace-root `hypergraph-rs/LICENSE` into this package verbatim (e.g.
> `cp ../../LICENSE ./LICENSE` from the package dir) so the two are byte-identical. The
> block above is a reference only; the canonical file wins on any difference (copyright
> line, year, wording).

Write to `npm/@hypergraph-rs/react/README.md`:

```markdown
# @hypergraph-rs/react

Pure-TypeScript React components that render a `hypergraph-rs` **SceneGraph** (the JSON
emitted by `H.render()` / `hypergraph-rs::viz`) as SVG. No WASM dependency — this
package consumes already-parsed JSON.

## Install

```bash
npm install @hypergraph-rs/react
# react is a peer dependency (>=18) — your app provides it
```

## Usage

```tsx
import { HypergraphSVG, type SceneGraph } from "@hypergraph-rs/react";

export function View({ scene }: { scene: SceneGraph }) {
  return <HypergraphSVG scene={scene} width={640} height={480} />;
}
```

Where `scene` is a `SceneGraph` obtained from any hypergraph-rs surface:
- the WASM module's `H.render(...)` (returns a `SceneGraph` object),
- the CLI's `hypergraph-rs render --format json`,
- the Python binding's `H.render(...)`.

## Components

- `HypergraphSVG` — renders a whole scene (`<svg>` with a hyperedge layer behind a node
  layer, `viewBox` derived from the scene's bounding box).
- `HyperedgeShape` — one hyperedge; emits a `<polygon>` (ConvexHull), `<path>` (Spline
  / Arc), or `<rect>` (Stripe) per geometry variant.
- `NodeCircle` — one node as a `<circle>`.

## Geometry variants

The `HyperedgeGeometry` union mirrors the Rust serde external tagging exactly:
`{ ConvexHull: { points } }`, `{ Spline: { control_points } }`,
`{ Stripe: { x, y_min, y_max } }`, `{ Arc: { center, radius, angle_range } }`.

BSD-3-Clause.
```

- [ ] **Step 2: Install deps and run the test to verify it fails**

Run: `cd npm/@hypergraph-rs/react && npm install && npx vitest run src/types.test.ts`
Expected: FAIL — `./types.js` (i.e. `src/types.ts`) cannot be resolved; the module
does not exist yet. The `npm install` here first materializes `node_modules` from the
`package.json` written in Step 1, so vitest is present to run; the failure is the
genuine red phase (missing module under test), not a missing-toolchain error.

- [ ] **Step 3: Create the wire types (green step)**

Write to `npm/@hypergraph-rs/react/src/types.ts`:

```ts
/**
 * TypeScript transcription of the `hypergraph-rs::viz` SceneGraph wire format.
 *
 * This is a 1-for-1 mirror of the serde JSON emitted by
 * `crates/hypergraph-rs/src/viz/mod.rs` and `.../layout/mod.rs` (Phase 5). Do NOT
 * rename fields: the Rust structs carry no `#[serde(rename_all)]`, so the JSON keys
 * are the snake_case Rust field names. Rust tuples serialize as JSON arrays.
 */

/** A 2-D point `(f64, f64)` → JSON `[x, y]`. */
export type Point = [number, number];

/** `NodeStyle` (viz/mod.rs). */
export interface NodeStyle {
  fill: string;
  stroke: string;
  stroke_width: number;
}

/** `HyperedgeStyle` (viz/mod.rs). */
export interface HyperedgeStyle {
  fill: string;
  stroke: string;
  opacity: number;
}

/**
 * `HyperedgeGeometry` — serde **externally tagged** enum (no `#[serde(tag)]`),
 * i.e. a single-key object whose key is the variant name.
 */
export type HyperedgeGeometry =
  | { ConvexHull: { points: Point[] } }
  | { Spline: { control_points: Point[] } }
  | { Stripe: { x: number; y_min: number; y_max: number } }
  | { Arc: { center: Point; radius: number; angle_range: Point } };

/** `NodeGlyph` (viz/mod.rs). `attrs` is free-form JSON and may be `null`. */
export interface NodeGlyph {
  id: string;
  x: number;
  y: number;
  radius: number;
  attrs: unknown;
  style: NodeStyle;
}

/** `HyperedgeGlyph` (viz/mod.rs). */
export interface HyperedgeGlyph {
  id: string;
  member_ids: string[];
  geometry: HyperedgeGeometry;
  attrs: unknown;
  style: HyperedgeStyle;
}

/**
 * `SceneGraph` (viz/mod.rs). `bounding_box` is `[min_x, min_y, max_x, max_y]`.
 */
export interface SceneGraph {
  nodes: NodeGlyph[];
  hyperedges: HyperedgeGlyph[];
  bounding_box: [number, number, number, number];
  metadata: Record<string, unknown>;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd npm/@hypergraph-rs/react && npx vitest run src/types.test.ts`
Expected: PASS — 5 tests.

Also run the typechecker to confirm the config is coherent:
Run: `cd npm/@hypergraph-rs/react && npm run typecheck`
Expected: no errors.

- [ ] **Step 5: Execution-time wire-contract check against the landed Rust (ruling)**

The fixture and `types.ts` are hand-transcribed from the Phase 5 *plan text*. Blind
trust in a hand-written fixture is not acceptable (orchestrator ruling): Phase 10 runs
**after Phase 5 and Phase 9 have landed**, and it is the only phase running at that
time, so this step emits a **real** `SceneGraph` JSON from the compiled Rust and
structurally compares it against `scene.fixture.json` before any component is built.
This is a build/verify step only — **no publish, no cargo publish**.

Generate a real SceneGraph from the landed core via the Phase 9 CLI `render`
subcommand (a `cargo run`, allowed here at execution time). Use a tiny hypergraph in
xgi-data JSON so the shape — not the exact float layout — is what we compare:

```bash
cd /home/user/projects/game/babylon/hypergraph-rs
# Minimal 3-node / 1-hyperedge input in the CLI's read format.
cat > /tmp/hgrs_contract_in.json <<'JSON'
{"edge-dict": {"e1": ["a", "b", "c"]}, "node-dict": {"a": {}, "b": {}, "c": {}}}
JSON
# Emit a real SceneGraph JSON. Confirm exact flags against `... render --help`
# (Phase 9); the render subcommand writes SceneGraph JSON to stdout with --format json.
cargo run -q -p hypergraph-rs-cli -- render \
  --input /tmp/hgrs_contract_in.json --format json \
  --strategy hyperedge-aware > /tmp/hgrs_scene_real.json
```

Structurally compare the real emitter output against the fixture (keys, external
tagging, tuple-as-array, snake_case) — NOT exact coordinates (layout differs run to
run of the algorithm). Write and run a throwaway node checker:

```bash
cd /home/user/projects/game/babylon/hypergraph-rs/npm/@hypergraph-rs/react
node --input-type=module -e '
import fs from "node:fs";
const real = JSON.parse(fs.readFileSync("/tmp/hgrs_scene_real.json", "utf8"));
const fix  = JSON.parse(fs.readFileSync("./src/__fixtures__/scene.fixture.json", "utf8"));
const fail = (m) => { console.error("CONTRACT MISMATCH: " + m); process.exit(1); };
const top = ["nodes", "hyperedges", "bounding_box", "metadata"];
for (const k of top) if (!(k in real)) fail("missing top-level key " + k);
if (!Array.isArray(real.bounding_box) || real.bounding_box.length !== 4)
  fail("bounding_box is not a 4-array");
const nk = ["id", "x", "y", "radius", "attrs", "style"];
for (const k of nk) if (!(k in real.nodes[0])) fail("node missing key " + k);
for (const k of ["fill", "stroke", "stroke_width"])
  if (!(k in real.nodes[0].style)) fail("NodeStyle missing key " + k);
const ek = ["id", "member_ids", "geometry", "attrs", "style"];
for (const k of ek) if (!(k in real.hyperedges[0])) fail("hyperedge missing key " + k);
for (const k of ["fill", "stroke", "opacity"])
  if (!(k in real.hyperedges[0].style)) fail("HyperedgeStyle missing key " + k);
const geo = real.hyperedges[0].geometry;
const keys = Object.keys(geo);
const variants = ["ConvexHull", "Spline", "Stripe", "Arc"];
if (keys.length !== 1 || !variants.includes(keys[0]))
  fail("geometry is not a single-key external-tagged variant: " + JSON.stringify(keys));
// Both fixture and real must agree on the external-tagging + snake_case shape the TS
// types encode. Mirror the same checks against the fixture so drift on either side trips.
const fgeo = Object.keys(fix.hyperedges[0].geometry);
if (fgeo.length !== 1 || !variants.includes(fgeo[0]))
  fail("fixture geometry shape diverged from external tagging");
console.log("wire contract OK: real Rust SceneGraph matches TS fixture shape");
'
```

Expected: `wire contract OK: …`. If it prints `CONTRACT MISMATCH`, **STOP** — reconcile
the fixture and `types.ts` to match the real serde output (real data wins over the
hand-written fixture); do not weaken the check. Delete the throwaway `/tmp` files after.

- [ ] **Step 6: Commit**

```bash
cd /home/user/projects/game/babylon/hypergraph-rs
git add npm/@hypergraph-rs/react
git commit -m "feat(react): scaffold @hypergraph-rs/react + SceneGraph wire types

Pure-TypeScript package skeleton (package.json, tsconfig, vitest) plus the
SceneGraph type family transcribed 1-for-1 from the hypergraph-rs::viz serde
format, pinned by a byte-copy JSON fixture (external-tagged geometry, array
tuples, snake_case keys).

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 2: `geometry.ts` — pure geometry → SVG-string helpers

**Files:**
- Create: `npm/@hypergraph-rs/react/src/geometry.ts`
- Create: `npm/@hypergraph-rs/react/src/geometry.test.ts`

**Interfaces produced:** `geometryKind`, `pointsAttr`, `polylinePath`, `arcPath` —
pure functions (no React, no DOM) that turn geometry data into SVG attribute strings.
Isolating the math here keeps the components trivial and makes the load-bearing
arc/path computation unit-testable without a DOM.

- [ ] **Step 1: Write the failing test**

Write to `npm/@hypergraph-rs/react/src/geometry.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { geometryKind, pointsAttr, polylinePath, arcPath } from "./geometry.js";

describe("geometryKind", () => {
  it("discriminates each externally-tagged variant by its key", () => {
    expect(geometryKind({ ConvexHull: { points: [] } })).toBe("ConvexHull");
    expect(geometryKind({ Spline: { control_points: [] } })).toBe("Spline");
    expect(geometryKind({ Stripe: { x: 0, y_min: 0, y_max: 1 } })).toBe("Stripe");
    expect(
      geometryKind({ Arc: { center: [0, 0], radius: 1, angle_range: [0, 1] } }),
    ).toBe("Arc");
  });
});

describe("pointsAttr", () => {
  it("formats points as an SVG points string", () => {
    expect(
      pointsAttr([
        [0, 0],
        [1, 2],
        [3, 4],
      ]),
    ).toBe("0,0 1,2 3,4");
  });

  it("returns empty string for no points", () => {
    expect(pointsAttr([])).toBe("");
  });
});

describe("polylinePath", () => {
  it("builds a closed path through the points", () => {
    expect(
      polylinePath([
        [0, 0],
        [1, 0],
        [1, 1],
      ]),
    ).toBe("M 0 0 L 1 0 L 1 1 Z");
  });

  it("returns empty string for no points", () => {
    expect(polylinePath([])).toBe("");
  });
});

describe("arcPath", () => {
  it("computes start point and flags for a half-circle (span == PI)", () => {
    const d = arcPath([0, 0], 1, [0, Math.PI]);
    // start = (cos 0, sin 0) = (1, 0)
    expect(d.startsWith("M 1 0 ")).toBe(true);
    // span == PI is NOT > PI, so large-arc flag is 0; sweep is 1 (a1 >= a0)
    expect(d).toContain("A 1 1 0 0 1 ");
  });

  it("sets the large-arc flag when the span exceeds PI", () => {
    const d = arcPath([0, 0], 2, [0, Math.PI * 1.5]);
    expect(d).toContain("A 2 2 0 1 1 ");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd npm/@hypergraph-rs/react && npx vitest run src/geometry.test.ts`
Expected: FAIL — `./geometry.js` cannot be resolved.

- [ ] **Step 3: Implement `geometry.ts`**

Write to `npm/@hypergraph-rs/react/src/geometry.ts`:

```ts
import type { HyperedgeGeometry, Point } from "./types.js";

/** The discriminant name of a geometry variant. */
export type GeometryKind = "ConvexHull" | "Spline" | "Stripe" | "Arc";

/** Narrow an externally-tagged geometry to its variant name. */
export function geometryKind(g: HyperedgeGeometry): GeometryKind {
  if ("ConvexHull" in g) return "ConvexHull";
  if ("Spline" in g) return "Spline";
  if ("Stripe" in g) return "Stripe";
  return "Arc";
}

/**
 * SVG `points` attribute — `"x1,y1 x2,y2 ..."` — for a `<polygon>`/`<polyline>`.
 * Bounded loop: one pass over `points`.
 */
export function pointsAttr(points: Point[]): string {
  return points.map(([x, y]) => `${x},${y}`).join(" ");
}

/**
 * Closed SVG path `d` through the given points (minimal spline: straight segments,
 * closed with `Z`). A true smoothing spline is intentionally out of scope for this
 * minimal renderer. Bounded loop: one pass over `points`.
 */
export function polylinePath(points: Point[]): string {
  if (points.length === 0) return "";
  const [first, ...rest] = points;
  const move = `M ${first[0]} ${first[1]}`;
  const lines = rest.map(([x, y]) => `L ${x} ${y}`).join(" ");
  return lines.length > 0 ? `${move} ${lines} Z` : `${move} Z`;
}

/**
 * SVG arc path `d` for the `Arc` geometry: a single elliptical-arc segment on the
 * circle of the given `center`/`radius` spanning `angleRange` (radians).
 *
 * Endpoints: `(cx + r·cos θ, cy + r·sin θ)` for θ = a0, a1.
 * `large-arc-flag` = 1 when `|a1 - a0| > π`, else 0.
 * `sweep-flag` = 1 when `a1 >= a0` (counter-clockwise in SVG's y-down space), else 0.
 */
export function arcPath(center: Point, radius: number, angleRange: Point): string {
  const [cx, cy] = center;
  const [a0, a1] = angleRange;
  const sx = cx + radius * Math.cos(a0);
  const sy = cy + radius * Math.sin(a0);
  const ex = cx + radius * Math.cos(a1);
  const ey = cy + radius * Math.sin(a1);
  const largeArc = Math.abs(a1 - a0) > Math.PI ? 1 : 0;
  const sweep = a1 >= a0 ? 1 : 0;
  return `M ${sx} ${sy} A ${radius} ${radius} 0 ${largeArc} ${sweep} ${ex} ${ey}`;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd npm/@hypergraph-rs/react && npx vitest run src/geometry.test.ts`
Expected: PASS — 7 tests (geometryKind 1 + pointsAttr 2 + polylinePath 2 + arcPath 2).

- [ ] **Step 5: Commit**

```bash
cd /home/user/projects/game/babylon/hypergraph-rs
git add npm/@hypergraph-rs/react/src/geometry.ts npm/@hypergraph-rs/react/src/geometry.test.ts
git commit -m "feat(react): geometry helpers (variant discriminator + SVG path builders)

Pure functions mapping each HyperedgeGeometry variant to SVG attribute strings:
geometryKind, pointsAttr (polygon points), polylinePath (closed path), arcPath
(elliptical arc with endpoint + large-arc/sweep-flag math).

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 3: `NodeCircle` component

**Files:**
- Create: `npm/@hypergraph-rs/react/src/NodeCircle.tsx`
- Create: `npm/@hypergraph-rs/react/src/NodeCircle.test.tsx`

**Interfaces produced:** `NodeCircle` (renders one `<circle>` from a `NodeGlyph`) and
its props type `NodeCircleProps`.

- [ ] **Step 1: Write the failing test**

Write to `npm/@hypergraph-rs/react/src/NodeCircle.test.tsx`:

```tsx
import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { NodeCircle } from "./NodeCircle.js";
import type { NodeGlyph } from "./types.js";

const glyph: NodeGlyph = {
  id: "n1",
  x: 0.5,
  y: 0.25,
  radius: 0.02,
  attrs: null,
  style: { fill: "#111111", stroke: "#000000", stroke_width: 1 },
};

describe("NodeCircle", () => {
  it("renders a circle with position, radius, id, and style", () => {
    const { container } = render(
      <svg>
        <NodeCircle glyph={glyph} />
      </svg>,
    );
    const c = container.querySelector("circle");
    expect(c).not.toBeNull();
    expect(c!.getAttribute("cx")).toBe("0.5");
    expect(c!.getAttribute("cy")).toBe("0.25");
    expect(c!.getAttribute("r")).toBe("0.02");
    expect(c!.getAttribute("data-node-id")).toBe("n1");
    expect(c!.getAttribute("fill")).toBe("#111111");
    expect(c!.getAttribute("stroke")).toBe("#000000");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd npm/@hypergraph-rs/react && npx vitest run src/NodeCircle.test.tsx`
Expected: FAIL — `./NodeCircle.js` cannot be resolved.

- [ ] **Step 3: Implement `NodeCircle.tsx`**

Write to `npm/@hypergraph-rs/react/src/NodeCircle.tsx`:

```tsx
import type { ReactElement } from "react";
import type { NodeGlyph } from "./types.js";

export interface NodeCircleProps {
  glyph: NodeGlyph;
}

/**
 * One node as an SVG `<circle>`. Coordinates and radius are in layout units (the
 * scene's `bounding_box` space); `HypergraphSVG` sets the `viewBox` so they render
 * correctly. `vector-effect="non-scaling-stroke"` keeps the outline visible when the
 * viewBox is small (layouts are typically normalized to ~[0,1]).
 */
export function NodeCircle({ glyph }: NodeCircleProps): ReactElement {
  const { id, x, y, radius, style } = glyph;
  return (
    <circle
      data-node-id={id}
      cx={x}
      cy={y}
      r={radius}
      fill={style.fill}
      stroke={style.stroke}
      strokeWidth={style.stroke_width}
      vectorEffect="non-scaling-stroke"
    />
  );
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd npm/@hypergraph-rs/react && npx vitest run src/NodeCircle.test.tsx`
Expected: PASS — 1 test.

- [ ] **Step 5: Commit**

```bash
cd /home/user/projects/game/babylon/hypergraph-rs
git add npm/@hypergraph-rs/react/src/NodeCircle.tsx npm/@hypergraph-rs/react/src/NodeCircle.test.tsx
git commit -m "feat(react): NodeCircle component (one <circle> per NodeGlyph)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 4: `HyperedgeShape` component — one SVG element per geometry variant

**Files:**
- Create: `npm/@hypergraph-rs/react/src/HyperedgeShape.tsx`
- Create: `npm/@hypergraph-rs/react/src/HyperedgeShape.test.tsx`

**Interfaces produced:** `HyperedgeShape` — the minimal SVG renderer per
`HyperedgeGeometry` variant (spec §7.4 "a minimal SVG renderer component per
HyperedgeGeometry variant"). ConvexHull → `<polygon>`, Spline → `<path>`, Stripe →
`<rect>`, Arc → `<path fill="none">`. Every element carries `data-edge-id` and
`data-geometry` for consumers and tests.

- [ ] **Step 1: Write the failing test**

Write to `npm/@hypergraph-rs/react/src/HyperedgeShape.test.tsx`:

```tsx
import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { HyperedgeShape } from "./HyperedgeShape.js";
import type { HyperedgeGeometry, HyperedgeGlyph } from "./types.js";

function renderShape(geometry: HyperedgeGeometry) {
  const glyph: HyperedgeGlyph = {
    id: "e1",
    member_ids: ["a", "b"],
    geometry,
    attrs: null,
    style: { fill: "#ff0000", stroke: "#ff0000", opacity: 0.4 },
  };
  return render(
    <svg>
      <HyperedgeShape glyph={glyph} />
    </svg>,
  ).container;
}

describe("HyperedgeShape", () => {
  it("renders ConvexHull as a <polygon>", () => {
    const c = renderShape({
      ConvexHull: {
        points: [
          [0, 0],
          [1, 0],
          [1, 1],
        ],
      },
    });
    const p = c.querySelector("polygon");
    expect(p).not.toBeNull();
    expect(p!.getAttribute("points")).toBe("0,0 1,0 1,1");
    expect(p!.getAttribute("data-geometry")).toBe("ConvexHull");
    expect(p!.getAttribute("data-edge-id")).toBe("e1");
    expect(p!.getAttribute("fill")).toBe("#ff0000");
  });

  it("renders Spline as a closed <path>", () => {
    const c = renderShape({
      Spline: {
        control_points: [
          [0, 0],
          [1, 0],
          [1, 1],
        ],
      },
    });
    const p = c.querySelector('path[data-geometry="Spline"]');
    expect(p).not.toBeNull();
    expect(p!.getAttribute("d")).toBe("M 0 0 L 1 0 L 1 1 Z");
  });

  it("renders Stripe as a <rect> spanning y_min..y_max", () => {
    const c = renderShape({ Stripe: { x: 1, y_min: 0, y_max: 2 } });
    const r = c.querySelector("rect");
    expect(r).not.toBeNull();
    expect(r!.getAttribute("data-geometry")).toBe("Stripe");
    expect(r!.getAttribute("y")).toBe("0");
    expect(r!.getAttribute("height")).toBe("2");
    // width is a small fixed stripe, centered on x
    expect(r!.getAttribute("width")).toBe("0.01");
  });

  it("renders Arc as an unfilled <path>", () => {
    const c = renderShape({
      Arc: { center: [0, 0], radius: 1, angle_range: [0, Math.PI] },
    });
    const p = c.querySelector('path[data-geometry="Arc"]');
    expect(p).not.toBeNull();
    expect(p!.getAttribute("fill")).toBe("none");
    expect(p!.getAttribute("d")!.startsWith("M 1 0 A 1 1 0 0 1 ")).toBe(true);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd npm/@hypergraph-rs/react && npx vitest run src/HyperedgeShape.test.tsx`
Expected: FAIL — `./HyperedgeShape.js` cannot be resolved.

- [ ] **Step 3: Implement `HyperedgeShape.tsx`**

Write to `npm/@hypergraph-rs/react/src/HyperedgeShape.tsx`:

```tsx
import type { ReactElement } from "react";
import type { HyperedgeGlyph } from "./types.js";
import { arcPath, geometryKind, pointsAttr, polylinePath } from "./geometry.js";

export interface HyperedgeShapeProps {
  glyph: HyperedgeGlyph;
}

/** Width (layout units) of the thin rectangle used to draw a bipartite `Stripe`. */
const STRIPE_WIDTH = 0.01;

/**
 * One hyperedge as a single SVG element, chosen by geometry variant:
 * - `ConvexHull` → `<polygon>` (filled halo around members)
 * - `Spline`     → closed `<path>` through the control points
 * - `Stripe`     → thin `<rect>` centered on `x`, spanning `y_min..y_max`
 * - `Arc`        → unfilled `<path>` elliptical arc
 *
 * Every element carries `data-edge-id` and `data-geometry` so consumers can style or
 * hit-test by hyperedge.
 */
export function HyperedgeShape({ glyph }: HyperedgeShapeProps): ReactElement {
  const { id, geometry, style } = glyph;
  const common = {
    "data-edge-id": id,
    "data-geometry": geometryKind(geometry),
    fill: style.fill,
    stroke: style.stroke,
    fillOpacity: style.opacity,
    vectorEffect: "non-scaling-stroke" as const,
  };

  if ("ConvexHull" in geometry) {
    return <polygon {...common} points={pointsAttr(geometry.ConvexHull.points)} />;
  }
  if ("Spline" in geometry) {
    return <path {...common} d={polylinePath(geometry.Spline.control_points)} />;
  }
  if ("Stripe" in geometry) {
    const { x, y_min, y_max } = geometry.Stripe;
    return (
      <rect
        {...common}
        x={x - STRIPE_WIDTH / 2}
        y={y_min}
        width={STRIPE_WIDTH}
        height={y_max - y_min}
      />
    );
  }
  // Exhaustive: the only remaining variant is `Arc`.
  const { center, radius, angle_range } = geometry.Arc;
  return <path {...common} fill="none" d={arcPath(center, radius, angle_range)} />;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd npm/@hypergraph-rs/react && npx vitest run src/HyperedgeShape.test.tsx`
Expected: PASS — 4 tests.

- [ ] **Step 5: Commit**

```bash
cd /home/user/projects/game/babylon/hypergraph-rs
git add npm/@hypergraph-rs/react/src/HyperedgeShape.tsx npm/@hypergraph-rs/react/src/HyperedgeShape.test.tsx
git commit -m "feat(react): HyperedgeShape — minimal SVG element per geometry variant

ConvexHull -> polygon, Spline -> closed path, Stripe -> rect, Arc -> unfilled
path. data-edge-id / data-geometry on every element.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 5: `HypergraphSVG` — compose the whole scene

**Files:**
- Create: `npm/@hypergraph-rs/react/src/HypergraphSVG.tsx`
- Create: `npm/@hypergraph-rs/react/src/HypergraphSVG.test.tsx`

**Interfaces produced:** `HypergraphSVG` (the top-level component) + `viewBoxOf` (a
pure exported helper that derives the SVG `viewBox` from a scene bounding box, padding
degenerate spans). Hyperedges render in a layer BEHIND the nodes so halos sit under
their member circles.

- [ ] **Step 1: Write the failing test**

Write to `npm/@hypergraph-rs/react/src/HypergraphSVG.test.tsx`:

```tsx
import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { HypergraphSVG, viewBoxOf } from "./HypergraphSVG.js";
import scene from "./__fixtures__/scene.fixture.json";
import type { SceneGraph } from "./types.js";

// `as unknown as` — see D4 note in types.test.ts (JSON-module inferred type vs the
// tuple/`unknown` wire types).
const fixture = scene as unknown as SceneGraph;

describe("viewBoxOf", () => {
  it("maps a well-formed bounding box to 'minX minY width height'", () => {
    expect(viewBoxOf([0, 0, 1, 1])).toBe("0 0 1 1");
    expect(viewBoxOf([-2, -3, 2, 3])).toBe("-2 -3 4 6");
  });

  it("pads a degenerate (zero-width) span so the viewBox stays valid", () => {
    // width 0 → pad 0.05 each side: x = 2-0.05, w = 0.1
    expect(viewBoxOf([2, 0, 2, 3])).toBe("1.95 0 0.1 3");
  });
});

describe("HypergraphSVG", () => {
  it("renders one circle per node and one shape per hyperedge", () => {
    const { container } = render(<HypergraphSVG scene={fixture} />);
    expect(container.querySelectorAll("circle")).toHaveLength(3);
    // ConvexHull hyperedge → one polygon
    expect(container.querySelectorAll("[data-edge-id]")).toHaveLength(1);
  });

  it("sets the viewBox from the scene bounding box", () => {
    const { container } = render(<HypergraphSVG scene={fixture} />);
    const svg = container.querySelector("svg");
    expect(svg!.getAttribute("viewBox")).toBe("0 0 1 1");
  });

  it("renders hyperedges behind nodes (edge layer precedes node layer)", () => {
    const { container } = render(<HypergraphSVG scene={fixture} />);
    const layers = container.querySelectorAll("g[data-layer]");
    expect(layers[0].getAttribute("data-layer")).toBe("hyperedges");
    expect(layers[1].getAttribute("data-layer")).toBe("nodes");
  });

  it("honors width/height props", () => {
    const { container } = render(
      <HypergraphSVG scene={fixture} width={800} height={600} />,
    );
    const svg = container.querySelector("svg");
    expect(svg!.getAttribute("width")).toBe("800");
    expect(svg!.getAttribute("height")).toBe("600");
  });

  it("renders an empty scene without crashing", () => {
    const empty: SceneGraph = {
      nodes: [],
      hyperedges: [],
      bounding_box: [0, 0, 0, 0],
      metadata: {},
    };
    const { container } = render(<HypergraphSVG scene={empty} />);
    expect(container.querySelector("svg")).not.toBeNull();
    expect(container.querySelectorAll("circle")).toHaveLength(0);
    // fully-degenerate bbox padded on both axes
    expect(container.querySelector("svg")!.getAttribute("viewBox")).toBe(
      "-0.05 -0.05 0.1 0.1",
    );
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd npm/@hypergraph-rs/react && npx vitest run src/HypergraphSVG.test.tsx`
Expected: FAIL — `./HypergraphSVG.js` cannot be resolved.

- [ ] **Step 3: Implement `HypergraphSVG.tsx`**

Write to `npm/@hypergraph-rs/react/src/HypergraphSVG.tsx`:

```tsx
import type { ReactElement } from "react";
import type { SceneGraph } from "./types.js";
import { NodeCircle } from "./NodeCircle.js";
import { HyperedgeShape } from "./HyperedgeShape.js";

export interface HypergraphSVGProps {
  scene: SceneGraph;
  /** Rendered pixel width of the `<svg>` element. Default 600. */
  width?: number;
  /** Rendered pixel height of the `<svg>` element. Default 400. */
  height?: number;
  /** Optional className passed through to the `<svg>`. */
  className?: string;
}

/** Below this span an axis is treated as degenerate and padded. */
const MIN_SPAN = 1e-6;
/** Half-span added to each side of a degenerate axis. */
const DEGENERATE_PAD = 0.05;

/**
 * Derive an SVG `viewBox` (`"minX minY width height"`) from a scene bounding box
 * `[min_x, min_y, max_x, max_y]`. A zero- or near-zero-span axis (empty scene, a
 * single point, a horizontal bipartite stripe) is padded so the viewBox is always
 * valid and the content is visible.
 */
export function viewBoxOf(
  bbox: [number, number, number, number],
): string {
  const [minX, minY, maxX, maxY] = bbox;
  let x = minX;
  let y = minY;
  let w = maxX - minX;
  let h = maxY - minY;
  if (w < MIN_SPAN) {
    x -= DEGENERATE_PAD;
    w = DEGENERATE_PAD * 2;
  }
  if (h < MIN_SPAN) {
    y -= DEGENERATE_PAD;
    h = DEGENERATE_PAD * 2;
  }
  return `${x} ${y} ${w} ${h}`;
}

/**
 * Render a whole `SceneGraph` as SVG: a hyperedge layer (drawn first, so halos sit
 * behind their members) followed by a node layer. Array order is preserved
 * (insertion order from the layout).
 *
 * Coordinate note: this renderer maps layout coordinates straight into the viewBox
 * WITHOUT flipping the y-axis, exactly matching the core Rust emitter
 * `hypergraph-rs::viz::svg::scene_to_svg` (Phase 5 plan `viz/svg.rs`), which likewise
 * "emit[s] layout coords directly for a lossless round-trip with the scene-graph JSON"
 * and leaves any "math"/matplotlib y-flip to consumers (phase-5 plan lines 1986-1989).
 * Consistency with the core SVG output orientation is the governing constraint
 * (orchestrator ruling); a rendered scene is therefore vertically mirrored relative to
 * a matplotlib plot on both surfaces identically, and a y-flip transform is a consumer
 * concern. See the deviations block.
 */
export function HypergraphSVG({
  scene,
  width = 600,
  height = 400,
  className,
}: HypergraphSVGProps): ReactElement {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      width={width}
      height={height}
      viewBox={viewBoxOf(scene.bounding_box)}
      data-testid="hypergraph-svg"
    >
      <g data-layer="hyperedges">
        {scene.hyperedges.map((g) => (
          <HyperedgeShape key={g.id} glyph={g} />
        ))}
      </g>
      <g data-layer="nodes">
        {scene.nodes.map((n) => (
          <NodeCircle key={n.id} glyph={n} />
        ))}
      </g>
    </svg>
  );
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd npm/@hypergraph-rs/react && npx vitest run src/HypergraphSVG.test.tsx`
Expected: PASS — 7 tests (viewBoxOf 2 + HypergraphSVG 5).

- [ ] **Step 5: Commit**

```bash
cd /home/user/projects/game/babylon/hypergraph-rs
git add npm/@hypergraph-rs/react/src/HypergraphSVG.tsx npm/@hypergraph-rs/react/src/HypergraphSVG.test.tsx
git commit -m "feat(react): HypergraphSVG — compose scene (hyperedge layer behind nodes)

Top-level component + viewBoxOf helper (degenerate-span padding). Renders one
circle per node and one shape per hyperedge, preserving insertion order.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 6: Public barrel + build + `npm pack --dry-run` packaging gate

**Files:**
- Create: `npm/@hypergraph-rs/react/src/index.ts`
- Create: `npm/@hypergraph-rs/react/src/index.test.ts`

**Interfaces produced:** the package's public entry point (`src/index.ts` → built to
`dist/index.js` + `dist/index.d.ts`). This task also runs the full test suite, the
production build, and the **packaging gate** (`npm pack --dry-run`) — the phase's
definition of done.

- [ ] **Step 1: Write the failing test**

Write to `npm/@hypergraph-rs/react/src/index.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import * as pkg from "./index.js";

describe("public API surface", () => {
  it("exports the three components", () => {
    expect(typeof pkg.HypergraphSVG).toBe("function");
    expect(typeof pkg.HyperedgeShape).toBe("function");
    expect(typeof pkg.NodeCircle).toBe("function");
  });

  it("exports the geometry helpers", () => {
    expect(typeof pkg.geometryKind).toBe("function");
    expect(typeof pkg.pointsAttr).toBe("function");
    expect(typeof pkg.polylinePath).toBe("function");
    expect(typeof pkg.arcPath).toBe("function");
    expect(typeof pkg.viewBoxOf).toBe("function");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd npm/@hypergraph-rs/react && npx vitest run src/index.test.ts`
Expected: FAIL — `./index.js` cannot be resolved.

- [ ] **Step 3: Implement `index.ts`**

Write to `npm/@hypergraph-rs/react/src/index.ts`:

```ts
// Public entry point for @hypergraph-rs/react.

export type {
  Point,
  NodeStyle,
  HyperedgeStyle,
  HyperedgeGeometry,
  NodeGlyph,
  HyperedgeGlyph,
  SceneGraph,
} from "./types.js";

export type { GeometryKind } from "./geometry.js";
export { geometryKind, pointsAttr, polylinePath, arcPath } from "./geometry.js";

export { NodeCircle } from "./NodeCircle.js";
export type { NodeCircleProps } from "./NodeCircle.js";

export { HyperedgeShape } from "./HyperedgeShape.js";
export type { HyperedgeShapeProps } from "./HyperedgeShape.js";

export { HypergraphSVG, viewBoxOf } from "./HypergraphSVG.js";
export type { HypergraphSVGProps } from "./HypergraphSVG.js";
```

- [ ] **Step 4: Run the full suite, typecheck, build, and the packaging gate**

Run the whole vitest suite:
Run: `cd npm/@hypergraph-rs/react && npm test`
Expected: PASS — all files (types 5, geometry 7, NodeCircle 1, HyperedgeShape 4,
HypergraphSVG 7, index 2 = 26 tests), 0 failures.

Typecheck (strict, includes tests):
Run: `cd npm/@hypergraph-rs/react && npm run typecheck`
Expected: no errors.

Production build (emits `dist/`):
Run: `cd npm/@hypergraph-rs/react && npm run build`
Expected: `dist/index.js`, `dist/index.d.ts`, and the per-module `.js`/`.d.ts`
are written; no `.test.*` or `__fixtures__` output (excluded by
`tsconfig.build.json`), and no `.d.ts.map`/`.js.map` files (declaration/source maps
are intentionally disabled — see D5 in the deviations block: `src/` is never shipped,
so shipped maps would dangle at consumers).

**Packaging gate** — `npm pack --dry-run` (runs `prepack` → `npm run build`, then
lists the tarball WITHOUT writing or publishing it):
Run: `cd npm/@hypergraph-rs/react && npm pack --dry-run`
Expected: the printed contents list contains `package.json`, `README.md`, `LICENSE`,
and `dist/index.js` + `dist/index.d.ts`; it does NOT contain `src/`, `node_modules/`,
tests, or `tsconfig*`. No `.tgz` is written (dry-run) and nothing is published.

> **This is the phase's definition of done.** If any expected file is missing from the
> pack list, or any test/typecheck/build fails, STOP and fix before committing — do not
> weaken a test to pass the gate.

- [ ] **Step 5: Commit**

```bash
cd /home/user/projects/game/babylon/hypergraph-rs
git add npm/@hypergraph-rs/react/src/index.ts npm/@hypergraph-rs/react/src/index.test.ts
git commit -m "feat(react): public barrel + packaging gate (npm pack --dry-run)

Exports the three components + geometry/viewBox helpers + all wire types. Full
vitest suite green (26 tests), tsc build emits dist/, npm pack --dry-run lists
dist + README + LICENSE only. No publish.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Definition of done (Phase 10)

1. `npm/@hypergraph-rs/react/` exists as a self-contained pure-TypeScript package with
   `react` as its only peer dependency.
2. TypeScript `SceneGraph` types mirror the `hypergraph-rs::viz` serde format
   byte-for-byte, pinned by a JSON fixture that is a verbatim copy of serde output.
3. The fixture is verified against a **real** SceneGraph JSON emitted by the landed
   Rust (Task 1 Step 5 execution-time wire-contract check) — not trusted blind.
4. A minimal SVG renderer covers all four `HyperedgeGeometry` variants
   (ConvexHull → polygon, Spline → path, Stripe → rect, Arc → path) plus nodes, with
   the same no-y-flip orientation as the core `render_to_svg`.
5. `npm test` — all 26 vitest (jsdom) tests green.
6. `npm run typecheck` — strict, no errors.
7. `npm run build` — emits `dist/` (`.js` + `.d.ts`, no source/declaration maps),
   tests/fixtures excluded.
8. `npm pack --dry-run` — the packaging gate — lists `dist/`, `README.md`, `LICENSE`
   and nothing else; no publish anywhere.

**Explicitly out of scope** (kept lean per spec §10.3's 2–3 day sizing):
- **npm publish** (owner decision — omitted deliberately).
- **Storybook** (spec §10.3 mentions it for component development; dropped to stay
  lean — the vitest component tests are the development harness).
- **WASM integration** (spec §7.4: this package "takes JSON", no WASM dep of its own).
- **A y-axis flip / matplotlib-parity coordinate transform.** The renderer deliberately
  matches the core `render_to_svg` orientation (no y-flip); a "math"-oriented flip is a
  consumer concern, documented in `HypergraphSVG`.
- **A React 18.x test leg.** The dev/test toolchain pins a single React version (19);
  the peer range is `>=18` but this phase does not add a matrix leg that runs the suite
  against 18.x (ruling — single dev React version).
- **A new npm CI job.** The packaging-gate commands live in this plan only; no CI
  workflow is added for the Node subtree (ruling; matches spec §9.7).

---

## Deviations & rulings folded in (2026-07-19)

Applied from the Phase 10 review and orchestrator rulings; each is realized in the plan
text above.

- **Ruling — Storybook dropped.** Lean scope; the gate is vitest + `npm pack --dry-run`.
  (Out-of-scope list.)
- **Ruling — y-axis matches the core.** Verified against Phase 5 plan `viz/svg.rs`
  (`scene_to_svg`, lines 1986-1989): the core emits layout coords with **no y-flip**.
  The React `HypergraphSVG` matches this exactly; the coordinate note and out-of-scope
  entry document that consistency with the core beats matplotlib parity.
- **Ruling — no new npm CI job.** Packaging-gate commands appear in the plan only.
- **Ruling — React peer `>=18`, dev-pin 19.** Single dev React version; the 18.x test
  leg is out of scope (noted above).
- **Ruling — execution-time serde external-tagging contract check.** Added as Task 1
  Step 5: a `cargo run` of the landed Phase 9 CLI `render` emits a real SceneGraph JSON,
  structurally compared against the TS fixture before any component is built. Blind
  trust in the hand-written fixture is not acceptable; real serde output wins on drift.
- **Ruling — LICENSE copied byte-for-byte** from workspace-root `hypergraph-rs/LICENSE`
  (`cp`, not hand-typed).
- **D1 (major) — Task 1 step-order inversion fixed.** Scaffold + fixture + failing test
  now precede `npm install`; `src/types.ts` is created as the Step 3 green step after
  the red run. Only Task 1 had the bootstrap constraint.
- **D2 (minor) — `@testing-library/dom` ^10.4.0** added to devDependencies (explicit
  RTL v16 peer, not left to npm auto-install-peers).
- **D3 (minor) — test counts corrected:** geometry 7 (not 8), HypergraphSVG 7 (not 8);
  suite total **26** (types 5 + geometry 7 + NodeCircle 1 + HyperedgeShape 4 +
  HypergraphSVG 7 + index 2). Swept in Task 2/5 step outputs, the Task 6 tally, the Task
  6 commit message, and DoD.
- **D4 (advisory) — `scene as unknown as SceneGraph`** in `types.test.ts` and
  `HypergraphSVG.test.tsx` to avoid a possible TS2352 under strict typecheck.
- **D5 (advisory) — declaration/source maps dropped** from `tsconfig.build.json` (`src/`
  is never shipped, so shipped maps would dangle); build-output and DoD wording swept.

**Re-verified constraint:** NO publishing anywhere — no `npm publish`, `cargo publish`,
`wasm-pack publish`, `twine`/PyPI, or `gh release`. The only new command introduced by
these fixes is a `cargo run` of the CLI `render` (build/emit) plus a throwaway `node`
structural check; the packaging gate remains `npm pack --dry-run`.
