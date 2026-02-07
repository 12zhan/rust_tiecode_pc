use std::ops::Range;
use ropey::Rope;
use super::completion::CompletionItem;
use super::undo::{UndoHistory, EditOperation};

#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
    pub preferred_column: Option<usize>,
}

impl Selection {
    pub fn new(anchor: usize, head: usize) -> Self {
        Self { anchor, head, preferred_column: None }
    }

    pub fn range(&self) -> Range<usize> {
        if self.anchor <= self.head {
            self.anchor..self.head
        } else {
            self.head..self.anchor
        }
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }
}

pub struct EditorCore {
    pub content: Rope,
    pub selections: Vec<Selection>,
    pub marked_range: Option<Range<usize>>,
    pub completion_active: bool,
    pub completion_items: Vec<CompletionItem>,
    pub completion_index: usize,
    pub history: UndoHistory,
}

impl EditorCore {
    pub fn new() -> Self {
        Self {
            content: Rope::new(),
            selections: vec![Selection::new(0, 0)],
            marked_range: None,
            completion_active: false,
            completion_items: Vec::new(),
            completion_index: 0,
            history: UndoHistory::new(),
        }
    }

    pub fn primary_selection(&self) -> Selection {
        self.selections.last().cloned().unwrap_or(Selection::new(0, 0))
    }

    pub fn set_cursor(&mut self, index: usize) {
        self.selections = vec![Selection::new(index, index)];
        self.marked_range = None;
    }

    pub fn add_cursor(&mut self, index: usize) {
        self.selections.push(Selection::new(index, index));
        self.merge_selections();
    }

    pub fn select_to(&mut self, index: usize) {
        if let Some(last) = self.selections.last_mut() {
            last.head = index;
            last.preferred_column = None;
        }
        self.merge_selections();
        self.marked_range = None;
    }

    pub fn select_all(&mut self) {
        let len = self.content.len_bytes();
        self.selections = vec![Selection::new(0, len)];
        self.marked_range = None;
    }

    pub fn merge_selections(&mut self) {
        // Sort by start position
        self.selections.sort_by_key(|s| s.range().start);

        let mut merged = Vec::new();
        if let Some(mut current) = self.selections.first().cloned() {
            for next in self.selections.iter().skip(1) {
                let current_range = current.range();
                let next_range = next.range();

                if current_range.end >= next_range.start {
                    // Overlapping or adjacent, merge
                    // We need to decide anchor/head direction. 
                    // For simplicity, if we merge, we might lose directionality or try to preserve "outer" bounds.
                    // Let's just create a forward selection covering both.
                    let start = current_range.start.min(next_range.start);
                    let end = current_range.end.max(next_range.end);
                    current = Selection::new(start, end);
                } else {
                    merged.push(current);
                    current = next.clone();
                }
            }
            merged.push(current);
        }
        self.selections = merged;
    }

    pub fn apply_edits(&mut self, mut edits: Vec<(Range<usize>, String)>) {
        self.history.begin_transaction();
        // Sort descending by start to avoid offset issues
        edits.sort_by(|a, b| b.0.start.cmp(&a.0.start));
        
        for (range, text) in edits {
            self.replace_range_internal(range, &text);
        }
        self.history.end_transaction();
        self.marked_range = None;
    }

    pub fn replace_selections(&mut self, text: &str) {
        self.merge_selections();
        self.history.begin_transaction();
        
        // Process from bottom to top to preserve indices of earlier selections
        // Sort selections descending by start index
        let mut sorted_indices: Vec<usize> = (0..self.selections.len()).collect();
        sorted_indices.sort_by(|&a, &b| {
            self.selections[b].range().start.cmp(&self.selections[a].range().start)
        });

        for i in 0..sorted_indices.len() {
            let idx = sorted_indices[i];
            let selection = self.selections[idx].clone();
            let range = selection.range();
            
            // Apply edit
            self.replace_range_internal(range.clone(), text);
            
            // Calculate delta
            let old_len = range.end - range.start;
            let new_len = text.len();
            let delta = new_len as isize - old_len as isize;
             
            // Update THIS selection
            let new_pos = range.start + new_len;
            self.selections[idx] = Selection::new(new_pos, new_pos);

            // Update previously processed selections (which are physically AFTER this one)
            if delta != 0 {
                for &prev_idx in &sorted_indices[0..i] {
                    let mut sel = self.selections[prev_idx].clone();
                    // We must use isize for calculation to allow negative delta (deletion)
                    // Ensure we don't underflow usize
                    let new_anchor = (sel.anchor as isize + delta).max(0) as usize;
                    let new_head = (sel.head as isize + delta).max(0) as usize;
                    
                    sel.anchor = new_anchor;
                    sel.head = new_head;
                    self.selections[prev_idx] = sel;
                }
            }
        }
        
        self.history.end_transaction();
        self.marked_range = None;
    }

