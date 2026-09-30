use serde::{Deserialize, Serialize};

/// Masks are GPU textures produced by brush / gradient / AI segmentation.
/// `texture_key` refers to a GPU-resident texture; `feather_px` keeps edges cheap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerMask {
    pub texture_key: String,
    pub feather_px: f32,
    pub inverted: bool,
    /// True when produced by background removal (cached, undoable).
    pub from_ai_segmentation: bool,
}

impl LayerMask {
    pub fn new(key: impl Into<String>) -> Self {
        Self { texture_key: key.into(), feather_px: 0.0, inverted: false, from_ai_segmentation: false }
    }
}
