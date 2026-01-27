use serde::{Deserialize, Serialize};

/// 光标位置（0-based）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// 行号
    pub line: usize,
    /// 列号
    pub column: usize,
}

/// 文本范围 `[start, end)`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    /// 起始位置（包含）
    pub start: Position,
    /// 结束位置（不包含）
    pub end: Position,
}

/// 文本编辑操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    /// 变更范围
    pub range: Range,
    /// 替换后的文本
    #[serde(rename = "newText")]
    pub new_text: String,
}

/// 文件位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    /// 文件 URI
    pub uri: String,
    /// 文件内的位置范围
    pub range: Range,
}

/// 通用光标参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorParams {
    /// 文件 URI
    pub uri: String,
    /// 光标位置
    pub position: Position,
    /// 当前行文本（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_text: Option<String>,
}

/// 增量文本变更
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChange {
    /// 变更范围
    pub range: Range,
    /// 新文本内容
    #[serde(rename = "newText")]
    pub new_text: String,
}

/// 转到定义结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionResult {
    /// 光标处标识符范围
    #[serde(rename = "identifierRange")]
    pub identifier_range: Range,
    /// 定义位置
    pub location: Location,
}

/// 查找引用结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceResult {
    /// 光标处标识符范围
    #[serde(rename = "identifierRange")]
    pub identifier_range: Range,
    /// 所有引用位置
    pub locations: Vec<Location>,
}

/// 代码操作项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAction {
    /// 操作标题
    pub title: String,
    /// 需要应用的编辑列表
    pub edits: Vec<TextEdit>,
}

/// 代码操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeActionResult {
    /// 操作集合
    pub actions: Vec<CodeAction>,
}
