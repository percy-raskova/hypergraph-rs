//! The three Lawverian 3D instruments (PHASE-R-MAP §3): pure `DeckWorld ->
//! SceneGraph3D` builders, one per instrument, direct transcriptions of the
//! deck's draw functions minus canvas calls. Feature-gated `cells3d` (S9
//! lane gate — 3D spatial embeddings are library/CLI capability only).

pub mod cylinder;
pub mod sankey;
pub mod terrain;

use crate::raster::deck::DeckWorld;
use crate::raster::SceneGraph3D;

/// Which of the three Lawverian instruments to build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instrument {
    /// Instrument I — the adjoint cylinder (Task 7).
    Cylinder,
    /// Instrument II — the contradiction terrain (Task 8).
    Terrain,
    /// Instrument III — the OCC hyper-Sankey (Task 9).
    Sankey,
}

/// Which scalar sets a community's height on the cylinder (D-R1). Default
/// `OrgDegree` mirrors the deck's `o(C) = orgOf(c)`. `HullFill` is a
/// structural alternative — internal-link density as a fraction of the
/// maximum possible pairwise links (`|links| / C(|C|,2)`, clamped to
/// `[0,1]`) — exposed for callers who want a "how full is this hull"
/// reading instead of "how connected is this community"; no golden number
/// asserts it (no owner ruling needed for a non-default option).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CylinderHeight {
    #[default]
    OrgDegree,
    HullFill,
}

/// Options shared across instrument builders. Only `cylinder_height`
/// applies today (Task 7); future tasks may add Terrain/Sankey knobs here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InstrumentOpts {
    pub cylinder_height: CylinderHeight,
}

/// Build the scene graph for `inst` over `world`. Pure function of
/// `(inst, world, opts)` — camera-independent, deterministic (Task 7 Step 3
/// test contract).
pub fn build(inst: Instrument, world: &DeckWorld, opts: &InstrumentOpts) -> SceneGraph3D {
    match inst {
        Instrument::Cylinder => cylinder::build_cylinder(world, opts),
        Instrument::Terrain => terrain::build_terrain(world, opts),
        Instrument::Sankey => {
            let zoom = if world.zoom == 1 {
                sankey::Zoom::Coarse
            } else {
                sankey::Zoom::Fine
            };
            sankey::build_sankey(&world.industries, zoom).scene
        }
    }
}
