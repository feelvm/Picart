# Picsart-clone — mobile-only GPU-first photo editor

Shared Rust core + native iOS (Swift/SwiftUI + direct Metal) + Android (Kotlin/Compose + Vulkan, planned).

**GPU decision (2026-10-02): straight direct Metal on iOS, no wgpu.** The Rust
core never touches pixels — it emits typed *frame plans* (pass list + packed
uniform floats) and the platform's native GPU API encodes and executes them.
Shaders are hand-written MSL in `ios/Sources/Shaders` (single source of truth;
the WGSL/GLSL drafts were removed to prevent drift).

```
Touch → SwiftUI (controller) → Rust engine state (document/layers/history)
      → Rust frame plan (passes + 112-byte uniform blocks)
      → Swift MetalRenderer encodes MTLRenderCommandEncoders → Metal → display
```

No per-frame JSON, no pixel copies, no GPU→CPU→GPU round-trips on the hot path.

## Layout

```
rust/editor-core/          shared engine (document, layers, effects, graph,
                           history, cache, preview, export, perf, FFI)
ios/
  project.yml              XcodeGen definition (generates PicsartClone.xcodeproj)
  build-rust.sh            Xcode pre-build phase: cargo → libeditor_core.a
  Sources/
    PicsartCloneApp.swift  @main app entry
    EditorEngine.swift     Rust FFI bridge (JSON edits + flat frame-plan reads)
    MetalRenderer.swift    device/queue/pipelines/texture upload/encode/present
    EditorUI.swift         SwiftUI editor, gestures, perf overlay
    TestImage.swift        procedural 12MP test image (no binary assets)
    Shaders/*.metal        MSL sources (compiled to default.metallib by Xcode)
android/                   scaffold only (Vulkan encoder is a later phase)
docs/ARCHITECTURE.md       threading, memory budgets, phase status
```

## Build

### Windows (logic development)

```sh
cargo check -p picsart-editor-core
cargo test  -p picsart-editor-core   # 16 tests: document, undo, graph, uniforms, plans
```

### Mac (native iOS build — first-time setup)

```sh
# 1. Xcode from the App Store, then:
xcode-select --install

# 2. Rust with iOS targets
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim

# 3. XcodeGen
brew install xcodegen

# 4. Generate + open the project (repo root)
cd ios && xcodegen && open PicsartClone.xcodeproj
```

Then: pick the iOS Simulator → **Cmd+R**. The pre-build phase runs
`build-rust.sh` (cargo for the sim target) and links `libeditor_core.a`
automatically. For a real iPhone: select your device, sign with your Apple ID
(Personal Team is fine), **Cmd+R**.

First build compiles the Rust staticlib (~30s release); subsequent builds are
incremental.

## Performance contract

- 60fps target during gestures; the engine sheds preview resolution before
  dropping frames (policy in `preview.rs`, wired to real frame times later).
- Preview ≠ export render paths; export is async off the UI/render threads.
- Undo entries are param deltas with gesture coalescing — a full slider drag
  or pinch is ONE history entry (tested).
- What to expect from the first run: 12MP test image, fused color-grade pass,
  pan/pinch at interactive rates, perf overlay (FPS/Frame/GPU/CPU/Passes).
