use glam::Vec2;

/// Calculates 2.5D soft shadows over a heightmap grid using a raymarching algorithm.
/// 
/// `depth_buffer` is a 1D slice of size `width * height`.
/// `light_dir` is the normalized 3D vector pointing to the light source.
/// `shadow_softness` controls how soft the penumbra is.
/// `max_steps` controls the maximum raymarching steps.
/// `depth_scale` controls the relative scale of the depth compared to pixel distance (e.g., 0.05).
/// 
/// Returns a `Vec<u8>` of the same size, representing the shadow mask (0-255).
pub fn shadow_raymarch_2d(
    width: u32,
    height: u32,
    depth_buffer: &[f32],
    light_dir: glam::Vec3,
    shadow_softness: f32,
    max_steps: u32,
    depth_scale: f32,
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
                let sample_y = y as f32 + ray_dir.y * step_f; // assuming Y is inverted in image space
                
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
