import MetalKit
import SwiftUI

// MARK: - Metal canvas: displays GPU textures produced from the Rust render plan.
// Touch → EditorEngine → ec_render_preview → MTL command buffer → display.
// Uses unified memory; texture lifetimes tracked per-frame to avoid stalls.

public struct MetalCanvas: UIViewRepresentable {
    public var engine: EditorEngine
    public init(engine: EditorEngine) { self.engine = engine }

    public func makeUIView(context: Context) -> MTKView {
        let v = MTKView()
        v.device = MTLCreateSystemDefaultDevice()
        v.isPaused = false
        v.enableSetNeedsDisplay = false
        v.preferredFramesPerSecond = 60
        v.delegate = context.coordinator
        return v
    }
    public func updateUIView(_ uiView: MTKView, context: Context) {}
    public func makeCoordinator() -> Coordinator { Coordinator(engine: engine) }

    public final class Coordinator: NSObject, MTKViewDelegate {
        let engine: EditorEngine
        var interacting = false
        init(engine: EditorEngine) { self.engine = engine }
        public func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {}
        public func draw(in view: MTKView) {
            // Ask Rust for the fused pass list; encode it here (composite.metal).
            _ = try? engine.renderPreview(viewportW: UInt32(view.drawableSize.width),
                                          viewportH: UInt32(view.drawableSize.height),
                                          interacting: interacting)
            // TODO: picks pipeline from plan JSON, encodes composite_blend kernel,
            // presents drawable. Thermal state (ProcessInfo.thermalState) lowers
            // drawableSize before dropping frames.
        }
    }
}

// MARK: - Editor screen: full-screen canvas + bottom toolbar (original UI).

public struct EditorScreen: View {
    @StateObject private var model = EditorModel()
    public init() {}
    public var body: some View {
        ZStack {
            MetalCanvas(engine: model.engine)
                .ignoresSafeArea()
                .gesture(MagnifyGesture().onChanged { model.pinch($0.magnification) }
                    .simultaneously(with: DragGesture().onChanged { model.pan($0.translation) }))
            VStack {
                Spacer()
                PerfOverlay(text: model.perfText)
                BottomToolbar(model: model)
            }
        }
    }
}

public final class EditorModel: ObservableObject {
    let engine = EditorEngine()
    @Published var perfText = "FPS: --"
    @Published var brightness: Double = 0
    private var gestureId: String?
    func pinch(_ m: CGFloat) { /* build transform JSON → setTransform(layer, gesture) */ }
    func pan(_ t: CGSize) {}
    func brightnessChanged(_ v: Double, ended: Bool) {
        if gestureId == nil { gestureId = UUID().uuidString }
        let json = "{\"brightness\":\(v),\"contrast\":1.0,\"saturation\":1.0,\"exposure\":0.0,\"temperature\":0.0,\"tint\":0.0,\"hue_shift\":0.0,\"sharpen\":0.0,\"blur_radius\":0.0,\"vignette\":0.0,\"grain\":0.0}"
        _ = try? engine.setFilter(layer: model_layer(), colorJson: json, gesture: gestureId)
        if ended { engine.endGesture(); gestureId = nil }
    }
    private func model_layer() -> UInt64 { 1 }
}

struct BottomToolbar: View {
    @ObservedObject var model: EditorModel
    var body: some View {
        VStack {
            Slider(value: $model.brightness, in: -1...1) { Text("Brightness") }
                .onChange(of: model.brightness) { model.brightnessChanged($0, ended: false) }
            HStack {
                Button("Undo") { _ = try? model.engine.undo() }
                Button("Redo") { _ = try? model.engine.redo() }
                Button("Export") { /* enqueue export on IO queue */ }
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
