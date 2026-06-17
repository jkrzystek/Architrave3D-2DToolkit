# Toolkit Setup & Installation Guide

This guide details the process of setting up a Cargo workspace, installing the stable Rust toolchain, and verifying graphics hardware configuration to build applications using the Velatura Toolkit.

---

## 1. Prerequisites

Because the toolkit utilizes a pure Rust GPU-compute pipeline, there are no external C/C++ dependencies. However, you must verify that your development machine has the necessary GPU drivers:

- **Windows:** Latest drivers supporting DirectX 12 and Vulkan.
- **macOS:** Native Metal support (macOS 10.13+).
- **Linux:** Vulkan ICD loaders (e.g., `mesa-vulkan-drivers`, `vulkan-icd-loader`).

---

## 2. Rust Toolchain Configuration

We build exclusively on the **Stable Rust** toolchain to ensure long-term stability and compilation reliability.

### Step 1: Update the Toolchain
```bash
rustup update stable
```

### Step 2: Add Required Components
- `rust-analyzer`: Real-time code analysis, type definitions, and auto-completion.
- `clippy`: An extensive collection of lints to catch performance, styling, and safety issues in graphics code.
```bash
rustup component add rust-analyzer clippy
```

---

## 3. Scaffolding a New Application Workspace

It is highly recommended to structure your creative application as a **Cargo Workspace**. This facilitates clean separation of concerns and parallelizes compiler execution.

### Creating the Virtual Manifest
Create a `Cargo.toml` in your project root with the following structure:

```toml
[workspace]
resolver = "2" # Prevents optional feature bleeding across crates
members = [
    "app_shell",         # Entry point: Window loop and OS input mapping
    "app_core",          # Core State: Document stack, Undo/Redo, serialization
    "app_graphics",      # Graphics: Custom wgpu rendering pipelines
]

[workspace.dependencies]
# Centralize toolkit and core dependency versions
wgpu = "23.0"
egui = "0.30"
glam = "0.29"
tokio = { version = "1.38", features = ["full"] }
```

### Windows PowerShell Scaffolding Script
Run the following PowerShell script from your workspace root to initialize directories and Cargo manifests:

```powershell
$Crates = @("app_shell", "app_core", "app_graphics")

foreach ($Crate in $Crates) {
    if ($Crate -eq "app_shell") {
        cargo new $Crate --bin
    } else {
        cargo new $Crate --lib
    }
    Write-Host "Scaffolded $Crate Crate"
}
```

---

## 4. Hardware Verification Test

Before writing rendering code, verify that your system successfully initializes a `wgpu` graphics adapter. Place the following test in your rendering library:

```rust
// app_graphics/src/lib.rs

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn verify_gpu_adapter() {
        let instance = wgpu::Instance::default();
        let adapters = instance.enumerate_adapters(wgpu::Backends::all());
        
        println!("Available GPU Adapters on Host:");
        for adapter in adapters {
            let info = adapter.get_info();
            println!("- [{:?}] {} (Driver: {})", info.backend, info.name, info.driver_info);
        }
        
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await;
            
        assert!(adapter.is_some(), "Failed to acquire a default GPU adapter context.");
        println!("Successfully initialized high-performance GPU context.");
    }
}
```

Run the test using:
```bash
cargo test -p app_graphics -- --nocapture
```
