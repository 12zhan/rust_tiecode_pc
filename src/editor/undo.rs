use std::ops::Range;

#[derive(Clone, Debug)]
pub enum EditOperation {
    Insert { range: Range<usize>, text: String },
    Delete { range: Range<usize>, text: String },
}

impl EditOperation {
    pub fn inverse(&self) -> Self {
        match self {
            EditOperation::Insert { range, text } => EditOperation::Delete {
                range: range.clone(),
                text: text.clone(),
            },
            EditOperation::Delete { range, text } => EditOperation::Insert {
                range: range.clone(),
                text: text.clone(),
            },
        }
    }
}

pub struct UndoHistory {
    undo_stack: Vec<Vec<EditOperation>>,
    redo_stack: Vec<Vec<EditOperation>>,
    current_transaction: Option<Vec<EditOperation>>,
}

impl UndoHistory {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_transaction: None,
        }
    }

    pub fn push(&mut self, op: EditOperation) {
        if let Some(transaction) = &mut self.current_transaction {
            transaction.push(op);
        } else {
            self.undo_stack.push(vec![op]);
        }
        self.redo_stack.clear();
    }

    pub fn begin_transaction(&mut self) {
        if self.current_transaction.is_none() {
            self.current_transaction = Some(Vec::new());
        }
    }

    pub fn end_transaction(&mut self) {
        if let Some(transaction) = self.current_transaction.take() {
            if !transaction.is_empty() {
                self.undo_stack.push(transaction);
            }
        }
    }

    pub fn undo(&mut self) -> Option<Vec<EditOperation>> {
        if let Some(ops) = self.undo_stack.pop() {
            self.redo_stack.push(ops.iter().map(|op| op.inverse()).collect());
            Some(ops.iter().rev().map(|op| op.inverse()).collect())
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<Vec<EditOperation>> {
        if let Some(ops) = self.redo_stack.pop() {
            self.undo_stack.push(ops.iter().map(|op| op.inverse()).collect());
            Some(ops.iter().rev().map(|op| op.inverse()).collect())
        } else {
            None
        }
    }
}
