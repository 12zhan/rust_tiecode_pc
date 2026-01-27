use super::common::{Range, TextEdit};
use serde::{Deserialize, Serialize};

/// 重命名前置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameSymbolInfo {
    /// 光标处符号名称
    pub name: String,
    /// 符号范围
    pub range: Range,
    /// 符号类型
    pub kind: super::symbols::ElementKind,
}

/// 重命名结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameResult {
    /// key 为文件 URI，value 为对应文件内的编辑列表
    #[serde(rename = "projectEdit")]
    pub project_edit: std::collections::HashMap<String, Vec<TextEdit>>,
}
