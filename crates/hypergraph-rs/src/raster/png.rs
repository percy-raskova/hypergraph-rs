//! Rust-side PNG pixel-plate emission (Task 14, ADR097 Tier-1).
//!
//! A **pixel** plate: one `Rgb` per pixel at arbitrary `(width_px,
//! height_px)`, rasterized from the SAME `SceneGraph3D` the instruments
//! build — a different tier from the Tier-0 braille `CellGrid`. Reuses the
//! shared geometry core verbatim (reuse ledger, no duplicated math):
//! near-plane clip + `project_wh` + `depth_sort` from `camera.rs` (Task 2),
//! and the edge-function triangle fill + integer Bresenham + NaN-safe depth
//! test + bounds-clamped writes from `grid.rs` (Task 3), targeting a pixel
//! `SubBuffer` instead of the `cols*2 x rows*4` braille one.

use super::grid::{draw_scene, SubBuffer};
use super::{Camera, Rgb, SceneGraph3D};

/// A full-resolution RGB8 pixel raster, row-major, `px.len() == w*h`.
/// Every pixel carries an `Rgb` unconditionally — the Tier-1 analogue of
/// the Tier-0 glyph floor (background pixels get the fixed clear color).
#[derive(Debug, Clone, PartialEq)]
pub struct PixelBuffer {
    pub w: u32,
    pub h: u32,
    pub px: Vec<Rgb>,
}

/// Fixed clear/background color for pixels the z-test never wrote — pinned
/// so the buffer stays fully defined (mirrors the Tier-0 glyph floor).
const CLEAR: Rgb = Rgb(0, 0, 0);

/// Rasterize `scene` under `cam` into a `width_px x height_px` `PixelBuffer`
/// (Task 14) via the SAME `grid::draw_scene` core the Tier-0 braille
/// `rasterize` calls — near-plane clip, painter's depth sort, triangle
/// fill, integer Bresenham struts and `Node3` discs (review MAJOR 3), only
/// the target resolution differs — then read the buffer out directly as
/// `Rgb` per pixel (no braille composition — Tier-1 is raw pixels, and no
/// glyph banner: a pixel plate has no text cells). Pure function of
/// `(scene, cam, width_px, height_px)` — byte-exact golden-testable.
pub fn render_pixels(
    scene: &SceneGraph3D,
    cam: &Camera,
    width_px: u32,
    height_px: u32,
) -> PixelBuffer {
    let mut buf = SubBuffer::new(width_px as usize, height_px as usize);
    draw_scene(&mut buf, scene, cam, width_px as f64, height_px as f64);

    PixelBuffer {
        w: width_px,
        h: height_px,
        px: buf.to_rgb_plate(CLEAR),
    }
}

/// Encode a `render_pixels` plate as PNG bytes (RGB8, 8-bit) via the `png`
/// crate with PINNED encoder settings (`Compression::Fast` +
/// `Filter::NoFilter`, frozen — never varied — for reproducibility). The
/// pixel geometry (basic IEEE-754 arithmetic) is byte-exact cross-platform;
/// the DEFLATE-compressed bytes returned here are only guaranteed
/// byte-exact on the reference platform under this pinned `png` 0.18
/// (CLAUDE.md §4 — decode-and-compare is the portable golden, see
/// `test_png_decodes_back_to_exact_pixelbuffer`).
pub fn render_png(scene: &SceneGraph3D, cam: &Camera, width_px: u32, height_px: u32) -> Vec<u8> {
    let pb = render_pixels(scene, cam, width_px, height_px);

    let mut bytes: Vec<u8> = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, pb.w, pb.h);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        encoder.set_filter(png::Filter::NoFilter);

        let mut writer = encoder.write_header().expect("valid PNG header");
        let mut data = Vec::with_capacity(pb.px.len() * 3);
        for Rgb(r, g, b) in &pb.px {
            data.push(*r);
            data.push(*g);
            data.push(*b);
        }
        writer
            .write_image_data(&data)
            .expect("in-memory PNG write cannot fail");
    }

    bytes
}
