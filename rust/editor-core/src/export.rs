use serde::{Deserialize, Serialize};

/// Export runs on the IO queue at full resolution, never blocking UI/render.
/// Preview and export are separate pipelines by design.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub doc_id: crate::document::DocumentId,
    pub format: ExportFormat,
    pub quality: u8,       // JPEG/WebP 0..100
    pub scale: f32,        // 1.0 = original
    pub output_uri: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat { Jpeg, Png, WebP, Heif }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportStatus { Queued, Rendering, Encoding, Done, Failed(String) }

impl ExportRequest {
    pub fn validate(&self) -> Result<(), String> {
        if !(0.05..=1.0).contains(&self.scale) { return Err("scale must be 0.05..=1.0".into()); }
        if self.quality > 100 { return Err("quality must be 0..=100".into()); }
        if self.output_uri.is_empty() { return Err("output_uri required".into()); }
        Ok(())
    }
}
