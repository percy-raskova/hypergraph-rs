use bevy::prelude::*;
use hypergraph_rs::geometry::{GeometryError, Outline, OutlineParams, Positions};
use hypergraph_rs::Hypergraph;
use hypergraph_rs_bevy::{
    diff_sets, set_frames, FieldError, FieldFrame, FieldStyles, Glyph, LineStyle, ParticleFrame,
    ParticleStyle, RegionReading, SetChange, SetFrame, SetStyle,
};

type H = Hypergraph<(), (), f64>;

fn link(h: &mut H, edge: &str, node: &str, w: f64) {
    h.add_node_to_edge_with_data(edge, node, w).unwrap();
}

fn params() -> OutlineParams {
    OutlineParams::new(1.0, 0.125, 0.9)
}

fn at(list: &[(&str, (f64, f64))]) -> Positions {
    list.iter().map(|(id, p)| ((*id).to_string(), *p)).collect()
}

fn set(id: &str, members: &[(&str, f64)]) -> SetFrame {
    SetFrame {
        id: id.to_string(),
        style: 0,
        height: 0.0,
        members: members
            .iter()
            .map(|(m, w)| ((*m).to_string(), *w))
            .collect(),
        outline: None,
        held_share: None,
    }
}

fn styles() -> FieldStyles {
    let particle = ParticleStyle {
        glyph: Glyph::Disc,
        size: 1.0,
        swatch: 0,
    };
    FieldStyles {
        particles: vec![particle.clone(), particle],
        sets: vec![SetStyle {
            swatch: 0,
            line: LineStyle {
                width: 0.1,
                dash: None,
            },
            hatch: None,
        }],
        streams: vec![],
        region_outline: None,
    }
}

fn particle(id: &str, style: u16) -> ParticleFrame {
    ParticleFrame {
        id: id.to_string(),
        at: Vec3::ZERO,
        style,
    }
}

#[test]
fn set_frames_read_every_membership_and_the_outline() {
    let mut h = H::new();
    link(&mut h, "club", "a", 2.0);
    link(&mut h, "club", "b", 1.0);
    link(&mut h, "club", "z", 0.0);
    let positions = at(&[("a", (0.0, 0.0)), ("b", (5.0, 0.0))]);
    let frames = set_frames(&h, &[("club", 1)], &positions, |w| *w, &params(), 0.5).unwrap();
    assert_eq!(frames.len(), 1);
    let f = &frames[0];
    assert_eq!(f.id, "club");
    assert_eq!(f.style, 1);
    assert_eq!(f.height, 0.5);
    assert_eq!(
        f.members,
        vec![
            ("a".to_string(), 2.0),
            ("b".to_string(), 1.0),
            ("z".to_string(), 0.0)
        ]
    );
    assert_eq!(
        f.outline,
        Some(Outline::Capsule {
            a: (0.0, 0.0),
            radius_a: 1.0,
            b: (5.0, 0.0),
            radius_b: 0.75
        })
    );
    assert_eq!(f.held_share, Some(1.0));
}

#[test]
fn a_set_of_zeros_has_no_outline() {
    let mut h = H::new();
    link(&mut h, "club", "a", 0.0);
    link(&mut h, "club", "b", 0.0);
    let frames = set_frames(
        &h,
        &[("club", 0)],
        &Positions::new(),
        |w| *w,
        &params(),
        0.0,
    )
    .unwrap();
    assert_eq!(frames[0].outline, None);
    assert_eq!(frames[0].held_share, None);
    assert_eq!(frames[0].members.len(), 2);
}

#[test]
fn an_unknown_edge_is_refused() {
    let h = H::new();
    let err = set_frames(
        &h,
        &[("nope", 0)],
        &Positions::new(),
        |w| *w,
        &params(),
        0.0,
    )
    .unwrap_err();
    assert_eq!(
        err,
        FieldError::Geometry(GeometryError::UnknownEdge("nope".to_string()))
    );
}

#[test]
fn a_change_of_number_is_not_a_change_of_membership() {
    let previous = [set("club", &[("a", 1.0), ("b", 1.0)])];
    let next = [set("club", &[("a", 1.0), ("b", 0.0)])];
    assert_eq!(
        diff_sets(&previous, &next),
        vec![SetChange::Changed {
            set: "club".to_string(),
            joined: vec![],
            left: vec![],
            renumbered: vec!["b".to_string()]
        }]
    );
}

#[test]
fn joins_leaves_appearances_and_vanishings_are_told_apart() {
    let previous = [
        set("club", &[("a", 1.0), ("b", 1.0)]),
        set("team", &[("c", 1.0)]),
    ];
    let next = [
        set("club", &[("b", 1.0), ("d", 1.0)]),
        set("crew", &[("e", 1.0)]),
    ];
    assert_eq!(
        diff_sets(&previous, &next),
        vec![
            SetChange::Changed {
                set: "club".to_string(),
                joined: vec!["d".to_string()],
                left: vec!["a".to_string()],
                renumbered: vec![]
            },
            SetChange::Appeared {
                set: "crew".to_string()
            },
            SetChange::Vanished {
                set: "team".to_string()
            },
        ]
    );
}

#[test]
fn an_unchanged_set_reports_nothing() {
    let frames = [set("club", &[("a", 1.0), ("b", 2.0)]), set("team", &[])];
    assert_eq!(diff_sets(&frames, &frames), vec![]);
}

#[test]
fn a_frame_names_what_is_wrong_with_it() {
    let styles = styles();
    let ok = FieldFrame::default();
    assert_eq!(ok.validate(&styles), Ok(()));

    let frame = FieldFrame {
        particles: vec![particle("p1", 0), particle("p1", 0)],
        ..FieldFrame::default()
    };
    assert_eq!(
        frame.validate(&styles),
        Err(FieldError::DuplicateId("p1".to_string()))
    );

    let frame = FieldFrame {
        regions: vec![RegionReading {
            region: "r0".to_string(),
            height: Some(1.0),
            tone: Some(1.5),
        }],
        ..FieldFrame::default()
    };
    assert_eq!(
        frame.validate(&styles),
        Err(FieldError::ToneOutOfRange {
            region: "r0".to_string(),
            value: 1.5
        })
    );

    let frame = FieldFrame {
        particles: vec![particle("p1", 9)],
        ..FieldFrame::default()
    };
    assert_eq!(
        frame.validate(&styles),
        Err(FieldError::UnknownStyle {
            what: "particle style",
            index: 9,
            len: 2
        })
    );

    let mut nan_height = set("club", &[]);
    nan_height.height = f32::NAN;
    let frame = FieldFrame {
        sets: vec![nan_height],
        ..FieldFrame::default()
    };
    assert_eq!(
        frame.validate(&styles),
        Err(FieldError::NonFinite {
            item: "club".to_string()
        })
    );

    let frame = FieldFrame {
        sets: vec![set("club", &[("a", f64::INFINITY)])],
        ..FieldFrame::default()
    };
    assert_eq!(
        frame.validate(&styles),
        Err(FieldError::NonFinite {
            item: "club".to_string()
        })
    );
}
