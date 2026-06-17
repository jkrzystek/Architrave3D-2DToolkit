# Toolkit Data Flow & Messaging Specification

To achieve responsive, non-blocking interfaces in creative applications, the Velatura Toolkit relies on unidirectional message passing rather than shared mutable state. This document defines the channels, message categories, and event-loop sequence.

---

## 1. Core Communication Channels

Data is routed across crates and threads using three independent message pipelines:

```
                  ┌────────────────────────────────────────┐
                  │          winit Event Loop              │
                  └──────┬──────────────────────────┬──────┘
                         │ ViewportInputEvent       │ UI Interaction
                         │ (High-Freq Channel)      │
                         ▼                          ▼
┌────────────────────────┴────────┐        ┌────────┴────────┐
│         velatura_render         │        │   velatura_ui   │
│         (wgpu Queue)            │        │ (egui Layout)   │
└─────────────────────────────────┘        └────────┬────────┘
                 ▲                                  │
                 │ RenderCommand                    │ DocumentCommand
                 │ (Low-Freq Channel)               │ (Low-Freq Channel)
         ┌───────┴────────┐                         │
         │ velatura_state │◄────────────────────────┘
         │ (State / Undo) │
         └────────────────┘
```

### A. The High-Frequency Track (Input Telemetry)
Routes raw mouse, keyboard, or stylus movements from the windowing layer directly to the rendering thread, bypassing the state machine entirely to minimize rendering latency.
- **Message Types:** Stylus position, tilt, pressure, mouse coordinates, camera orbit, zoom, pan commands.
- **Channel Type:** Lock-free Single-Producer Single-Consumer (SPSC) ring buffer or synchronous crossbeam channel.

### B. The State Modification Track (UI Command System)
Routes user actions (such as layer additions, property adjustments, or node connections) from UI widgets to the central document state machine.
- **Message Types:** Layer adjustments, node modifications, history navigation (Undo/Redo).
- **Channel Type:** Multi-Producer Single-Consumer (MPSC) channel (e.g., `crossbeam-channel` or `flume`).

### C. The Rendering Track (Engine Dispatch)
Routes update requests from the state machine or the asset pipeline to the GPU render queue.
- **Message Types:** Command to rebuild procedural graphs, request to bake topology data, queueing a texture upload to VRAM.
- **Channel Type:** Multi-Producer Single-Consumer (MPSC) channel.

---

## 2. Event Loop Sequence

The main thread runs the windowing event loop. To prevent lag, each iteration of the frame loop processes events in a strict sequence:

1. **Input Polling:** Collect OS events.
   - If it is a viewport interaction (e.g., dragging, drawing), package it as a `ViewportInputEvent` and send it directly to the render engine.
   - If it is a panel or button click, let the UI engine consume the event to generate a `DocumentCommand`.
2. **State Processing (Non-Blocking):**
   - The state processor drains the `DocumentCommand` channel.
   - It updates layer structures, traverses DAG node invalidations, and saves history.
   - It issues a `RenderCommand` to signal which resources need updating.
3. **Asset Processing:**
   - Drain asynchronous resource results (e.g., completed disk-read buffers from background workers) and issue texture upload commands to the render queue.
4. **GPU Command Encoding:**
   - The rendering pipeline drains its `RenderCommand` queue.
   - Execute brush/projection compute passes first.
   - Run composting passes over dirty layers or procedural graphs.
   - Process one chunk of any active background task (e.g., terrain erosion or topology baking) to avoid OS driver timeouts.
   - Submit the command encoder to the GPU queue.
5. **UI Rendering:**
   - Run the immediate-mode UI pass.
   - Draw the viewport texture outputs onto the screen canvas.

---

## 3. Asynchronous Asset Streaming

When loading massive graphics assets (e.g., 4K texture packs, dense OBJ/glTF meshes):
1. The UI sends a `DocumentCommand::ImportAsset(path)` to the state machine.
2. The state registers a lightweight proxy resource (e.g., a 256x256 texture or a bounding box mesh representation) and instructs the asset thread to load the full resource.
3. A background thread pool (e.g., running under the `tokio` runtime) reads and decodes the file from disk asynchronously, keeping the main thread responsive.
4. Once completed, the thread transmits a `RenderCommand::UploadTexture` or `RenderCommand::UploadMesh` containing the byte buffer.
5. The rendering pipeline catches the message, updates the GPU resources in VRAM, and swaps the proxy out for the high-resolution asset.
