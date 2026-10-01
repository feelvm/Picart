//! C ABI for Swift (FFI) and Kotlin (JNI).
//!
//! Two kinds of calls:
//! * State mutations (`ec_set_*`, undo/redo) — mutate the active document
//!   and return immediately; they enqueue nothing and touch no pixels.
//! * Frame-plan reads (`ec_begin_frame` + `ec_pass_*`) — the native encoder
//!   calls these once per frame to fetch *what* to render. Only floats and
//!   ids cross the boundary; full-res pixel buffers never do.
//!
//! Documents live in a registry keyed by [`DocumentId`]; one document is
//! active at a time. Layer/asset ids are process-unique.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Mutex, OnceLock};

use crate::document::{Document, DocumentId};
use crate::renderer::{FramePlan, Renderer};
use crate::uniforms::UNIFORM_FLOATS;

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

/// Frame plan published by `ec_begin_frame`, read back pass-by-pass.
#[derive(Debug, Clone)]
struct PlanPass {
    pipeline: u32,
    texture_id: u64,
    uniforms: [f32; UNIFORM_FLOATS],
}

static PLAN: OnceLock<Mutex<PlanFrame>> = OnceLock::new();

#[derive(Debug, Clone, Default)]
struct PlanFrame {
    clear_rgba: [f32; 4],
    passes: Vec<PlanPass>,
}

fn plan_frame() -> &'static Mutex<PlanFrame> {
    PLAN.get_or_init(|| Mutex::new(PlanFrame::default()))
}

static PERF: OnceLock<Mutex<crate::perf::PerfMonitor>> = OnceLock::new();

fn perf() -> &'static Mutex<crate::perf::PerfMonitor> {
    PERF.get_or_init(|| Mutex::new(crate::perf::PerfMonitor::new()))
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

fn err_ptr(e: String) -> *mut c_char {
    ok_json(&serde_json::json!({ "error": e }))
}

/// Caller must release with `ec_free_string`.
#[no_mangle]
pub extern "C" fn ec_free_string(p: *mut c_char) {
    if !p.is_null() { unsafe { let _ = CString::from_raw(p); } }
}

// -- document lifecycle ------------------------------------------------------

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

// -- state mutations -----------------------------------------------------------

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
    let asset_id = src.asset_id;
    let res = with_active(|d| {
        let layer_id = d.add_image(src);
        serde_json::json!({ "layer_id": layer_id, "asset_id": asset_id })
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

// -- frame plan (read by the native encoder each frame) ------------------------

/// Rebuild the frame plan for the active document and return the pass count.
#[no_mangle]
pub extern "C" fn ec_begin_frame(viewport_w: u32, viewport_h: u32) -> u32 {
    let built = with_active(|d| Renderer::build_frame_plan(d, viewport_w, viewport_h));
    let Some(FramePlan { clear_rgba, passes }) = built else { return 0 };
    let mut pf = plan_frame().lock().unwrap();
    pf.clear_rgba = clear_rgba;
    pf.passes = passes.into_iter().map(|p| PlanPass {
        pipeline: p.pipeline.id(),
        texture_id: p.texture_id,
        uniforms: p.uniforms,
    }).collect();
    pf.passes.len() as u32
}

#[no_mangle]
pub extern "C" fn ec_pass_kind(index: u32) -> u32 {
    plan_frame().lock().unwrap().passes.get(index as usize).map(|p| p.pipeline).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn ec_pass_texture(index: u32) -> u64 {
    plan_frame().lock().unwrap().passes.get(index as usize).map(|p| p.texture_id).unwrap_or(0)
}

/// Write the packed uniform floats for `index` into `out` (up to `len`).
#[no_mangle]
pub extern "C" fn ec_pass_uniforms(index: u32, out: *mut f32, len: u32) {
    if out.is_null() || len == 0 { return; }
    let n = (len as usize).min(UNIFORM_FLOATS);
    let frame = plan_frame().lock().unwrap();
    if let Some(p) = frame.passes.get(index as usize) {
        unsafe { std::ptr::copy_nonoverlapping(p.uniforms.as_ptr(), out, n); }
    }
}

#[no_mangle]
pub extern "C" fn ec_clear_rgba(out: *mut f32) {
    if out.is_null() { return; }
    let frame = plan_frame().lock().unwrap();
    unsafe { std::ptr::copy_nonoverlapping(frame.clear_rgba.as_ptr(), out, 4); }
}

// -- persistence ---------------------------------------------------------------

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

// -- performance instrumentation -------------------------------------------------

/// Feed one frame's measurements (GPU ms from the command-buffer completion
/// handler, CPU ms from the encode span, pass count).
#[no_mangle]
pub extern "C" fn ec_perf_frame(gpu_ms: f32, cpu_ms: f32, passes: u32) {
    let mut p = perf().lock().unwrap();
    p.begin_frame();
    p.end_frame(gpu_ms, cpu_ms, passes as u64);
}

/// Developer overlay text (FPS / frame / GPU / CPU / passes).
#[no_mangle]
pub extern "C" fn ec_perf_text() -> *mut c_char {
    let text = perf().lock().unwrap().overlay_text();
    match CString::new(text) {
        Ok(c) => c.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
