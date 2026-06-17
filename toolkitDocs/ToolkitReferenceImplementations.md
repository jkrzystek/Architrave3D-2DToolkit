# Toolkit Reference Implementations & Mathematical Resources

This document catalogs academic papers, specifications, and open-source implementations that serve as references for implementing 2D/3D graphics subsystems using the Velatura Toolkit.

---

## 1. Reference Usage & Licensing Guidelines

Developers using these references must adhere to intellectual property rules:

- **Permissive Libraries (MIT, Apache 2.0, BSD):** Code may be used directly or ported. Include the appropriate copyright notice.
- **Copyleft / Academic-Only Code (GPL, AGPL, CC BY-NC):** Do NOT copy or directly port. Instead, study the mathematical approach, extract the underlying equations, and implement them independently in your own WGSL/Rust pipelines.
- **Academic Papers:** Scientific algorithms and mathematical formulas are not copyrightable. You are free to implement them in your software.

---

## 2. Core Rendering & Graphics Math

### A. Physically Based Rendering (PBR) Shaders
To construct PBR rendering pipelines evaluating standard BRDF channels:
- **Khronos glTF-Sample-Renderer (Apache-2.0):** The definitive standard for glTF PBR shading. Port the GLSL math evaluating GGX Normal Distribution, Smith masking, and Fresnel equations to WGSL.
  - [glTF BRDF Fragment Shader](https://github.com/KhronosGroup/glTF-Sample-Renderer/blob/main/source/Renderer/shaders/pbr.frag)
  - [glTF 2.0 Specification BRDF Appendix](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
- **Rust/wgpu Reference Implementations:**
  - [rust-pbr (MIT):](https://github.com/BrassLion/rust-pbr) A PBR renderer in Rust and wgpu featuring HDR, environment lighting, and texture loading.
  - [wgpu-pbr (MIT):](https://github.com/tedsta/wgpu-pbr) A simple wgpu PBR shader implementation.
  - [renderling (MIT/Apache 2.0):](https://github.com/schell/renderling) Modern GPU-driven wgpu renderer.

### B. Image-Based Lighting (IBL) Prefiltering
For converting equirectangular HDRI images into diffuse irradiance and specular reflection maps:
- **Compute Shader Prefiltering:**
  - [Alin Loghin — Compute Shaders for IBL:](http://alinloghin.com/articles/compute_ibl.html) Direct guide to converting HDRIs via compute passes.
  - [LearnOpenGL — Specular IBL (CC BY-NC 4.0):](https://learnopengl.com/PBR/IBL/Specular-IBL) Classic tutorial for cubemap convolution and split-sum approximations.
  - [Bruno Opsenica — IBL with Multiple Scattering:](https://bruop.github.io/ibl/) Advanced guide for energy-conserving multiple-scattering models.

---

## 3. Viewport Decals & Coordinate Projection

### A. Screen-Space Decal Projection
For painting or projecting 2D stamps onto 3D surfaces:
- **Inverse Depth Projection:** Reconstructing pixel world-space coordinates from the depth buffer using the inverse view-projection matrix.
  - [Wicked Engine Decals (MIT):](https://github.com/turanszkij/WickedEngine) C++ engine with HLSL shaders demonstrating inverse depth-reconstruction for decals.
  - [Unreal Engine Deferred Decals:](https://github.com/EpicGames/UnrealEngine) Look for `DeferredDecal.usf` in their source repository to study projection box clipping math.

### B. Lazy Mouse (Stroke Smoothing)
- **Spring-Damper Model:** Smooth out hand jitter by having the paint coordinate lag behind the cursor.
  - **Math:** A virtual point $P_v$ is connected to the cursor $P_c$ by a spring. The velocity of $P_v$ is proportional to the distance: $V = k \cdot (P_c - P_v)$.

---

## 4. Layer Stack Blending & Compositing

- **SVG Compositing Specification (W3C Standard):** Defines the mathematical formulas for all industry-standard blend modes (Multiply, Overlay, Soft Light, Screen, Dodge, Burn, etc.).
  - [W3C Compositing & Blending Spec](https://www.w3.org/TR/compositing/)

---

## 5. GPU-Compute Operations

### A. Jump Flooding Algorithm (JFA) for Dilation
For expanding texture borders (padding UV islands) on the GPU to prevent mipmap sampling bleeding:
- **Foundational Paper:**
  - Rong & Tan, 2006: *"Jump Flooding in GPU with Applications to Voronoi Diagram and Distance Transform"*.
  - [JFA Thesis PDF](https://www.comp.nus.edu.sg/~tants/jfa/rong-guodong-phd-thesis.pdf)
- **Implementations:**
  - [Unity JFA (MIT):](https://github.com/alpacasking/JumpFloodingAlgorithm) Compute shader expanding islands.
  - [Ben Golus — Wide Outlines via JFA:](https://bgolus.medium.com/the-quest-for-very-wide-outlines-ba82ed442cd9) Practical breakdown of JFA coordinate offset loops.

### B. Height-to-Normal Conversion
- **Sobel Filter derivatives:** Generating normal vectors from heightmaps.
  - WGSL Compute Shader samples adjacent pixels: $dh/dx = (h_{right} - h_{left}) / 2.0$ and $dh/dy = (h_{bottom} - h_{top}) / 2.0$. The normal vector is normalized: $N = \text{normalize}(-dh/dx, -dh/dy, 1.0)$.

---

## 6. Color Science & Pigment Mixing

### A. Kubelka-Munk Subtractive Color Mixing
For physical paint color blending (e.g., yellow and blue making green instead of muddy gray):
- **Upsampling RGB to Reflectance:**
  - [Scott Burns — Subtractive Color Mixture Computation:](http://scottburns.us/subtractive-color-mixture/) Algorithmic breakdown of converting sRGB to physical absorption ($K$) and scattering ($S$) coefficients.
  - [Spectral.js (MIT):](https://github.com/rvanwijnen/spectral.js) JavaScript library executing 7-band spectral color mixing based on Kubelka-Munk theory.
- **Physical Paint Accumulation:**
  - Baxter et al., NPAR '04: *"IMPaSTo: A Realistic, Interactive Model for Paint"*.
  - [IMPaSTo Paper PDF](http://gamma.cs.unc.edu/IMPASTO/publications/Baxter-IMPaSTo_Web-NPAR04.pdf)

---

## 7. Fluid Dynamics & Physics Simulation

For simulation-heavy modules (e.g., erosion or paint smudging):
- **Stable Fluids:**
  - Jos Stam, 1999: *"Stable Fluids"*. The foundational grid-advection paper using semi-Lagrangian methods.
  - [GPU Gems Ch 38 — Fast Fluid Dynamics on GPU:](https://developer.nvidia.com/gpugems/gpugems/part-vi-beyond-triangles/chapter-38-fast-fluid-dynamics-simulation-gpu) GLSL-based implementation guide detailing advection, Jacobi diffusion, and pressure projection steps.
