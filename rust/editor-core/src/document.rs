use serde::{Deserialize, Serialize};
use crate::{
    drawing::Stroke,
    effects::{ColorAdjust, ParamOp},
    history::History,
    image_source::ImageSource,
    layer::{BlendMode, Layer, LayerId},
    mask::LayerMask,
    text::TextStyle,
    transform::Transform,
};

pub type DocumentId = u64;

/// Non-destructive document. Originals are immutable; all edits are params.
#[derive(Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub width: u32,
    pub height: u32,
    layers: Vec<Layer>,
    pub assets: Vec<ImageSource>,
    #[serde(skip)]
    pub history: History,
}

impl Document {
    pub fn new(width: u32, height: u32) -> Self {
        Self { id: crate::next_id(), width, height, layers: vec![], assets: vec![], history: History::new() }
    }

    // -- spec API -----------------------------------------------------------
    pub fn add_image(&mut self, asset: ImageSource) -> LayerId {
        let asset_id = asset.asset_id;
        self.assets.push(asset);
        let l = Layer::image(asset_id);
        let id = l.id;
        self.layers.push(l);
        id
    }

    pub fn add_text(&mut self, style: TextStyle, transform: Transform) -> LayerId {
        let l = Layer { id: Layer::next_id(), kind: crate::layer::LayerKind::Text(style), transform, opacity: 1.0, blend: BlendMode::Normal, visible: true, crop: None, mask: None, color: ColorAdjust::default() };
        let id = l.id;
        self.layers.push(l);
        id
    }

    pub fn add_sticker(&mut self, asset_id: u64, transform: Transform) -> LayerId {
        let l = Layer { id: Layer::next_id(), kind: crate::layer::LayerKind::Sticker { asset: asset_id }, transform, opacity: 1.0, blend: BlendMode::Normal, visible: true, crop: None, mask: None, color: ColorAdjust::default() };
        let id = l.id;
        self.layers.push(l);
        id
    }

    pub fn add_drawing_layer(&mut self) -> LayerId {
        let l = Layer { id: Layer::next_id(), kind: crate::layer::LayerKind::Drawing { stroke_doc: Default::default() }, transform: Transform::default(), opacity: 1.0, blend: BlendMode::Normal, visible: true, crop: None, mask: None, color: ColorAdjust::default() };
        let id = l.id;
        self.layers.push(l);
        id
    }

    pub fn remove_layer(&mut self, id: LayerId) -> bool {
        if let Some(i) = self.layers.iter().position(|l| l.id == id) {
            let l = self.layers.remove(i);
            self.history.push("remove_layer", None, ParamOp::RemoveLayer { index: i, layer: l, present: false });
            true
        } else { false }
    }

    pub fn move_layer(&mut self, id: LayerId, to_index: usize) -> bool {
        let Some(from) = self.layers.iter().position(|l| l.id == id) else { return false };
        let l = self.layers.remove(from);
        let to = to_index.min(self.layers.len());
        self.layers.insert(to, l);
        true
    }

    pub fn layer_order(&self) -> Vec<LayerId> { self.layers.iter().map(|l| l.id).collect() }

    pub fn set_layer_order(&mut self, order: &[LayerId], gesture: Option<String>) {
        let before = self.layer_order();
        let mut next = vec![];
        for id in order {
            if let Some(l) = self.layers.iter().find(|l| &l.id == id).cloned() { next.push(l); }
        }
        for l in self.layers.iter() {
            if !order.contains(&l.id) { next.push(l.clone()); }
        }
        self.layers = next;
        let after = self.layer_order();
        self.history.push("reorder", gesture, ParamOp::MoveLayer { before, after });
    }

    pub fn layer(&self, id: LayerId) -> Option<&Layer> { self.layers.iter().find(|l| l.id == id) }
    pub fn layer_mut(&mut self, id: LayerId) -> Option<&mut Layer> { self.layers.iter_mut().find(|l| l.id == id) }
    pub fn layers_in_order(&self) -> Vec<&Layer> { self.layers.iter().collect() }
    pub fn layer_count(&self) -> usize { self.layers.len() }

