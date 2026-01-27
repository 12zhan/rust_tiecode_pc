use super::common::TextEdit;
use serde::{Deserialize, Serialize};

/// 格式化结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingResult {
    /// 需要应用的文本编辑
    pub edits: Vec<TextEdit>,
}
