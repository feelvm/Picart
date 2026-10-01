//! Shared Rust editing engine.
//!
//! Owns document state, layers, effect graph, undo/redo, caches,
//! preview/export orchestration and perf counters. Native UI (Swift/Kotlin)
//! is a thin controller that calls into this crate via [`ffi`].
//!
//! GPU rule: the engine never touches pixels. It emits *frame plans* — typed
//! pass lists with packed uniform floats ([`renderer`], [`uniforms`]) — and
//! the platform's native GPU API (Metal on iOS, Vulkan on Android) encodes
//! and executes them. Shader sources live in `ios/Sources/Shaders` (MSL).

pub mod bg_remove;
pub mod cache;
pub mod document;
pub mod drawing;
pub mod effects;
pub mod ffi;
pub mod graph;
pub mod history;
pub mod image_source;
pub mod layer;
pub mod mask;
pub mod perf;
pub mod preview;
pub mod renderer;
pub mod text;
pub mod threading;
pub mod transform;
pub mod uniforms;

pub use document::{Document, DocumentId};
pub use effects::{ColorAdjust, Effect};
pub use layer::{BlendMode, Layer, LayerId, LayerKind};
pub use transform::Transform;

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}
