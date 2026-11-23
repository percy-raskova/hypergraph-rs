//! The ground: regions drawn as one bending surface or as blocks that rise
//! and sink.
//!
//! Both forms share one set of [`RegionShape`]s, so a region's outline in
//! plan is the same in either. The ground carries vertex colours and nothing
//! else: it is never hatched.

use std::collections::BTreeMap;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;
use hypergraph_rs::geometry::{ClosedPath, Point};

use crate::error::FieldError;
use crate::lines::plan_to_scene;

/// One region of the ground: triangles to fill it and rings that outline it.
///
/// The caller triangulates; this crate has no triangulator.
#[derive(Clone, Debug, PartialEq)]
pub struct RegionShape {
    /// The region's id, unique within a ground.
    pub id: String,
    /// The triangles that fill the region, in plan coordinates.
    pub triangles: Vec<[Point; 3]>,
    /// The closed rings that bound the region (its holes included), in plan
    /// coordinates.
    pub rings: Vec<ClosedPath>,
}

/// How a ground is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GroundForm {
    /// One surface: shared corners meet without a step.
    #[default]
    Surface,
    /// Each region a block that rises or sinks, with walls to `y = 0`.
    Blocks,
}

/// A ground of regions, as a component on an entity.
#[derive(Component, Clone, Debug)]
#[require(Transform, Visibility)]
pub struct Ground {
    /// The regions of this ground.
    pub shapes: Arc<[RegionShape]>,
    /// How the ground is drawn.
    pub form: GroundForm,
}

/// A ring edge with the corner of each end (when the end is a triangle vertex
/// of its own region).
#[derive(Clone, Debug)]
struct Edge {
    from: Point,
    to: Point,
    from_corner: Option<u32>,
    to_corner: Option<u32>,
}

#[derive(Clone, Debug)]
struct Region {
    triangles: Vec<[Point; 3]>,
    corners: Vec<[u32; 3]>,
    edges: Vec<Edge>,
}

/// The fixed structure of a ground: what is drawn where, apart from heights
/// and colours.
///
/// Built once; every write afterwards is linear in the number of vertices.
#[derive(Clone, Debug)]
pub struct GroundTopology {
    form: GroundForm,
    index: BTreeMap<String, usize>,
    regions: Vec<Region>,
    /// For corner `c`, `corner_regions[corner_spans[c].0..corner_spans[c].1]`
    /// are the regions that have a triangle vertex there, sorted.
    corner_spans: Vec<(u32, u32)>,
    corner_regions: Vec<u32>,
}

type Key = (u64, u64);

fn key(p: Point) -> Key {
    (p.0.to_bits(), p.1.to_bits())
}

fn finite(p: Point) -> bool {
    p.0.is_finite() && p.1.is_finite()
}

fn up_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let n = (b - a).cross(c - a);
    let len = n.length();
    if len.is_finite() && len > 1e-12 {
        n / len
    } else {
        Vec3::Y
    }
}

