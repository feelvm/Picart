use serde::{Deserialize, Serialize};

/// Image handle. Decoding is lazy + tiled; the CPU keeps metadata while
/// pixel tiles live in the LRU [`crate::cache`] or as GPU textures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageSource {
    pub asset_id: crate::layer::AssetId,
    pub uri: String,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub tile_px: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat { Jpeg, Png, WebP, Heif }

impl ImageSource {
    pub fn megapixels(&self) -> f32 {
        self.width as f32 * self.height as f32 / 1e6
    }
    pub fn tile_count(&self) -> u32 {
        let t = self.tile_px.max(256);
        ((self.width + t - 1) / t) * ((self.height + t - 1) / t)
    }
}
