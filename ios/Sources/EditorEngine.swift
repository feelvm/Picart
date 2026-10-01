import Foundation

// MARK: - Rust FFI bridge (controller → engine, no pixel copies on hot path)
//
// Build: cargo build -p picsart-editor-core --release --target aarch64-apple-ios
// Link libeditor_core.a (wired by ios/build-rust.sh in the Xcode build phase).
//
// Two call classes:
// * State mutations (create/add/set/undo) — JSON payloads, mutate engine state.
// * Frame-plan reads (beginFrame/pass*) — flat floats+ids, called every frame
//   by the Metal encoder. No pixel buffers and no per-frame JSON cross here.

@_silgen_name("ec_create_document") func ec_create_document(_ w: UInt32, _ h: UInt32) -> UnsafeMutablePointer<CChar>?
@_silgen_name("ec_use_document") func ec_use_document(_ doc: UInt64) -> UnsafeMutablePointer<CChar>?
@_silgen_name("ec_add_image") func ec_add_image(_ json: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?
@_silgen_name("ec_set_filter") func ec_set_filter(_ layer: UInt64, _ json: UnsafePointer<CChar>, _ gesture: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?
@_silgen_name("ec_set_transform") func ec_set_transform(_ layer: UInt64, _ json: UnsafePointer<CChar>, _ gesture: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?
@_silgen_name("ec_set_opacity") func ec_set_opacity(_ layer: UInt64, _ v: Float, _ gesture: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?
@_silgen_name("ec_move_layer") func ec_move_layer(_ layer: UInt64, _ to: UInt32) -> UnsafeMutablePointer<CChar>?
@_silgen_name("ec_remove_layer") func ec_remove_layer(_ layer: UInt64) -> UnsafeMutablePointer<CChar>?
@_silgen_name("ec_undo") func ec_undo() -> UnsafeMutablePointer<CChar>?
@_silgen_name("ec_redo") func ec_redo() -> UnsafeMutablePointer<CChar>?
@_silgen_name("ec_end_gesture") func ec_end_gesture()
@_silgen_name("ec_free_string") func ec_free_string(_ p: UnsafeMutablePointer<CChar>?)

// Frame plan (flat ABI — see editor-core/src/ffi.rs)
@_silgen_name("ec_begin_frame") func ec_begin_frame(_ w: UInt32, _ h: UInt32) -> UInt32
@_silgen_name("ec_pass_kind") func ec_pass_kind(_ index: UInt32) -> UInt32
@_silgen_name("ec_pass_texture") func ec_pass_texture(_ index: UInt32) -> UInt64
@_silgen_name("ec_pass_uniforms") func ec_pass_uniforms(_ index: UInt32, _ out: UnsafeMutablePointer<Float>, _ len: UInt32)
@_silgen_name("ec_clear_rgba") func ec_clear_rgba(_ out: UnsafeMutablePointer<Float>)

// Performance instrumentation
@_silgen_name("ec_perf_frame") func ec_perf_frame(_ gpuMs: Float, _ cpuMs: Float, _ passes: UInt32)
@_silgen_name("ec_perf_text") func ec_perf_text() -> UnsafeMutablePointer<CChar>?

public enum EngineError: Error { case engine(String) }

func takeString(_ p: UnsafeMutablePointer<CChar>?) throws -> String {
    guard let p else { throw EngineError.engine("null") }
    defer { ec_free_string(p) }
    return String(cString: p)
}

func withCStringOpt(_ s: String?, _ body: (UnsafePointer<CChar>?) throws -> UnsafeMutablePointer<CChar>?) rethrows -> UnsafeMutablePointer<CChar>? {
    if let s { return try s.withCString { try body($0) } }
    return try body(nil)
}

// MARK: - Typed wire structs (JSON-encoded; field names match the Rust types)

public struct ColorAdjust: Codable {
    public var brightness: Float = 0      // -1..1
    public var contrast: Float = 1        // 0..2, 1 = neutral
    public var saturation: Float = 1      // 0..2
    public var exposure: Float = 0        // EV stops
    public var temperature: Float = 0     // -1..1
    public var tint: Float = 0            // -1..1
    public var hueShift: Float = 0        // radians
    public var sharpen: Float = 0         // 0..1
    public var blurRadius: Float = 0      // px at preview scale, 0 = off
    public var vignette: Float = 0        // 0..1
    public var grain: Float = 0           // 0..1

    enum CodingKeys: String, CodingKey {
        case brightness, contrast, saturation, exposure, temperature, tint
        case hueShift = "hue_shift", sharpen
        case blurRadius = "blur_radius"
        case vignette, grain
    }
    public init() {}
}

public struct EngineTransform: Codable {
    public var translate: [Float] = [0, 0]
    public var scale: [Float] = [1, 1]
    public var rotationRad: Float = 0

    enum CodingKeys: String, CodingKey {
        case translate, scale, rotationRad = "rotation_rad"
    }
    public init() {}
}

public struct ImageSpec: Encodable {
    public var uri: String
    public var width: UInt32
    public var height: UInt32
    public var format: String   // "jpeg" | "png" | "webp" | "heif"
    public var tilePx: UInt32 = 512

    enum CodingKeys: String, CodingKey {
        case uri, width, height, format, tilePx = "tile_px"
    }
    public init(uri: String, width: UInt32, height: UInt32, format: String) {
        self.uri = uri; self.width = width; self.height = height; self.format = format
    }
}

public struct AddImageResult: Decodable {
    public let layerId: UInt64
    public let assetId: UInt64
    enum CodingKeys: String, CodingKey {
        case layerId = "layer_id", assetId = "asset_id"
    }
}

func encodeJSON<T: Encodable>(_ v: T) throws -> String {
    let data = try JSONEncoder().encode(v)
    guard let s = String(data: data, encoding: .utf8) else { throw EngineError.engine("utf8") }
    return s
}

/// Thin controller over the Rust engine. Owns no pixels.
public final class EditorEngine {
    public init() {}

    // -- lifecycle --

    @discardableResult
    public func createDocument(w: UInt32, h: UInt32) throws -> UInt64 {
        // {"doc_id":N,"w":W,"h":H}
        struct Resp: Decodable { let doc_id: UInt64 }
        let resp = try JSONDecoder().decode(Resp.self, from: Data(try takeString(ec_create_document(w, h)).utf8))
        return resp.doc_id
    }

    public func useDocument(_ id: UInt64) throws { _ = try takeString(ec_use_document(id)) }

    /// Encodes via JSONEncoder — URIs with quotes/backslashes stay valid JSON.
    @discardableResult
    public func addImage(_ spec: ImageSpec) throws -> AddImageResult {
        let json = try encodeJSON(spec)
        let out = try json.withCString { takeString(ec_add_image($0)) }
        return try JSONDecoder().decode(AddImageResult.self, from: Data(out.utf8))
    }

    // -- edits --

    @discardableResult
    public func setFilter(layer: UInt64, color: ColorAdjust, gesture: String?) throws -> String {
        let json = try encodeJSON(color)
        return try withCStringOpt(gesture) { g in
            try json.withCString { cj in try takeString(ec_set_filter(layer, cj, g)) }
        }
    }

    @discardableResult
    public func setTransform(layer: UInt64, transform: EngineTransform, gesture: String?) throws -> String {
        let json = try encodeJSON(transform)
        return try withCStringOpt(gesture) { g in
            try json.withCString { tj in try takeString(ec_set_transform(layer, tj, g)) }
        }
    }

    @discardableResult
    public func setOpacity(layer: UInt64, opacity: Float, gesture: String?) throws -> String {
        try withCStringOpt(gesture) { g in try takeString(ec_set_opacity(layer, opacity, g)) }
    }

    public func endGesture() { ec_end_gesture() }
    @discardableResult public func undo() throws -> String { try takeString(ec_undo()) }
    @discardableResult public func redo() throws -> String { try takeString(ec_redo()) }

    // -- frame plan (called every frame by MetalRenderer) --

    public func beginFrame(viewportW: UInt32, viewportH: UInt32) -> Int {
        Int(ec_begin_frame(viewportW, viewportH))
    }
    public func passKind(_ index: Int) -> UInt32 { ec_pass_kind(UInt32(index)) }
    public func passTexture(_ index: Int) -> UInt64 { ec_pass_texture(UInt32(index)) }
    /// 28 packed floats (112 bytes) — layout in editor-core/src/uniforms.rs.
    public func passUniforms(_ index: Int) -> [Float] {
        var out = [Float](repeating: 0, count: 28)
        ec_pass_uniforms(UInt32(index), &out, UInt32(out.count))
        return out
    }
    public func clearRgba() -> [Float] {
        var out = [Float](repeating: 0, count: 4)
        ec_clear_rgba(&out)
        return out
    }

    // -- performance --

    public func perfFrame(gpuMs: Float, cpuMs: Float, passes: Int) {
        ec_perf_frame(gpuMs, cpuMs, UInt32(passes))
    }
    public func perfText() -> String? { try? takeString(ec_perf_text()) }
}
