use serde::{Deserialize, Serialize};

/// 2D affine transform in layer-local space. Column-vector convention:
/// world = T(translation) * R(rotation) * S(scale) * p.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub translate: [f32; 2],
    pub scale: [f32; 2],
    pub rotation_rad: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self { translate: [0.0, 0.0], scale: [1.0, 1.0], rotation_rad: 0.0 }
    }
}

impl Transform {
    pub fn new(translate: [f32; 2], scale: [f32; 2], rotation_rad: f32) -> Self {
        Self { translate, scale, rotation_rad }
    }

    /// Row-major 3x3 matrix for uploading as a shader uniform.
    pub fn to_mat3(&self) -> [f32; 9] {
        let (s, c) = self.rotation_rad.sin_cos();
        let (sx, sy) = (self.scale[0], self.scale[1]);
        let [tx, ty] = self.translate;
        // R*S then translation in last column
        [c * sx, -s * sy, tx, s * sx, c * sy, ty, 0.0, 0.0, 1.0]
    }

    /// Rows of the *inverse* mapping: screen uv → layer uv. This is what a
    /// fragment shader needs (given a screen pixel, which layer texel shows
    /// there). Returns 3 rows of 3 floats, padded for vec4 uniform packing.
    pub fn uv_from_screen(&self) -> [[f32; 3]; 3] {
        let (s, c) = self.rotation_rad.sin_cos();
        let (sx, sy) = (self.scale[0], self.scale[1]);
        let [tx, ty] = self.translate;
        // Forward is A = R*S (2x2), t. Inverse: A⁻¹ (p - t).
        let det = sx * sy; // R is orthonormal, so det(A) = sx*sy
        debug_assert!(det.abs() > f32::EPSILON, "degenerate layer scale");
        let det = if det.abs() > f32::EPSILON { det } else { 1.0 };
        // A = [[c*sx, -s*sy], [s*sx, c*sy]] ⇒ A⁻¹ = 1/det [[c*sy, s*sy], [-s*sx, c*sx]]
        let a00 = c * sy / det;
        let a01 = s * sy / det;
        let a10 = -s * sx / det;
        let a11 = c * sx / det;
        // p' = A⁻¹ p - A⁻¹ t
        let itx = -(a00 * tx + a01 * ty);
        let ity = -(a10 * tx + a11 * ty);
        [[a00, a01, itx], [a10, a11, ity], [0.0, 0.0, 1.0]]
    }
}
