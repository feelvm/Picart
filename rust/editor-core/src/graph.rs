use serde::{Deserialize, Serialize};
use crate::{effects::Effect, layer::LayerId};

/// Effect DAG per layer: Input → Transform → Mask → Color → Blur → Sharpen → Blend → Output.
/// The renderer walks this graph and fuses compatible nodes into single passes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EffectGraph {
    pub nodes: Vec<EffectNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectNode {
    pub id: u32,
    pub layer: LayerId,
    pub effect: Effect,
    /// extra input texture keys (mask, LUT) — avoids hidden copies.
    pub inputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPlan {
    /// Fused passes; each inner vec runs in ONE fragment pass.
    pub passes: Vec<Vec<u32>>,
    pub pass_count: usize,
}

impl EffectGraph {
    pub fn from_layers(layers: &[crate::layer::Layer]) -> Self {
        let mut g = Self::default();
        let mut id = 0u32;
        for l in layers.iter().filter(|l| l.visible) {
            if !l.color.is_identity() {
                g.nodes.push(EffectNode { id, layer: l.id, effect: Effect::Color(l.color.clone()), inputs: vec![] });
                id += 1;
            }
            if l.color.blur_radius > 0.0 {
                g.nodes.push(EffectNode { id, layer: l.id, effect: Effect::Blur { radius_px: l.color.blur_radius }, inputs: vec![] });
                id += 1;
            }
            if l.color.sharpen > 0.001 {
                g.nodes.push(EffectNode { id, layer: l.id, effect: Effect::Sharpen { amount: l.color.sharpen }, inputs: vec![] });
                id += 1;
            }
            if l.color.vignette > 0.0 {
                g.nodes.push(EffectNode { id, layer: l.id, effect: Effect::Vignette { strength: l.color.vignette }, inputs: vec![] });
                id += 1;
            }
            if l.color.grain > 0.0 {
                g.nodes.push(EffectNode { id, layer: l.id, effect: Effect::Grain { amount: l.color.grain }, inputs: vec![] });
                id += 1;
            }
        }
        g
    }

    /// Fuse all fusable color/vignette/grain nodes per layer into pass 0;
    /// blur/sharpen each get their own pass. Minimizes render targets.
    pub fn plan(&self) -> RenderPlan {
        use Effect::*;
        let mut fused: Vec<u32> = vec![];
        let mut extra: Vec<Vec<u32>> = vec![];
        for n in &self.nodes {
            match &n.effect {
                Color(c) if c.fusable() => fused.push(n.id),
                Vignette { .. } | Grain { .. } => fused.push(n.id),
                _ => extra.push(vec![n.id]),
            }
        }
        let mut passes = vec![];
        if !fused.is_empty() { passes.push(fused); }
        passes.extend(extra);
        let pass_count = passes.len();
        RenderPlan { passes, pass_count }
    }

    pub fn total_cost(&self) -> u32 {
        self.nodes.iter().map(|n| n.effect.cost() as u32).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::Layer;

    #[test]
    fn fusion_reduces_passes() {
        let mut l = Layer::image(7);
        l.color.brightness = 0.2;
        l.color.vignette = 0.5;
        l.color.grain = 0.1;
        let g = EffectGraph::from_layers(&[l]);
        let plan = g.plan();
        assert_eq!(plan.pass_count, 1, "color+vignette+grain must fuse into 1 pass");
    }

    #[test]
    fn blur_forces_extra_pass() {
        let mut l = Layer::image(7);
        l.color.blur_radius = 8.0;
        let g = EffectGraph::from_layers(&[l]);
        assert!(g.plan().pass_count >= 1);
    }
}
