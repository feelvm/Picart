# Picsart-clone — mobile-only GPU-first photo editor

Shared Rust core + native iOS (Swift/SwiftUI + Metal) + Android (Kotlin/Compose + Vulkan).
`wgpu` is used only as a portable GPU abstraction; direct Metal/Vulkan paths are
preferred where they win on bandwidth / pass-count.

```
Mobile App
   │  Native UI (controller)      Shared Rust Core (owner of state)
   ├─ iOS: Swift/SwiftUI/UIKit    ├─ Document / layers / history / cache
   └─ Android: Kotlin/Compose     └─ Renderer → GPU abstraction → Metal/Vulkan → Mobile GPU
```

Hot path: `Touch → Native UI → Rust state → GPU commands → Metal/Vulkan → Display`.
No `GPU→CPU→GPU` round-trips on the hot path. CPU owns state/IO/decode/encode,
GPU owns per-pixel realtime work.

## Layout

```
rust/editor-core/      shared engine (document, layers, effects, graph, cache,
                       history, preview, export, perf, FFI)
rust/editor-core/shaders/  WGSL (wgpu) + .metal + Vulkan GLSL
ios/                   Swift package: FFI bridge, Metal view, SwiftUI editor
android/               Kotlin module: JNI bridge, Compose editor, Vulkan view
docs/                  architecture, threading, performance budgets
```

## Build

```sh
# Rust core (host check; on-device uses cargo-ndk / xcode cargo-lipo equivalent)
cargo check -p picsart-editor-core
cargo test -p picsart-editor-core
```

iOS/Android shells are scaffolds, not buildable apps yet (see
`docs/ARCHITECTURE.md` for honest phase status). The iOS app project
(Xcode project + Rust staticlib wiring) is the next milestone.

## Performance contract

- 60fps target during gestures; degrade preview resolution before dropping frames.
- Preview ≠ export render paths. Preview is low-latency reduced-res GPU;
  export is full-res async off the UI/render threads.
- Undo entries are param deltas with slider coalescing, never full image copies.
- LRU caches for tiles / preview textures / GPU resources with pressure callbacks.
