use std::time::Duration;

use bevy::prelude::*;
use hypergraph_rs_bevy::{
    FieldConfig, FieldError, FieldStyles, Glyph, Motion, Palette, ParticleStyle, StreamStyle,
};
use std::num::NonZeroU64;

fn palette() -> Palette {
    Palette {
        ramp: [
            Color::srgb(1.0, 0.0, 0.0),
            Color::srgb(0.5, 0.5, 0.5),
            Color::srgb(0.0, 0.0, 1.0),
        ],
        no_reading: Color::srgb(0.2, 0.2, 0.2),
        swatches: vec![Color::WHITE, Color::BLACK],
    }
}

fn comps(c: LinearRgba) -> [f32; 4] {
    [c.red, c.green, c.blue, c.alpha]
}

#[test]
fn the_ramp_runs_through_its_three_colours() {
    let p = palette();
    assert_eq!(p.tone(Some(-1.0)), p.ramp[0].to_linear());
    assert_eq!(p.tone(Some(0.0)), p.ramp[1].to_linear());
    assert_eq!(p.tone(Some(1.0)), p.ramp[2].to_linear());
    let mid = comps(p.tone(Some(0.5)));
    let a = comps(p.ramp[1].to_linear());
    let b = comps(p.ramp[2].to_linear());
    for i in 0..4 {
        assert!((mid[i] - (a[i] + b[i]) / 2.0).abs() < 1e-6);
    }
    assert_eq!(p.tone(Some(7.0)), p.tone(Some(1.0)));
    assert_eq!(p.tone(Some(-7.0)), p.tone(Some(-1.0)));
}

#[test]
fn no_reading_is_its_own_colour() {
    assert_eq!(palette().tone(None), palette().no_reading.to_linear());
}

#[test]
fn swatches_are_looked_up_by_index() {
    let p = palette();
    assert_eq!(p.swatch(1), Some(Color::BLACK));
    assert_eq!(p.swatch(2), None);
}

#[test]
fn three_beats_is_a_valid_config() {
    let c = FieldConfig::three_beats(Duration::from_millis(1200));
    assert_eq!(c.beats, vec![Duration::from_millis(1200); 3]);
    assert_eq!(
        (
            c.ease_beat,
            c.mark_travel,
            c.bow,
            c.motion,
            c.curve_segments,
            c.outline_lift
        ),
        (2, 0.4, 0.08, Motion::Full, 48, 0.004)
    );
    assert_eq!(c.validate(), Ok(()));
}

#[test]
fn a_bad_config_names_its_field() {
    let good = FieldConfig::three_beats(Duration::from_millis(100));
    let name = |c: FieldConfig| match c.validate() {
        Err(FieldError::BadParameter { name, .. }) => name,
        other => panic!("expected BadParameter, got {other:?}"),
    };
    assert_eq!(
        name(FieldConfig {
            beats: vec![],
            ..good.clone()
        }),
        "beats"
    );
    assert_eq!(
        name(FieldConfig {
            beats: vec![Duration::ZERO],
            ease_beat: 0,
            ..good.clone()
        }),
        "beats"
    );
    assert_eq!(
        name(FieldConfig {
            ease_beat: 3,
            ..good.clone()
        }),
        "ease_beat"
    );
    assert_eq!(
        name(FieldConfig {
            mark_travel: 0.0,
            ..good.clone()
        }),
        "mark_travel"
    );
    assert_eq!(
        name(FieldConfig {
            curve_segments: 4,
            ..good.clone()
        }),
        "curve_segments"
    );
}

fn stream(beat: usize, share_drawn: f64) -> StreamStyle {
    StreamStyle {
        beat,
        unit: NonZeroU64::new(1).unwrap(),
        full: Glyph::Disc,
        hollow: Glyph::Ring,
        size: 1.0,
        swatch: 0,
        share_drawn,
        max_marks: 10,
    }
}

#[test]
fn styles_refuse_an_unknown_swatch_and_a_stream_without_its_beat() {
    let p = palette();
    let c = FieldConfig::three_beats(Duration::from_millis(100));
    let s = FieldStyles {
        particles: vec![ParticleStyle {
            glyph: Glyph::Disc,
            size: 1.0,
            swatch: 5,
        }],
        ..Default::default()
    };
    assert_eq!(
        s.validate(&p, &c),
        Err(FieldError::UnknownStyle {
            what: "swatch",
            index: 5,
            len: 2
        })
    );
    let s = FieldStyles {
        streams: vec![stream(3, 1.0)],
        ..Default::default()
    };
    assert_eq!(
        s.validate(&p, &c),
        Err(FieldError::UnknownStyle {
            what: "beat",
            index: 3,
            len: 3
        })
    );
    let s = FieldStyles {
        streams: vec![stream(0, 0.0)],
        ..Default::default()
    };
    assert!(matches!(
        s.validate(&p, &c),
        Err(FieldError::BadParameter {
            name: "share_drawn",
            ..
        })
    ));
    let s = FieldStyles {
        streams: vec![stream(0, 1.0)],
        ..Default::default()
    };
    assert_eq!(s.validate(&p, &c), Ok(()));
}
