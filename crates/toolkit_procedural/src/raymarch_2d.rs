//! 2.5D shadow raymarching over a heightmap.
//!
//! This module implements a screen-space raymarching algorithm that generates
//! soft shadows for a 2D heightmap (a.k.a. depth buffer). For every pixel the
//! algorithm casts a ray along the projected light direction through the
//! heightmap, comparing sampled depths against the expected ray height. Where
//! the terrain occludes the ray the pixel is darkened, producing a shadow mask
//! suitable for compositing onto a final image.
//!
//! The result is a `Vec<u8>` shadow mask where **255 = fully lit** and
//! **0 = fully shadowed**.

use glam::Vec2;

/// Calculates 2.5D soft shadows over a heightmap grid using raymarching.
///
/// # Arguments
///
/// * `width`, `height` — dimensions of the heightmap and output mask.
/// * `depth_buffer` — flat slice of `width * height` depth values.
/// * `light_dir` — normalised 3D direction **towards** the light source.
///   The XY components determine the 2D march direction; Z determines the
///   slope of the shadow ray through the depth.
/// * `shadow_softness` — controls the penumbra width. Higher values produce
///   softer shadow edges (see formula below).
/// * `max_steps` — maximum number of samples along each ray.
/// * `depth_scale` — scales how fast the ray climbs relative to pixel
///   distance (e.g. `0.05`).
/// * `y_axis_up` — when `true` the Y axis points upward (world space), so
///   the march subtracts `ray_dir.y`. When `false` the Y axis points
///   downward (image space, the typical convention for pixel buffers).
///
/// # Penumbra formula
///
/// At each marching step the algorithm computes an *attenuation* value:
///
/// ```text
/// attenuation = clamp((sample_depth - ray_height) / (shadow_softness * step), 0, 1)
/// ```
///
/// The running shadow value is the minimum of `1 − attenuation` over all
/// steps, producing soft falloff near shadow edges.
///
/// # Returns
///
/// A `Vec<u8>` of size `width * height`. Each value lies in `[0, 255]`
/// where **255 = fully lit** and **0 = fully shadowed**.
pub fn shadow_raymarch_2d(
    width: u32,
    height: u32,
    depth_buffer: &[f32],
    light_dir: glam::Vec3,
    shadow_softness: f32,
    max_steps: u32,
    depth_scale: f32,
    y_axis_up: bool,
) -> Vec<u8> {
    let mut shadow_mask = vec![255; (width * height) as usize];
    
    let light_dir_2d = Vec2::new(light_dir.x, light_dir.y);
    let light_len = light_dir_2d.length();
    let ray_dir = if light_len > 0.001 { light_dir_2d / light_len } else { Vec2::X };
    let depth_slope = light_dir.z / light_len.max(0.001);
    
    for y in 0..height {
        for x in 0..width {
            let start_idx = (y * width + x) as usize;
            let p0_depth = depth_buffer[start_idx];
            
            let mut shadow_val = 1.0f32;
            
            for step in 1..=max_steps {
                let step_f = step as f32;
                let sample_x = x as f32 - ray_dir.x * step_f;
                let sample_y = if y_axis_up {
                    y as f32 - ray_dir.y * step_f
                } else {
                    y as f32 + ray_dir.y * step_f
                };
                
                if sample_x < 0.0 || sample_x >= width as f32 || sample_y < 0.0 || sample_y >= height as f32 {
                    break;
                }
                
                let sx = sample_x as usize;
                let sy = sample_y as usize;
                let s_idx = sy * width as usize + sx;
                
                let sample_depth = depth_buffer[s_idx];
                let ray_height = p0_depth + step_f * depth_slope * depth_scale;
                
                if sample_depth > ray_height {
                    let attenuation = ((sample_depth - ray_height) / (shadow_softness * step_f)).clamp(0.0, 1.0);
                    shadow_val = shadow_val.min(1.0 - attenuation);
                }
            }
            
            shadow_mask[start_idx] = (shadow_val * 255.0).clamp(0.0, 255.0) as u8;
        }
    }
    
    shadow_mask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_heightmap_no_shadows() {
        // A perfectly flat heightmap should produce no shadows (all 255).
        let w = 8;
        let h = 8;
        let depth = vec![0.5f32; (w * h) as usize];
        let mask = shadow_raymarch_2d(
            w, h, &depth,
            glam::Vec3::new(1.0, 0.0, 0.5).normalize(),
            1.0, 16, 0.05, false,
        );
        for &v in &mask {
            assert_eq!(v, 255, "Flat heightmap should be fully lit everywhere");
        }
    }

    #[test]
    fn tall_pixel_casts_shadow() {
        // Place one tall pixel and verify that at least one neighbour is
        // shadowed (value < 255).
        let w: u32 = 8;
        let h: u32 = 8;
        let mut depth = vec![0.0f32; (w * h) as usize];
        // Make pixel (4, 4) very tall.
        depth[(4 * w + 4) as usize] = 10.0;

        let mask = shadow_raymarch_2d(
            w, h, &depth,
            glam::Vec3::new(1.0, 0.0, 0.3).normalize(),
            1.0, 16, 0.05, false,
        );

        let has_shadow = mask.iter().any(|&v| v < 255);
        assert!(has_shadow, "A tall pixel should cast a shadow on at least one neighbour");
    }

    #[test]
    fn y_axis_up_vs_down() {
        // With a light coming from positive Y, y_axis_up should march in the
        // opposite Y direction compared to y_axis_down, producing different
        // shadow masks on an asymmetric heightmap.
        let w: u32 = 8;
        let h: u32 = 8;
        let mut depth = vec![0.0f32; (w * h) as usize];
        // Tall column at row 2 only — asymmetric in Y.
        for x in 0..w {
            depth[(2 * w + x) as usize] = 5.0;
        }

        let light = glam::Vec3::new(0.0, 1.0, 0.3).normalize();

        let mask_up = shadow_raymarch_2d(w, h, &depth, light, 1.0, 16, 0.05, true);
        let mask_down = shadow_raymarch_2d(w, h, &depth, light, 1.0, 16, 0.05, false);

        assert_ne!(mask_up, mask_down, "y_axis_up and y_axis_down should produce different masks on an asymmetric heightmap");
    }
}
