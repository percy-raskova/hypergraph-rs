#![cfg(feature = "raster-png")]
use hypergraph_rs::raster::deck::build_world;
use hypergraph_rs::raster::instruments::{build, Instrument, InstrumentOpts};
use hypergraph_rs::raster::png::{render_pixels, render_png};
use hypergraph_rs::raster::{Camera, Rgb, SceneGraph3D};

fn fixture() -> (SceneGraph3D, Camera) {
    let w = build_world(1917);
    let s = build(Instrument::Cylinder, &w, &InstrumentOpts::default());
    (
        s,
        Camera {
            ry: 0.7,
            rx: 0.3,
            dist: 4.0,
            fov: 70.0,
        },
    )
}

#[test]
fn test_render_pixels_dims_and_len() {
    let (s, c) = fixture();
    let pb = render_pixels(&s, &c, 128, 96);
    assert_eq!((pb.w, pb.h), (128, 96));
    assert_eq!(pb.px.len(), 128 * 96);
}

#[test]
fn test_render_pixels_is_pure_function_byte_exact() {
    let (s, c) = fixture();
    assert_eq!(render_pixels(&s, &c, 128, 96), render_pixels(&s, &c, 128, 96));
}

#[test]
fn test_png_decodes_back_to_exact_pixelbuffer() {
    // PRIMARY golden — platform-INDEPENDENT for the pure-arithmetic geometry path:
    // decode the encoded PNG and assert the RGB pixels are byte-exact vs render_pixels.
    // (The decode dodges the cross-platform DEFLATE trap: pixels, not compressed bytes.)
    let (s, c) = fixture();
    let pb = render_pixels(&s, &c, 128, 96);
    let bytes = render_png(&s, &c, 128, 96);

    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("valid png header");
    let mut buf = vec![0u8; reader.output_buffer_size().expect("known output size")];
    let info = reader.next_frame(&mut buf).expect("decode frame");
    assert_eq!(info.color_type, png::ColorType::Rgb);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    let bytes = &buf[..info.buffer_size()];

    let decoded: Vec<Rgb> = bytes
        .chunks_exact(3)
        .map(|c| Rgb(c[0], c[1], c[2]))
        .collect();
    assert_eq!(decoded, pb.px);
}

#[test]
fn test_png_encoded_bytes_match_pinned_fixture() {
    // SECONDARY golden — PLATFORM-PINNED: encoded .png bytes are byte-exact on the
    // reference platform under the pinned `png` 0.18 crate + pinned encoder settings.
    // DEFLATE output is NOT guaranteed identical across arbitrary zlib impls (CLAUDE.md §4),
    // so this is a platform-pinned fixture, NOT a universal byte-exact guarantee.
    let (s, c) = fixture();
    assert_eq!(
        render_png(&s, &c, 128, 96),
        include_bytes!("fixtures/png/cylinder_128x96.png")
    );
}

#[test]
fn test_png_random_camera_deterministic_and_panic_free() {
    // Determinism + panic-free property: many adversarial cameras -> stable output,
    // never NaN/inf/oob. Reuses the near-plane clip (Task 2) + NaN-safe depth compare +
    // bounds-clamped pixel writes (Task 3): a camera behind/through the scene yields a
    // stable frame, never a panic. Render each camera twice; assert equal.
    let (s, _) = fixture();
    let cams = [
        Camera {
            ry: 0.0,
            rx: 0.0,
            dist: 0.0,
            fov: 70.0,
        },
        Camera {
            ry: 3.5,
            rx: -2.1,
            dist: -10.0,
            fov: 179.0,
        },
        Camera {
            ry: -1.2,
            rx: 6.4,
            dist: 1e-9,
            fov: 1.0,
        },
        Camera {
            ry: 100.0,
            rx: -100.0,
            dist: 1000.0,
            fov: 90.0,
        },
    ];
    for cam in cams {
        let a = render_png(&s, &cam, 32, 24);
        let b = render_png(&s, &cam, 32, 24);
        assert_eq!(a, b);
    }
}
