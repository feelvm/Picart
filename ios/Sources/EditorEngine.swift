import Foundation

// MARK: - Rust FFI bridge (controller → engine, no pixel copies on hot path)
//
// Build: cargo build -p picsart-editor-core --release --target aarch64-apple-ios
// Link libeditor_core.a + add this module. All calls enqueue GPU work and return.

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
@_silgen_name("ec_render_preview") func ec_render_preview(_ w: UInt32, _ h: UInt32, _ interacting: Bool) -> UnsafeMutablePointer<CChar>?
@_silgen_name("ec_save_document") func ec_save_document() -> UnsafeMutablePointer<CChar>?
@_silgen_name("ec_load_document") func ec_load_document(_ json: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?
@_silgen_name("ec_free_string") func ec_free_string(_ p: UnsafeMutablePointer<CChar>?)

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

func encodeJSON<T: Encodable>(_ v: T) throws -> String {
    let data = try JSONEncoder().encode(v)
    guard let s = String(data: data, encoding: .utf8) else { throw EngineError.engine("utf8") }
    return s
}

/// Thin controller over the Rust engine. Owns no pixels.
public final class EditorEngine {
    public init() {}

    @discardableResult
    public func createDocument(w: UInt32, h: UInt32) throws -> UInt64 {
        // {"doc_id":N,"w":W,"h":H}
        struct Resp: Decodable { let doc_id: UInt64 }
        let resp = try JSONDecoder().decode(Resp.self, from: Data(try takeString(ec_create_document(w, h)).utf8))
        return resp.doc_id
    }

    public func useDocument(_ id: UInt64) throws { _ = try takeString(ec_use_document(id)) }

    /// Returns the new layer id. Encodes via JSONEncoder — URIs with quotes or
    /// backslashes stay valid JSON (no string interpolation into the payload).
    @discardableResult
    public func addImage(_ spec: ImageSpec) throws -> UInt64 {
        // {"layer_id":N}
        struct Resp: Decodable { let layer_id: UInt64 }
        let json = try encodeJSON(spec)
        let out = try json.withCString { takeString(ec_add_image($0)) }
        return try JSONDecoder().decode(Resp.self, from: Data(out.utf8)).layer_id
    }

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
    public func undo() throws -> String { try takeString(ec_undo()) }
    public func redo() throws -> String { try takeString(ec_redo()) }

    /// Returns render-plan JSON → Metal command encoder. No pixel readback.
    public func renderPreview(viewportW: UInt32, viewportH: UInt32, interacting: Bool) throws -> String {
        try takeString(ec_render_preview(viewportW, viewportH, interacting))
    }
}
