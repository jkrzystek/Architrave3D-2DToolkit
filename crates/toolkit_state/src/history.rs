use toolkit_core::ToolkitResult;

use crate::document::Document;

/// A reversible action that can be applied to or undone from a document.
pub trait UndoAction: Send + Sync {
    /// Apply this action to the document (do / redo).
    fn apply(&self, doc: &mut Document) -> ToolkitResult<()>;
    /// Reverse this action from the document.
    fn undo(&self, doc: &mut Document) -> ToolkitResult<()>;
    /// Human-readable description of the action.
    fn description(&self) -> &str;
}

/// A bounded undo/redo history stack.
///
/// When a new action is pushed while the cursor is not at the tip, the
/// redo branch is truncated. When the stack exceeds `max_size`, the oldest
/// entry is dropped.
pub struct HistoryStack {
    actions: Vec<Box<dyn UndoAction>>,
    /// Index of the next action to undo (i.e., the number of applied actions).
    cursor: usize,
    max_size: usize,
}

impl HistoryStack {
    /// Create a new history stack that keeps at most `max_size` entries.
    pub fn new(max_size: usize) -> Self {
        Self {
            actions: Vec::new(),
            cursor: 0,
            max_size,
        }
    }

    /// Push a new action onto the history, applying it to the document.
    ///
    /// If the cursor is not at the tip, the redo branch is truncated first.
    /// If the stack would exceed `max_size`, the oldest entry is removed.
    pub fn push(&mut self, action: Box<dyn UndoAction>) {
        // Truncate redo branch.
        self.actions.truncate(self.cursor);

        self.actions.push(action);
        self.cursor += 1;

        // Enforce capacity.
        if self.actions.len() > self.max_size {
            let excess = self.actions.len() - self.max_size;
            self.actions.drain(0..excess);
            self.cursor = self.cursor.saturating_sub(excess);
        }
    }

    /// Undo the most recent action.
    pub fn undo(&mut self, doc: &mut Document) -> ToolkitResult<()> {
        if !self.can_undo() {
            return Ok(());
        }
        self.cursor -= 1;
        self.actions[self.cursor].undo(doc)
    }

    /// Redo the next action.
    pub fn redo(&mut self, doc: &mut Document) -> ToolkitResult<()> {
        if !self.can_redo() {
            return Ok(());
        }
        let result = self.actions[self.cursor].apply(doc);
        self.cursor += 1;
        result
    }

    /// Whether there is an action to undo.
    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    /// Whether there is an action to redo.
    pub fn can_redo(&self) -> bool {
        self.cursor < self.actions.len()
    }

    /// Description of the action that would be undone, if any.
    pub fn undo_description(&self) -> Option<&str> {
        if self.can_undo() {
            Some(self.actions[self.cursor - 1].description())
        } else {
            None
        }
    }