impl GroundTopology {
    /// Check the shapes and build the structure.
    ///
    /// # Errors
    ///
    /// [`FieldError::DuplicateId`] for a repeated id,
    /// [`FieldError::NonFinite`] for a coordinate that is `NaN` or infinite,
    /// and [`FieldError::BadShape`] for a region with no triangles or a ring of
    /// fewer than 2 points.
    pub fn new(shapes: &[RegionShape], form: GroundForm) -> Result<Self, FieldError> {
        let mut index = BTreeMap::new();
        for (i, shape) in shapes.iter().enumerate() {
            if index.insert(shape.id.clone(), i).is_some() {
                return Err(FieldError::DuplicateId(shape.id.clone()));
            }
            let all_finite = shape
                .triangles
                .iter()
                .flatten()
                .chain(shape.rings.iter().flatten())
                .all(|p| finite(*p));
            if !all_finite {
                return Err(FieldError::NonFinite {
                    item: shape.id.clone(),
                });
            }
            if shape.triangles.is_empty() {
                return Err(FieldError::BadShape {
                    region: shape.id.clone(),
                    problem: "no triangles",
                });
            }
            if shape.rings.iter().any(|ring| ring.len() < 2) {
                return Err(FieldError::BadShape {
                    region: shape.id.clone(),
                    problem: "a ring has fewer than 2 points",
                });
            }
        }

        // The corner table: every distinct triangle-vertex position, with the
        // regions that have one there.
        let mut pairs: Vec<(Key, u32)> = Vec::new();
        for (r, shape) in shapes.iter().enumerate() {
            for p in shape.triangles.iter().flatten() {
                pairs.push((key(*p), r as u32));
            }
        }
        pairs.sort_unstable();
        pairs.dedup();
        let mut keys: Vec<Key> = Vec::new();
        let mut corner_spans: Vec<(u32, u32)> = Vec::new();
        let mut corner_regions: Vec<u32> = Vec::with_capacity(pairs.len());
        for (k, r) in &pairs {
            if keys.last() != Some(k) {
                if let Some(last) = corner_spans.last_mut() {
                    last.1 = corner_regions.len() as u32;
                }
                keys.push(*k);
                corner_spans.push((corner_regions.len() as u32, 0));
            }
            corner_regions.push(*r);
        }
        if let Some(last) = corner_spans.last_mut() {
            last.1 = corner_regions.len() as u32;
        }

        let corner_of =
            |p: Point| -> Option<u32> { keys.binary_search(&key(p)).ok().map(|c| c as u32) };
        let has_region = |c: u32, r: u32| -> bool {
            let (s, e) = corner_spans[c as usize];
            corner_regions[s as usize..e as usize]
                .binary_search(&r)
                .is_ok()
        };

        let regions = shapes
            .iter()
            .enumerate()
            .map(|(r, shape)| {
                let corners = shape
                    .triangles
                    .iter()
                    .map(|t| t.map(|p| corner_of(p).unwrap_or(0)))
                    .collect();
                let own = |p: Point| corner_of(p).filter(|c| has_region(*c, r as u32));
                let mut edges = Vec::new();
                for ring in &shape.rings {
                    let n = ring.len();
                    for i in 0..n {
                        let (from, to) = (ring[i], ring[(i + 1) % n]);
                        edges.push(Edge {
                            from,
                            to,
                            from_corner: own(from),
                            to_corner: own(to),
                        });
                    }
                }
                Region {
                    triangles: shape.triangles.clone(),
                    corners,
                    edges,
                }
            })
            .collect();

        Ok(Self {
            form,
            index,
            regions,
            corner_spans,
            corner_regions,
        })
    }

    /// How this ground is drawn.
    #[must_use]
    pub fn form(&self) -> GroundForm {
        self.form
    }

    /// The number of regions.
    #[must_use]
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// The index of the region with this id, in input order.
    #[must_use]
    pub fn region_index(&self, id: &str) -> Option<usize> {
        self.index.get(id).copied()
    }

    /// The height at each corner: the mean of the heights of the regions that
    /// have a triangle vertex there, each counted once.
    fn corner_heights(&self, heights: &[f32]) -> Vec<f32> {
        self.corner_spans
            .iter()
            .map(|&(s, e)| {
                let list = &self.corner_regions[s as usize..e as usize];
                let sum: f32 = list.iter().map(|r| heights[*r as usize]).sum();
                sum / list.len() as f32
            })
            .collect()
    }

    fn check(&self, heights: &[f32], colours: Option<&[LinearRgba]>) {
        assert_eq!(
            heights.len(),
            self.regions.len(),
            "heights must hold one value per region"
        );
        if let Some(c) = colours {
            assert_eq!(
                c.len(),
                self.regions.len(),
                "colours must hold one value per region"
            );
        }
    }

