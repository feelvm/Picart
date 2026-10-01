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
    /// Layer receiving filter edits; set by importImage. Never hardcoded.
    private var layerId: UInt64?

    /// Adds the image and remembers its layer for subsequent edits.
    @discardableResult
    public func importImage(uri: String, width: UInt32, height: UInt32, format: String) -> UInt64? {
        do {
            _ = try engine.createDocument(w: width, h: height)
            let layer = try engine.addImage(ImageSpec(uri: uri, width: width, height: height, format: format))
            layerId = layer
            return layer
        } catch { return nil }
    }

    func pinch(_ m: CGFloat) { /* build transform → setTransform(layer, gesture) */ }
    func pan(_ t: CGSize) {}

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
