# Toolkit Testing Implementation Plan

Creative applications that interact with the GPU, handle complex geometry, and manage nested document histories require robust testing. This document defines the testing strategy recommended for applications built with the Velatura Toolkit.

---

## 1. Core Testing Philosophy

To maximize stability and maintainability, our testing strategy separates structural logic from UI styling:

- **No End-to-End (E2E) UI Tests:** UI layouts are highly prone to change. Avoid testing visual elements (e.g., button margins or color values) in favor of asserting the underlying commands and actions.
- **Mathematical State Determinism:** Use property-based testing to verify that nested undo/redo actions always return the document to a mathematically identical state.
- **Shader Regression Testing:** Use headless GPU rendering tests to assert that shader modifications yield byte-for-byte identical output images compared to reference baselines.
- **Fast CI Throughput:** Run complex tests inside headless software rasterizers to ensure testing pipelines execute in seconds without requiring dedicated hardware GPUs.

---

## 2. Rust Implementation Patterns

Adhere to the following architectural rules when writing tests:

1. **Pure Function Extraction:** Isolate geometry math (e.g., ray intersections or bounding calculations) into standard functions. Keep them free of `wgpu` or `egui` structs so they can be run in standard, synchronous unit tests.
2. **Synchronous Hardware Abstraction:** Avoid spawning asynchronous runtimes for graphics tests. Use `pollster::block_on()` to block execution while waiting for GPU buffers to map, preventing threading overhead.
3. **Mocking UI Despatches:** Decouple your UI widgets from execution logic by routing events through a trait dispatcher (e.g., `CommandDispatcher`). In unit tests, inject a `MockDispatcher` that records commands for assertions.

---

## 3. Targeted Domain Verification

### A. Document State (History & Undo/Redo)
- **Method:** Property-Based State Verification.
- **Approach:** Instead of writing single-case assertions, use the `proptest` crate to generate sequences of random actions (e.g., adding/deleting layers, changing opacities).
- **Rule:** The test must assert that applying an action and then calling undo restores the document state exactly: `State + Execute(Action) + Undo(Action) == State`.

### B. Procedural Graph (Shader Composition)
- **Method:** Snapshot Diffing.
- **Approach:** Assemble a node graph in memory, compile it to WGSL using the shader composer, and pass the resulting string to `insta::assert_snapshot!()`.
- **Workflow:** When the generator changes, the test will display the diff. Approve intended modifications locally using `cargo insta review`.

### C. Geometry Pipeline (BVH & Processing)
- **Method:** Deterministic Fixture Comparison.
- **Approach:** Store reference meshes (e.g., a primitive cube or UV sphere) in a `tests/fixtures/` directory.
- **Workflow:** Parse and flatten the geometry BVH into buffer arrays. Assert that the resulting byte streams match the reference binaries exactly, preventing vertex corruption.

### D. Render Pipelines (Compute Shaders)
- **Method:** Headless GPU "Golden Image" Execution.
- **Approach:** Initialize a `wgpu::Device` without asking for a viewport window (headless).
- **Workflow:** Upload mock textures, dispatch the compute shaders (such as image filters, advection solvers, or erosion steps), read the output buffer back to the CPU, and run `image-compare` against a reference "Golden Image".

---

## 4. Implementation Code Blueprints

### Property-Based State Verification
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_undo_state_reversion(
        layer_name in "[a-zA-Z0-9]{1,10}",
        opacity in 0.0f32..1.0f32
    ) {
        let mut document = DocumentState::default();
        let reference_state = document.clone();

        document.apply(Command::AddLayer { name: layer_name, opacity });
        document.undo();

        prop_assert_eq!(document, reference_state);
    }
}
```

### Headless GPU Compute Testing
```rust
#[test]
fn test_compute_shader_headless() {
    // 1. Initialize headless GPU (Synchronously via pollster)
    let (device, queue) = pollster::block_on(init_headless_wgpu());

    // 2. Upload input buffer and dispatch pipeline
    let input_texture = create_test_texture(&device, &queue);
    let output_texture = dispatch_compute_pass(&device, &queue, &input_texture);

    // 3. Retrieve output bytes and compare
    let output_bytes = pollster::block_on(read_texture_to_cpu(&device, &output_texture));
    let golden_bytes = include_bytes!("../fixtures/golden_output.png");

    let result = image_compare::rgb_similarity(output_bytes, golden_bytes);
    assert!(result.score > 0.99, "Compute shader output has drifted from golden baseline.");
}
```

---

## 5. CI/CD Matrix Configuration (GitHub Actions)

To execute headless GPU tests in virtual machines that do not have physical graphic cards, configure a software rasterizer (like Mesa Lavapipe) prior to running tests:

```yaml
name: Continuous Integration
on: [push, pull_request]

jobs:
  test-headless-gpu:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable

      - name: Install Software GPU (Lavapipe)
        run: sudo apt-get update && sudo apt-get install -y mesa-vulkan-drivers libvulkan1

      - name: Run Headless Graphics Tests
        env:
          WGPU_BACKEND: vulkan # Directs wgpu to target the software Vulkan driver
          RUST_BACKTRACE: 1
        run: cargo test --workspace
```
