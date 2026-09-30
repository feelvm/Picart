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
        let (tx, ty) = (self.translate[0], self.translate[1]);
        // R*S then translation in last column
        [c * sx, -s * sy, tx, s * sx, c * sy, ty, 0.0, 0.0, 1.0]
    }

    /// True when pinch/zoom/rotate gestures can take the fast path
    /// (no re-decode, just a uniform update + re-composite).
    pub fn is_fast_path(&self, prev: &Self) -> bool {
        (self.scale[0] - prev.scale[0]).abs() < 4.0
            && (self.scale[1] - prev.scale[1]).abs() < 4.0
    }
}
