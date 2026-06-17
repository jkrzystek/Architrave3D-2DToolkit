//! NURBS surfaces (tensor-product), with tessellation to a renderable mesh.

use glam::{Vec3, Vec4};
use serde::{Deserialize, Serialize};
use toolkit_geometry::{Mesh, Vertex};

use crate::knot::{clamped_uniform_knots, de_boor4, domain};

/// A NURBS surface defined by a grid of control points (`rows_u × cols_v`) with
/// per-point weights and a knot vector / degree in each parametric direction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NurbsSurface {
    /// Row-major control grid: index `i*cols_v + j` is row `i`, column `j`.
    pub control: Vec<Vec3>,
    pub weights: Vec<f32>,
    pub rows_u: usize,
    pub cols_v: usize,
    pub knots_u: Vec<f32>,
    pub knots_v: Vec<f32>,
    pub degree_u: usize,
    pub degree_v: usize,
}

impl NurbsSurface {
    /// Build a clamped, uniform surface from a control grid (`grid[u][v]`).
    /// Weights default to 1. Degrees are reduced if the grid is too small.
    pub fn new(grid: Vec<Vec<Vec3>>, degree_u: usize, degree_v: usize) -> Self {
        let rows_u = grid.len();
        let cols_v = grid.first().map(|r| r.len()).unwrap_or(0);
        let control: Vec<Vec3> = grid.into_iter().flatten().collect();
        let weights = vec![1.0; control.len()];
        let degree_u = degree_u.min(rows_u.saturating_sub(1)).max(1);
        let degree_v = degree_v.min(cols_v.saturating_sub(1)).max(1);
        Self {
            knots_u: clamped_uniform_knots(rows_u, degree_u),
            knots_v: clamped_uniform_knots(cols_v, degree_v),
            control,
            weights,
            rows_u,
            cols_v,
            degree_u,
            degree_v,
        }
    }

    pub fn domain_u(&self) -> (f32, f32) {
        domain(&self.knots_u, self.degree_u)
    }
    pub fn domain_v(&self) -> (f32, f32) {
        domain(&self.knots_v, self.degree_v)
    }

    fn h(&self, i: usize, j: usize) -> Vec4 {
        let idx = i * self.cols_v + j;
        let w = self.weights[idx];
        (self.control[idx] * w).extend(w)
    }

    /// Evaluate the surface point at `(u, v)` (clamped to the domain).
    pub fn evaluate(&self, u: f32, v: f32) -> Vec3 {
        let (ulo, uhi) = self.domain_u();
        let (vlo, vhi) = self.domain_v();
        let u = u.clamp(ulo, uhi);
        let v = v.clamp(vlo, vhi);

        // Evaluate along v for each u-row, then along u over the results.
        let mut tmp: Vec<Vec4> = Vec::with_capacity(self.rows_u);
        for i in 0..self.rows_u {
            let row: Vec<Vec4> = (0..self.cols_v).map(|j| self.h(i, j)).collect();
            tmp.push(de_boor4(self.cols_v - 1, self.degree_v, v, &self.knots_v, &row));
        }
        let r = de_boor4(self.rows_u - 1, self.degree_u, u, &self.knots_u, &tmp);
        if r.w.abs() < 1e-9 {
            r.truncate()
        } else {
            r.truncate() / r.w
        }
    }

    /// Approximate surface normal at `(u, v)` via finite differences.
    pub fn normal(&self, u: f32, v: f32) -> Vec3 {
        let (ulo, uhi) = self.domain_u();
        let (vlo, vhi) = self.domain_v();
        let eps_u = (uhi - ulo) * 1e-3;
        let eps_v = (vhi - vlo) * 1e-3;
        let du = self.evaluate((u + eps_u).min(uhi), v) - self.evaluate((u - eps_u).max(ulo), v);
        let dv = self.evaluate(u, (v + eps_v).min(vhi)) - self.evaluate(u, (v - eps_v).max(vlo));
        du.cross(dv).normalize_or_zero()
    }

    /// Tessellate into a triangle [`Mesh`] of `nu × nv` quads. UVs span `[0,1]`.
    pub fn tessellate(&self, nu: usize, nv: usize) -> Mesh {
        let nu = nu.max(1);
        let nv = nv.max(1);
        let (ulo, uhi) = self.domain_u();
        let (vlo, vhi) = self.domain_v();

        let mut vertices = Vec::with_capacity((nu + 1) * (nv + 1));
        for iu in 0..=nu {
            let su = iu as f32 / nu as f32;
            let u = ulo + (uhi - ulo) * su;
            for iv in 0..=nv {
                let sv = iv as f32 / nv as f32;
                let v = vlo + (vhi - vlo) * sv;
                let pos = self.evaluate(u, v);
                let nrm = self.normal(u, v);
                vertices.push(Vertex::new(pos, nrm, glam::Vec2::new(su, sv)));
            }
        }

        let stride = nv + 1;
        let mut indices = Vec::with_capacity(nu * nv * 6);
        for iu in 0..nu {
            for iv in 0..nv {
                let a = (iu * stride + iv) as u32;
                let b = a + 1;
                let c = a + stride as u32;
                let d = c + 1;
                indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
        Mesh::with_vertices("nurbs_surface", vertices, indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_grid() -> NurbsSurface {
        // A flat 3x3 plane on XZ at y = 0.
        let grid: Vec<Vec<Vec3>> = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| Vec3::new(i as f32, 0.0, j as f32))
                    .collect()
            })
            .collect();
        NurbsSurface::new(grid, 2, 2)
    }

    #[test]
    fn corners_match_control_points() {
        let s = flat_grid();
        assert!((s.evaluate(0.0, 0.0) - Vec3::new(0.0, 0.0, 0.0)).length() < 1e-5);
        assert!((s.evaluate(1.0, 1.0) - Vec3::new(2.0, 0.0, 2.0)).length() < 1e-5);
    }

    #[test]
    fn flat_surface_normal_is_up() {
        let s = flat_grid();
        let n = s.normal(0.5, 0.5);
        assert!(n.dot(Vec3::Y).abs() > 0.99, "normal = {n:?}");
    }

    #[test]
    fn tessellate_mesh_counts() {
        let s = flat_grid();
        let mesh = s.tessellate(4, 6);
        assert_eq!(mesh.vertex_count(), 5 * 7);
        assert_eq!(mesh.triangle_count(), 4 * 6 * 2);
    }

    #[test]
    fn curved_surface_bulges() {
        // Lift the center control point; the surface should rise there.
        let mut grid: Vec<Vec<Vec3>> = (0..3)
            .map(|i| (0..3).map(|j| Vec3::new(i as f32, 0.0, j as f32)).collect())
            .collect();
        grid[1][1].y = 3.0;
        let s = NurbsSurface::new(grid, 2, 2);
        assert!(s.evaluate(0.5, 0.5).y > 0.5);
    }
}
