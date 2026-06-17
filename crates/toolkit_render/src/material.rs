//! Physically-based material model (metallic-roughness workflow, the glTF 2.0
//! standard) plus a GPU-ready uniform block and a reference Cook-Torrance WGSL
//! shader.

use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};
use serde::{Deserialize, Serialize};
use toolkit_core::TextureId;

/// A metallic-roughness PBR material. Texture maps are referenced by
/// [`TextureId`]; when absent, the scalar/factor fields are used directly.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PbrMaterial {
    pub name: String,
    /// Linear base color (albedo) and alpha.
    pub base_color: Vec4,
    /// 0 = dielectric, 1 = metal.
    pub metallic: f32,
    /// 0 = mirror-smooth, 1 = fully rough.
    pub roughness: f32,
    /// Linear emissive color.
    pub emissive: Vec3,
    /// Multiplier on `emissive` (HDR emission).
    pub emissive_strength: f32,
    /// Strength of the tangent-space normal map.
    pub normal_scale: f32,
    /// Strength of the ambient-occlusion map.
    pub occlusion_strength: f32,
    /// Alpha below this is discarded when `alpha_mask` is set.
    pub alpha_cutoff: f32,
    pub alpha_mask: bool,
    pub double_sided: bool,

    pub base_color_texture: Option<TextureId>,
    pub metallic_roughness_texture: Option<TextureId>,
    pub normal_texture: Option<TextureId>,
    pub emissive_texture: Option<TextureId>,
    pub occlusion_texture: Option<TextureId>,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            name: "material".into(),
            base_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            metallic: 0.0,
            roughness: 0.5,
            emissive: Vec3::ZERO,
            emissive_strength: 1.0,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            alpha_cutoff: 0.5,
            alpha_mask: false,
            double_sided: false,
            base_color_texture: None,
            metallic_roughness_texture: None,
            normal_texture: None,
            emissive_texture: None,
            occlusion_texture: None,
        }
    }
}

