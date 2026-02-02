use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub process_id: Option<i32>,
    pub root_uri: Option<String>,
    pub capabilities: Value,
    pub initialization_options: Option<InitializationOptions>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InitializationOptions {
    #[serde(default)]
    pub jars: Vec<String>,
    #[serde(default)]
    pub java_sources: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializedParams {
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentItem {
    pub uri: String,
    pub language_id: String,
    pub version: i32,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidOpenTextDocumentParams {
    pub text_document: TextDocumentItem,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionedTextDocumentIdentifier {
    pub uri: String,
    pub version: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentContentChangeEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_length: Option<u32>,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidChangeTextDocumentParams {
    pub text_document: VersionedTextDocumentIdentifier,
    pub content_changes: Vec<TextDocumentContentChangeEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentIdentifier {
    pub uri: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentPositionParams {
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

pub type CompletionParams = TextDocumentPositionParams;
pub type HoverParams = TextDocumentPositionParams;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionList {
    #[serde(default)]
    pub is_incomplete: bool,
    #[serde(default)]
    pub items: Vec<CompletionItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionItem {
    pub label: String,
    #[serde(default)]
    pub kind: Option<u32>,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub insert_text: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CompletionResult {
    List(CompletionList),
    Items(Vec<CompletionItem>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hover {
    pub contents: Value,
    #[serde(default)]
    pub range: Option<Range>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishDiagnosticsParams {
    pub uri: String,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub range: Range,
    #[serde(default)]
    pub severity: Option<u32>,
    #[serde(default)]
    pub code: Option<Value>,
    #[serde(default)]
    pub source: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

pub fn file_uri_for_path(path: &Path) -> String {
    let s = path.display().to_string().replace('\\', "/");
    if s.len() >= 2 && s.as_bytes()[1] == b':' {
        format!("file:///{}", s)
    } else if s.starts_with('/') {
        format!("file://{}", s)
    } else {
        format!("file:///{}", s)
    }
}

pub fn default_root_uri() -> Option<String> {
    std::env::current_dir()
        .ok()
        .map(|p| file_uri_for_path(&p))
}

pub fn default_doc_uri() -> String {
    "file:///virtual/untitled.t".to_string()
}

pub fn position_to_global_utf16(
    line_start_utf16: usize,
    character_utf16: u32,
    len_utf16: usize,
) -> usize {
    (line_start_utf16 + character_utf16 as usize).min(len_utf16)
}

pub fn flatten_hover_contents(contents: &Value) -> String {
    if let Some(s) = contents.as_str() {
        return s.to_string();
    }
    if let Some(obj) = contents.as_object() {
        if let Some(v) = obj.get("value").and_then(|v| v.as_str()) {
            return v.to_string();
        }
        if let Some(v) = obj.get("contents") {
            return flatten_hover_contents(v);
        }
    }
    if let Some(arr) = contents.as_array() {
        let parts: Vec<String> = arr.iter().map(flatten_hover_contents).collect();
        return parts.join("\n");
    }
    contents.to_string()
}

pub fn diagnostic_display_message(d: &Diagnostic) -> String {
    let mut parts = Vec::new();
    if let Some(source) = &d.source {
        parts.push(source.clone());
    }
    if let Some(code) = &d.code {
        if let Some(s) = code.as_str() {
            parts.push(s.to_string());
        } else if let Some(n) = code.as_i64() {
            parts.push(n.to_string());
        }
    }
    if parts.is_empty() {
        d.message.clone()
    } else {
        format!("{}: {}", parts.join(" "), d.message)
    }
}