    pub fn set_transform(&mut self, id: LayerId, t: Transform, gesture: Option<String>) {
        if let Some(l) = self.layer_mut(id) {
            let before = l.transform;
            l.transform = t;
            self.history.push("transform", gesture, ParamOp::SetTransform { layer: id, before, after: t });
        }
    }

    pub fn set_opacity(&mut self, id: LayerId, opacity: f32, gesture: Option<String>) {
        let (before, after) = match self.layer_mut(id) {
            Some(l) => {
                let before = l.opacity;
                l.opacity = opacity.clamp(0.0, 1.0);
                (before, l.opacity)
            }
            None => return,
        };
        self.history.push("opacity", gesture, ParamOp::SetOpacity { layer: id, before, after });
    }

    pub fn set_blend_mode(&mut self, id: LayerId, blend: BlendMode) {
        if let Some(l) = self.layer_mut(id) {
            let before = l.blend;
            l.blend = blend;
            self.history.push("blend", None, ParamOp::SetBlend { layer: id, before, after: blend });
        }
    }

    /// Whole-block color update from a slider panel (coalesced by gesture id).
    pub fn set_filter(&mut self, id: LayerId, color: ColorAdjust, gesture: Option<String>) {
        if let Some(l) = self.layer_mut(id) {
            let before = l.color.clone();
            l.color = color.clone();
            self.history.push("filter", gesture, ParamOp::SetColor { layer: id, before, after: color });
        }
    }

    /// Setting a mask is undoable (spec: AI/brush masks must be removable).
    pub fn set_mask(&mut self, id: LayerId, mask: Option<LayerMask>) {
        let (before, after) = match self.layer_mut(id) {
            Some(l) => {
                if l.mask == mask { return; }
                let before = l.mask.clone();
                l.mask = mask.clone();
                (before, mask)
            }
            None => return,
        };
        self.history.push("mask", None, ParamOp::SetMask { layer: id, before, after });
    }

    pub fn set_visible(&mut self, id: LayerId, visible: bool) {
        if let Some(l) = self.layer_mut(id) {
            let before = l.visible;
            l.visible = visible;
            self.history.push("visibility", None, ParamOp::SetVisible { layer: id, before, after: visible });
        }
    }

    /// Setting a crop is undoable.
    pub fn set_crop(&mut self, id: LayerId, crop: Option<crate::layer::CropRect>) {
        let (before, after) = match self.layer_mut(id) {
            Some(l) => {
                if l.crop == crop { return; }
                let before = l.crop.clone();
                l.crop = crop.clone();
                (before, crop)
            }
            None => return,
        };
        self.history.push("crop", None, ParamOp::SetCrop { layer: id, before, after });
    }

    pub fn push_stroke(&mut self, id: LayerId, stroke: Stroke) -> bool {
        if let Some(l) = self.layer_mut(id) {
            if let crate::layer::LayerKind::Drawing { stroke_doc } = &mut l.kind {
                stroke_doc.add_stroke(stroke);
                return true;
            }
        }
        false
    }

    // -- undo/redo ------------------------------------------------------------
    pub fn undo(&mut self) -> bool {
        let Some(op) = self.history.undo() else { return false };
        self.apply_param(&op);
        true
    }
    pub fn redo(&mut self) -> bool {
        let Some(op) = self.history.redo() else { return false };
        self.apply_param(&op);
        true
    }
    pub fn end_gesture(&mut self) { self.history.end_gesture(); }

    fn apply_param(&mut self, op: &ParamOp) {
        match op.clone() {
            ParamOp::SetTransform { layer, after, .. } => { if let Some(l) = self.layer_mut(layer) { l.transform = after; } }
            ParamOp::SetOpacity { layer, after, .. } => { if let Some(l) = self.layer_mut(layer) { l.opacity = after; } }
            ParamOp::SetBlend { layer, after, .. } => { if let Some(l) = self.layer_mut(layer) { l.blend = after; } }
            ParamOp::SetColor { layer, after, .. } => { if let Some(l) = self.layer_mut(layer) { l.color = after; } }
            ParamOp::SetMask { layer, after, .. } => { if let Some(l) = self.layer_mut(layer) { l.mask = after; } }
            ParamOp::SetCrop { layer, after, .. } => { if let Some(l) = self.layer_mut(layer) { l.crop = after; } }
            ParamOp::SetVisible { layer, after, .. } => { if let Some(l) = self.layer_mut(layer) { l.visible = after; } }
            ParamOp::RemoveLayer { index, layer, present } => {
                if present {
                    let i = index.min(self.layers.len());
                    self.layers.insert(i, layer);
                } else {
                    self.layers.retain(|l| l.id != layer.id);
                }
            }
            ParamOp::MoveLayer { after, .. } => {
                let mut next = vec![];
                for id in after {
                    if let Some(l) = self.layers.iter().find(|l| l.id == id).cloned() { next.push(l); }
                }
                if next.len() == self.layers.len() { self.layers = next; }
            }
        }
    }

