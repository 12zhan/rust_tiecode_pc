use super::common::TextEdit;
use super::diagnostics::Diagnostic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};

/// TLY 序列化格式
#[repr(i32)]
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
pub enum TlyFormat {
    /// TLY 文本格式
    Tly = 0,
    /// JSON 结构格式
    Json = 1,
}

/// UI 绑定支持信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIBindingSupportInfo {
    /// 是否支持布局绑定
    #[serde(rename = "isSupport")]
    pub is_support: bool,
    /// 当前类的符号信息（可选）
    pub element: Option<super::symbols::Element>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlyClass {
    /// 组件类名
    #[serde(rename = "className")]
    pub class_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlyPropName {
    /// 属性名
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlyPropValue {
    /// 属性值（可能是字符串、数字或其他 JSON 类型）
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlyProperty {
    /// 属性名
    #[serde(rename = "propName")]
    pub prop_name: TlyPropName,
    /// 属性值
    #[serde(rename = "propValue")]
    pub prop_value: TlyPropValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlyEntity {
    /// 组件类
    pub class: TlyClass,
    /// 名称属性（可选）
    #[serde(default)]
    #[serde(rename = "nameProp")]
    pub name_prop: Option<TlyProperty>,
    /// 组件属性
    #[serde(default)]
    pub properties: Vec<TlyProperty>,
    /// 子组件
    #[serde(default)]
    pub children: Vec<TlyEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlyParsingResult {
    /// 解析后的根节点
    pub root: Option<TlyEntity>,
    /// 解析诊断信息
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIBindingEditResult {
    /// 对源码的编辑列表
    pub edits: Vec<TextEdit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewProperty {
    /// 属性名称
    pub name: String,
    /// 属性类型（对应 JSON 的 type 字段）
    #[serde(rename = "type")]
    pub property_type: String,
    /// 属性输出名
    #[serde(rename = "mangledName")]
    pub mangled_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewClassInfo {
    /// 组件完整类名
    pub name: String,
    /// 组件输出名
    #[serde(rename = "mangledName")]
    pub mangled_name: String,
    /// 是否为容器组件
    #[serde(rename = "isContainer")]
    pub is_container: bool,
    /// 组件自身属性列表
    #[serde(default)]
    #[serde(rename = "viewProperties")]
    pub view_properties: Vec<ViewProperty>,
    /// 容器布局属性列表
    #[serde(default)]
    #[serde(rename = "containerProperties")]
    pub container_properties: Vec<ViewProperty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewClassInfoResult {
    /// 可视化组件类型信息
    #[serde(default)]
    #[serde(rename = "viewClasses")]
    pub view_classes: Vec<ViewClassInfo>,
    /// 所有组件共有的基础属性
    #[serde(default)]
    #[serde(rename = "basicProperties")]
    pub basic_properties: Vec<ViewProperty>,
}
