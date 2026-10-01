//! C ABI for Swift (FFI) and Kotlin (JNI). All functions are async-friendly:
//! they mutate state + enqueue GPU/IO/AI work and return immediately.
//! No full-res pixel buffers cross this boundary on the hot path.
//!
//! Documents live in a registry keyed by [`DocumentId`]; one document is
//! active at a time. `ec_create_document` inserts + activates, `ec_load_document`
//! adds a new active doc, `ec_use_document` switches. Layer ids are process-unique.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Mutex, OnceLock};

use crate::document::{Document, DocumentId};

struct Registry {
    docs: HashMap<DocumentId, Document>,
    active: Option<DocumentId>,
}

static REG: OnceLock<Mutex<Registry>> = OnceLock::new();

fn registry() -> &'static Mutex<Registry> {
    REG.get_or_init(|| Mutex::new(Registry { docs: HashMap::new(), active: None }))
}

/// Run `f` on the active document. `None` when no document is active yet.
fn with_active<T>(f: impl FnOnce(&mut Document) -> T) -> Option<T> {
    let mut reg = registry().lock().unwrap();
    let id = reg.active?;
    reg.docs.get_mut(&id).map(f)
}

fn to_str<'a>(p: *const c_char) -> Result<&'a str, String> {
    if p.is_null() { return Err("null string".into()); }
    unsafe { CStr::from_ptr(p).to_str().map_err(|e| e.to_string()) }
}

