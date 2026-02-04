use std::collections::HashMap;
use std::path::{Path, PathBuf};
use gpui::*;
use url::Url;

pub fn default_doc_uri(path: &Path) -> String {
    if let Ok(url) = Url::from_file_path(path) {
        return url.to_string();
    }
    // Fallback if from_file_path fails (e.g. relative path)
    let s = path.to_string_lossy().replace('\\', "/");
    if s.starts_with('/') {
        format!("file://{}", s)
    } else {
        format!("file:///{}", s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LspRequestKind {
    #[allow(dead_code)]
    Initialize,
    Completion { index: usize },
    Hover { index: usize, #[allow(dead_code)] pos: Point<Pixels> },
    Definition { index: usize },
    SignatureHelp { index: usize },
    Formatting,
}

pub struct LspManager {
    pub pending_requests: HashMap<usize, LspRequestKind>,
    pub version: i32,
    pub doc_uri: String,
    pub root_uri: String,
}

impl LspManager {
    pub fn new(doc_uri: String) -> Self {
        Self {
            pending_requests: HashMap::new(),
            version: 1,
            doc_uri,
            root_uri: String::new(),
        }
    }

    pub fn detect_project_root(path: &std::path::Path) -> PathBuf {
        for ancestor in path.ancestors() {
            if ancestor.ends_with("源代码") {
                if let Some(parent) = ancestor.parent() {
                    return parent.to_path_buf();
                }
            }
        }
        
        // Fallback: use file's directory if no better root found
        if path.is_file() {
            path.parent().unwrap_or(path).to_path_buf()
        } else {
            path.to_path_buf()
        }
    }

    pub fn restart(&mut self, root_path: PathBuf, _content: &str) {
        // LSP functionality removed
        println!("LSP restart requested but disabled (root: {:?})", root_path);
    }

    pub fn initialize(&mut self, _content: &str) {
         // LSP functionality removed
    }

    pub fn notify_change(&mut self, _content: &str) {
        // LSP functionality removed
        self.version += 1;
    }

    pub fn update_doc_uri(&mut self, new_uri: String, _content: &str) {
        self.doc_uri = new_uri;
        self.version = 1;
    }
}
