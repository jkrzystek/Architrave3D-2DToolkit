//! Atlas packing: arrange a set of charts (by their UV bounding-box sizes) into
//! a single `[0,1]²` texture space without overlap, using a shelf packer.

use glam::Vec2;

use crate::chart::Chart;

/// Where a chart was placed in the atlas: `atlas_uv = local_uv * scale + offset`.
#[derive(Clone, Copy, Debug)]
pub struct AtlasPlacement {
    pub offset: Vec2,
    pub scale: f32,
}

impl AtlasPlacement {
    pub fn apply(&self, uv: Vec2) -> Vec2 {
        uv * self.scale + self.offset
    }
}

/// Pack charts whose UV-space sizes are `sizes` into the unit square.
///
/// `margin` is the gap between charts as a fraction of the final atlas (e.g.
/// `0.01`). Returns one placement per input size, all mapping into `[0,1]²`.
pub fn pack_sizes(sizes: &[Vec2], margin: f32) -> Vec<AtlasPlacement> {
    let n = sizes.len();
    if n == 0 {
        return Vec::new();
    }

    // Order by height (tallest first) for a tighter shelf pack; remember the
    // original index so results map back correctly.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| sizes[b].y.total_cmp(&sizes[a].y));

    // Target shelf width: roughly the square root of the total area, but never
    // narrower than the widest chart.
    let total_area: f32 = sizes.iter().map(|s| s.x.max(1e-6) * s.y.max(1e-6)).sum();
    let widest = sizes.iter().map(|s| s.x).fold(0.0_f32, f32::max);
    let shelf_width = total_area.sqrt().max(widest).max(1e-6);
    let gap = margin * shelf_width;

    // Lay out in unscaled space first.
    let mut placements = vec![AtlasPlacement {
        offset: Vec2::ZERO,
        scale: 1.0,
    }; n];

    let mut cursor = Vec2::ZERO;
    let mut shelf_height = 0.0_f32;
    let mut used_width = 0.0_f32;

    for &i in &order {
        let s = sizes[i];
        if cursor.x > 0.0 && cursor.x + s.x > shelf_width {
            // Wrap to a new shelf.
            cursor.x = 0.0;
            cursor.y += shelf_height + gap;
            shelf_height = 0.0;
        }
        placements[i].offset = cursor;
        cursor.x += s.x + gap;
        shelf_height = shelf_height.max(s.y);
        used_width = used_width.max(cursor.x);
    }
    let used_height = cursor.y + shelf_height;

    // Scale the whole arrangement to fit the unit square (preserve aspect).
    let extent = used_width.max(used_height).max(1e-6);
    let scale = 1.0 / extent;
    for p in &mut placements {
        p.offset *= scale;
        p.scale = scale;
    }
    placements
}

/// Pack charts in place: each chart's `uvs` are rewritten into atlas space.
/// Charts must already be unwrapped (`uvs` populated). Returns the placements.
pub fn pack_charts(charts: &mut [Chart], margin: f32) -> Vec<AtlasPlacement> {
    // Normalise each chart to origin so size == extent, then collect sizes.
    let mut sizes = Vec::with_capacity(charts.len());
    for chart in charts.iter_mut() {
        let mut min = Vec2::splat(f32::INFINITY);
        for uv in &chart.uvs {
            min = min.min(*uv);
        }
        if chart.uvs.is_empty() {
            min = Vec2::ZERO;
        }
        for uv in &mut chart.uvs {
            *uv -= min;
        }
        sizes.push(chart.uv_extent());
    }

    let placements = pack_sizes(&sizes, margin);
    for (chart, placement) in charts.iter_mut().zip(&placements) {
        for uv in &mut chart.uvs {
            *uv = placement.apply(*uv);
        }
    }
    placements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_chart_fills_unit_square() {
        let placements = pack_sizes(&[Vec2::new(1.0, 1.0)], 0.0);
        assert_eq!(placements.len(), 1);
        let p = placements[0];
        assert!((p.apply(Vec2::ZERO) - Vec2::ZERO).length() < 1e-5);
        assert!((p.apply(Vec2::ONE) - Vec2::ONE).length() < 1e-4);
    }

    #[test]
    fn all_placements_within_unit_square() {
        let sizes = vec![
            Vec2::new(1.0, 0.5),
            Vec2::new(0.5, 0.5),
            Vec2::new(0.3, 0.8),
            Vec2::new(0.6, 0.4),
        ];
        let placements = pack_sizes(&sizes, 0.02);
        for (s, p) in sizes.iter().zip(&placements) {
            let corner = p.apply(*s);
            assert!(corner.x <= 1.0 + 1e-4, "x overflow: {}", corner.x);
            assert!(corner.y <= 1.0 + 1e-4, "y overflow: {}", corner.y);
            assert!(p.offset.x >= -1e-4 && p.offset.y >= -1e-4);
        }
    }

    #[test]
    fn charts_do_not_overlap() {
        let sizes = vec![
            Vec2::new(0.4, 0.4),
            Vec2::new(0.4, 0.4),
            Vec2::new(0.4, 0.4),
            Vec2::new(0.4, 0.4),
        ];
        let placements = pack_sizes(&sizes, 0.01);
        // Pairwise AABB overlap test in atlas space.
        let rects: Vec<(Vec2, Vec2)> = sizes
            .iter()
            .zip(&placements)
            .map(|(s, p)| (p.offset, p.offset + *s * p.scale))
            .collect();
        for i in 0..rects.len() {
            for j in i + 1..rects.len() {
                let (amin, amax) = rects[i];
                let (bmin, bmax) = rects[j];
                let overlap =
                    amin.x < bmax.x && amax.x > bmin.x && amin.y < bmax.y && amax.y > bmin.y;
                assert!(!overlap, "charts {i} and {j} overlap");
            }
        }
    }
}
