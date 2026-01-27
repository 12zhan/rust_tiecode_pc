use super::common::Range;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// 日志级别
#[repr(i32)]
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
pub enum LogLevel {
    /// 调试信息
    Debug = 0,
    /// 一般信息
    Info = 1,
    /// 警告信息
    Warning = 2,
    /// 错误信息
    Error = 3,
}

/// 编译诊断信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// 诊断所在文件 URI
    pub uri: String,
    /// 诊断范围
    pub range: Range,
    /// 诊断标识 key
    pub key: String,
    /// 诊断文本
    pub message: String,
    /// 诊断等级
    pub level: LogLevel,
}

/// Lint 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    /// 诊断列表
    pub diagnostics: Vec<Diagnostic>,
}
