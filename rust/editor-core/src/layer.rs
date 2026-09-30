use serde::{Deserialize, Serialize};
use crate::{effects::ColorAdjust, next_id, transform::Transform};

pub type LayerId = u64;
pub type AssetId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    Normal, Multiply, Screen, Overlay, SoftLight, HardLight,
    Darken, Lighten, Add, Difference,
}

impl BlendMode {
    pub fn all() -> &'static [BlendMode] {
        use BlendMode::*;
        &[Normal, Multiply, Screen, Overlay, SoftLight, HardLight, Darken, Lighten, Add, Difference]
    }
    /// GLSL/Metal blend fn name — renderer maps 1:1 so new modes don't
    /// require renderer rewrites, just a shader fn.
    pub fn shader_fn(&self) -> &'static str {
        match self {
            BlendMode::Normal => "blend_normal",
            BlendMode::Multiply => "blend_multiply",
            BlendMode::Screen => "blend_screen",
            BlendMode::Overlay => "blend_overlay",
            BlendMode::SoftLight => "blend_soft_light",
            BlendMode::HardLight => "blend_hard_light",
            BlendMode::Darken => "blend_darken",
            BlendMode::Lighten => "blend_lighten",
            BlendMode::Add => "blend_add",
            BlendMode::Difference => "blend_difference",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayerKind {
    Image { asset: AssetId },
    Text(crate::text::TextStyle),
    Sticker { asset: AssetId },
    Drawing { stroke_doc: crate::drawing::StrokeDoc },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CropRect {
    /// Normalized 0..1 in source space.
    pub x: f32, pub y: f32, pub w: f32, pub h: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub kind: LayerKind,
    pub transform: Transform,
    pub opacity: f32,
    pub blend: BlendMode,
    pub visible: bool,
    pub crop: Option<CropRect>,
    pub mask: Option<crate::mask::LayerMask>,
    pub color: ColorAdjust,
}

impl Layer {
    pub fn image(asset: AssetId) -> Self {
        Self {
            id: crate::next_id(), kind: LayerKind::Image { asset },
            transform: Transform::default(), opacity: 1.0,
            blend: BlendMode::Normal, visible: true, crop: None,
            mask: None, color: ColorAdjust::default(),
        }
    }
    pub fn next_id() -> LayerId { next_id() }
}
