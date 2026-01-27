use super::common::Range;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;

/// 符号类型
#[repr(i32)]
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
pub enum ElementKind {
    /// 未知
    Unknown = 0,
    /// 类
    Class = 1,
    /// 方法
    Method = 2,
    /// 字段
    Field = 3,
    /// 变量
    Variable = 4,
    /// 函数
    Function = 5,
}

/// 符号标记
#[repr(i32)]
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
pub enum ElementTag {
    /// 静态
    Static = 1,
    /// 已废弃
    Deprecated = 2,
}

/// 符号信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    /// 符号类型
    pub kind: ElementKind,
    /// 附加标记
    #[serde(default)]
    pub tags: Vec<ElementTag>,
    /// 符号名称
    pub name: String,
    /// 符号详情（如签名、包名）
    #[serde(default)]
    pub detail: String,
    /// 定义范围
    pub range: Range,
    /// 标识符范围
    #[serde(rename = "identifierRange")]
    pub identifier_range: Range,
}

/// 嵌套符号节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementNode {
    /// 当前节点的符号信息
    pub element: Element,
    /// 子节点
    #[serde(default)]
    pub children: Vec<ElementNode>,
}

/// SourceElements 返回值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceElementsResult {
    /// 符号树根节点列表
    pub elements: Vec<ElementNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    /// 高亮范围
    pub range: Range,
    /// 高亮符号类型
    pub kind: ElementKind,
    /// 高亮标记
    #[serde(default)]
    pub tags: Vec<ElementTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightResult {
    /// 高亮列表
    pub highlights: Vec<Highlight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceElementsResult {
    /// key 为文件 URI，value 为该文件的符号列表
    pub elements: HashMap<String, Vec<Element>>,
}
