use super::common::Position;
use serde::{Deserialize, Serialize};

/// 方法签名帮助请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureHelpParams {
    /// 文件 URI
    pub uri: String,
    /// 光标位置
    pub position: Position,
    /// 触发字符（如 '(' 或 ','）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "triggerChar")]
    pub trigger_char: Option<String>,
}

/// 方法签名帮助结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureHelpResult {
    /// 方法签名文本
    pub signature: String,
    /// 当前活跃参数的签名片段
    #[serde(rename = "activeParameter")]
    pub active_parameter: String,
}
