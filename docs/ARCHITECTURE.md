# Architecture & performance budgets

## Queues

```
UI Thread (gestures/controls) → Rust engine → Render / IO / AI queues → GPU / Disk / NPU
```

Render has priority. Decode/encode/export/bg-removal never touch UI or render threads.

## Memory budgets (mid-range phone, 12MP edit)

| Pool | Cap | Notes |
|---|---|---|
| Decoded tiles (LRU) | 128 MB | 512px tiles, evict oldest first |
| Preview textures | 64 MB | single fused pass target + mask |
| GPU scratch | 64 MB | blur ping-pong max 2 targets |
| Undo history | param deltas only | slider drag = 1 entry |
| Masks | ≤ preview res | AI mask cached once |

On `onTrimMemory` / memory-warning: drop preview LRU, shrink long-edge to 1024.

## Phase status (honest as of 2026-10-02)

- [~] Phase 1 — partial: document model ✓ (tested); pan/zoom/rotate = uniform math only, no gesture wiring; texture upload = trait + NullBackend only; Metal/Vulkan shader files exist but are not compiled/loaded; perf overlay = struct, not wired to a display link
- [~] Phase 2 — partial: ColorAdjust params + fused WGSL pass (now with vertex stage) on disk; no GPU pipeline executes it yet
- [~] Phase 3 — partial: layer/mask/text/drawing data models ✓; undo covers params, masks, crop, layer removal ✓ (tested); no GPU compositing
- [~] Phase 4 — partial: graph fusion planner ✓ (tested); LRU implemented but unused; tiling = metadata only, no decode
- [ ] Phase 5 — interface + NullSegmenter stub only (no Core ML / ONNX)
- [ ] Phase 6 — export request validation only (no rendering/encoding)
- [ ] Phase 7 — not started