    #[allow(clippy::type_complexity, reason = "three parallel vertex arrays")]
    fn body_arrays(
        &self,
        heights: &[f32],
        colours: &[LinearRgba],
    ) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 4]>) {
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut normals: Vec<[f32; 3]> = Vec::new();
        let mut cols: Vec<[f32; 4]> = Vec::new();
        let mut push = |tri: [Vec3; 3], flip_up: bool, colour: [f32; 4]| {
            let mut n = up_normal(tri[0], tri[1], tri[2]);
            if flip_up && n.y < 0.0 {
                n = -n;
            }
            for v in tri {
                positions.push(v.to_array());
                normals.push(n.to_array());
                cols.push(colour);
            }
        };
        let corner_h = match self.form {
            GroundForm::Surface => self.corner_heights(heights),
            GroundForm::Blocks => Vec::new(),
        };
        for (r, region) in self.regions.iter().enumerate() {
            let h = heights[r];
            let colour = colours[r].to_f32_array();
            match self.form {
                GroundForm::Blocks => {
                    for t in &region.triangles {
                        push(t.map(|p| plan_to_scene(p, h)), true, colour);
                    }
                    for e in &region.edges {
                        let (p0, q0) = (plan_to_scene(e.from, 0.0), plan_to_scene(e.to, 0.0));
                        let (p1, q1) = (plan_to_scene(e.from, h), plan_to_scene(e.to, h));
                        push([p0, q0, q1], false, colour);
                        push([p0, q1, p1], false, colour);
                    }
                }
                GroundForm::Surface => {
                    for (t, c) in region.triangles.iter().zip(&region.corners) {
                        let centre = (
                            (t[0].0 + t[1].0 + t[2].0) / 3.0,
                            (t[0].1 + t[1].1 + t[2].1) / 3.0,
                        );
                        let v = [0, 1, 2].map(|i| plan_to_scene(t[i], corner_h[c[i] as usize]));
                        let m = plan_to_scene(centre, h);
                        push([v[0], v[1], m], true, colour);
                        push([v[1], v[2], m], true, colour);
                        push([v[2], v[0], m], true, colour);
                    }
                }
            }
        }
        (positions, normals, cols)
    }

    /// The body of the ground: a non-indexed triangle list with flat normals
    /// and vertex colours, regions in input order.
    ///
    /// In `Blocks`, each region's triangles at its height, then a wall for
    /// every ring edge. In `Surface`, each triangle split at its centroid,
    /// with corners shared between regions at the mean of their heights.
    ///
    /// # Panics
    ///
    /// If `heights` or `colours` is not [`region_count`](Self::region_count)
    /// long.
    #[must_use]
    pub fn body_mesh(&self, heights: &[f32], colours: &[LinearRgba]) -> Mesh {
        self.check(heights, Some(colours));
        let (positions, normals, cols) = self.body_arrays(heights, colours);
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, cols);
        mesh
    }

    /// Overwrite positions, normals and colours of a mesh made by
    /// [`body_mesh`](Self::body_mesh) of this topology.
    ///
    /// # Panics
    ///
    /// If `heights` or `colours` is not [`region_count`](Self::region_count)
    /// long.
    pub fn write_body(&self, mesh: &mut Mesh, heights: &[f32], colours: &[LinearRgba]) {
        self.check(heights, Some(colours));
        let (positions, normals, cols) = self.body_arrays(heights, colours);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, cols);
    }

    fn outline_positions(&self, heights: &[f32], lift: f32) -> Vec<[f32; 3]> {
        let corner_h = match self.form {
            GroundForm::Surface => self.corner_heights(heights),
            GroundForm::Blocks => Vec::new(),
        };
        let mut out = Vec::new();
        for (r, region) in self.regions.iter().enumerate() {
            let h = heights[r];
            let at = |corner: Option<u32>| match self.form {
                GroundForm::Blocks => h + lift,
                GroundForm::Surface => corner.map_or(h, |c| corner_h[c as usize]) + lift,
            };
            for e in &region.edges {
                out.push(plan_to_scene(e.from, at(e.from_corner)).to_array());
                out.push(plan_to_scene(e.to, at(e.to_corner)).to_array());
            }
        }
        out
    }

    /// The outlines of the regions: a line list, two vertices per ring edge,
    /// at the ground's height plus `lift`.
    ///
    /// # Panics
    ///
    /// If `heights` is not [`region_count`](Self::region_count) long.
    #[must_use]
    pub fn outline_mesh(&self, heights: &[f32], lift: f32) -> Mesh {
        self.check(heights, None);
        let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default());
        let positions = self.outline_positions(heights, lift);
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![[0.0, 1.0, 0.0]; positions.len()],
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh
    }

    /// Overwrite the positions of a mesh made by
    /// [`outline_mesh`](Self::outline_mesh) of this topology.
    ///
    /// # Panics
    ///
    /// If `heights` is not [`region_count`](Self::region_count) long.
    pub fn write_outline(&self, mesh: &mut Mesh, heights: &[f32], lift: f32) {
        self.check(heights, None);
        let positions = self.outline_positions(heights, lift);
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![[0.0, 1.0, 0.0]; positions.len()],
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    }
}
