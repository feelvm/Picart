use serde::{Deserialize, Serialize};
use crate::{layer::LayerId, transform::Transform};

/// Non-destructive per-layer color/effect parameters. Changing a slider only
/// mutates these floats — the source image is never rewritten.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorAdjust {
    pub brightness: f32, // -1..1
    pub contrast: f32,   // 0..2, 1 = neutral
    pub saturation: f32, // 0..2
    pub exposure: f32,   // EV stops
    pub temperature: f32, // -1..1
    pub tint: f32,       // -1..1
    pub hue_shift: f32,  // radians
    pub sharpen: f32,    // 0..1
    pub blur_radius: f32, // px at preview scale, 0 = off
    pub vignette: f32,   // 0..1
    pub grain: f32,      // 0..1
}

impl Default for ColorAdjust {
    fn default() -> Self {
        Self {
            brightness: 0.0, contrast: 1.0, saturation: 1.0, exposure: 0.0,
            temperature: 0.0, tint: 0.0, hue_shift: 0.0, sharpen: 0.0,
            blur_radius: 0.0, vignette: 0.0, grain: 0.0,
        }
    }
}

impl ColorAdjust {
    /// All color-grade ops that can fuse into ONE fragment pass (see
    /// shaders/color_grade.wgsl). Blur/sharpen may force extra passes.
    pub fn fusable(&self) -> bool {
        self.blur_radius <= 0.0 && self.sharpen <= 0.001
    }
    pub fn is_identity(&self) -> bool {
        *self == Self::default()
    }
}

/// Single parametric effect node. The [`crate::graph`] decides fusion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    Color(ColorAdjust),
    Blur { radius_px: f32 },
    Sharpen { amount: f32 },
    Vignette { strength: f32 },
    Grain { amount: f32 },
    Lut { lut_id: String, strength: f32 },
    Curves { points_r: Vec<[f32; 2]>, points_g: Vec<[f32; 2]>, points_b: Vec<[f32; 2]> },
}

impl Effect {
    pub fn kind(&self) -> &'static str {
        match self {
            Effect::Color(_) => "color",
            Effect::Blur { .. } => "blur",
            Effect::Sharpen { .. } => "sharpen",
            Effect::Vignette { .. } => "vignette",
            Effect::Grain { .. } => "grain",
            Effect::Lut { .. } => "lut",
            Effect::Curves { .. } => "curves",
        }
    }
    /// Rough cost score used by adaptive preview to shed passes first.
    pub fn cost(&self) -> u8 {
        match self {
            Effect::Color(_) => 1,
            Effect::Vignette { .. } | Effect::Grain { .. } => 1,
            Effect::Curves { .. } | Effect::Lut { .. } => 2,
            Effect::Sharpen { .. } => 3,
            Effect::Blur { radius_px } => 3 + (*radius_px as u8).min(5),
        }
    }
}

/// Command deltas for coalesced undo (never full image copies).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParamOp {
    SetTransform { layer: LayerId, before: Transform, after: Transform },
    SetOpacity { layer: LayerId, before: f32, after: f32 },
    SetBlend { layer: LayerId, before: crate::layer::BlendMode, after: crate::layer::BlendMode },
    SetColor { layer: LayerId, before: ColorAdjust, after: ColorAdjust },
    MoveLayer { before: Vec<LayerId>, after: Vec<LayerId> },
    SetVisible { layer: LayerId, before: bool, after: bool },
}
