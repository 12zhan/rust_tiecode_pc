use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// 悬停文本类型
#[repr(i32)]
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
pub enum MarkupKind {
    /// 纯文本
    PlainText = 0,
    /// Markdown
    Markdown = 1,
}

/// 悬停内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkupContent {
    /// 内容类型
    pub kind: MarkupKind,
    /// 内容文本
    pub text: String,
}
