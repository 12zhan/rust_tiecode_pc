use ropey::Rope;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct BlockPair {
    start: String,
    end: String,
}

#[derive(Debug, Deserialize)]
struct GrammarConfig {
    #[serde(default, rename = "blockPairs")]
    block_pairs: Vec<BlockPair>,
}

use std::collections::HashMap;

#[derive(Clone)]
pub struct BlockMap {
    // Map of line index to its block depth
    pub depths: Arc<Vec<usize>>,
    // Map of line index to the line index of the block start that covers it
    pub parents: Arc<Vec<Option<usize>>>,
    // Map of block start line to block end line
    pub scopes: Arc<HashMap<usize, usize>>,
    // Cache of block pairs for the current grammar
    pairs: Vec<(String, String)>,
    last_grammar_ptr: *const u8,
}

impl BlockMap {
    pub fn new() -> Self {
        Self {
            depths: Arc::new(Vec::new()),
            parents: Arc::new(Vec::new()),
            scopes: Arc::new(HashMap::new()),
            pairs: Vec::new(),
            last_grammar_ptr: std::ptr::null(),
        }
    }

    pub fn update(&mut self, text: &Rope, grammar_json: &str) {
        // Update pairs if grammar changed
        if self.last_grammar_ptr != grammar_json.as_ptr() {
            if let Ok(config) = serde_json::from_str::<GrammarConfig>(grammar_json) {
                self.pairs = config.block_pairs.into_iter()
                    .map(|p| (p.start, p.end))
                    .collect();
            } else {
                self.pairs.clear();
            }
            self.last_grammar_ptr = grammar_json.as_ptr();
        }

        let line_count = text.len_lines();
        
        if self.pairs.is_empty() {
             self.depths = Arc::new(Vec::new());
             self.parents = Arc::new(Vec::new());
             self.scopes = Arc::new(HashMap::new());
             return;
        }

        let mut depths = Vec::with_capacity(line_count);
        let mut parents = Vec::with_capacity(line_count);
        let mut scopes = HashMap::new();
        let mut stack: Vec<usize> = Vec::new();
        
        for i in 0..line_count {
            let line = text.line(i);
            let line_str = line.to_string();
            let trimmed = line_str.trim();

            let mut matched_end = false;
            for (_, end) in &self.pairs {
                if trimmed.starts_with(end) {
                    matched_end = true;
                    break;
                }
            }

            // Push state before popping to include the end line in the block
            let parent = stack.last().copied();
            parents.push(parent);
            depths.push(stack.len());

            if matched_end {
                if let Some(start) = stack.pop() {
                    scopes.insert(start, i);
                }
            }

            let mut matched_start = false;
            for (start, _) in &self.pairs {
                if trimmed.starts_with(start) {
                    matched_start = true;
                    break;
                }
            }

            if matched_start {
                stack.push(i);
            }
        }
        
        self.depths = Arc::new(depths);
        self.parents = Arc::new(parents);
        self.scopes = Arc::new(scopes);
    }
}