    // Internal helper that doesn't manage transaction/selection update logic directly (or does it?)
    // Actually `replace_range` in previous code managed history.
    fn replace_range_internal(&mut self, range: Range<usize>, text: &str) {
        let len = self.content.len_bytes();
        let start = range.start.min(len);
        let end = range.end.min(len);

        if start > end {
            eprintln!(
                "Warning: replace_range_internal invalid range: {}..{}",
                start, end
            );
            return;
        }

        let start_char_idx = self.content.byte_to_char(start);
        let end_char_idx = self.content.byte_to_char(end);

        if start_char_idx < end_char_idx {
            let deleted_text = self
                .content
                .slice(start_char_idx..end_char_idx)
                .to_string();
            self.content.remove(start_char_idx..end_char_idx);
            self.history.push(EditOperation::Delete {
                range: start..end,
                text: deleted_text,
            });
        }

        if !text.is_empty() {
            self.content.insert(start_char_idx, text);
            self.history.push(EditOperation::Insert {
                range: start..start + text.len(),
                text: text.to_string(),
            });
        }
    }
    
    // Kept for compatibility/single selection operations if needed
    pub fn replace_range(&mut self, range: Range<usize>, text: &str) {
         self.history.begin_transaction();
         let len = self.content.len_bytes();
         let start = range.start.min(len);
         let end = range.end.min(len);
         self.replace_range_internal(start..end, text);
         
         // Update selections?
         // If we use this method, we assume single selection or manual control.
         // Let's just update the primary selection to match behavior.
         let new_pos = start + text.len();
         self.selections = vec![Selection::new(new_pos, new_pos)];
         self.marked_range = None;
         
         self.history.end_transaction();
    }

    pub fn insert_text(&mut self, text: &str) {
        self.replace_selections(text);
    }

    #[allow(dead_code)]
    pub fn delete_range(&mut self, range: Range<usize>) {
        self.replace_range(range, "");
    }
    
    pub fn delete_selection(&mut self) {
        self.replace_selections("");
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
        let len = self.content.len_bytes();
        match op {
            EditOperation::Insert { range, text } => {
                let start = range.start.min(len);
                let start_char_idx = self.content.byte_to_char(start);
                self.content.insert(start_char_idx, &text);
                // Update cursor
                let new_pos = start + text.len();
                self.selections = vec![Selection::new(new_pos, new_pos)];
            },
            EditOperation::Delete { range, .. } => {
                 let start = range.start.min(len);
                 let end = range.end.min(len);
                 
                 if start < end {
                     let start_char_idx = self.content.byte_to_char(start);
                     let end_char_idx = self.content.byte_to_char(end);
                     self.content.remove(start_char_idx..end_char_idx);
                 }
                 self.selections = vec![Selection::new(start, start)];
            }
        }
    }

    pub fn offset_to_utf16(&self, offset: usize) -> usize {
        let len = self.content.len_bytes();
        if offset > len {
            eprintln!(
                "Warning: offset_to_utf16 out of bounds: {} > {}",
                offset, len
            );
            return self.content.len_utf16_cu();
        }
        let char_idx = self.content.byte_to_char(offset);
        self.content.slice(0..char_idx).len_utf16_cu()
    }

