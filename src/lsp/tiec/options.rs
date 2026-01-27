use super::diagnostics::LogLevel;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// 目标输出平台
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
pub enum Platform {
    Undefined = 0,
    Android = 1,
    Harmony = 2,
    Linux = 3,
    Windows = 4,
    Ios = 5,
    Apple = 6,
    Html = 7,
}

/// 符号输出名称模式
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
pub enum FriendlyNameMode {
    Random = 0,
    Pinyin = 1,
    Original = 2,
}

/// 编译部署模式
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
pub enum CompileProfile {
    Standard = 0,
    Designer = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchPrefixes {
    /// [可选] 外部依赖库(.jar/.aar/.so) 的附加搜寻路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lib: Option<Vec<String>>,
    /// [可选] 外部源文件(.java/.tie) 的附加搜寻路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Vec<String>>,
    /// [可选] Android 资源的附加搜寻路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub res: Option<Vec<String>>,
    /// [可选] 资产文件(assets) 的附加搜寻路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AndroidAppConfig {
    /// [可选] App 显示名称
    #[serde(rename = "appName", skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    /// [可选] App 启动图标路径
    #[serde(rename = "appIcon", skip_serializing_if = "Option::is_none")]
    pub app_icon: Option<String>,
    /// [可选] Android Min SDK Version
    #[serde(rename = "minSdk", skip_serializing_if = "Option::is_none")]
    pub min_sdk: Option<i32>,
    /// [可选] Android Target SDK Version
    #[serde(rename = "targetSdk", skip_serializing_if = "Option::is_none")]
    pub target_sdk: Option<i32>,
    /// [可选] 版本号(整数)
    #[serde(rename = "versionCode", skip_serializing_if = "Option::is_none")]
    pub version_code: Option<i32>,
    /// [可选] 版本名称
    #[serde(rename = "versionName", skip_serializing_if = "Option::is_none")]
    pub version_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AndroidOptions {
    /// [可选] Android 平台专用配置
    #[serde(rename = "appConfig", skip_serializing_if = "Option::is_none")]
    pub app_config: Option<AndroidAppConfig>,
    /// [可选] 是否以 Gradle 工程目录结构输出产物
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradle: Option<bool>,
    /// [可选] 基础库路径，用于解决 AndroidX 依赖冲突
    #[serde(rename = "foundationLibPath", skip_serializing_if = "Option::is_none")]
    pub foundation_lib_path: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OptionsError {
    #[error("options.ideMode 必须指定")]
    IdeModeRequired,
    #[error("编译模式下 options.outputDir 必须指定")]
    OutputDirRequiredInCompileMode,
}

/// 创建编译器上下文的 Options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Options {
    /// [可选] 默认包名，强烈建议设置为项目实际包名
    #[serde(rename = "packageName", skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    /// [编译模式必选] 编译产物输出目录（ideMode=false 时必填）
    #[serde(rename = "outputDir", skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,
    /// [可选] 行号映射表输出路径
    #[serde(rename = "lineMapPath", skip_serializing_if = "Option::is_none")]
    pub line_map_path: Option<String>,
    /// [可选] 是否为硬输出模式
    #[serde(rename = "hardMode", skip_serializing_if = "Option::is_none")]
    pub hard_mode: Option<bool>,
    /// [可选] 是否开启 Debug 模式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<bool>,
    /// [可选] 是否开启顶级语句语法特性支持
    #[serde(rename = "enableTopLevelStmt", skip_serializing_if = "Option::is_none")]
    pub enable_top_level_stmt: Option<bool>,
    /// [可选] 符号输出名称模式
    #[serde(rename = "friendlyName", skip_serializing_if = "Option::is_none")]
    pub friendly_name: Option<FriendlyNameMode>,
    /// [必须] 是否为 IDE 模式（仅做代码分析不输出产物）
    #[serde(rename = "ideMode", skip_serializing_if = "Option::is_none")]
    pub ide_mode: Option<bool>,
    /// [可选] 编译部署模式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<CompileProfile>,
    /// [可选] 编译优化级别 0-3
    #[serde(rename = "optimizeLevel", skip_serializing_if = "Option::is_none")]
    pub optimize_level: Option<i32>,
    /// [可选] 需要屏蔽的 Lint 检查项列表
    #[serde(rename = "lintDisable", skip_serializing_if = "Option::is_none")]
    pub lint_disable: Option<Vec<String>>,
    /// [可选] 日志等级
    #[serde(rename = "logLevel", skip_serializing_if = "Option::is_none")]
    pub log_level: Option<LogLevel>,
    /// [可选] 目标输出平台
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<Platform>,
    /// [可选] 混淆/重命名映射表输出路径
    #[serde(rename = "emitNamesPath", skip_serializing_if = "Option::is_none")]
    pub emit_names_path: Option<String>,
    /// [可选] 稳定名称映射表读取路径
    #[serde(rename = "stableNamesPath", skip_serializing_if = "Option::is_none")]
    pub stable_names_path: Option<String>,
    /// [可选] 附加文件搜寻路径配置
    #[serde(rename = "searchPrefixes", skip_serializing_if = "Option::is_none")]
    pub search_prefixes: Option<SearchPrefixes>,
    /// [可选] Android 平台专用配置（platform=Android 时生效）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android: Option<AndroidOptions>,
}

impl Options {
    pub fn ide() -> Self {
        Self {
            ide_mode: Some(true),
            enable_top_level_stmt: Some(true),
            ..Default::default()
        }
    }

    pub fn compile(output_dir: impl Into<String>) -> Self {
        Self {
            ide_mode: Some(false),
            output_dir: Some(output_dir.into()),
            enable_top_level_stmt: Some(true),
            ..Default::default()
        }
    }

    pub fn validate(&self) -> Result<(), OptionsError> {
        let ide_mode = self.ide_mode.ok_or(OptionsError::IdeModeRequired)?;
        if !ide_mode && self.output_dir.as_ref().is_none() {
            return Err(OptionsError::OutputDirRequiredInCompileMode);
        }
        Ok(())
    }
}
