use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    pub text: String,
    pub font_family: String,
    pub size_pt: f32,
    pub weight: u16,
    pub alignment: TextAlign,
    pub color_rgba: [f32; 4],
    pub letter_spacing: f32,
    pub line_spacing: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign { Left, Center, Right }

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            text: String::new(), font_family: "System".into(), size_pt: 32.0,
            weight: 600, alignment: TextAlign::Center, color_rgba: [1.0; 4],
            letter_spacing: 0.0, line_spacing: 1.2,
        }
    }
}
