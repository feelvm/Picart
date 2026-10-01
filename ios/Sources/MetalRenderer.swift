import MetalKit

/// Owns the Metal objects and encodes the Rust frame plan into command
/// buffers. The engine decides *what* to render (pass list + uniform bytes);
/// this class only turns those passes into GPU work and presents the result.
public final class MetalRenderer {
    public let device: MTLDevice
    let queue: MTLCommandQueue
    private var pipelineCache: [UInt64: MTLRenderPipelineState] = [:]
    private var textures: [UInt64: MTLTexture] = [:]
    /// 1×1 opaque white — "no mask" until real masks land in Phase 3.
    private let noMaskTexture: MTLTexture

    public init?(device: MTLDevice) {
        self.device = device
        guard let queue = device.makeCommandQueue() else { return nil }
        self.queue = queue
        guard let t = device.makeTexture(descriptor: {
            let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .r8Unorm, width: 1, height: 1, depth: 1)
            d.usage = .shaderRead
            return d
        }()) else { return nil }
        var opaque: UInt8 = 255
        t.replace(region: MTLRegionMake2D(0, 0, 1, 1), mipmapLevel: 0, withBytes: &opaque, bytesPerRow: 1)
        self.noMaskTexture = t
    }

    /// Map a Rust asset id to a GPU texture (uploaded once at import).
    public func register(texture: MTLTexture, assetId: UInt64) {
        textures[assetId] = texture
    }

    private func pipeline(kind: UInt32, pixelFormat: MTLPixelFormat) -> MTLRenderPipelineState? {
        let key = (UInt64(kind) << 32) | UInt64(pixelFormat.rawValue)
        if let p = pipelineCache[key] { return p }
        guard kind == 0, // Pipeline::ColorGrade
              let lib = device.makeDefaultLibrary(),
              let vs = lib.makeFunction(name: "vs_fullscreen"),
              let fs = lib.makeFunction(name: "fs_color_grade") else { return nil }
        let d = MTLRenderPipelineDescriptor()
        d.vertexFunction = vs
        d.fragmentFunction = fs
        d.colorAttachments[0].pixelFormat = pixelFormat
        guard let p = try? device.makeRenderPipelineState(descriptor: d) else { return nil }
        pipelineCache[key] = p
        return p
    }

    /// Encode + commit one frame from the Rust plan. Returns false when the
    /// drawable/pipeline wasn't ready (frame skipped, never stalled).
    @discardableResult
    func draw(view: MTKView, engine: EditorEngine) -> Bool {
        let cpuStart = CFAbsoluteTimeGetCurrent()
        guard let drawable = view.currentDrawable,
              let rpd = view.currentRenderPassDescriptor,
              let buf = queue.makeCommandBuffer() else { return false }

        let passCount = engine.beginFrame(viewportW: UInt32(view.drawableSize.width),
                                          viewportH: UInt32(view.drawableSize.height))
        let clear = engine.clearRgba()
        rpd.colorAttachments[0].loadAction = passCount > 0 ? .clear : .dontCare
        rpd.colorAttachments[0].storeAction = .store
        rpd.colorAttachments[0].clearColor = MTLClearColor(red: Double(clear[0]),
                                                           green: Double(clear[1]),
                                                           blue: Double(clear[2]),
                                                           alpha: Double(clear[3]))
        guard let enc = buf.makeRenderCommandEncoder(descriptor: rpd) else { return false }
        for i in 0..<passCount {
            guard let pso = pipeline(kind: engine.passKind(i), pixelFormat: view.colorPixelFormat),
                  let tex = textures[engine.passTexture(i)] else { continue }
            enc.setRenderPipelineState(pso)
            enc.setFragmentTexture(tex, index: 0)
            enc.setFragmentTexture(noMaskTexture, index: 1)
            let uniforms = engine.passUniforms(i)
            uniforms.withUnsafeBytes { raw in
                enc.setFragmentBytes(raw.baseAddress!, length: raw.count, index: 0)
            }
            enc.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 3)
        }
        enc.endEncoding()
        buf.present(drawable)

        let cpuMs = Float((CFAbsoluteTimeGetCurrent() - cpuStart) * 1000)
        buf.addCompletionHandler { cb in
            let gpuMs = Float((cb.GPUEndTime - cb.GPUStartTime) * 1000)
            engine.perfFrame(gpuMs: gpuMs, cpuMs: cpuMs, passes: passCount)
        }
        buf.commit()
        return true
    }
}