    pub fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        let start = self.offset_to_utf16(range.start);
        let end = self.offset_to_utf16(range.end);
        start..end
    }

    pub fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        let len_utf16 = self.content.len_utf16_cu();
        let start = range_utf16.start.min(len_utf16);
        let end = range_utf16.end.min(len_utf16);

        let start_char = self.content.utf16_cu_to_char(start);
        let end_char = self.content.utf16_cu_to_char(end);
        let start_byte = self.content.char_to_byte(start_char);
        let end_byte = self.content.char_to_byte(end_char);
        if start_byte <= end_byte {
            start_byte..end_byte
        } else {
            end_byte..start_byte
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_selections() {
        let mut core = EditorCore::new();
        core.content = Rope::from("abcdef");
        
        // Overlapping selections
        core.selections = vec![
            Selection::new(0, 2), // "ab"
            Selection::new(1, 3), // "bc"
        ];
        core.merge_selections();
        assert_eq!(core.selections.len(), 1);
        assert_eq!(core.selections[0].range(), 0..3);

        // Adjacent selections
        core.selections = vec![
            Selection::new(0, 2), // "ab"
            Selection::new(2, 4), // "cd"
        ];
        core.merge_selections();
        assert_eq!(core.selections.len(), 1);
        assert_eq!(core.selections[0].range(), 0..4);

        // Disjoint selections
        core.selections = vec![
            Selection::new(0, 1), // "a"
            Selection::new(2, 3), // "c"
        ];
        core.merge_selections();
        assert_eq!(core.selections.len(), 2);
    }

    #[test]
    fn test_multi_cursor_insert() {
        let mut core = EditorCore::new();
        core.content = Rope::from("abc\ndef\n");
        
        // Cursors at start of each line
        core.selections = vec![
            Selection::new(0, 0),
            Selection::new(4, 4),
        ];
        
        core.insert_text("- ");
        
        assert_eq!(core.content.to_string(), "- abc\n- def\n");
        // Check cursors moved
        assert_eq!(core.selections.len(), 2);
        // Original 0 -> 2
        // Original 4 -> 4 + 2 (first) + 2 (second) = 8
        // Let's check ranges
        core.selections.sort_by_key(|s| s.head);
        assert_eq!(core.selections[0].head, 2);
        assert_eq!(core.selections[1].head, 8);
    }

    #[test]
    fn test_multi_cursor_delete() {
        let mut core = EditorCore::new();
        core.content = Rope::from("abc1\ndef1\n");
        
        // Cursors at '1's (indices 3 and 8)
        // "abc1" -> 0,1,2,3(1),4(\n)
        // "def1" -> 5,6,7,8(1),9(\n)
        core.selections = vec![
            Selection::new(3, 4), // Select '1'
            Selection::new(8, 9), // Select '1'
        ];
        
        core.delete_selection();
        
        assert_eq!(core.content.to_string(), "abc\ndef\n");
        
        // Check cursors
        // First selection: 3..4 deleted. Cursor should be at 3.
        // Second selection: 8..9 deleted. 
        // Index 8 shifted by -1 (first deletion) -> 7.
        // Then deleted -> cursor at 7.
        // So cursors at 3 and 7.
        core.selections.sort_by_key(|s| s.head);
        assert_eq!(core.selections.len(), 2);
        assert_eq!(core.selections[0].head, 3);
        assert_eq!(core.selections[1].head, 7);
    }

    #[test]
    fn test_select_all() {
        let mut core = EditorCore::new();
        core.content = Rope::from("abc\ndef");
        core.select_all();
        assert_eq!(core.selections.len(), 1);
        assert_eq!(core.selections[0].range(), 0..core.content.len_bytes());
    }

    #[test]
    fn test_utf16_conversions_and_ime_crash_simulation() {
        let mut core = EditorCore::new();
        // "Hello"
        core.content = Rope::from("Hello");
        
        // Test basic conversion
        let range_utf16 = 0..5;
        let range = core.range_from_utf16(&range_utf16);
        assert_eq!(range, 0..5);
        
        let range_utf16_back = core.range_to_utf16(&range);
        assert_eq!(range_utf16_back, 0..5);
        
        // Test Chinese (3 bytes per char, 1 UTF-16 unit)
        // "你好" -> 6 bytes, 2 chars, 2 UTF-16 units
        core.content = Rope::from("你好");
        let range_utf16 = 0..2;
        let range = core.range_from_utf16(&range_utf16);
        assert_eq!(range, 0..6);
        
        let range_utf16_back = core.range_to_utf16(&range);
        assert_eq!(range_utf16_back, 0..2);
        
        // Test partial Chinese char (should not happen normally but check safety)
        // 1 UTF-16 unit -> 1 char -> 3 bytes
        let range_utf16 = 0..1;
        let range = core.range_from_utf16(&range_utf16);
        assert_eq!(range, 0..3);
        
        // Test Emoji (4 bytes, 2 UTF-16 units)
        // "👋" -> \u{1F44B} -> 4 bytes. UTF-16: 0xD83D 0xDC4B (2 units)
        core.content = Rope::from("👋");
        assert_eq!(core.content.len_bytes(), 4);
        assert_eq!(core.content.len_utf16_cu(), 2);
        
        let range_utf16 = 0..2;
        let range = core.range_from_utf16(&range_utf16);
        assert_eq!(range, 0..4);
        
        // Simulation of IME crash
        // 1. User types "z"
        core.content = Rope::from("");
        core.replace_range(0..0, "z");
        core.marked_range = Some(0..1); // "z"
        
        // 2. User types "h"
        // Replace marked range "z" (0..1) with "zh"
        let range = core.marked_range.clone().unwrap();
        core.replace_range(range.clone(), "zh");
        // Update marked range
        let new_end = range.start + 2; // "zh".len()
        core.marked_range = Some(range.start..new_end);
        
        // 3. User selects "中" (3 bytes)
        // Replace marked range "zh" (0..2) with "中"
        let range = core.marked_range.clone().unwrap();
        // range is 0..2. content is "zh" (2 bytes).
        // replace_range(0..2, "中")
        core.replace_range(range, "中");
        // marked_range cleared by replace_range
        assert!(core.marked_range.is_none());
        assert_eq!(core.content.to_string(), "中");
        
        // Test Out of Bounds
        core.content = Rope::from("a");
        // range_utf16 out of bounds
        let range_utf16 = 0..100;
        let range = core.range_from_utf16(&range_utf16);
        // Should clamp to 0..1 (byte range)
        assert_eq!(range, 0..1);
        
        // range_utf16 start out of bounds
        let range_utf16 = 50..100;
        let range = core.range_from_utf16(&range_utf16);
        // Should clamp to 1..1 (empty at end)
        assert_eq!(range, 1..1);
    }
}
