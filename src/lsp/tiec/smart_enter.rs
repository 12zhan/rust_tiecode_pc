use super::common::Range;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// 智能键入类型
#[repr(i32)]
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
pub enum SmartEnterKind {
    /// 未知类型
    Unknown = 0,
    /// 选择文件路径
    SelectFile = 1,
    /// 选择枚举常量
    SelectEnum = 2,
    /// 真/假开关
    BooleanSwitch = 3,
}

/// 智能键入结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartEnterResult {
    /// 智能键入类型
    pub kind: SmartEnterKind,
    /// 需要替换的范围
    pub range: Range,
    /// 用于替换的格式字符串（包含 %s）
    #[serde(rename = "replaceFormat")]
    pub replace_format: String,
    /// 枚举候选值
    #[serde(default)]
    pub enums: Vec<String>,
    /// 真/假开关的当前值
    #[serde(default)]
    #[serde(rename = "isTrue")]
    pub is_true: Option<bool>,
}
