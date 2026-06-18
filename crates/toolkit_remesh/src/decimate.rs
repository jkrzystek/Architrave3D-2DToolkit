//! QEM (quadric error metric) mesh simplification by iterative edge collapse.
//!
//! Each vertex accumulates the quadric of its incident triangle planes. Edges
//! are collapsed cheapest-first; the surviving vertex moves to whichever of the
//! two endpoints or their midpoint minimises the combined quadric error. A
//! union-find remap tracks merged vertices, and lazy versioning skips stale heap
//! entries — so the whole thing stays a few flat arrays plus a binary heap.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

use glam::Vec3;
use toolkit_geometry::{Mesh, Vertex};

use crate::quadric::Quadric;

struct Candidate {
    cost: f32,
    target: Vec3,
    u: usize,
    v: usize,
    vu: u32,
    vv: u32,
}

impl PartialEq for Candidate {
    fn eq(&self, o: &Self) -> bool {
        self.cost == o.cost
    }
}
impl Eq for Candidate {}
impl Ord for Candidate {
    fn cmp(&self, o: &Self) -> Ordering {
        // Reversed: the smallest cost is the "greatest" so the max-heap pops it.
        o.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

struct Decimator {
    positions: Vec<Vec3>,
    faces: Vec<[usize; 3]>,
    alive: Vec<bool>,
    alive_count: usize,
    quadrics: Vec<Quadric>,
    parent: Vec<usize>,
    version: Vec<u32>,
    vertex_faces: Vec<Vec<usize>>,
}

impl Decimator {
    fn new(mesh: &Mesh) -> Self {
        let positions: Vec<Vec3> = mesh.vertices.iter().map(|v| v.position_vec3()).collect();
        let faces: Vec<[usize; 3]> = mesh
            .indices
            .chunks_exact(3)
            .map(|t| [t[0] as usize, t[1] as usize, t[2] as usize])
            .collect();
        let n = positions.len();
        let mut quadrics = vec![Quadric::zero(); n];
        let mut vertex_faces = vec![Vec::new(); n];
        for (fi, f) in faces.iter().enumerate() {
            let (a, b, c) = (positions[f[0]], positions[f[1]], positions[f[2]]);
            let normal = (b - a).cross(c - a);
            let q = Quadric::from_plane(normal, a);
            for &v in f {
                quadrics[v].add_assign(q);
                vertex_faces[v].push(fi);
            }
        }
        let alive_count = faces.len();
        Self {
            positions,
            alive: vec![true; faces.len()],
            faces,
            alive_count,
            quadrics,
            parent: (0..n).collect(),
            version: vec![0; n],
            vertex_faces,
        }
    }

    fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.parent[r] != r {
            r = self.parent[r];
        }
        // Path compression.
        let mut c = x;
        while self.parent[c] != r {
            let next = self.parent[c];
            self.parent[c] = r;
            c = next;
        }
        r
    }

    /// Best target position and cost for collapsing the edge (u, v).
    fn evaluate(&self, u: usize, v: usize) -> (f32, Vec3) {
        let q = self.quadrics[u].add(self.quadrics[v]);
        let mid = (self.positions[u] + self.positions[v]) * 0.5;
        let candidates = [self.positions[u], self.positions[v], mid];
        let mut best = (f32::INFINITY, mid);
        for &p in &candidates {
            let e = q.error(p);
            if e < best.0 {
                best = (e, p);
            }
        }
        best
    }

    fn push_edge(&mut self, heap: &mut BinaryHeap<Candidate>, a: usize, b: usize) {
        let u = self.find(a);
        let v = self.find(b);
        if u == v {
            return;
        }
        let (cost, target) = self.evaluate(u, v);
        heap.push(Candidate {
            cost,
            target,
            u,
            v,
            vu: self.version[u],
            vv: self.version[v],
        });
    }

    fn collapse(&mut self, u: usize, v: usize, target: Vec3, heap: &mut BinaryHeap<Candidate>) {
        // Merge v into u.
        self.positions[u] = target;
        let qv = self.quadrics[v];
        self.quadrics[u].add_assign(qv);
        self.parent[v] = u;
        self.version[u] += 1;
        self.version[v] += 1;

        // u inherits v's incident faces.
        let v_faces = std::mem::take(&mut self.vertex_faces[v]);
        self.vertex_faces[u].extend(v_faces);

        // Kill faces that became degenerate (two corners now the same vertex).
        let faces_of_u = self.vertex_faces[u].clone();
        for fi in faces_of_u {
            if !self.alive[fi] {
                continue;
            }
            let f = self.faces[fi];
            let (a, b, c) = (self.find(f[0]), self.find(f[1]), self.find(f[2]));
            if a == b || b == c || a == c {
                self.alive[fi] = false;
                self.alive_count -= 1;
            }
        }

        // Refresh the edges around u.
        let mut neighbors: HashSet<usize> = HashSet::new();
        for fi in self.vertex_faces[u].clone() {
            if !self.alive[fi] {
                continue;
            }
            let face = self.faces[fi];
            for &w in &face {
                let rw = self.find(w);
                if rw != u {
                    neighbors.insert(rw);
                }
            }
        }
        for w in neighbors {
            self.push_edge(heap, u, w);
        }
    }

    fn run(&mut self, target_faces: usize) {
        let mut heap = BinaryHeap::new();
        let mut seen: HashSet<(usize, usize)> = HashSet::new();
        for f in &self.faces.clone() {
            for &(a, b) in &[(f[0], f[1]), (f[1], f[2]), (f[2], f[0])] {
                let key = (a.min(b), a.max(b));
                if seen.insert(key) {
                    self.push_edge(&mut heap, a, b);
                }
            }
        }

        while self.alive_count > target_faces {
            let cand = match heap.pop() {
                Some(c) => c,
                None => break,
            };
            // Stale if either endpoint is no longer a root or was modified.
            if self.find(cand.u) != cand.u
                || self.find(cand.v) != cand.v
                || self.version[cand.u] != cand.vu
                || self.version[cand.v] != cand.vv
            {
                continue;
            }
            self.collapse(cand.u, cand.v, cand.target, &mut heap);
        }
    }

    fn to_mesh(&mut self, name: &str) -> Mesh {
        let mut remap: Vec<Option<u32>> = vec![None; self.positions.len()];
        let mut out_positions: Vec<Vec3> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        let face_count = self.faces.len();
        for fi in 0..face_count {
            if !self.alive[fi] {
                continue;
            }
            let f = self.faces[fi];
            let r = [self.find(f[0]), self.find(f[1]), self.find(f[2])];
            if r[0] == r[1] || r[1] == r[2] || r[0] == r[2] {
                continue;
            }
            for &root in &r {
                let idx = match remap[root] {
                    Some(i) => i,
                    None => {
                        let i = out_positions.len() as u32;
                        out_positions.push(self.positions[root]);
                        remap[root] = Some(i);
                        i
                    }
                };
                indices.push(idx);
            }
        }

        // Smooth normals.
        let mut normals = vec![Vec3::ZERO; out_positions.len()];
        for t in indices.chunks_exact(3) {
            let (i, j, k) = (t[0] as usize, t[1] as usize, t[2] as usize);
            let n = (out_positions[j] - out_positions[i]).cross(out_positions[k] - out_positions[i]);
            normals[i] += n;
            normals[j] += n;
            normals[k] += n;
        }
        let verts: Vec<Vertex> = out_positions
            .iter()
            .enumerate()
            .map(|(i, &p)| Vertex::new(p, normals[i].normalize_or_zero(), glam::Vec2::ZERO))
            .collect();
        Mesh::with_vertices(name, verts, indices)
    }
}

/// Simplify a mesh down to about `target_triangles` using QEM edge collapses.
pub fn decimate_to(mesh: &Mesh, target_triangles: usize) -> Mesh {
    let mut d = Decimator::new(mesh);
    d.run(target_triangles.max(1));
    d.to_mesh("decimated")
}

/// Simplify a mesh to a fraction of its triangle count (`ratio` in `(0, 1]`).
pub fn decimate_ratio(mesh: &Mesh, ratio: f32) -> Mesh {
    let target = ((mesh.triangle_count() as f32) * ratio.clamp(0.0, 1.0)).round() as usize;
    decimate_to(mesh, target.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimation_reduces_triangle_count() {
        let sphere = Mesh::uv_sphere(1.0, 24, 16);
        let before = sphere.triangle_count();
        let simplified = decimate_to(&sphere, before / 2);
        assert!(
            simplified.triangle_count() <= before / 2 + 8,
            "expected ~{}, got {}",
            before / 2,
            simplified.triangle_count()
        );
        assert!(simplified.triangle_count() > 0);
    }

    #[test]
    fn decimated_sphere_stays_near_unit_radius() {
        let sphere = Mesh::uv_sphere(1.0, 24, 16);
        let simplified = decimate_ratio(&sphere, 0.3);
        for v in &simplified.vertices {
            let r = v.position_vec3().length();
            assert!((r - 1.0).abs() < 0.25, "vertex drifted to radius {r}");
        }
    }

    #[test]
    fn ratio_one_keeps_geometry() {
        let cube = Mesh::cube(1.0);
        let same = decimate_ratio(&cube, 1.0);
        assert_eq!(same.triangle_count(), cube.triangle_count());
    }
}
