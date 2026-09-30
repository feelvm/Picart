//! Background-removal interface. The ONLY AI feature.
//! Image → model → segmentation mask → GPU texture → layer mask.
//! Runs on the AI queue with platform acceleration (Core ML / NNAPI);
//! result is cached and undoable like any other mask op.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentationRequest {
    pub asset_id: u64,
    pub model: SegmentationModel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentationModel {
    /// Small on-device portrait/subject model (~2-10MB class).
    MobilePortrait,
    GeneralSubject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentationResult {
    pub asset_id: u64,
    /// RLE or PNG bytes of the alpha mask at model resolution.
    pub mask_bytes: Vec<u8>,
    pub mask_w: u32,
    pub mask_h: u32,
    pub inference_ms: u64,
}

pub trait Segmenter: Send {
    fn segment(&self, req: &SegmentationRequest, rgba: &[u8], w: u32, h: u32) -> Result<SegmentationResult, String>;
}

/// Stub used on host; platform code injects Core ML (iOS) / ONNX+NNAPI (Android).
pub struct NullSegmenter;
impl Segmenter for NullSegmenter {
    fn segment(&self, req: &SegmentationRequest, _rgba: &[u8], w: u32, h: u32) -> Result<SegmentationResult, String> {
        Ok(SegmentationResult {
            asset_id: req.asset_id,
            mask_bytes: vec![255u8; (w.min(64) * h.min(64)) as usize],
            mask_w: w.min(64), mask_h: h.min(64), inference_ms: 0,
        })
    }
}
