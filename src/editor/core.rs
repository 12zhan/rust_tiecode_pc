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

    pub fn replace_selections(&mut self, text: &str) {
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
         self.replace_range_internal(range.clone(), text);
         
         // Update selections?
         // If we use this method, we assume single selection or manual control.
         // Let's just update the primary selection to match behavior.
         let new_pos = range.start + text.len();
         self.selections = vec![Selection::new(new_pos, new_pos)];
         
         self.history.end_transaction();
    }

    pub fn insert_text(&mut self, text: &str) {
        self.replace_selections(text);
    }

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
        match op {
            EditOperation::Insert { range, text } => {
                let start_char_idx = self.content.byte_to_char(range.start);
                self.content.insert(start_char_idx, &text);
                // Update cursor?
                // For undo/redo, we might want to just select the affected range or end of it.
                // Simple approach: Set cursor to end.
                // With multiple cursors, this will collapse them?
                // Yes, simpler for now.
                self.selections = vec![Selection::new(range.end, range.end)];
            },
            EditOperation::Delete { range, .. } => {
                 let start_char_idx = self.content.byte_to_char(range.start);
                 let end_char_idx = self.content.byte_to_char(range.end);
                 self.content.remove(start_char_idx..end_char_idx);
                 self.selections = vec![Selection::new(range.start, range.start)];
            }
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
}
