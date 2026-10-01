import MetalKit
import SwiftUI

// MARK: - Metal canvas: encodes the Rust frame plan every frame.
// Touch → EditorEngine (state) → ec_begin_frame → MTLCommandBuffer → display.
// The MTKView owns drawable management/pacing; the renderer owns encoding.

public struct MetalCanvas: UIViewRepresentable {
    public let engine: EditorEngine
    public let renderer: MetalRenderer
    public weak var model: EditorModel?

    public init(engine: EditorEngine, renderer: MetalRenderer, model: EditorModel?) {
        self.engine = engine; self.renderer = renderer; self.model = model
    }

    public func makeUIView(context: Context) -> MTKView {
        let v = MTKView(frame: .zero, device: renderer.device)
        v.isPaused = false
        v.enableSetNeedsDisplay = false
        v.preferredFramesPerSecond = 60
        v.framebufferOnly = true
        v.delegate = context.coordinator
        return v
    }
    public func updateUIView(_ uiView: MTKView, context: Context) {}
    public func makeCoordinator() -> Coordinator {
        Coordinator(engine: engine, renderer: renderer, model: model)
    }

    public final class Coordinator: NSObject, MTKViewDelegate {
        let engine: EditorEngine
        let renderer: MetalRenderer
        weak var model: EditorModel?
        var frameCount = 0
        init(engine: EditorEngine, renderer: MetalRenderer, model: EditorModel?) {
            self.engine = engine; self.renderer = renderer; self.model = model
        }
        public func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {}
        public func draw(in view: MTKView) {
            renderer.draw(view: view, engine: engine)
            frameCount += 1
            if frameCount % 20 == 0, let model, let text = engine.perfText() {
                model.perfText = text
            }
        }
    }
}

// MARK: - Editor screen: full-screen canvas + bottom toolbar (original UI).

public struct EditorScreen: View {
    @StateObject private var model = EditorModel()
    public init() {}
    public var body: some View {
        GeometryReader { geo in
            ZStack {
                MetalCanvas(engine: model.engine, renderer: model.renderer, model: model)
                    .ignoresSafeArea()
                    .gesture(
                        MagnifyGesture()
                            .onChanged { model.pinch($0.magnification) }
                            .onEnded { _ in model.endGesture() }
                            .simultaneously(with:
                                DragGesture()
                                    .onChanged { model.pan($0.translation) }
                                    .onEnded { _ in model.endGesture() })
                    )
                    .onAppear { model.viewportSize = geo.size }
                    .onChange(of: geo.size) { _, newSize in model.viewportSize = newSize }
                VStack {
                    Spacer()
                    PerfOverlay(text: model.perfText)
                    BottomToolbar(model: model)
                }
            }
        }
    }
}

public final class EditorModel: ObservableObject {
    let engine = EditorEngine()
    let renderer: MetalRenderer?
    @Published var perfText = "FPS: --"
    @Published var brightness: Double = 0
    /// Point size of the canvas; pan deltas are normalized by it.
    var viewportSize: CGSize = .zero
    private var layerId: UInt64?
    /// Mirror of the active layer transform; the engine is the source of truth.
    private var transform = EngineTransform()
    private var gestureStart: EngineTransform?
    private var gestureId: String?

    public init() {
        let device = MTLCreateSystemDefaultDevice()
        renderer = device.flatMap { MetalRenderer(device: $0) }
        do {
            _ = try engine.createDocument(w: UInt32(TestImage.width), h: UInt32(TestImage.height))
            let result = try engine.addImage(ImageSpec(
                uri: "generated://test", width: UInt32(TestImage.width),
                height: UInt32(TestImage.height), format: "png"))
            layerId = result.layerId
            if let renderer, let img = TestImage.make(), let device {
                // Sync decode of the 12MP test image (~50ms) at launch; the
                // async photo-import path replaces this in a later phase.
                let loader = MTKTextureLoader(device: device)
                let tex = try loader.newTexture(cgImage: img, options: [
                    .textureUsage: MTLTextureUsage.shaderRead.rawValue,
                    .textureStorageMode: MTLStorageMode.private.rawValue,
                ])
                renderer.register(texture: tex, assetId: result.assetId)
            }
        } catch {
            // Engine missing in this environment; UI launches but stays inert.
            layerId = nil
        }
    }

    // -- gestures: pinch/pan update engine transform; gesture id coalesces
    //    the whole interaction into ONE undo entry (engine-side coalescing).

    private func beginGestureIfNeeded() {
        guard gestureStart == nil else { return }
        gestureStart = transform
        gestureId = UUID().uuidString
    }

    func pan(_ t: CGSize) {
        beginGestureIfNeeded()
        guard let layerId, let start = gestureStart,
              viewportSize.width > 0, viewportSize.height > 0 else { return }
        var tr = start
        tr.translate = [start.translate[0] - Float(t.width / viewportSize.width),
                        start.translate[1] - Float(t.height / viewportSize.height)]
        apply(tr, layer: layerId)
    }

    func pinch(_ m: CGFloat) {
        beginGestureIfNeeded()
        guard let layerId, let start = gestureStart else { return }
        let m = Float(max(m, 0.05))
        var tr = start
        tr.scale = [start.scale[0] / m, start.scale[1] / m]
        // zoom about screen center (0.5, 0.5)
        let c: [Float] = [0.5, 0.5]
        tr.translate = [(start.translate[0] - c[0]) / m + c[0],
                        (start.translate[1] - c[1]) / m + c[1]]
        apply(tr, layer: layerId)
    }

    func endGesture() {
        engine.endGesture()
        gestureId = nil
        gestureStart = nil
    }

    private func apply(_ tr: EngineTransform, layer: UInt64) {
        transform = tr
        _ = try? engine.setTransform(layer: layer, transform: tr, gesture: gestureId)
    }

    func brightnessChanged(_ v: Double, ended: Bool) {
        guard let layer = layerId else { return }
        if gestureId == nil { gestureId = UUID().uuidString }
        var color = ColorAdjust()
        color.brightness = Float(v)
        _ = try? engine.setFilter(layer: layer, color: color, gesture: gestureId)
        if ended { engine.endGesture(); gestureId = nil }
    }
}

struct BottomToolbar: View {
    @ObservedObject var model: EditorModel
    var body: some View {
        VStack {
            Slider(value: $model.brightness, in: -1...1) { Text("Brightness") }
                .onChange(of: model.brightness) { _, newValue in
                    model.brightnessChanged(newValue, ended: false)
                }
            HStack {
                Button("Undo") { _ = try? model.engine.undo() }
                Button("Redo") { _ = try? model.engine.redo() }
                Button("Export") { /* enqueue export on IO queue (Phase 6) */ }
            }
        }.padding().background(.ultraThinMaterial)
    }
}

struct PerfOverlay: View {
    let text: String
    var body: some View {
        Text(text).font(.caption2).monospaced()
            .padding(6).background(Color.black.opacity(0.6)).foregroundColor(.green)
            .cornerRadius(6).padding(.trailing)
    }
}
