use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

// --- Compiler Options ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CompilerOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_map_path: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hard_mode: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub friendly_name: Option<i32>,
    
    pub ide_mode: bool,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<i32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimize_level: Option<i32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lint_disable: Option<Vec<String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_level: Option<i32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<i32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emit_names_path: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stable_names_path: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_prefixes: Option<SearchPrefixes>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android: Option<AndroidConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_path: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchPrefixes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lib: Option<Vec<String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Vec<String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub res: Option<Vec<String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AndroidConfig {
    pub app_config: AppConfig,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradle: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foundation_lib_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub app_name: String,
    pub app_icon: String,
    pub min_sdk: i32,
    pub target_sdk: i32,
    pub version_code: i32,
    pub version_name: String,
}

// --- Diagnostic ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub uri: String,
    pub range: Range,
    pub key: String,
    pub message: String,
    pub level: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    pub diagnostics: Vec<Diagnostic>,
}

// --- Editing ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextChange {
    pub range: Range,
    pub new_text: String,
}

// --- Completion ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionParams {
    pub uri: String,
    pub position: Position,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_text: Option<String>,
    pub partial: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_char: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResult {
    pub items: Vec<CompletionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionItem {
    pub kind: i32,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_name: Option<String>,
    pub insert_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_edits: Option<Vec<TextEdit>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

// --- Hover ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorParams {
    pub uri: String,
    pub position: Position,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverResult {
    pub kind: i32, // MarkupKind
    pub text: String,
}

// --- Highlight ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightResult {
    pub highlights: Vec<HighlightItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightItem {
    pub range: Range,
    pub kind: i32, // ElementKind
    pub tags: Vec<i32>,
}

// --- Source Elements ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceElementsResult {
    pub elements: Vec<SourceElementNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceElementNode {
    pub element: SourceElement,
    pub children: Vec<SourceElementNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceElement {
    pub kind: i32,
    pub tags: Vec<i32>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub range: Range,
    pub identifier_range: Range,
}
