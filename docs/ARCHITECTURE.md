# Architecture & performance budgets

## GPU decision (2026-10-02)

**Direct Metal on iOS; no wgpu.** Rationale: develop on the shipping platform
(Mac/iPhone), zero abstraction between edited code and profiled commands,
full TBDR feature access (memoryless render targets, tile shading) if/when
profiling demands it. wgpu was rejected as a first step because its main
benefit — Windows-side rendering development — was not needed, and it would
have added a dev/product gap to debug through.

The cross-platform seam is **data, not code**: the Rust engine emits a typed
frame plan (pass list + packed 112-byte uniform floats, `renderer.rs` +
`uniforms.rs`); the native side encodes. Android later consumes the same plan
with a Vulkan encoder. Uniform packing is Rust's job, so native encoders stay
thin and identical in responsibility.

Shaders: hand-written MSL in `ios/Sources/Shaders` is the single source of
truth (WGSL/GLSL drafts removed). When Android lands, the three small shaders
get hand-ported to GLSL — bounded, reviewable work.

## Queues

```
UI Thread (gestures/controls) → Rust engine → Render / IO / AI queues → GPU / Disk / NPU
```

Render has priority. Decode/encode/export/bg-removal never touch UI or render threads.
(Swift side v1: MTKView drives the render loop; the async job queues land with
import/export/bg-removal phases.)

## Memory budgets (mid-range phone, 12MP edit)

| Pool | Cap | Notes |
|---|---|---|
| Decoded tiles (LRU) | 128 MB | 512px tiles, evict oldest first |
| Preview textures | 64 MB | single fused pass target + mask |
| GPU scratch | 64 MB | blur ping-pong max 2 targets |
| Undo history | param deltas only | slider drag = 1 entry |
| Masks | ≤ preview res | AI mask cached once |

On `onTrimMemory` / memory-warning: drop preview LRU, shrink long-edge to 1024.
(LRU implemented in `cache.rs`, wired to real resources in Phase 4.)

## FFI shape

* Edits: JSON payloads, mutate engine state, return immediately (`ec_set_*`).
* Frame plan: flat ABI — `ec_begin_frame` returns a pass count; `ec_pass_*`
  fetch pipeline kind, asset id, and 28 packed uniform floats per pass.
  No per-frame JSON; no pixel buffers ever cross the boundary.
* Perf: `ec_perf_frame(gpu_ms, cpu_ms, passes)` fed from the MTLCommandBuffer
  completion handler; `ec_perf_text()` renders the overlay.

## Phase status (honest as of 2026-10-02)

- [~] Phase 1 — partial: document model ✓ (tested); frame-plan builder + uniform packing + inverse transform ✓ (tested); Swift MetalRenderer + MSL authored — **not yet device-verified (needs first Mac build)**
- [~] Phase 2 — partial: ColorAdjust params + fused MSL color-grade pass written; brightness wired end-to-end in the UI; remaining sliders are parameter plumbing only
- [~] Phase 3 — partial: layer/mask/text/drawing data models ✓; undo covers params, masks, crop, layer removal ✓ (tested); no multi-layer GPU compositing yet
- [~] Phase 4 — partial: graph fusion planner ✓ (tested); blur MSL kernel written, not wired into the plan; LRU implemented but unused; tiling = metadata only
- [ ] Phase 5 — interface + NullSegmenter stub only (no Core ML / ONNX)
- [ ] Phase 6 — export request validation only (no rendering/encoding)
- [ ] Phase 7 — not started

## Next verification gate (Mac)

1. `cargo test` green (16 tests) — already true on Windows.
2. `cd ios && xcodegen && open`, Cmd+R on simulator: test image renders, pan/pinch smooth, brightness slider live, perf overlay counting.
3. Same on a physical iPhone with a 12MP image; check thermal behavior after 5 min of continuous gesturing.
