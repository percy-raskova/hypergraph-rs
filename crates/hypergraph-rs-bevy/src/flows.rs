//! Which flows are drawn, and the report of what is not.

use bevy::prelude::*;

use crate::error::FieldError;
use crate::style::StreamStyle;

/// An amount of one stream moving between two places.
///
/// `amount` is in the stream's own smallest unit. `hollow` marks a flow that is
/// shown but counted apart.
#[derive(Clone, Debug, PartialEq)]
pub struct Flow {
    /// The stream's index in the styles.
    pub stream: u16,
    /// Where it leaves.
    pub from: Vec3,
    /// Where it arrives.
    pub to: Vec3,
    /// The amount, in the stream's smallest unit.
    pub amount: u64,
    /// Whether it is counted apart as hollow.
    pub hollow: bool,
}

/// A flow chosen to be drawn, and how many marks it gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawnFlow {
    /// Its index in the input.
    pub flow: usize,
    /// Its stream.
    pub stream: u16,
    /// Whether it is hollow.
    pub hollow: bool,
    /// How many marks stand for it.
    pub marks: u32,
}

/// What one stream (full or hollow) drew and did not draw.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StreamReport {
    /// The stream's index.
    pub stream: u16,
    /// Whether this is the hollow count.
    pub hollow: bool,
    /// The sum of every flow's amount.
    pub total: u128,
    /// The amount the drawn marks stand for.
    pub drawn: u128,
    /// `total - drawn`.
    pub not_drawn: u128,
    /// The number of marks.
    pub marks: u32,
    /// The number of flows.
    pub paths: u32,
    /// The number of flows drawn.
    pub paths_drawn: u32,
}

/// Every stream's report, apart. There is no grand total.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct FlowReport {
    /// The period the flows belong to.
    pub period: Option<u64>,
    /// One entry per stream and variant, in stream order, full then hollow.
    pub streams: Vec<StreamReport>,
}

/// Choose which flows to draw, and count the rest.
///
/// For each stream, full flows then hollow flows: flows are taken largest
/// first until `share_drawn` of the total is shown or `max_marks` is reached;
/// a flow gets `amount / unit` marks, and a flow of less than one mark or with
/// the same place at both ends is counted, not drawn.
///
/// # Errors
///
/// `UnknownStyle` for a stream not in `streams`; `NonFinite` for an endpoint
/// that is not finite.
pub fn select_flows(
    flows: &[Flow],
    streams: &[StreamStyle],
) -> Result<(Vec<DrawnFlow>, FlowReport), FieldError> {
    for (i, f) in flows.iter().enumerate() {
        if usize::from(f.stream) >= streams.len() {
            return Err(FieldError::UnknownStyle {
                what: "stream",
                index: f.stream,
                len: streams.len(),
            });
        }
        if !f.from.is_finite() || !f.to.is_finite() {
            return Err(FieldError::NonFinite {
                item: format!("flow #{i}"),
            });
        }
    }
    let mut drawn_flows = Vec::new();
    let mut report = FlowReport::default();
    for (s, style) in streams.iter().enumerate() {
        let stream = s as u16;
        let unit = u128::from(style.unit.get());
        for hollow in [false, true] {
            let mut r = StreamReport {
                stream,
                hollow,
                ..StreamReport::default()
            };
            let mut candidates: Vec<usize> = Vec::new();
            for (i, f) in flows.iter().enumerate() {
                if f.stream != stream || f.hollow != hollow {
                    continue;
                }
                r.total += u128::from(f.amount);
                r.paths += 1;
                if f.from.x != f.to.x || f.from.z != f.to.z {
                    candidates.push(i);
                }
            }
            candidates.sort_by(|a, b| flows[*b].amount.cmp(&flows[*a].amount).then(a.cmp(b)));
            let target_f = (style.share_drawn * r.total as f64).ceil();
            let target = if target_f >= r.total as f64 {
                r.total
            } else {
                target_f.max(0.0) as u128
            };
            for i in candidates {
                if r.drawn >= target {
                    break;
                }
                let room = u128::from(style.max_marks - r.marks);
                let n = (u128::from(flows[i].amount) / unit).min(room) as u32;
                if n == 0 {
                    if r.marks < style.max_marks {
                        continue;
                    }
                    break;
                }
                r.drawn += u128::from(n) * unit;
                r.marks += n;
                r.paths_drawn += 1;
                drawn_flows.push(DrawnFlow {
                    flow: i,
                    stream,
                    hollow,
                    marks: n,
                });
            }
            r.not_drawn = r.total - r.drawn;
            report.streams.push(r);
        }
    }
    Ok((drawn_flows, report))
}
