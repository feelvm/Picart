//! Packed uniform blocks shared with the Metal shaders
//! (`ios/Sources/Shaders/ColorGrade.metal`).
//!
//! The Rust core packs floats; the native encoder uploads the bytes as-is.
//! Layout is 7 × float4 = 112 bytes (28 floats), all vec4-aligned so the MSL
//! struct and this packing stay trivially in sync:
//!
//! ```text
//!  0.. 4  params0  brightness, contrast, saturation, exposure
//!  4.. 8  params1  temperature, tint, hue_shift, opacity
//!  8..12  params2  vignette, grain, lut_strength, mask_feather
//! 12..16  params3  viewport_w, viewport_h, time_s, blend_mode
//! 16..28  m0, m1, m2  screen-uv → layer-uv affine matrix rows (w unused)
//! ```

use crate::effects::ColorAdjust;

pub const UNIFORM_FLOATS: usize = 28;

#[derive(Debug, Clone)]
pub struct PassUniforms {
    pub grade: ColorAdjust,
    pub opacity: f32,
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub time_s: f32,
    pub blend_mode: f32,
    /// Rows of the affine matrix mapping screen uv → layer uv
    /// (inverse of the layer transform).
    pub uv_from_screen: [[f32; 3]; 3],
}

impl PassUniforms {
    pub fn pack(&self) -> [f32; UNIFORM_FLOATS] {
        let g = &self.grade;
        let mut out = [0f32; UNIFORM_FLOATS];
        out[0] = g.brightness;
        out[1] = g.contrast;
        out[2] = g.saturation;
        out[3] = g.exposure;
        out[4] = g.temperature;
        out[5] = g.tint;
        out[6] = g.hue_shift;
        out[7] = self.opacity;
        out[8] = g.vignette;
        out[9] = g.grain;
        out[10] = 0.0; // lut_strength (reserved)
        out[11] = 0.0; // mask_feather (reserved)
        out[12] = self.viewport_w;
        out[13] = self.viewport_h;
        out[14] = self.time_s;
        out[15] = self.blend_mode;
        for r in 0..3 {
            out[16 + r * 4] = self.uv_from_screen[r][0];
            out[17 + r * 4] = self.uv_from_screen[r][1];
            out[18 + r * 4] = self.uv_from_screen[r][2];
            out[19 + r * 4] = 0.0;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::Transform;

    #[test]
    fn pack_layout_matches_shader() {
        let grade = ColorAdjust { brightness: 0.25, contrast: 1.5, ..Default::default() };
        let u = PassUniforms {
            grade,
            opacity: 0.75,
            viewport_w: 1170.0,
            viewport_h: 2532.0,
            time_s: 1.0,
            blend_mode: 0.0,
            uv_from_screen: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        };
        let p = u.pack();
        assert_eq!(p.len(), UNIFORM_FLOATS);
        assert_eq!(p[0], 0.25); // brightness
        assert_eq!(p[1], 1.5); // contrast
        assert_eq!(p[7], 0.75); // opacity
        assert_eq!(p[12], 1170.0); // viewport_w
        // identity matrix rows land at 16, 20, 24
        assert_eq!(&p[16..28], &[1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    }

    #[test]
    fn identity_transform_maps_screen_to_layer_unchanged() {
        let m = Transform::default().uv_from_screen();
        assert_eq!(m, [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    }

    #[test]
    fn inverse_roundtrips() {
        let t = Transform::new([0.25, -0.1], [1.6, 0.8], 0.7);
        let fwd = t.to_mat3();
        let inv = t.uv_from_screen();
        for (sx, sy) in [(0.1, 0.9), (0.5, 0.5), (0.8, 0.2)] {
            // forward (row-major 3x3), then inverse rows → back to start
            let fx = fwd[0] * sx + fwd[1] * sy + fwd[2];
            let fy = fwd[3] * sx + fwd[4] * sy + fwd[5];
            let rx = inv[0][0] * fx + inv[0][1] * fy + inv[0][2];
            let ry = inv[1][0] * fx + inv[1][1] * fy + inv[1][2];
            assert!((rx - sx).abs() < 1e-4 && (ry - sy).abs() < 1e-4, "({sx},{sy}) != ({rx},{ry})");
        }
    }
}
