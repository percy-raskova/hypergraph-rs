mod common;

use std::time::Duration;

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use common::{advance_to, field_app, Collected};
use hypergraph_rs::geometry::Outline;
use hypergraph_rs_bevy::{
    diff_sets, FieldConfig, FieldFrame, FieldStyles, Hatch, HatchDirection, LineStyle, Palette,
    SetFrame, SetId, SetStyle, ShowPeriod,
};

fn palette() -> Palette {
    Palette {
        ramp: [
            Color::srgb(0.0, 0.0, 1.0),
            Color::srgb(0.5, 0.5, 0.5),
            Color::srgb(1.0, 0.0, 0.0),
        ],
        no_reading: Color::srgb(0.3, 0.3, 0.3),
        swatches: vec![Color::srgb(1.0, 1.0, 0.0), Color::srgb(0.0, 1.0, 1.0)],
    }
}

fn styles() -> FieldStyles {
    FieldStyles {
        particles: vec![],
        sets: vec![
            SetStyle {
                swatch: 0,
                line: LineStyle {
                    width: 0.05,
                    dash: None,
                },
                hatch: Some(Hatch {
                    direction: HatchDirection::Deg45,
                    spacing: 0.2,
                    width: 0.02,
                }),
            },
            SetStyle {
                swatch: 1,
                line: LineStyle {
                    width: 0.05,
                    dash: Some((0.3, 0.2)),
                },
                hatch: None,
            },
            SetStyle {
                swatch: 0,
                line: LineStyle {
                    width: 0.05,
                    dash: None,
                },
                hatch: Some(Hatch {
                    direction: HatchDirection::Deg135,
                    spacing: 0.2,
                    width: 0.02,
                }),
            },
        ],
        streams: vec![],
        region_outline: None,
    }
}

fn app() -> App {
    let mut app = field_app(
        FieldConfig::three_beats(Duration::from_secs(1)),
        styles(),
        palette(),
    );
    app.update();
    app
}

fn ring(id: &str, style: u16, cx: f64, radius: f64, members: &[(&str, f64)]) -> SetFrame {
    SetFrame {
        id: id.to_string(),
        style,
        height: 0.5,
        members: members
            .iter()
            .map(|(m, n)| ((*m).to_string(), *n))
            .collect(),
        outline: Some(Outline::Ring {
            center: (cx, 0.0),
            radius,
        }),
        held_share: Some(1.0),
    }
}

fn frame(period: u64, sets: Vec<SetFrame>) -> FieldFrame {
    FieldFrame {
        period,
        sets,
        ..FieldFrame::default()
    }
}

fn send(app: &mut App, frame: FieldFrame) -> Duration {
    app.world_mut().write_message(ShowPeriod(Box::new(frame)));
    app.update();
    app.world().resource::<Time>().elapsed()
}

fn secs(s: f64) -> Duration {
    Duration::from_secs_f64(s)
}

fn children_of(app: &mut App, id: &str) -> Option<Vec<Entity>> {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &SetId)>();
    let entity = q.iter(world).find(|(_, s)| s.0 == id).map(|(e, _)| e)?;
    Some(
        world
            .get::<Children>(entity)
            .map(|c| c.iter().collect())
            .unwrap_or_default(),
    )
}

fn child_positions(app: &mut App, child: Entity) -> Vec<[f32; 3]> {
    let world = app.world();
    let handle = world.get::<Mesh3d>(child).expect("a mesh").0.clone();
    match world
        .resource::<Assets<Mesh>>()
        .get(&handle)
        .unwrap()
        .attribute(Mesh::ATTRIBUTE_POSITION)
    {
        Some(VertexAttributeValues::Float32x3(v)) => v.clone(),
        other => panic!("positions: {other:?}"),
    }
}

fn handles(app: &mut App, id: &str) -> Vec<AssetId<Mesh>> {
    children_of(app, id)
        .unwrap()
        .into_iter()
        .map(|c| app.world().get::<Mesh3d>(c).unwrap().0.id())
        .collect()
}

#[test]
fn a_set_gets_a_line_and_a_hatch() {
    let mut app = app();
    send(
        &mut app,
        frame(1, vec![ring("club", 0, 0.0, 1.0, &[("a", 1.0)])]),
    );
    advance_to(&mut app, secs(4.0));
    let kids = children_of(&mut app, "club").unwrap();
    assert_eq!(kids.len(), 2);
    for kid in kids {
        let p = child_positions(&mut app, kid);
        assert!(!p.is_empty());
        for v in p {
            assert!(v[1] == 0.5 || v[1] == 0.5 + 0.001, "y = {}", v[1]);
        }
    }
}

