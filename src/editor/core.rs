use std::ops::Range;
use ropey::Rope;
use super::completion::CompletionItem;

pub struct EditorCore {
    pub content: Rope,
    pub selected_range: Range<usize>,
    pub selection_anchor: usize,
    pub marked_range: Option<Range<usize>>,
    pub preferred_column: Option<usize>,
    pub completion_active: bool,
    pub completion_items: Vec<CompletionItem>,
    pub completion_index: usize,
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

    pub fn insert_text(&mut self, text: &str) {
        let range = self.selected_range.clone();
        
        // Convert byte index to char index for Rope
        let start_char_idx = self.content.byte_to_char(range.start);
        let end_char_idx = self.content.byte_to_char(range.end);
        
        // Remove selected text if any
        if start_char_idx != end_char_idx {
             self.content.remove(start_char_idx..end_char_idx);
        }
        
        // Insert new text
        self.content.insert(start_char_idx, text);
        
        // Calculate new cursor position (byte index)
        let new_cursor_byte = range.start + text.len();
        
        self.selected_range = new_cursor_byte..new_cursor_byte;
        self.selection_anchor = new_cursor_byte;
        self.marked_range = None;
        self.preferred_column = None;
    }

    pub fn delete_range(&mut self, range: Range<usize>) {
        // Convert byte range to char range for Rope
        let start_char_idx = self.content.byte_to_char(range.start);
        let end_char_idx = self.content.byte_to_char(range.end);
        
        self.content.remove(start_char_idx..end_char_idx);
        
        self.selected_range = range.start..range.start;
        self.selection_anchor = range.start;
        self.marked_range = None;
        self.preferred_column = None;
    }
}
