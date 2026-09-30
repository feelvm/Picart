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

## Phase checklist

- [x] Phase 1 scaffold: doc model, 1 image layer, texture upload trait, Metal/Vulkan shaders, pan/zoom/rotate uniforms, perf overlay struct
- [x] Phase 2: brightness/contrast/saturation/exposure/temp/tint/hue/crop/opacity/blend (fused WGSL pass)
- [x] Phase 3: layers/masks/stickers/text/drawing/order/visibility/undo-redo
- [x] Phase 4: blur/sharpen/LUT/curves/effect graph + fusion + tiled sources + adaptive preview + LRU
- [x] Phase 5: bg-removal interface (Core ML / ONNX+NNAPI), async, mask cache, undoable
- [x] Phase 6: export pipeline (separate from preview, async, quality/scale)
- [ ] Phase 7: benchmark 12/24/48MP on low/mid/flagship Android + old/recent iPhone; optimize by profile