impl PbrMaterial {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// A plain dielectric of the given linear color.
    pub fn dielectric(color: Vec3, roughness: f32) -> Self {
        Self {
            base_color: color.extend(1.0),
            metallic: 0.0,
            roughness: roughness.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// A metal of the given linear color.
    pub fn metal(color: Vec3, roughness: f32) -> Self {
        Self {
            base_color: color.extend(1.0),
            metallic: 1.0,
            roughness: roughness.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Pack into the GPU uniform block.
    pub fn uniforms(&self) -> MaterialUniforms {
        let mut flags = 0u32;
        if self.double_sided {
            flags |= MaterialFlags::DOUBLE_SIDED;
        }
        if self.alpha_mask {
            flags |= MaterialFlags::ALPHA_MASK;
        }
        if self.base_color_texture.is_some() {
            flags |= MaterialFlags::HAS_BASE_COLOR_TEX;
        }
        if self.metallic_roughness_texture.is_some() {
            flags |= MaterialFlags::HAS_METALLIC_ROUGHNESS_TEX;
        }
        if self.normal_texture.is_some() {
            flags |= MaterialFlags::HAS_NORMAL_TEX;
        }
        if self.emissive_texture.is_some() {
            flags |= MaterialFlags::HAS_EMISSIVE_TEX;
        }
        if self.occlusion_texture.is_some() {
            flags |= MaterialFlags::HAS_OCCLUSION_TEX;
        }

        MaterialUniforms {
            base_color: self.base_color.to_array(),
            emissive: (self.emissive * self.emissive_strength)
                .extend(1.0)
                .to_array(),
            metallic: self.metallic,
            roughness: self.roughness,
            normal_scale: self.normal_scale,
            occlusion_strength: self.occlusion_strength,
            alpha_cutoff: self.alpha_cutoff,
            flags,
            _pad: [0.0; 2],
        }
    }
}

/// Bit flags packed into [`MaterialUniforms::flags`].
pub struct MaterialFlags;
impl MaterialFlags {
    pub const DOUBLE_SIDED: u32 = 1 << 0;
    pub const ALPHA_MASK: u32 = 1 << 1;
    pub const HAS_BASE_COLOR_TEX: u32 = 1 << 2;
    pub const HAS_METALLIC_ROUGHNESS_TEX: u32 = 1 << 3;
    pub const HAS_NORMAL_TEX: u32 = 1 << 4;
    pub const HAS_EMISSIVE_TEX: u32 = 1 << 5;
    pub const HAS_OCCLUSION_TEX: u32 = 1 << 6;
}

/// GPU uniform block for a PBR material. `#[repr(C)]`, 16-byte aligned, `Pod`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MaterialUniforms {
    pub base_color: [f32; 4],
    pub emissive: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
    pub alpha_cutoff: f32,
    pub flags: u32,
    pub _pad: [f32; 2],
}

/// A reference Cook-Torrance (GGX) PBR fragment/vertex shader in WGSL. Apps can
/// use it directly or as a starting point. Bind groups: 0 = view, 1 = model,
/// 2 = material+light.
pub const PBR_SHADER_WGSL: &str = r#"
struct View {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    viewport: vec2<f32>,
    near: f32,
    far: f32,
};
struct Model { model: mat4x4<f32>, normal_matrix: mat4x4<f32> };
struct Material {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    metallic: f32,
    roughness: f32,
    normal_scale: f32,
    occlusion: f32,
    alpha_cutoff: f32,
    flags: u32,
    pad: vec2<f32>,
};
struct Light {
    direction: vec4<f32>,
    color: vec4<f32>,
    ambient: vec4<f32>,
    intensity: f32,
    ambient_intensity: f32,
    pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u_view: View;
@group(1) @binding(0) var<uniform> u_model: Model;
@group(2) @binding(0) var<uniform> u_material: Material;
@group(2) @binding(1) var<uniform> u_light: Light;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> VsOut {
    var out: VsOut;
    let world = u_model.model * vec4<f32>(position, 1.0);
    out.world_pos = world.xyz;
    out.normal = normalize((u_model.normal_matrix * vec4<f32>(normal, 0.0)).xyz);
    out.uv = uv;
    out.clip = u_view.view_proj * world;
    return out;
}

const PI: f32 = 3.14159265359;

fn distribution_ggx(n: vec3<f32>, h: vec3<f32>, rough: f32) -> f32 {
    let a = rough * rough;
    let a2 = a * a;
    let ndoth = max(dot(n, h), 0.0);
    let d = (ndoth * ndoth) * (a2 - 1.0) + 1.0;
    return a2 / max(PI * d * d, 1e-5);
}

fn geometry_schlick(ndotv: f32, rough: f32) -> f32 {
    let k = (rough * rough) / 2.0;
    return ndotv / (ndotv * (1.0 - k) + k);
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - cos_theta, 5.0);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let albedo = u_material.base_color.rgb;
    let metallic = u_material.metallic;
    let rough = u_material.roughness;

    let n = normalize(in.normal);
    let v = normalize(u_view.camera_pos.xyz - in.world_pos);
    let l = normalize(-u_light.direction.xyz);
    let h = normalize(v + l);

    let f0 = mix(vec3<f32>(0.04), albedo, metallic);
    let ndf = distribution_ggx(n, h, rough);
    let ndotv = max(dot(n, v), 0.0);
    let ndotl = max(dot(n, l), 0.0);
    let g = geometry_schlick(ndotv, rough) * geometry_schlick(ndotl, rough);
    let f = fresnel_schlick(max(dot(h, v), 0.0), f0);

    let numerator = ndf * g * f;
    let denom = max(4.0 * ndotv * ndotl, 1e-4);
    let specular = numerator / denom;

    let ks = f;
    let kd = (vec3<f32>(1.0) - ks) * (1.0 - metallic);
    let radiance = u_light.color.rgb * u_light.intensity;
    let diffuse = kd * albedo / PI;
    let direct = (diffuse + specular) * radiance * ndotl;

    let ambient = u_light.ambient.rgb * u_light.ambient_intensity * albedo * u_material.occlusion;
    var color = direct + ambient + u_material.emissive.rgb;

    return vec4<f32>(color, u_material.base_color.a);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_block_is_16_byte_aligned() {
        assert_eq!(std::mem::size_of::<MaterialUniforms>() % 16, 0);
    }

    #[test]
    fn default_is_rough_dielectric() {
        let m = PbrMaterial::default();
        assert_eq!(m.metallic, 0.0);
        let u = m.uniforms();
        assert_eq!(u.flags & MaterialFlags::DOUBLE_SIDED, 0);
    }

    #[test]
    fn metal_constructor_sets_metallic() {
        let m = PbrMaterial::metal(Vec3::new(1.0, 0.8, 0.3), 0.2);
        assert_eq!(m.metallic, 1.0);
        assert!((m.roughness - 0.2).abs() < 1e-6);
    }

    #[test]
    fn texture_presence_sets_flags() {
        let mut m = PbrMaterial::default();
        m.base_color_texture = Some(TextureId::from_raw(7));
        m.normal_texture = Some(TextureId::from_raw(8));
        let u = m.uniforms();
        assert_ne!(u.flags & MaterialFlags::HAS_BASE_COLOR_TEX, 0);
        assert_ne!(u.flags & MaterialFlags::HAS_NORMAL_TEX, 0);
        assert_eq!(u.flags & MaterialFlags::HAS_EMISSIVE_TEX, 0);
    }

    #[test]
    fn emissive_is_premultiplied_by_strength() {
        let mut m = PbrMaterial::default();
        m.emissive = Vec3::new(1.0, 0.0, 0.0);
        m.emissive_strength = 3.0;
        let u = m.uniforms();
        assert!((u.emissive[0] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn material_serializes() {
        let m = PbrMaterial::metal(Vec3::ONE, 0.3);
        let json = serde_json::to_string(&m).unwrap();
        let back: PbrMaterial = serde_json::from_str(&json).unwrap();
        assert_eq!(back.metallic, m.metallic);
    }

    #[test]
    fn shader_has_entry_points() {
        assert!(PBR_SHADER_WGSL.contains("fn vs_main"));
        assert!(PBR_SHADER_WGSL.contains("fn fs_main"));
    }
}
