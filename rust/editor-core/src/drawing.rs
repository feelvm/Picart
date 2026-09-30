use serde::{Deserialize, Serialize};

/// Vector stroke storage — never a full-res bitmap per stroke. The GPU
/// incrementally tessellates new segments into a small dynamic texture atlas.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StrokeDoc {
    pub strokes: Vec<Stroke>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub points: Vec<[f32; 2]>,
    pub size_px: f32,
    pub opacity: f32,
    pub hardness: f32,
    pub color_rgba: [f32; 4],
    pub eraser: bool,
    pub smoothing: f32,
}

impl StrokeDoc {
    pub fn add_stroke(&mut self, s: Stroke) { self.strokes.push(s); }
    pub fn clear(&mut self) { self.strokes.clear(); }
}
