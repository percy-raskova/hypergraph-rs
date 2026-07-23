//! Color under constraint (Task 4): the deck's gold->bone->crimson heat
//! canon, a deterministic 256-palette quantization, and the per-cell
//! "one fg + one bg" pick under the braille glyph floor.
//!
//! `heat` is a verbatim transcription of `topology-deck.html:277-284`
//! (`mix(a,b,t)=a+(b-a)*t`, `|0` = `.floor() as u8`). `to_ansi256` mines
//! the 6x6x6 cube + 24-step grayscale ramp technique described in the
//! plan (Task 4 Step 3) — this crate carries no vendored copy of
//! `tui-3d-renderer`, so the formula is transcribed from the plan text,
//! which is itself the authoritative source for this fn.

use super::Rgb;

/// Linear mix: `a + (b-a)*t`, matching the deck's `mix`.
#[inline]
fn mix(a: u8, b: u8, t: f64) -> u8 {
    let v = a as f64 + (b as f64 - a as f64) * t;
    // Deck's `|0` is a floor-toward-zero on a non-negative float here
    // (channel values stay within [0,255] for t in [0,1]).
    v.floor() as u8
}

/// Gold(low) -> bone -> crimson(high), transcribed verbatim from the deck's
/// `heat()` (`topology-deck.html:278-284`). `t` is clamped to `[0,1]`.
pub fn heat(t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    const GOLD: (u8, u8, u8) = (217, 164, 65);
    const BONE: (u8, u8, u8) = (233, 223, 201);
    const CRIMSON: (u8, u8, u8) = (196, 58, 46);

    let (from, to, k) = if t < 0.5 {
        (GOLD, BONE, t * 2.0)
    } else {
        (BONE, CRIMSON, (t - 0.5) * 2.0)
    };

    Rgb(
        mix(from.0, to.0, k),
        mix(from.1, to.1, k),
        mix(from.2, to.2, k),
    )
}

/// Quantize a truecolor `Rgb` to an ANSI-256 palette index: the 24-step
/// grayscale ramp (232..=255) when `r==g==b`, else the 6x6x6 color cube
/// (16..=231). Tie-break pinned to `f64::round` (round-half-away-from-zero,
/// IEEE-754 reproducible per user CLAUDE.md §4) — never truncating `as u8`.
pub fn to_ansi256(c: Rgb) -> u8 {
    let Rgb(r, g, b) = c;
    if r == g && g == b {
        let v = r as f64;
        if v < 8.0 {
            return 16;
        }
        if v > 248.0 {
            return 231;
        }
        let step = ((v - 8.0) / 247.0 * 24.0).round() as i32;
        return (232 + step) as u8;
    }

    let cube = |ch: u8| -> u32 { (ch as f64 / 255.0 * 5.0).round() as u32 };
    let idx = 16 + 36 * cube(r) + 6 * cube(g) + cube(b);
    idx as u8
}

/// The output palette a `CellGrid` is rendered under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Palette {
    TrueColor,
    Ansi256,
}

/// A lit subcell's color+depth, in the cell's fixed subcell scan order
/// (`(sub_col,sub_row)`: (0,0),(1,0),(0,1),(1,1),(0,2),(1,2),(0,3),(1,3) —
/// `grid::compose_braille`'s iteration order).
#[derive(Debug, Clone, Copy)]
pub(crate) struct LitSubcell {
    pub(crate) color: Rgb,
    pub(crate) depth: f64,
}

/// Pick `(fg, bg)` for one cell from its lit subcells, under the braille
/// "one fg + one bg" constraint. `fg` = color of the **majority** of lit
/// subcells; ties broken by (a) nearest depth (the minimum depth among a
/// candidate color's own subcells — smaller wins), then (b) lowest subcell
/// scan index. `bg` = the next-most-common lit color (by the same
/// tie-break, excluding `fg`'s color), else `Rgb(0,0,0)` if no other color
/// is present. Bounded: `lit.len() <= 8` (one cell's subcells).
pub(crate) fn pick_fg_bg(lit: &[LitSubcell]) -> (Rgb, Rgb) {
    if lit.is_empty() {
        return (Rgb(0, 0, 0), Rgb(0, 0, 0));
    }

    // Group by color (first-seen order == scan order, since `lit` is
    // built in scan order) — a bounded linear scan over <= 8 entries.
    struct Group {
        color: Rgb,
        count: usize,
        min_depth: f64,
        first_index: usize,
    }
    let mut groups: Vec<Group> = Vec::new();
    for (i, s) in lit.iter().enumerate() {
        if let Some(g) = groups.iter_mut().find(|g| g.color == s.color) {
            g.count += 1;
            if s.depth < g.min_depth {
                g.min_depth = s.depth;
            }
        } else {
            groups.push(Group {
                color: s.color,
                count: 1,
                min_depth: s.depth,
                first_index: i,
            });
        }
    }

    // Rank: highest count first; ties by nearest (smallest) min_depth;
    // remaining ties by lowest first-seen scan index.
    groups.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| {
                a.min_depth
                    .partial_cmp(&b.min_depth)
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .then_with(|| a.first_index.cmp(&b.first_index))
    });

    let fg = groups[0].color;
    let bg = groups.get(1).map(|g| g.color).unwrap_or(Rgb(0, 0, 0));
    (fg, bg)
}
