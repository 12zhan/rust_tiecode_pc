use std::collections::HashMap;
use std::path::{Path, PathBuf};
use gpui::*;
use log::{info, warn};
use url::Url;

use crate::plugin::lsp::LspPlugin;
use crate::editor::completion::{CompletionItem, CompletionKind};

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
    plugin: Option<LspPlugin>,
    plugin_load_attempted: bool,
}

impl LspManager {
    pub fn new(doc_uri: String) -> Self {
        Self {
            pending_requests: HashMap::new(),
            version: 1,
            doc_uri,
            root_uri: String::new(),
            plugin: None,
            plugin_load_attempted: false,
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

    fn ensure_plugin(&mut self) -> Option<&mut LspPlugin> {
        if self.plugin.is_some() || self.plugin_load_attempted {
            return self.plugin.as_mut();
        }

        self.plugin_load_attempted = true;

        let loaded = unsafe { LspPlugin::load_default() };
        match loaded {
            Ok(Some(plugin)) => {
                info!("LSP plugin loaded: {}", plugin.name());
                self.plugin = Some(plugin);
            }
            Ok(None) => {}
            Err(err) => {
                warn!("LSP plugin load failed: {err}");
            }
        }

        self.plugin.as_mut()
    }

    pub fn restart(&mut self, root_path: PathBuf, content: &str) {
        self.root_uri = default_doc_uri(&root_path);
        let root_uri = self.root_uri.clone();
        let doc_uri = self.doc_uri.clone();
        if let Some(plugin) = self.ensure_plugin() {
            if let Err(err) = plugin.initialize(&root_uri, &doc_uri, content) {
                warn!("LSP plugin initialize failed: {err}");
            }
        }
    }

    pub fn initialize(&mut self, content: &str) {
        let root_uri = self.root_uri.clone();
        let doc_uri = self.doc_uri.clone();
        if let Some(plugin) = self.ensure_plugin() {
            if let Err(err) = plugin.initialize(&root_uri, &doc_uri, content) {
                warn!("LSP plugin initialize failed: {err}");
            }
        }
    }

    pub fn notify_change(&mut self, content: &str) {
        self.version += 1;
        let doc_uri = self.doc_uri.clone();
        let version = self.version;
        if let Some(plugin) = self.ensure_plugin() {
            if let Err(err) = plugin.did_change(&doc_uri, version, content) {
                warn!("LSP plugin didChange failed: {err}");
            }
        }
    }

    pub fn update_doc_uri(&mut self, new_uri: String, content: &str) {
        self.doc_uri = new_uri;
        self.version = 1;
        let root_uri = self.root_uri.clone();
        let doc_uri = self.doc_uri.clone();
        if let Some(plugin) = self.ensure_plugin() {
            if let Err(err) = plugin.initialize(&root_uri, &doc_uri, content) {
                warn!("LSP plugin initialize failed: {err}");
            }
        }
    }

    pub fn completion(&mut self, line: usize, character: usize) -> Option<Vec<CompletionItem>> {
        let doc_uri = self.doc_uri.clone();
        if let Some(plugin) = self.ensure_plugin() {
            match plugin.completion(&doc_uri, line, character) {
                Ok(value) => {
                    if let Some(items) = value.get("items").and_then(|i| i.as_array()) {
                        let result = items.iter().filter_map(|item| {
                             let label = item.get("label")?.as_str()?.to_string();
                             let kind_int = item.get("kind").and_then(|k| k.as_i64()).unwrap_or(1);
                             let kind = match kind_int {
                                 2 | 3 => CompletionKind::Function,
                                 6 => CompletionKind::Variable,
                                 7 => CompletionKind::Class,
                                 14 => CompletionKind::Keyword,
                                 _ => CompletionKind::Text,
                             };
                             let detail = item.get("detail").and_then(|s| s.as_str()).unwrap_or("").to_string();
                             
                             Some(CompletionItem {
                                 label,
                                 kind,
                                 detail,
                             })
                        }).collect();
                        return Some(result);
                    }
                }
                Err(err) => {
                    warn!("LSP plugin completion failed: {err}");
                }
            }
        }
        None
    }
}
