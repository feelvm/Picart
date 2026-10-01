//! Builds the per-frame render plan consumed by the native Metal encoder.
//!
//! The plan is pure data — pass list + packed uniform floats — so no pixel
//! data and no per-frame JSON cross the FFI. Native side (Swift) owns command
//! encoding and presentation; the Rust core owns every rendering *decision*.

use crate::document::Document;
use crate::uniforms::{PassUniforms, UNIFORM_FLOATS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pipeline {
    /// Fused color grade + mask + opacity, one fragment pass
    /// (`fs_color_grade` in ColorGrade.metal).
    ColorGrade = 0,
}

impl Pipeline {
    pub fn id(&self) -> u32 { *self as u32 }
}

#[derive(Debug, Clone)]
pub struct FramePass {
    pub pipeline: Pipeline,
    /// Rust asset id; the native side maps id → GPU texture.
    pub texture_id: u64,
    pub uniforms: [f32; UNIFORM_FLOATS],
}

#[derive(Debug, Clone, Default)]
pub struct FramePlan {
    pub clear_rgba: [f32; 4],
    pub passes: Vec<FramePass>,
}

pub struct Renderer;

impl Renderer {
    /// Build the plan for one frame from the active document.
    ///
    /// v1 (Phase 1/2): one fused ColorGrade pass per visible image layer,
    /// back-to-front. Blur/sharpen passes and multi-layer blend passes extend
    /// this list as later phases land.
    pub fn build_frame_plan(doc: &Document, viewport_w: u32, viewport_h: u32) -> FramePlan {
        let mut plan = FramePlan {
            clear_rgba: [0.08, 0.08, 0.09, 1.0],
            passes: Vec::new(),
        };
        for layer in doc.layers_in_order() {
            let asset = match &layer.kind {
                crate::layer::LayerKind::Image { asset } => *asset,
                _ => continue,
            };
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }
            let uniforms = PassUniforms {
                grade: layer.color.clone(),
                opacity: layer.opacity,
                viewport_w: viewport_w as f32,
                viewport_h: viewport_h as f32,
                time_s: 0.0,
                blend_mode: 0.0, // Normal
                uv_from_screen: layer.transform.uv_from_screen(),
            };
            plan.passes.push(FramePass {
                pipeline: Pipeline::ColorGrade,
                texture_id: asset,
                uniforms: uniforms.pack(),
            });
        }
        plan
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_source::ImageSource;
    use crate::transform::Transform;

    fn doc_with_image() -> (Document, u64) {
        let mut d = Document::new(100, 100);
        let asset_id = crate::next_id();
        d.add_image(ImageSource {
            asset_id, uri: "test.jpg".into(), width: 100, height: 100,
            format: crate::image_source::ImageFormat::Jpeg, tile_px: 512,
        });
        (d, asset_id)
    }

    #[test]
    fn plan_has_one_pass_per_visible_layer() {
        let (d, asset) = doc_with_image();
        let plan = Renderer::build_frame_plan(&d, 800, 600);
        assert_eq!(plan.passes.len(), 1);
        assert_eq!(plan.passes[0].pipeline, Pipeline::ColorGrade);
        assert_eq!(plan.passes[0].texture_id, asset);
        assert_eq!(plan.passes[0].uniforms.len(), UNIFORM_FLOATS);
    }

    #[test]
    fn invisible_layers_are_skipped() {
        let (mut d, _) = doc_with_image();
        let id = d.layer_order()[0];
        d.set_visible(id, false);
        // set_visible pushed history but didn't undo the visibility
        let plan = Renderer::build_frame_plan(&d, 800, 600);
        assert!(plan.passes.is_empty());
    }

    #[test]
    fn uniform_reflects_layer_state() {
        let (mut d, _) = doc_with_image();
        let id = d.layer_order()[0];
        let mut g = crate::effects::ColorAdjust::default();
        g.brightness = 0.3;
        d.set_filter(id, g.clone(), None);
        let t = Transform::new([0.1, 0.2], [2.0, 2.0], 0.0);
        d.set_transform(id, t, None);
        let plan = Renderer::build_frame_plan(&d, 1000, 500);
        let u = &plan.passes[0].uniforms;
        assert_eq!(u[0], 0.3); // brightness
        assert_eq!(u[12], 1000.0); // viewport_w
        assert_eq!(u[13], 500.0); // viewport_h
        // 2x scale ⇒ inverse matrix scales by 0.5
        assert!((u[16] - 0.5).abs() < 1e-6);
    }
}
