# Creating New Modules

This guide explains how to add a new crate to the toolkit that follows the existing conventions.

## Step 1: Create the Crate

```bash
# From the workspace root
cargo init crates/toolkit_my_module --lib
```

## Step 2: Configure Cargo.toml

```toml
[package]
name = "toolkit_my_module"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true
description = "Short description of what this module does"

[dependencies]
toolkit_core = { workspace = true }
serde = { workspace = true }
# Add only what you need from workspace deps

[dev-dependencies]
serde_json = { workspace = true }
```

## Step 3: Register in Workspace

Add to the root `Cargo.toml`:

```toml
# In [workspace] members:
members = [
    # ... existing crates ...
    "crates/toolkit_my_module",
]

# In [workspace.dependencies]:
toolkit_my_module = { path = "crates/toolkit_my_module" }
```

## Step 4: Structure the Code

Follow the existing pattern:

```
crates/toolkit_my_module/
├── Cargo.toml
└── src/
    ├── lib.rs          # Re-exports all public API
    ├── types.rs        # Core data structures
    ├── operations.rs   # Operations on those structures
    └── (more files)    # One concern per file
```

### lib.rs Pattern

```rust
pub mod types;
pub mod operations;

pub use types::{MyType, MyConfig};
pub use operations::my_operation;
```

Every public type should be re-exported from `lib.rs` so consumers can write `use toolkit_my_module::MyType` without knowing internal module paths.

## Step 5: Follow These Conventions

### Serialization
- Derive `Serialize, Deserialize` on all public data types
- Use `#[serde(skip)]` for transient fields (caches, GPU handles, dirty flags)
- Add a serialization roundtrip test

### IDs
- Use toolkit_core's typed IDs (`LayerId`, `NodeId`, etc.) for entity references
- If you need a new ID type, add it to `toolkit_core/src/id.rs` using the `define_id!` macro
- Never use raw `u64` or `usize` for entity identifiers

### Error Handling
- Use `toolkit_core::ToolkitResult<T>` and `ToolkitError` for fallible operations
- Don't create module-specific error types unless you have errors that can't map to `ToolkitError`

### GPU Data
- If your type will be sent to the GPU, derive `bytemuck::Pod` and `bytemuck::Zeroable`
- Use `[f32; N]` arrays instead of `Vec3`/`Vec2` for Pod-compatible fields
- Provide accessor methods that return glam types: `fn position_vec3(&self) -> Vec3`
- Ensure `#[repr(C)]` and 16-byte alignment for uniform buffers

### Testing
- Every file should have a `#[cfg(test)] mod tests` block
- Test at minimum: construction, basic operations, edge cases, serialization roundtrip
- Use `proptest` for property-based testing when dealing with numeric algorithms

## Step 6: Add AI Bridge Support (Optional)

If you want LLM access to your module's data, add an adapter in `toolkit_ai_bridge`:

1. Create `crates/toolkit_ai_bridge/src/adapters/my_module_adapter.rs`
2. Implement `AiProvider` trait for your adapter
3. Add a feature flag in `toolkit_ai_bridge/Cargo.toml`:
   ```toml
   [features]
   adapter-my-module = ["dep:toolkit_my_module"]
   
   [dependencies]
   toolkit_my_module = { workspace = true, optional = true }
   ```
4. Gate the adapter module with `#[cfg(feature = "adapter-my-module")]` in `adapters/mod.rs`

See `adapters/state_adapter.rs` for a full example.

### What to Expose via AI Bridge

**DO expose:**
- Metadata and summaries (counts, dimensions, names)
- Configuration values (settings, parameters)
- Property read/write (opacity, visibility, names)
- Semantic queries (raycast results, node graph topology)
- Point sampling (value at coordinate X,Y)

**DO NOT expose:**
- Raw buffers (pixel arrays, vertex data, index buffers)
- GPU handles (textures, pipelines, device references)
- Full grid dumps (entire heightmaps, density fields)
- Internal caches or temporary state

## Step 7: Document

Add your module to `docs/module-reference.md` with:
- What it does (one paragraph)
- Key types and their relationships
- Common usage patterns
- Which AI bridge tools/resources it exposes (if applicable)
