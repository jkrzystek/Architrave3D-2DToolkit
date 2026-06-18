//! Edge-loop insertion ("loop cut") across a strip of quads.
//!
//! Starting from one edge, the cut walks across each quad to the *opposite*
//! edge, hops to the neighbouring quad, and continues — closing into a ring or
//! stopping at a boundary. Every crossed edge gets a midpoint vertex and every
//! crossed quad is split into two, inserting a continuous new edge loop. Only
//! quad faces participate; the walk stops at triangles or n-gons.

use std::collections::{HashMap, HashSet};

use crate::edit_mesh::EditMesh;

#[inline]
fn key(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}

/// In a quad `face`, the edge directly across from the one matching `entry`
/// (undirected). `None` if the face is not a quad or has no such edge.
fn opposite_edge(face: &[usize], entry: (usize, usize)) -> Option<(usize, usize)> {
    if face.len() != 4 {
        return None;
    }
    let ek = key(entry.0, entry.1);
    for i in 0..4 {
        if key(face[i], face[(i + 1) % 4]) == ek {
            let j = (i + 2) % 4;
            return Some((face[j], face[(j + 1) % 4]));
        }
    }
    None
}

impl EditMesh {
    /// Insert an edge loop starting at the undirected edge `(a, b)`. Returns the
    /// number of quads that were split (0 if the edge is unknown or borders no
    /// quad).
    pub fn loop_cut(&mut self, a: usize, b: usize) -> usize {
        // Map every undirected edge to the faces using it.
        let mut edge_faces: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
        for (fi, f) in self.faces.iter().enumerate() {
            let k = f.len();
            for i in 0..k {
                edge_faces.entry(key(f[i], f[(i + 1) % k])).or_default().push(fi);
            }
        }

        let start = key(a, b);
        let start_faces = match edge_faces.get(&start) {
            Some(v) if !v.is_empty() => v.clone(),
            _ => return 0,
        };

        let mut visited: HashSet<usize> = HashSet::new();
        let mut strip: Vec<usize> = Vec::new();
        let mut cut: HashSet<(usize, usize)> = HashSet::new();

        // Walk from `start`, entering whichever adjacent face is not `blocked`.
        let mut walk = |edge_faces: &HashMap<(usize, usize), Vec<usize>>,
                        faces: &[Vec<usize>],
                        blocked: Option<usize>,
                        visited: &mut HashSet<usize>,
                        strip: &mut Vec<usize>,
                        cut: &mut HashSet<(usize, usize)>| {
            let mut cur = start;
            let mut from = blocked;
            loop {
                let adj = match edge_faces.get(&cur) {
                    Some(v) => v,
                    None => break,
                };
                let next_face = adj.iter().copied().find(|f| Some(*f) != from);
                let nf = match next_face {
                    Some(f) => f,
                    None => break,
                };
                if visited.contains(&nf) {
                    break;
                }
                let opp = match opposite_edge(&faces[nf], cur) {
                    Some(o) => o,
                    None => break,
                };
                visited.insert(nf);
                strip.push(nf);
                cut.insert(cur);
                cut.insert(key(opp.0, opp.1));
                from = Some(nf);
                cur = key(opp.0, opp.1);
                if cur == start {
                    break;
                }
            }
        };

        // Two passes so an interior start edge cuts in both directions; on a
        // closed ring the second pass is a no-op (faces already visited). The
        // first pass is blocked from the *second* adjacent face so it enters the
        // first; the second pass is blocked the other way.
        let block_first = start_faces.get(1).copied();
        walk(&edge_faces, &self.faces, block_first, &mut visited, &mut strip, &mut cut);
        if start_faces.len() > 1 {
            let block_second = start_faces.get(0).copied();
            walk(&edge_faces, &self.faces, block_second, &mut visited, &mut strip, &mut cut);
        }

        if strip.is_empty() {
            return 0;
        }

        // Midpoint vertex per cut edge, created once.
        let mut mids: HashMap<(usize, usize), usize> = HashMap::new();
        let mut midpoint = |this: &mut EditMesh, e: (usize, usize)| -> usize {
            *mids.entry(e).or_insert_with(|| {
                let p = (this.positions[e.0] + this.positions[e.1]) * 0.5;
                this.positions.push(p);
                this.positions.len() - 1
            })
        };

        let count = strip.len();
        for &fi in &strip {
            let f = self.faces[fi].clone();
            let (v0, v1, v2, v3) = (f[0], f[1], f[2], f[3]);
            let e01 = key(v0, v1);
            let e12 = key(v1, v2);
            let e23 = key(v2, v3);
            let e30 = key(v3, v0);
            if cut.contains(&e01) && cut.contains(&e23) {
                let m01 = midpoint(self, e01);
                let m23 = midpoint(self, e23);
                self.faces[fi] = vec![v0, m01, m23, v3];
                self.faces.push(vec![m01, v1, v2, m23]);
            } else if cut.contains(&e12) && cut.contains(&e30) {
                let m12 = midpoint(self, e12);
                let m30 = midpoint(self, e30);
                self.faces[fi] = vec![v1, m12, m30, v0];
                self.faces.push(vec![m12, v2, v3, m30]);
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit_mesh::quad_cube;

    #[test]
    fn loop_cut_around_cube_splits_four_faces() {
        let mut cube = quad_cube();
        // A vertical edge of the cube (between bottom vert 1 and top vert 5).
        let n = cube.loop_cut(1, 5);
        assert_eq!(n, 4, "should cross the four side faces");
        // 4 new midpoints.
        assert_eq!(cube.vertex_count(), 12);
        // Still a closed genus-0 surface.
        let he = cube.to_halfedge();
        assert_eq!(he.euler_characteristic(), 2);
    }

    #[test]
    fn loop_cut_on_open_quad_strip() {
        // Two quads in a row sharing a vertical edge -> cut splits both.
        let positions = vec![
            glam::Vec3::new(0.0, 0.0, 0.0),
            glam::Vec3::new(1.0, 0.0, 0.0),
            glam::Vec3::new(1.0, 1.0, 0.0),
            glam::Vec3::new(0.0, 1.0, 0.0),
            glam::Vec3::new(2.0, 0.0, 0.0),
            glam::Vec3::new(2.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3], vec![1, 4, 5, 2]];
        let mut m = EditMesh::from_polygons(positions, faces);
        // Cut starting at the left boundary edge (0-3) crosses horizontally.
        let n = m.loop_cut(0, 3);
        assert_eq!(n, 2);
        assert_eq!(m.face_count(), 4);
    }

    #[test]
    fn unknown_edge_is_noop() {
        let mut cube = quad_cube();
        assert_eq!(cube.loop_cut(100, 200), 0);
    }
}