#[test]
fn a_plain_grouping_gets_only_a_dashed_line() {
    let mut app = app();
    send(
        &mut app,
        frame(1, vec![ring("team", 1, 0.0, 1.0, &[("a", 1.0)])]),
    );
    advance_to(&mut app, secs(4.0));
    assert_eq!(children_of(&mut app, "team").unwrap().len(), 1);
}

#[test]
fn two_overlapping_sets_both_show_their_hatching() {
    let mut app = app();
    send(
        &mut app,
        frame(
            1,
            vec![
                ring("club", 0, 0.0, 1.0, &[("a", 1.0)]),
                ring("team", 2, 0.5, 1.0, &[("b", 1.0)]),
            ],
        ),
    );
    advance_to(&mut app, secs(4.0));
    for id in ["club", "team"] {
        let kids = children_of(&mut app, id).unwrap();
        assert_eq!(kids.len(), 2, "{id}");
        assert!(!child_positions(&mut app, kids[1]).is_empty(), "{id}");
    }
}

#[test]
fn set_changes_are_sent_in_order() {
    let mut app = app();
    let one = frame(
        1,
        vec![
            ring("club", 0, 0.0, 1.0, &[("a", 1.0), ("b", 1.0)]),
            ring("team", 1, 0.0, 1.0, &[("c", 1.0)]),
        ],
    );
    let two = frame(
        2,
        vec![
            ring("club", 0, 0.0, 1.0, &[("b", 1.0), ("d", 1.0)]),
            ring("crew", 1, 0.0, 1.0, &[("e", 1.0)]),
        ],
    );
    send(&mut app, one.clone());
    app.world_mut().resource_mut::<Collected>().changes.clear();
    send(&mut app, two.clone());
    let got: Vec<_> = app
        .world()
        .resource::<Collected>()
        .changes
        .iter()
        .map(|c| c.0.clone())
        .collect();
    assert_eq!(got, diff_sets(&one.sets, &two.sets));
    assert!(!got.is_empty());
}

#[test]
fn a_changed_set_swaps_its_meshes_at_the_end_of_the_ease_beat() {
    let mut app = app();
    send(
        &mut app,
        frame(1, vec![ring("club", 0, 0.0, 1.0, &[("a", 1.0)])]),
    );
    advance_to(&mut app, secs(4.0));
    let max_x = |app: &mut App| {
        let kids = children_of(app, "club").unwrap();
        child_positions(app, kids[0])
            .iter()
            .map(|v| v[0].abs())
            .fold(0.0_f32, f32::max)
    };
    let t0 = send(
        &mut app,
        frame(2, vec![ring("club", 0, 0.0, 2.0, &[("a", 1.0)])]),
    );
    advance_to(&mut app, t0 + secs(2.5));
    let x = max_x(&mut app);
    assert!((0.99..=1.03).contains(&x), "x = {x}");
    advance_to(&mut app, t0 + secs(3.0));
    let x = max_x(&mut app);
    assert!((1.99..=2.03).contains(&x), "x = {x}");
    advance_to(&mut app, t0 + secs(5.0));
    let x = max_x(&mut app);
    assert!((1.99..=2.03).contains(&x), "x = {x}");
}

#[test]
fn a_set_of_zeros_draws_nothing_and_says_so() {
    let mut app = app();
    let mut zeros = ring("club", 0, 0.0, 1.0, &[("a", 0.0)]);
    zeros.outline = None;
    zeros.held_share = None;
    let t0 = send(&mut app, frame(1, vec![zeros.clone()]));
    advance_to(&mut app, t0 + secs(2.5));
    assert_eq!(children_of(&mut app, "club").unwrap().len(), 0);
    advance_to(&mut app, secs(5.0));
    assert_eq!(children_of(&mut app, "club").unwrap().len(), 0);
    let t1 = send(&mut app, frame(2, vec![]));
    advance_to(&mut app, t1 + secs(2.5));
    assert!(children_of(&mut app, "club").is_some());
    advance_to(&mut app, t1 + secs(3.0));
    app.update();
    assert!(children_of(&mut app, "club").is_none());
}

#[test]
fn an_unchanged_set_is_not_rebuilt() {
    let mut app = app();
    let set = ring("club", 0, 0.0, 1.0, &[("a", 1.0)]);
    send(&mut app, frame(1, vec![set.clone()]));
    advance_to(&mut app, secs(4.0));
    let before = handles(&mut app, "club");
    let t0 = send(&mut app, frame(2, vec![set]));
    advance_to(&mut app, t0 + secs(4.0));
    assert_eq!(before, handles(&mut app, "club"));
}