    // -- persistence ------------------------------------------------------------
    pub fn save_document(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }
    pub fn load_document(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }

    // -- render entry points ------------------------------------------------------
    // Frame plans are built by `crate::renderer::Renderer::build_frame_plan`;
    // the native side encodes them. Export planning stays here for callers.

    pub fn render_export_plan(&self) -> crate::graph::RenderPlan {
        crate::graph::EffectGraph::from_layers(self.layers.iter()).plan()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crud_and_undo() {
        let mut d = Document::new(4000, 3000);
        let a = ImageSource { asset_id: 1, uri: "a.jpg".into(), width: 4000, height: 3000, format: crate::image_source::ImageFormat::Jpeg, tile_px: 512 };
        let id = d.add_image(a);
        d.set_opacity(id, 0.5, None);
        assert_eq!(d.layer(id).unwrap().opacity, 0.5);
        assert!(d.undo());
        assert_eq!(d.layer(id).unwrap().opacity, 1.0);
        assert!(d.redo());
        assert_eq!(d.layer(id).unwrap().opacity, 0.5);
    }
    #[test]
    fn save_load_roundtrip() {
        let mut d = Document::new(100, 100);
        d.add_text(TextStyle::default(), Transform::default());
        let json = d.save_document().unwrap();
        let d2 = Document::load_document(&json).unwrap();
        assert_eq!(d2.layer_count(), 1);
    }

    #[test]
    fn mask_undo_redo() {
        let mut d = Document::new(100, 100);
        let id = d.add_text(TextStyle::default(), Transform::default());
        assert!(d.layer(id).unwrap().mask.is_none());
        d.set_mask(id, Some(LayerMask::new("ai-mask-1")));
        assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().texture_key, "ai-mask-1");
        assert!(d.undo());
        assert!(d.layer(id).unwrap().mask.is_none(), "undo must remove the mask");
        assert!(d.redo());
        assert!(d.layer(id).unwrap().mask.is_some(), "redo must restore the mask");
    }

    #[test]
    fn crop_undo_redo() {
        let mut d = Document::new(100, 100);
        let id = d.add_image(ImageSource { asset_id: 1, uri: "a.jpg".into(), width: 100, height: 100, format: crate::image_source::ImageFormat::Jpeg, tile_px: 512 });
        let crop = crate::layer::CropRect { x: 0.1, y: 0.1, w: 0.5, h: 0.5 };
        d.set_crop(id, Some(crop));
        assert!(d.layer(id).unwrap().crop.is_some());
        assert!(d.undo());
        assert!(d.layer(id).unwrap().crop.is_none(), "undo must clear the crop");
        assert!(d.redo());
        assert!(d.layer(id).unwrap().crop.is_some());
    }

    #[test]
    fn remove_layer_undo_redo() {
        let mut d = Document::new(100, 100);
        let a = d.add_text(TextStyle::default(), Transform::default());
        let b = d.add_text(TextStyle::default(), Transform::default());
        assert!(d.remove_layer(a));
        assert!(d.layer(a).is_none());
        assert_eq!(d.layer_count(), 1);
        assert!(d.undo());
        assert!(d.layer(a).is_some(), "undo must restore the removed layer");
        // Restored at its original index (0), before the surviving layer.
        assert_eq!(d.layer_order(), vec![a, b]);
        assert!(d.redo());
        assert!(d.layer(a).is_none(), "redo must remove it again");
        assert_eq!(d.layer_order(), vec![b]);
    }
}
