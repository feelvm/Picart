use crate::effects::ParamOp;

/// Command/state-delta history. Slider drags coalesce: 100 updates → 1 entry.
/// Stores param deltas only — never full image copies.
#[derive(Debug, Default)]
pub struct History {
    undo_stack: Vec<CoalescedOp>,
    redo_stack: Vec<CoalescedOp>,
    /// Open gesture id (e.g. active slider drag). While open, updates merge.
    open_gesture: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CoalescedOp {
    pub label: String,
    pub gesture_id: Option<String>,
    pub first: ParamOp,
    pub last: ParamOp,
}

impl History {
    pub fn new() -> Self { Self::default() }

    /// Push an op. If `gesture_id` matches the open gesture and the op targets
    /// the same layer+kind, only `last` is updated (coalescing).
    pub fn push(&mut self, label: impl Into<String>, gesture_id: Option<String>, op: ParamOp) {
        let label = label.into();
        if let (Some(g), Some(open)) = (gesture_id.clone(), self.open_gesture.clone()) {
            if g == open {
                if let Some(top) = self.undo_stack.last_mut() {
                    if top.gesture_id.as_ref() == Some(&g) && same_target(&top.last, &op) {
                        top.last = op;
                        self.redo_stack.clear();
                        return;
                    }
                }
            }
        }
        if let Some(g) = gesture_id.clone() {
            self.open_gesture = Some(g.clone());
        }
        self.undo_stack.push(CoalescedOp { label, gesture_id, first: op.clone(), last: op });
        self.redo_stack.clear();
    }

    pub fn end_gesture(&mut self) { self.open_gesture = None; }
    pub fn undo(&mut self) -> Option<ParamOp> {
        let op = self.undo_stack.pop()?;
        self.redo_stack.push(op.clone());
        // Undo restores `first.before`
        Some(before_of(&op.first))
    }
    pub fn redo(&mut self) -> Option<ParamOp> {
        let op = self.redo_stack.pop()?;
        self.undo_stack.push(op.clone());
        // Redo applies `last.after`
        Some(after_of(&op.last))
    }
    pub fn can_undo(&self) -> bool { !self.undo_stack.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo_stack.is_empty() }
    pub fn undo_len(&self) -> usize { self.undo_stack.len() }
}

fn same_target(a: &ParamOp, b: &ParamOp) -> bool {
    use ParamOp::*;
    match (a, b) {
        (SetColor { layer: x, .. }, SetColor { layer: y, .. })
        | (SetTransform { layer: x, .. }, SetTransform { layer: y, .. })
        | (SetOpacity { layer: x, .. }, SetOpacity { layer: y, .. }) => x == y,
        _ => false,
    }
}

fn before_of(op: &ParamOp) -> ParamOp {
    use ParamOp::*;
    match op.clone() {
        SetColor { layer, before, .. } => {
            let cur = before.clone();
            SetColor { layer, before: cur.clone(), after: cur }
        }
        // Return the op with after := before so the document applies `before`.
        // Document::apply_param interprets Set* as "set to after".
        SetTransform { layer, before, .. } => SetTransform { layer, before: before.clone(), after: before },
        SetOpacity { layer, before, .. } => SetOpacity { layer, before, after: before },
        SetBlend { layer, before, .. } => SetBlend { layer, before: before.clone(), after: before },
        SetVisible { layer, before, .. } => SetVisible { layer, before, after: before },
        MoveLayer { before, .. } => MoveLayer { before: before.clone(), after: before },
    }
}

fn after_of(op: &ParamOp) -> ParamOp { op.clone() }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::ColorAdjust;

    #[test]
    fn slider_coalescing() {
        let mut h = History::new();
        let layer = 1;
        for i in 0..100 {
            h.push("brightness", Some("drag-1".into()), ParamOp::SetColor {
                layer,
                before: ColorAdjust::default(),
                after: ColorAdjust { brightness: i as f32 / 100.0, ..Default::default() },
            });
        }
        assert_eq!(h.undo_len(), 1, "100 slider updates must coalesce to 1 entry");
        assert!(h.can_undo());
        h.end_gesture();
        h.push("brightness", Some("drag-2".into()), ParamOp::SetColor {
            layer, before: ColorAdjust::default(), after: ColorAdjust::default(),
        });
        assert_eq!(h.undo_len(), 2);
    }
}
