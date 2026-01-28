use std::ops::Range;
use super::completion::CompletionItem;

pub struct EditorCore {
    pub content: String,
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
            content: String::new(),
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
        let mut new_content = String::new();
        new_content.push_str(&self.content[0..range.start]);
        new_content.push_str(text);
        new_content.push_str(&self.content[range.end..]);
        let cursor = range.start + text.len();
        self.content = new_content;
        self.selected_range = cursor..cursor;
        self.selection_anchor = cursor;
        self.marked_range = None;
        self.preferred_column = None;
    }

    pub fn delete_range(&mut self, range: Range<usize>) {
        let mut new_content = String::new();
        new_content.push_str(&self.content[0..range.start]);
        new_content.push_str(&self.content[range.end..]);
        self.content = new_content;
        self.selected_range = range.start..range.start;
        self.selection_anchor = range.start;
        self.marked_range = None;
        self.preferred_column = None;
    }
}
