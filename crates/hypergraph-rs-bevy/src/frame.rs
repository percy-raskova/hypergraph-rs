//! The frame a caller hands the plugin once per period, sets read from a
//! hypergraph, and the changes between two periods' sets.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use hypergraph_rs::geometry::{edge_outline, GeometryError, Outline, OutlineParams, Positions};
use hypergraph_rs::Hypergraph;

use crate::error::FieldError;
use crate::flows::{select_flows, Flow};
use crate::style::FieldStyles;

/// What one region reads this period.
///
/// `None` means there is no reading: the region is drawn flat in the palette's
/// no-reading colour.
#[derive(Clone, Debug, PartialEq)]
pub struct RegionReading {
    /// The region's id, as its shape names it.
    pub region: String,
    /// Its height in scene units.
    pub height: Option<f32>,
    /// Its tone in `-1..=1`.
    pub tone: Option<f32>,
}

/// One particle this period.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleFrame {
    /// Its id, unique within the frame.
    pub id: String,
    /// Where it is, in scene coordinates.
    pub at: Vec3,
    /// Its style, an index into the particle styles.
    pub style: u16,
}

/// One set (a hyperedge) this period: who is in it, how much, and its outline.
#[derive(Clone, Debug, PartialEq)]
pub struct SetFrame {
    /// The set's id, unique within the frame.
    pub id: String,
    /// Its style, an index into the set styles.
    pub style: u16,
    /// The height its outline is drawn at.
    pub height: f32,
    /// Every bare member with its number, zeros included.
    pub members: Vec<(String, f64)>,
    /// The outline of the members above zero; `None` when none is above zero.
    pub outline: Option<Outline>,
    /// The share of the numbers that the outline holds.
    pub held_share: Option<f64>,
}

/// Everything the picture shows for one period.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldFrame {
    /// The period's number.
    pub period: u64,
    /// The readings of regions.
    pub regions: Vec<RegionReading>,
    /// The particles.
    pub particles: Vec<ParticleFrame>,
    /// The sets.
    pub sets: Vec<SetFrame>,
    /// The flows between places.
    pub flows: Vec<Flow>,
}

fn non_finite(item: &str) -> FieldError {
    FieldError::NonFinite {
        item: item.to_owned(),
    }
}

fn first_duplicate<'a>(ids: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let mut seen = BTreeSet::new();
    ids.into_iter().find(|id| !seen.insert(*id))
}

fn finite_point(p: (f64, f64)) -> bool {
    p.0.is_finite() && p.1.is_finite()
}

fn outline_is_finite(outline: &Outline) -> bool {
    match outline {
        Outline::Ring { center, radius } => finite_point(*center) && radius.is_finite(),
        Outline::Capsule {
            a,
            radius_a,
            b,
            radius_b,
        } => finite_point(*a) && finite_point(*b) && radius_a.is_finite() && radius_b.is_finite(),
        Outline::Membrane { paths } => paths.iter().all(|p| p.iter().all(|q| finite_point(*q))),
        // A kind this crate does not draw is refused where it is drawn.
        _ => true,
    }
}

fn check_style(what: &'static str, index: u16, len: usize) -> Result<(), FieldError> {
    if usize::from(index) < len {
        Ok(())
    } else {
        Err(FieldError::UnknownStyle { what, index, len })
    }
}

impl FieldFrame {
    /// Checks the frame as far as it can without the grounds: ids are unique,
    /// numbers are finite, tones are in range, styles exist and flows can be
    /// chosen. Returns the first fault. Whether each region is in a ground is
    /// for the plugin to check.
    ///
    /// # Errors
    ///
    /// The first fault found, in the order: regions, particles, sets, flows.
    pub fn validate(&self, styles: &FieldStyles) -> Result<(), FieldError> {
        if let Some(id) = first_duplicate(self.regions.iter().map(|r| r.region.as_str())) {
            return Err(FieldError::DuplicateId(id.to_owned()));
        }
        for r in &self.regions {
            if r.height.is_some_and(|h| !h.is_finite()) {
                return Err(non_finite(&r.region));
            }
        }
        for r in &self.regions {
            if let Some(t) = r.tone {
                if !t.is_finite() {
                    return Err(non_finite(&r.region));
                }
                if !(-1.0..=1.0).contains(&t) {
                    return Err(FieldError::ToneOutOfRange {
                        region: r.region.clone(),
                        value: t,
                    });
                }
            }
        }
        if let Some(id) = first_duplicate(self.particles.iter().map(|p| p.id.as_str())) {
            return Err(FieldError::DuplicateId(id.to_owned()));
        }
        for p in &self.particles {
            if !p.at.is_finite() {
                return Err(non_finite(&p.id));
            }
            check_style("particle style", p.style, styles.particles.len())?;
        }
        if let Some(id) = first_duplicate(self.sets.iter().map(|s| s.id.as_str())) {
            return Err(FieldError::DuplicateId(id.to_owned()));
        }
        for s in &self.sets {
            let finite = s.height.is_finite()
                && s.members.iter().all(|(_, w)| w.is_finite())
                && s.outline.as_ref().is_none_or(outline_is_finite)
                && s.held_share.is_none_or(f64::is_finite);
            if !finite {
                return Err(non_finite(&s.id));
            }
            check_style("set style", s.style, styles.sets.len())?;
        }
        select_flows(&self.flows, &styles.streams)?;
        Ok(())
    }
}

