use super::common::TextEdit;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// 补全请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionParams {
    /// 文件 URI
    pub uri: String,
    /// 光标位置
    pub position: super::common::Position,
    /// 当前行文本（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lineText")]
    pub line_text: Option<String>,
    /// 已输入的补全前缀
    pub partial: String,
    /// 触发补全的字符（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "triggerChar")]
    pub trigger_char: Option<String>,
}

/// 补全项类型
#[repr(i32)]
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
pub enum CompletionItemKind {
    Text = 1,
    Method = 2,
    Function = 3,
    Variable = 6,
    Class = 7,
    Keyword = 14,
}

/// 单个补全项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    /// 补全项类型
    pub kind: CompletionItemKind,
    /// 展示文本
    pub label: String,
    /// 详情说明
    #[serde(default)]
    pub detail: String,
    /// 排序关键字
    #[serde(default)]
    pub sort_key: String,
    /// 符号原始名称
    #[serde(default)]
    pub symbol_name: String,
    /// 插入到编辑器的文本
    pub insert_text: String,
    /// 额外需要应用的编辑
    #[serde(default)]
    pub extra_edits: Vec<TextEdit>,
}

/// 补全结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResult {
    /// 补全项列表
    pub items: Vec<CompletionItem>,
}
