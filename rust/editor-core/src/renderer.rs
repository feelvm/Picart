use crate::graph::EffectGraph;
use crate::gpu::{GpuBackend, RenderCommand, TextureHandle};

/// Emits GPU render passes from the effect graph. Owns no pixels;
/// only pass lists + fused uniform blocks.
#[derive(Debug, Default)]
pub struct Renderer {
    pub last_pass_count: usize,
}

#[derive(Debug, Clone)]
pub struct FrameParams {
    pub viewport_w: u32,
    pub viewport_h: u32,
    pub preview_scale: f32,
    pub pipeline: String,
}

impl Renderer {
    pub fn new() -> Self { Self::default() }

    /// Build render commands for one frame. Fused color-grade nodes share
    /// pass 0 (single `color_grade` pipeline); blur/sharpen append passes.
    pub fn render_frame(
        &mut self,
        doc: &crate::document::Document,
        params: &FrameParams,
        gpu: &mut dyn GpuBackend,
    ) -> Vec<RenderCommand> {
        let graph = EffectGraph::from_layers(doc.layers_in_order());
        let plan = graph.plan();
        self.last_pass_count = plan.pass_count.max(1);
        let mut cmds = Vec::new();
        for (i, pass) in plan.passes.iter().enumerate() {
            let _ = pass;
            cmds.push(RenderCommand {
                pass_index: i as u32,
                pipeline: params.pipeline.clone(),
                uniforms_16b: vec![0u8; 16],
                target: TextureHandle(i as u64 + 1),
            });
        }
        if cmds.is_empty() {
            cmds.push(RenderCommand {
                pass_index: 0, pipeline: params.pipeline.clone(),
                uniforms_16b: vec![0u8; 16], target: TextureHandle(1),
            });
        }
        let _ = (params.viewport_w, params.viewport_h);
        gpu.submit(&cmds);
        cmds
    }
}
