//! Chart segmentation: splitting a mesh into disk-like patches along seams so
//! each can be flattened independently, then unwrapping each with LSCM.

use glam::{Vec2, Vec3};
use std::collections::HashMap;

use crate::lscm::{conformal_distortion, unwrap_lscm};

/// A single UV island: a connected set of triangles flattened into the plane.
/// Vertices are stored locally (a global vertex on a seam appears once per
/// chart it touches, so charts can be packed and edited independently).
#[derive(Clone, Debug)]
pub struct Chart {
    /// Indices into the original triangle list that make up this chart.
    pub source_faces: Vec<usize>,
    /// Local vertex positions.
    pub positions: Vec<Vec3>,
    /// Triangles in local vertex indices.
    pub triangles: Vec<[usize; 3]>,
    /// Map from local vertex index back to the original mesh vertex index.
    pub local_to_global: Vec<usize>,
    /// Per-local-vertex UVs (filled by [`Chart::unwrap`]).
    pub uvs: Vec<Vec2>,
}

impl Chart {
    /// Flatten this chart with LSCM, populating `uvs`.
    pub fn unwrap(&mut self) {
        self.uvs = unwrap_lscm(&self.positions, &self.triangles).uvs;
    }

    /// Mean angular distortion of the current UVs.
    pub fn distortion(&self) -> f32 {
        conformal_distortion(&self.positions, &self.uvs, &self.triangles)
    }

    /// Bounding-box size of the current UVs.
    pub fn uv_extent(&self) -> Vec2 {
        if self.uvs.is_empty() {
            return Vec2::ZERO;
        }
        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        for uv in &self.uvs {
            min = min.min(*uv);
            max = max.max(*uv);
        }
        max - min
    }
}

fn edge_key(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}

/// Split a triangle mesh into charts. Triangles sharing an edge stay in the
/// same chart unless that edge is a seam. With an empty seam set this yields the
/// connected components of the mesh.
pub fn segment_charts(
    positions: &[Vec3],
    triangles: &[[usize; 3]],
    seams: &[(usize, usize)],
) -> Vec<Chart> {
    let seam_set: std::collections::HashSet<(usize, usize)> =
        seams.iter().map(|&(a, b)| edge_key(a, b)).collect();

    // Map each non-seam edge to the faces that share it.
    let mut edge_faces: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (fi, t) in triangles.iter().enumerate() {
        for k in 0..3 {
            let key = edge_key(t[k], t[(k + 1) % 3]);
            if !seam_set.contains(&key) {
                edge_faces.entry(key).or_default().push(fi);
            }
        }
    }

    // Union-find over faces.
    let mut parent: Vec<usize> = (0..triangles.len()).collect();
    fn find(parent: &mut [usize], x: usize) -> usize {
        let mut root = x;
        while parent[root] != root {
            root = parent[root];
        }
        let mut cur = x;
        while parent[cur] != root {
            let next = parent[cur];
            parent[cur] = root;
            cur = next;
        }
        root
    }
    for faces in edge_faces.values() {
        for w in faces.windows(2) {
            let a = find(&mut parent, w[0]);
            let b = find(&mut parent, w[1]);
            if a != b {
                parent[a] = b;
            }
        }
    }

    // Group faces by root.
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for fi in 0..triangles.len() {
        let r = find(&mut parent, fi);
        groups.entry(r).or_default().push(fi);
    }

    // Build a Chart per group with a local vertex remap.
    let mut charts = Vec::new();
    for (_root, faces) in groups {
        let mut global_to_local: HashMap<usize, usize> = HashMap::new();
        let mut local_positions = Vec::new();
        let mut local_to_global = Vec::new();
        let mut local_tris = Vec::new();

        for &fi in &faces {
            let t = triangles[fi];
            let mut lt = [0usize; 3];
            for k in 0..3 {
                let g = t[k];
                let local = *global_to_local.entry(g).or_insert_with(|| {
                    local_positions.push(positions[g]);
                    local_to_global.push(g);
                    local_positions.len() - 1
                });
                lt[k] = local;
            }
            local_tris.push(lt);
        }

        charts.push(Chart {
            source_faces: faces,
            positions: local_positions,
            triangles: local_tris,
            local_to_global,
            uvs: Vec::new(),
        });
    }

    // Stable ordering (largest chart first) for deterministic packing.
    charts.sort_by(|a, b| b.triangles.len().cmp(&a.triangles.len()));
    charts
}

/// Segment along seams and unwrap every chart.
pub fn unwrap_charts(
    positions: &[Vec3],
    triangles: &[[usize; 3]],
    seams: &[(usize, usize)],
) -> Vec<Chart> {
    let mut charts = segment_charts(positions, triangles, seams);
    for chart in &mut charts {
        chart.unwrap();
    }
    charts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(n: usize) -> (Vec<Vec3>, Vec<[usize; 3]>) {
        let mut pos = Vec::new();
        for y in 0..=n {
            for x in 0..=n {
                pos.push(Vec3::new(x as f32, y as f32, 0.0));
            }
        }
        let w = n + 1;
        let mut tris = Vec::new();
        for y in 0..n {
            for x in 0..n {
                let tl = y * w + x;
                tris.push([tl, tl + w, tl + 1]);
                tris.push([tl + 1, tl + w, tl + w + 1]);
            }
        }
        (pos, tris)
    }

    #[test]
    fn no_seams_gives_single_chart() {
        let (pos, tris) = grid(3);
        let charts = segment_charts(&pos, &tris, &[]);
        assert_eq!(charts.len(), 1);
        assert_eq!(charts[0].triangles.len(), tris.len());
    }

    #[test]
    fn seam_splits_into_two_charts() {
        // Two separate triangles, no shared edge -> two charts.
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(3.0, 1.0, 0.0),
        ];
        let tris = vec![[0, 1, 2], [3, 4, 5]];
        let charts = segment_charts(&pos, &tris, &[]);
        assert_eq!(charts.len(), 2);
    }

    #[test]
    fn unwrap_charts_populates_uvs() {
        let (pos, tris) = grid(3);
        let charts = unwrap_charts(&pos, &tris, &[]);
        assert_eq!(charts.len(), 1);
        let chart = &charts[0];
        assert_eq!(chart.uvs.len(), chart.positions.len());
        // Flat grid -> low distortion.
        assert!(chart.distortion() < 0.05);
    }

    #[test]
    fn local_remap_is_consistent() {
        let (pos, tris) = grid(2);
        let charts = segment_charts(&pos, &tris, &[]);
        let chart = &charts[0];
        for (li, &gi) in chart.local_to_global.iter().enumerate() {
            assert_eq!(chart.positions[li], pos[gi]);
        }
    }
}