/// Reads sets from a hypergraph: one [`SetFrame`] for each `(edge id, style)`.
///
/// `members` lists every bare member of the edge in `members_with_data` order
/// with `weight` of its membership value, zeros included. The outline and the
/// held share come from the core's `edge_outline`.
///
/// # Errors
///
/// [`FieldError::Geometry`] for an unknown edge (`UnknownEdge`) or any fault
/// `edge_outline` finds in the numbers, positions or `params`.
pub fn set_frames<N, E, M>(
    h: &Hypergraph<N, E, M>,
    sets: &[(&str, u16)],
    positions: &Positions,
    weight: impl Fn(&M) -> f64,
    params: &OutlineParams,
    height: f32,
) -> Result<Vec<SetFrame>, FieldError> {
    let mut out = Vec::with_capacity(sets.len());
    for &(id, style) in sets {
        let listing = h
            .members_with_data(id)
            .ok_or_else(|| GeometryError::UnknownEdge(id.to_owned()))?;
        let members: Vec<(String, f64)> = listing
            .into_iter()
            .map(|(node, data)| (node, weight(data)))
            .collect();
        let (outline, held_share) = match edge_outline(h, id, positions, &weight, params)? {
            None => (None, None),
            Some(e) => {
                let w = e.outline;
                (Some(w.outline), Some(w.held_weight / w.total_weight))
            }
        };
        out.push(SetFrame {
            id: id.to_owned(),
            style,
            height,
            members,
            outline,
            held_share,
        });
    }
    Ok(out)
}

/// One set's change between two periods.
#[derive(Clone, Debug, PartialEq)]
pub enum SetChange {
    /// A set that was not there before.
    Appeared {
        /// The set's id.
        set: String,
    },
    /// A set that is gone.
    Vanished {
        /// The set's id.
        set: String,
    },
    /// A set whose members changed.
    Changed {
        /// The set's id.
        set: String,
        /// Members in the next and not in the previous, in the next's order.
        joined: Vec<String>,
        /// Members in the previous and not in the next, in the previous's order.
        left: Vec<String>,
        /// Members in both whose number differs, in the next's order.
        renumbered: Vec<String>,
    },
}

/// Tells what changed between two periods' sets.
///
/// Membership is the bare membership: a member whose number went to zero is
/// `renumbered`, not `left`. Order: first, in `next`'s order, `Appeared` and
/// `Changed`; then, in `previous`'s order, `Vanished`. The cost is linear in
/// the members.
#[must_use]
pub fn diff_sets(previous: &[SetFrame], next: &[SetFrame]) -> Vec<SetChange> {
    let before: BTreeMap<&str, &SetFrame> = previous.iter().map(|s| (s.id.as_str(), s)).collect();
    let after: BTreeSet<&str> = next.iter().map(|s| s.id.as_str()).collect();
    let mut out = Vec::new();
    for s in next {
        let Some(old) = before.get(s.id.as_str()) else {
            out.push(SetChange::Appeared { set: s.id.clone() });
            continue;
        };
        let old_numbers: BTreeMap<&str, f64> =
            old.members.iter().map(|(m, w)| (m.as_str(), *w)).collect();
        let new_numbers: BTreeMap<&str, f64> =
            s.members.iter().map(|(m, w)| (m.as_str(), *w)).collect();
        let mut joined = Vec::new();
        let mut renumbered = Vec::new();
        for (m, w) in &s.members {
            match old_numbers.get(m.as_str()) {
                None => joined.push(m.clone()),
                Some(o) if o.to_bits() != w.to_bits() => renumbered.push(m.clone()),
                Some(_) => {}
            }
        }
        let left: Vec<String> = old
            .members
            .iter()
            .filter(|(m, _)| !new_numbers.contains_key(m.as_str()))
            .map(|(m, _)| m.clone())
            .collect();
        if !(joined.is_empty() && left.is_empty() && renumbered.is_empty()) {
            out.push(SetChange::Changed {
                set: s.id.clone(),
                joined,
                left,
                renumbered,
            });
        }
    }
    for s in previous {
        if !after.contains(s.id.as_str()) {
            out.push(SetChange::Vanished { set: s.id.clone() });
        }
    }
    out
}
