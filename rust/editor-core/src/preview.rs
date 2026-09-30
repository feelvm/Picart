use serde::{Deserialize, Serialize};

/// Adaptive preview: reduced-res texture while interacting, higher quality
/// after release. Resolution chosen from source MP, frame time, effect cost,
/// and device GPU tier — never full 12–50MP during a gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceTier { Low, Mid, High }

#[derive(Debug, Clone, Copy)]
pub struct PreviewRequest {
    pub src_mp: f32,
    pub frame_ms_ema: f32,
    pub effect_cost: u32,
    pub tier: DeviceTier,
    pub interacting: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PreviewDecision {
    /// Long edge in px for the preview texture.
    pub long_edge_px: u32,
    pub high_quality: bool,
}

pub fn decide_preview(r: PreviewRequest) -> PreviewDecision {
    let mut edge: u32 = match r.tier {
        DeviceTier::Low => 1024,
        DeviceTier::Mid => 1536,
        DeviceTier::High => 2048,
    };
    if r.src_mp > 24.0 { edge = (edge as f32 * 0.85) as u32; }
    if r.frame_ms_ema > 20.0 { edge = (edge as f32 * 0.7) as u32; }
    else if r.frame_ms_ema > 16.7 { edge = (edge as f32 * 0.85) as u32; }
    if r.effect_cost > 8 { edge = (edge as f32 * 0.85) as u32; }
    if r.interacting { edge = (edge as f32 * 0.75) as u32; }
    edge = edge.clamp(512, 2048);
    PreviewDecision { long_edge_px: edge, high_quality: !r.interacting }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interaction_reduces_resolution() {
        let base = PreviewRequest { src_mp: 12.0, frame_ms_ema: 10.0, effect_cost: 2, tier: DeviceTier::High, interacting: false };
        let active = PreviewRequest { interacting: true, ..base };
        assert!(decide_preview(active).long_edge_px < decide_preview(base).long_edge_px);
    }
    #[test]
    fn slow_frames_shed_resolution() {
        let fast = PreviewRequest { src_mp: 12.0, frame_ms_ema: 10.0, effect_cost: 2, tier: DeviceTier::Mid, interacting: true };
        let slow = PreviewRequest { frame_ms_ema: 30.0, ..fast };
        assert!(decide_preview(slow).long_edge_px < decide_preview(fast).long_edge_px);
    }
}