    /// Description of the action that would be redone, if any.
    pub fn redo_description(&self) -> Option<&str> {
        if self.can_redo() {
            Some(self.actions[self.cursor].description())
        } else {
            None
        }
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.actions.clear();
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use toolkit_core::ToolkitResult;

    /// A trivial action that renames the document.
    struct RenameAction {
        old_name: String,
        new_name: String,
    }

    impl UndoAction for RenameAction {
        fn apply(&self, doc: &mut Document) -> ToolkitResult<()> {
            doc.name = self.new_name.clone();
            Ok(())
        }
        fn undo(&self, doc: &mut Document) -> ToolkitResult<()> {
            doc.name = self.old_name.clone();
            Ok(())
        }
        fn description(&self) -> &str {
            "Rename document"
        }
    }

    /// A trivial action that resizes the document.
    struct ResizeAction {
        old_w: u32,
        old_h: u32,
        new_w: u32,
        new_h: u32,
    }

    impl UndoAction for ResizeAction {
        fn apply(&self, doc: &mut Document) -> ToolkitResult<()> {
            doc.width = self.new_w;
            doc.height = self.new_h;
            Ok(())
        }
        fn undo(&self, doc: &mut Document) -> ToolkitResult<()> {
            doc.width = self.old_w;
            doc.height = self.old_h;
            Ok(())
        }
        fn description(&self) -> &str {
            "Resize document"
        }
    }

    fn make_doc() -> Document {
        Document::new("Test", 100, 100)
    }

    #[test]
    fn empty_history() {
        let h = HistoryStack::new(10);
        assert!(!h.can_undo());
        assert!(!h.can_redo());
        assert!(h.undo_description().is_none());
        assert!(h.redo_description().is_none());
    }

    #[test]
    fn push_and_undo() {
        let mut h = HistoryStack::new(10);
        let mut doc = make_doc();

        h.push(Box::new(RenameAction {
            old_name: "Test".into(),
            new_name: "Renamed".into(),
        }));
        // Manually apply (push doesn't auto-apply in this design)
        doc.name = "Renamed".into();

        assert!(h.can_undo());
        assert!(!h.can_redo());
        assert_eq!(h.undo_description(), Some("Rename document"));

        h.undo(&mut doc).unwrap();
        assert_eq!(doc.name, "Test");
        assert!(!h.can_undo());
        assert!(h.can_redo());
        assert_eq!(h.redo_description(), Some("Rename document"));
    }

    #[test]
    fn redo_reapplies() {
        let mut h = HistoryStack::new(10);
        let mut doc = make_doc();

        h.push(Box::new(RenameAction {
            old_name: "Test".into(),
            new_name: "Renamed".into(),
        }));
        doc.name = "Renamed".into();

        h.undo(&mut doc).unwrap();
        assert_eq!(doc.name, "Test");

        h.redo(&mut doc).unwrap();
        assert_eq!(doc.name, "Renamed");
    }

    #[test]
    fn push_truncates_redo_branch() {
        let mut h = HistoryStack::new(10);
        let mut doc = make_doc();

        // Push two actions
        h.push(Box::new(RenameAction {
            old_name: "Test".into(),
            new_name: "A".into(),
        }));
        doc.name = "A".into();

        h.push(Box::new(RenameAction {
            old_name: "A".into(),
            new_name: "B".into(),
        }));
        doc.name = "B".into();

        // Undo once (back to "A")
        h.undo(&mut doc).unwrap();
        assert_eq!(doc.name, "A");

        // Push a different action -- should discard the redo to "B"
        h.push(Box::new(RenameAction {
            old_name: "A".into(),
            new_name: "C".into(),
        }));
        doc.name = "C".into();

        assert!(!h.can_redo());

        // Undo back to "A"
        h.undo(&mut doc).unwrap();
        assert_eq!(doc.name, "A");

        // Undo back to "Test"
        h.undo(&mut doc).unwrap();
        assert_eq!(doc.name, "Test");

        assert!(!h.can_undo());
    }

    #[test]
    fn max_size_drops_oldest() {
        let mut h = HistoryStack::new(3);
        let mut doc = make_doc();

        for i in 0..5 {
            h.push(Box::new(ResizeAction {
                old_w: doc.width,
                old_h: doc.height,
                new_w: (i + 1) * 10,
                new_h: (i + 1) * 10,
            }));
            doc.width = (i + 1) * 10;
            doc.height = (i + 1) * 10;
        }

        // Only the last 3 actions should remain (resize to 30, 40, 50).
        // Undo 3 times:
        h.undo(&mut doc).unwrap(); // 50 -> 40
        assert_eq!(doc.width, 40);
        h.undo(&mut doc).unwrap(); // 40 -> 30
        assert_eq!(doc.width, 30);
        h.undo(&mut doc).unwrap(); // 30 -> 20
        assert_eq!(doc.width, 20);

        // No more undos.
        assert!(!h.can_undo());
    }

    #[test]
    fn clear_empties_everything() {
        let mut h = HistoryStack::new(10);
        let mut doc = make_doc();

        h.push(Box::new(RenameAction {
            old_name: "Test".into(),
            new_name: "A".into(),
        }));
        doc.name = "A".into();

        h.clear();
        assert!(!h.can_undo());
        assert!(!h.can_redo());
    }

    #[test]
    fn undo_on_empty_is_noop() {
        let mut h = HistoryStack::new(10);
        let mut doc = make_doc();
        // Should not panic or error.
        h.undo(&mut doc).unwrap();
        assert_eq!(doc.name, "Test");
    }

    #[test]
    fn redo_on_empty_is_noop() {
        let mut h = HistoryStack::new(10);
        let mut doc = make_doc();
        h.redo(&mut doc).unwrap();
        assert_eq!(doc.name, "Test");
    }
}
