import Foundation

// MARK: - Rust FFI bridge (controller → engine, no pixel copies on hot path)
//
// Build: cargo build -p picsart-editor-core --release --target aarch64-apple-ios
// Link libeditor_core.a + add this module. All calls enqueue GPU work and return.

@_silgen_name("ec_create_document") func ec_create_document(_ w: UInt32, _ h: UInt32) -> UnsafeMutablePointer<CChar>?
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

/// Thin controller over the Rust engine. Owns no pixels.
public final class EditorEngine {
    public init() {}
    @discardableResult
    public func createDocument(w: UInt32, h: UInt32) throws -> String {
        try takeString(ec_create_document(w, h))
    }
    @discardableResult
    public func addImage(uri: String, w: UInt32, h: UInt32) throws -> String {
        let spec = "{\"uri\":\"\(uri)\",\"width\":\(w),\"height\":\(h)}"
        return try spec.withCString { try takeString(ec_add_image($0)) }
    }
    @discardableResult
    public func setFilter(layer: UInt64, colorJson: String, gesture: String?) throws -> String {
        try colorJson.withCString { cj in try withCStringOpt(gesture) { g in
            try takeString(ec_set_filter(layer, cj, g))
        }}
    }
    @discardableResult
    public func setTransform(layer: UInt64, transformJson: String, gesture: String?) throws -> String {
        try transformJson.withCString { tj in try withCStringOpt(gesture) { g in
            try takeString(ec_set_transform(layer, tj, g))
        }}
    }
    public func endGesture() { ec_end_gesture() }
    public func undo() throws -> String { try takeString(ec_undo()) }
    public func redo() throws -> String { try takeString(ec_redo()) }
    /// Returns render-plan JSON → Metal command encoder. No pixel readback.
    public func renderPreview(viewportW: UInt32, viewportH: UInt32, interacting: Bool) throws -> String {
        try takeString(ec_render_preview(viewportW, viewportH, interacting))
    }
}
