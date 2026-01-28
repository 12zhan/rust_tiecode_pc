use std::ops::Range;
use ropey::Rope;
use super::completion::CompletionItem;
use super::undo::{UndoHistory, EditOperation};

pub struct EditorCore {
    pub content: Rope,
    pub selected_range: Range<usize>,
    pub selection_anchor: usize,
    pub marked_range: Option<Range<usize>>,
    pub preferred_column: Option<usize>,
    pub completion_active: bool,
    pub completion_items: Vec<CompletionItem>,
    pub completion_index: usize,
    pub history: UndoHistory,
}

impl EditorCore {
    pub fn new() -> Self {
        Self {
            content: Rope::new(),
            selected_range: 0..0,
            selection_anchor: 0,
            marked_range: None,
            preferred_column: None,
            completion_active: false,
            completion_items: Vec::new(),
            completion_index: 0,
            history: UndoHistory::new(),
        }
    }

    pub fn set_cursor(&mut self, index: usize) {
        self.selected_range = index..index;
        self.selection_anchor = index;
        self.marked_range = None;
        self.preferred_column = None;
    }

    pub fn select_to(&mut self, index: usize) {
        let start = self.selection_anchor.min(index);
        let end = self.selection_anchor.max(index);
        self.selected_range = start..end;
        self.marked_range = None;
        self.preferred_column = None;
    }

    pub fn replace_range(&mut self, range: Range<usize>, text: &str) {
        self.history.begin_transaction();
        
        let start_char_idx = self.content.byte_to_char(range.start);
        let end_char_idx = self.content.byte_to_char(range.end);
        
        // Remove text if range is not empty
        if start_char_idx < end_char_idx {
             let deleted_text = self.content.slice(start_char_idx..end_char_idx).to_string();
             self.content.remove(start_char_idx..end_char_idx);
             self.history.push(EditOperation::Delete { 
                 range: range.clone(), 
                 text: deleted_text 
             });
        }
        
        // Insert new text
        if !text.is_empty() {
            self.content.insert(start_char_idx, text);
            self.history.push(EditOperation::Insert { 
                range: range.start..range.start + text.len(), 
                text: text.to_string() 
            });
        }
        
        // Update cursor to end of inserted text
        let new_cursor_byte = range.start + text.len();
        self.selected_range = new_cursor_byte..new_cursor_byte;
        self.selection_anchor = new_cursor_byte;
        self.marked_range = None;
        self.preferred_column = None;
        
        self.history.end_transaction();
    }

    pub fn insert_text(&mut self, text: &str) {
        self.replace_range(self.selected_range.clone(), text);
    }

    pub fn delete_range(&mut self, range: Range<usize>) {
        self.replace_range(range, "");
    }
    
    pub fn undo(&mut self) {
        if let Some(ops) = self.history.undo() {
            for op in ops {
                self.apply_op(op);
            }
        }
    }

    pub fn redo(&mut self) {
        if let Some(ops) = self.history.redo() {
            for op in ops {
                self.apply_op(op);
            }
        }
    }

    fn apply_op(&mut self, op: EditOperation) {
        match op {
            EditOperation::Insert { range, text } => {
                let start_char_idx = self.content.byte_to_char(range.start);
                self.content.insert(start_char_idx, &text);
                // Update cursor to end of insertion
                self.selected_range = range.end..range.end;
                self.selection_anchor = range.end;
            },
            EditOperation::Delete { range, .. } => {
                 let start_char_idx = self.content.byte_to_char(range.start);
                 let end_char_idx = self.content.byte_to_char(range.end);
                 self.content.remove(start_char_idx..end_char_idx);
                 // Update cursor to start of deletion
                 self.selected_range = range.start..range.start;
                 self.selection_anchor = range.start;
            }
        }
    }
}