fn ok_json<T: serde::Serialize>(v: &T) -> *mut c_char {
    match serde_json::to_string(v) {
        Ok(s) => CString::new(s).map(|c| c.into_raw()).unwrap_or(std::ptr::null_mut()),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Caller must release with `ec_free_string`.
#[no_mangle]
pub extern "C" fn ec_free_string(p: *mut c_char) {
    if !p.is_null() { unsafe { let _ = CString::from_raw(p); } }
}

#[no_mangle]
pub extern "C" fn ec_create_document(width: u32, height: u32) -> *mut c_char {
    let d = Document::new(width, height);
    let id = d.id;
    let mut reg = registry().lock().unwrap();
    reg.docs.insert(id, d);
    reg.active = Some(id);
    ok_json(&serde_json::json!({ "doc_id": id, "w": width, "h": height }))
}

/// Make `doc_id` (previously created or loaded) the active document.
#[no_mangle]
pub extern "C" fn ec_use_document(doc_id: u64) -> *mut c_char {
    let mut reg = registry().lock().unwrap();
    if reg.docs.contains_key(&doc_id) {
        reg.active = Some(doc_id);
        ok_json(&serde_json::json!({ "ok": true, "doc_id": doc_id }))
    } else {
        ok_json(&serde_json::json!({ "error": "unknown doc_id" }))
    }
}

fn parse_format(s: &str) -> crate::image_source::ImageFormat {
    match s.to_ascii_lowercase().as_str() {
        "png" => crate::image_source::ImageFormat::Png,
        "webp" => crate::image_source::ImageFormat::WebP,
        "heif" | "heic" => crate::image_source::ImageFormat::Heif,
        _ => crate::image_source::ImageFormat::Jpeg,
    }
}

#[no_mangle]
pub extern "C" fn ec_add_image(spec_json: *const c_char) -> *mut c_char {
    let spec = match to_str(spec_json) { Ok(s) => s, Err(e) => return err_ptr(e) };
    let v: serde_json::Value = match serde_json::from_str(spec) { Ok(v) => v, Err(e) => return err_ptr(e.to_string()) };
    let src = crate::image_source::ImageSource {
        asset_id: crate::next_id(),
        uri: v.get("uri").and_then(|x| x.as_str()).unwrap_or("").into(),
        width: v.get("width").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
        height: v.get("height").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
        format: v.get("format").and_then(|x| x.as_str()).map(parse_format).unwrap_or(crate::image_source::ImageFormat::Jpeg),
        tile_px: v.get("tile_px").and_then(|x| x.as_u64()).unwrap_or(512) as u32,
    };
    let res = with_active(|d| {
        let id = d.add_image(src);
        serde_json::json!({ "layer_id": id })
    });
    match res { Some(j) => ok_json(&j), None => err_ptr("no active document".into()) }
}

#[no_mangle]
pub extern "C" fn ec_set_filter(layer_id: u64, spec_json: *const c_char, gesture: *const c_char) -> *mut c_char {
    let spec = match to_str(spec_json) { Ok(s) => s.to_owned(), Err(e) => return err_ptr(e) };
    let g = if gesture.is_null() { None } else { to_str(gesture).ok().map(|s| s.to_owned()) };
    let color: crate::effects::ColorAdjust = match serde_json::from_str(&spec) { Ok(c) => c, Err(e) => return err_ptr(e.to_string()) };
    let res = with_active(|d| {
        d.set_filter(layer_id, color, g);
        serde_json::json!({ "ok": true, "passes": d.render_export_plan().pass_count })
    });
    match res { Some(j) => ok_json(&j), None => err_ptr("no active document".into()) }
}

#[no_mangle]
pub extern "C" fn ec_set_transform(layer_id: u64, spec_json: *const c_char, gesture: *const c_char) -> *mut c_char {
    let spec = match to_str(spec_json) { Ok(s) => s.to_owned(), Err(e) => return err_ptr(e) };
    let g = if gesture.is_null() { None } else { to_str(gesture).ok().map(|s| s.to_owned()) };
    let t: crate::transform::Transform = match serde_json::from_str(&spec) { Ok(t) => t, Err(e) => return err_ptr(e.to_string()) };
    let res = with_active(|d| {
        d.set_transform(layer_id, t, g);
        serde_json::json!({ "ok": true })
    });
    match res { Some(j) => ok_json(&j), None => err_ptr("no active document".into()) }
}

#[no_mangle]
pub extern "C" fn ec_set_opacity(layer_id: u64, opacity: f32, gesture: *const c_char) -> *mut c_char {
    let g = if gesture.is_null() { None } else { to_str(gesture).ok().map(|s| s.to_owned()) };
    let res = with_active(|d| {
        d.set_opacity(layer_id, opacity, g);
        serde_json::json!({ "ok": true })
    });
    match res { Some(j) => ok_json(&j), None => err_ptr("no active document".into()) }
}

#[no_mangle]
pub extern "C" fn ec_move_layer(layer_id: u64, to_index: u32) -> *mut c_char {
    let res = with_active(|d| {
        let ok = d.move_layer(layer_id, to_index as usize);
        serde_json::json!({ "ok": ok })
    });
    match res { Some(j) => ok_json(&j), None => err_ptr("no active document".into()) }
}

#[no_mangle]
pub extern "C" fn ec_remove_layer(layer_id: u64) -> *mut c_char {
    let res = with_active(|d| serde_json::json!({ "ok": d.remove_layer(layer_id) }));
    match res { Some(j) => ok_json(&j), None => err_ptr("no active document".into()) }
}

#[no_mangle]
pub extern "C" fn ec_undo() -> *mut c_char {
    let res = with_active(|d| serde_json::json!({ "ok": d.undo() }));
    match res { Some(j) => ok_json(&j), None => err_ptr("no active document".into()) }
}

#[no_mangle]
pub extern "C" fn ec_redo() -> *mut c_char {
    let res = with_active(|d| serde_json::json!({ "ok": d.redo() }));
    match res { Some(j) => ok_json(&j), None => err_ptr("no active document".into()) }
}

#[no_mangle]
pub extern "C" fn ec_end_gesture() {
    with_active(|d| d.end_gesture());
}

/// Returns render-plan JSON (pass list) — native side turns it into
/// Metal/Vulkan command buffers without any pixel copies.
#[no_mangle]
pub extern "C" fn ec_render_preview(viewport_w: u32, viewport_h: u32, interacting: bool) -> *mut c_char {
    let res = with_active(|d| {
        let graph = crate::graph::EffectGraph::from_layers(d.layers_in_order());
        let plan = graph.plan();
        let mp = d.layers_in_order().iter()
            .filter_map(|l| match &l.kind {
                crate::layer::LayerKind::Image { asset }
                | crate::layer::LayerKind::Sticker { asset } => {
                    d.assets.iter().find(|a| a.asset_id == *asset).map(|a| a.megapixels())
                }
                _ => None,
            })
            .sum::<f32>();
        let dec = crate::preview::decide_preview(crate::preview::PreviewRequest {
            src_mp: mp, frame_ms_ema: if interacting { 18.0 } else { 10.0 },
            effect_cost: graph.total_cost(), tier: crate::preview::DeviceTier::Mid, interacting,
        });
        serde_json::json!({
            "passes": plan.passes, "pass_count": plan.pass_count,
            "preview_long_edge": dec.long_edge_px, "high_quality": dec.high_quality,
            "viewport": [viewport_w, viewport_h],
        })
    });
    match res { Some(j) => ok_json(&j), None => err_ptr("no active document".into()) }
}

#[no_mangle]
pub extern "C" fn ec_save_document() -> *mut c_char {
    let res = with_active(|d| d.save_document());
    match res {
        Some(Ok(s)) => CString::new(s).map(|c| c.into_raw()).unwrap_or(std::ptr::null_mut()),
        Some(Err(e)) => err_ptr(e),
        None => err_ptr("no active document".into()),
    }
}

#[no_mangle]
pub extern "C" fn ec_load_document(json: *const c_char) -> *mut c_char {
    let s = match to_str(json) { Ok(s) => s.to_owned(), Err(e) => return err_ptr(e) };
    match Document::load_document(&s) {
        Ok(mut nd) => {
            // Fresh id avoids colliding with documents already in the registry.
            nd.id = crate::next_id();
            let id = nd.id;
            let layers = nd.layer_count();
            let mut reg = registry().lock().unwrap();
            reg.docs.insert(id, nd);
            reg.active = Some(id);
            ok_json(&serde_json::json!({ "ok": true, "doc_id": id, "layers": layers }))
        }
        Err(e) => err_ptr(e),
    }
}

#[no_mangle]
pub extern "C" fn ec_perf_overlay() -> *mut c_char {
    // Real counters live in the per-session PerfMonitor on native side;
    // this reports the static contract so the overlay always renders.
    ok_json(&serde_json::json!({ "hint": "FPS: -- / wire PerfMonitor::overlay_text() to native CADisplayLink/Choreographer" }))
}

fn err_ptr(e: String) -> *mut c_char {
    ok_json(&serde_json::json!({ "error": e }))
}
