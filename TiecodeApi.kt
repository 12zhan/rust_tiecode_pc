package org.xiaoa.jna.tiec

import androidx.annotation.IntDef
import com.google.gson.Gson
import com.google.gson.JsonElement
import com.google.gson.annotations.SerializedName
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer


/** Tiecode C API 的 JNA 映射接口 */
interface TiecodeInterface : Library {
    companion object {
        val INSTANCE: TiecodeInterface =
                Native.load(
                        "tiec",
                        TiecodeInterface::class.java,
                        mapOf(
                                // 强制 JNA 将 Java String 转换为 UTF-8 编码的 C 字符串
                                Library.OPTION_STRING_ENCODING to "UTF-8"
                        )
                )
    }

    // ========================================================================
    // Context & Compiler APIs (编译器上下文与编译器相关接口)
    // ========================================================================

    /** 传入 Options 组件的 json 创建编译器上下文 */
    fun tc_create_context(optionsJson: String): Pointer?

    /** 销毁编译器上下文实例 */
    fun tc_free_context(contextHandle: Pointer?): Int

    /** 传入编译器上下文句柄创建编译器 */
    fun tc_create_compiler(contextHandle: Pointer?): Pointer?

    /** 编译指定源文件 */
    fun tc_compiler_compile_files(
            compilerHandle: Pointer?,
            fileCount: Int,
            files: Array<String>
    ): Int

    /** 销毁编译器实例 */
    fun tc_free_compiler(compilerHandle: Pointer?): Int

    // ========================================================================
    // IDE Service APIs (IDE 服务相关接口)
    // ========================================================================

    /** 传入编译器上下文句柄创建 IDE 服务 */
    fun tc_create_ide_service(contextHandle: Pointer?): Pointer?

    /** 为 IDE 服务预编译所有源文件 */
    fun tc_ide_service_compile_files(
            ideServiceHandle: Pointer?,
            fileCount: Int,
            files: Array<String>
    ): Int

    /** 通知 IDE 服务某个源文件内容发生变化（全量更新） */
    fun tc_ide_service_edit_source(
            ideServiceHandle: Pointer?,
            uri: String,
            initialText: String
    ): Int

    /** 通知 IDE 服务某个源文件内容发生变化（增量更新） */
    fun tc_ide_service_edit_source_incremental(
            ideServiceHandle: Pointer?,
            uri: String,
            changeJson: String
    ): Int

    /** 通知 IDE 服务有新文件创建 */
    fun tc_ide_service_create_source(
            ideServiceHandle: Pointer?,
            uri: String,
            initialText: String
    ): Int

    /** 通知 IDE 服务有文件被删除 */
    fun tc_ide_service_delete_source(ideServiceHandle: Pointer?, uri: String): Int

    /** 通知 IDE 服务有文件被重命名 */
    fun tc_ide_service_rename_source(ideServiceHandle: Pointer?, uri: String, newUri: String): Int

    /** 请求代码补全 */
    fun tc_ide_service_complete(ideServiceHandle: Pointer?, paramsJson: String): String?

    /** 请求光标悬停信息 */
    fun tc_ide_service_hover(ideServiceHandle: Pointer?, paramsJson: String): String?

    /** 请求指定文件代码查错 */
    fun tc_ide_service_lint_file(ideServiceHandle: Pointer?, uri: String): String?

    /** 请求全项目代码查错 */
    fun tc_ide_service_lint_all(ideServiceHandle: Pointer?): String?

    /** 请求语义高亮 */
    fun tc_ide_service_highlight(ideServiceHandle: Pointer?, uri: String): String?

    /** 转到定义 */
    fun tc_ide_service_find_definition(ideServiceHandle: Pointer?, paramsJson: String): String?

    /** 查找引用 */
    fun tc_ide_service_find_references(ideServiceHandle: Pointer?, paramsJson: String): String?

    /** 获取光标处要重命名符号的信息 */
    fun tc_ide_service_prepare_rename(ideServiceHandle: Pointer?, paramsJson: String): String?

    /** 在光标处执行重命名 */
    fun tc_ide_service_rename(
            ideServiceHandle: Pointer?,
            paramsJson: String,
            newName: String
    ): String?

    /** 判断光标处类是否支持组件布局 */
    fun tc_ide_service_support_ui_binding(ideServiceHandle: Pointer?, paramsJson: String): Boolean

    /** 获取光标处类的组件布局信息（仅安卓可用） */
    fun tc_ide_service_get_ui_bindings(
            ideServiceHandle: Pointer?,
            paramsJson: String,
            format: Int
    ): String?

    /** 替换光标处类原有的布局变量为新的 TLY 布局（仅安卓可用） */
    fun tc_ide_service_edit_ui_bindings(
            ideServiceHandle: Pointer?,
            paramsJson: String,
            newTlyData: String,
            format: Int
    ): String?

    /** 取消对 IDE 服务的请求 */
    fun tc_ide_service_cancel(ideServiceHandle: Pointer?): Int

    /** 销毁 IDE 服务实例 */
    fun tc_free_ide_service(ideServiceHandle: Pointer?): Int

    // ========================================================================
    // Mapping & Hash APIs (行号映射与哈希工具)
    // ========================================================================

    /** 从行号映射表创建行号表工具 */
    fun tc_decode_source_mapping(mappingPath: String): Pointer?

    /** 从行号表获取输出名对应的结绳符号原始名称 */
    fun tc_source_mapping_get_name(mappingHandle: Pointer?, outputName: String): String?

    /** 从行号表获取输出文件行号对应的结绳原始行号 */
    fun tc_source_mapping_get_line(
            mappingHandle: Pointer?,
            filename: String,
            lineNumber: Int
    ): String?

    /** 销毁行号表实例 */
    fun tc_free_source_mapping(mappingHandle: Pointer?): Int

    /** 快速计算文件哈希值 */
    fun tc_hash_file(filePath: String): Long

    /** 快速计算文本内容哈希值 */
    fun tc_hash_text(text: String): Long
}

/** 封装类，保持原有的调用习惯，内部自动处理 JSON 序列化与反序列化 */
object TiecodeNative {
    private val api = TiecodeInterface.INSTANCE
    private val gson = Gson()

    /** 创建编译器上下文 */
    fun createContext(options: TCOptions): Pointer? = api.tc_create_context(gson.toJson(options))
    fun freeContext(handle: Pointer?): Int = api.tc_free_context(handle)

    /** 创建编译器 */
    fun createCompiler(contextHandle: Pointer?): Pointer? = api.tc_create_compiler(contextHandle)
    fun compilerCompileFiles(handle: Pointer?, files: Array<String>): Int =
            api.tc_compiler_compile_files(handle, files.size, files)
    fun freeCompiler(handle: Pointer?): Int = api.tc_free_compiler(handle)

    /** IDE 服务操作 */
    fun createIdeService(contextHandle: Pointer?): Pointer? =
            api.tc_create_ide_service(contextHandle)
    fun ideServiceCompileFiles(handle: Pointer?, files: Array<String>): Int =
            api.tc_ide_service_compile_files(handle, files.size, files)
    fun ideServiceEditSource(handle: Pointer?, uri: String, text: String): Int =
            api.tc_ide_service_edit_source(handle, uri, text)
    fun ideServiceEditSourceIncremental(handle: Pointer?, uri: String, change: TCTextChange): Int =
            api.tc_ide_service_edit_source_incremental(handle, uri, gson.toJson(change))
    fun ideServiceCreateSource(handle: Pointer?, uri: String, text: String): Int =
            api.tc_ide_service_create_source(handle, uri, text)
    fun ideServiceDeleteSource(handle: Pointer?, uri: String): Int =
            api.tc_ide_service_delete_source(handle, uri)
    fun ideServiceRenameSource(handle: Pointer?, uri: String, newUri: String): Int =
            api.tc_ide_service_rename_source(handle, uri, newUri)

    /** IDE 功能请求 */
    fun ideServiceComplete(handle: Pointer?, params: TCCompletionParams): String? =
            api.tc_ide_service_complete(handle, gson.toJson(params))
    fun ideServiceHover(handle: Pointer?, params: TCCursorParams): String? =
            api.tc_ide_service_hover(handle, gson.toJson(params))
    fun ideServiceFindDefinition(handle: Pointer?, params: TCCursorParams): String? =
            api.tc_ide_service_find_definition(handle, gson.toJson(params))
    fun ideServiceLintFile(handle: Pointer?, uri: String): String? =
            api.tc_ide_service_lint_file(handle, uri)
    fun ideServiceLintAll(handle: Pointer?): String? = api.tc_ide_service_lint_all(handle)
    fun ideServiceHighlight(handle: Pointer?, uri: String): String? =
            api.tc_ide_service_highlight(handle, uri)

    fun ideServiceFindReferences(handle: Pointer?, params: TCCursorParams): String? =
            api.tc_ide_service_find_references(handle, gson.toJson(params))
    fun ideServicePrepareRename(handle: Pointer?, params: TCCursorParams): String? =
            api.tc_ide_service_prepare_rename(handle, gson.toJson(params))
    fun ideServiceRename(handle: Pointer?, params: TCCursorParams, newName: String): String? =
            api.tc_ide_service_rename(handle, gson.toJson(params), newName)
    fun ideServiceSupportUiBinding(handle: Pointer?, params: TCCursorParams): Boolean =
            api.tc_ide_service_support_ui_binding(handle, gson.toJson(params))
    fun ideServiceGetUiBindings(handle: Pointer?, params: TCCursorParams, format: Int): String? =
            api.tc_ide_service_get_ui_bindings(handle, gson.toJson(params), format)
    fun ideServiceEditUiBindings(
            handle: Pointer?,
            params: TCCursorParams,
            data: String,
            format: Int
    ): String? = api.tc_ide_service_edit_ui_bindings(handle, gson.toJson(params), data, format)
    fun ideServiceCancel(handle: Pointer?): Int = api.tc_ide_service_cancel(handle)
    fun freeIdeService(handle: Pointer?): Int = api.tc_free_ide_service(handle)

    /** 源码映射与哈希 */
    fun decodeSourceMapping(path: String): Pointer? = api.tc_decode_source_mapping(path)
    fun sourceMappingGetName(handle: Pointer?, name: String): String? =
            api.tc_source_mapping_get_name(handle, name)
    fun sourceMappingGetLine(handle: Pointer?, file: String, line: Int): String? =
            api.tc_source_mapping_get_line(handle, file, line)
    fun freeSourceMapping(handle: Pointer?): Int = api.tc_free_source_mapping(handle)
    fun hashFile(path: String): Long = api.tc_hash_file(path)
    fun hashText(text: String): Long = api.tc_hash_text(text)
}

// ========================================================================
// Enums & Data Structures (数据结构定义)
// ========================================================================

/** 错误码 */
enum class TCError(val code: Int) {
    TC_OK(0), // 没有错误
    TC_HANDLE_INVALID(1), // 句柄不合法
    TC_COMPILE_FAILED(2); // 编译失败
    companion object {
        fun fromInt(value: Int) = entries.find { it.code == value } ?: TC_OK
    }
}

/** 任务类型枚举 */
enum class TCTaskKind(val value: Int) {
    TC_PARSE(0),
    TC_ENTER(1),
    TC_ATTRIBUTE(2),
    TC_LOWER(3),
    TC_FINAL(4)
}

/** UI 序列化格式 */
enum class TCSerializeFormat(val value: Int) {
    TLY_FORMAT(0), // tly格式
    JSON_FORMAT(1) // json格式
}

/** 日志等级 */
enum class TCLogLevel(val value: Int) {
    DEBUG(0),
    INFO(1),
    WARNING(2),
    ERROR(3)
}

/** 坐标位置 (行/列) */
data class TCPosition(
        @SerializedName("line") val line: Int,
        @SerializedName("column") val column: Int
)

/** 文本范围 */
data class TCRange(
        @SerializedName("start") val start: TCPosition,
        @SerializedName("end") val end: TCPosition
)

/** 文本变更内容 (增量更新使用) */
data class TCTextChange(
        @SerializedName("range") val range: TCRange?,
        @SerializedName("text") val text: String
)

/**
 * 编译目标平台常量。
 *
 *
 * 使用 `@TCPlatform.Def` 代替裸 `int`，编译期即可检查合法性。
 *
 * @author your name
 * @since 1.0
 */
class TCPlatform private constructor() {

    @IntDef(UNDEFINED, ANDROID, HARMONY, LINUX, WINDOWS, IOS, APPLE, HTML)
    @Retention(AnnotationRetention.SOURCE)
    annotation class Def

    companion object {
        /* 值一旦发布永不复用，顺序与 C/C++ 层保持一致 */
        const val UNDEFINED = 0
        const val ANDROID   = 1
        const val HARMONY   = 2
        const val LINUX     = 3
        const val WINDOWS   = 4
        const val IOS       = 5
        const val APPLE     = 6   // macOS、Mac Catalyst
        const val HTML      = 7   // WebAssembly / Emscripten

        /**
         * 将平台常量转换为可读字符串，仅供日志/调试使用。
         */
        fun toName(@Def platform: Int): String = when (platform) {
            UNDEFINED -> "undefined"
            ANDROID   -> "android"
            HARMONY   -> "harmony"
            LINUX     -> "linux"
            WINDOWS   -> "windows"
            IOS       -> "ios"
            APPLE     -> "apple"
            HTML      -> "html"
            else      -> "unknown($platform)"
        }
    }
}
/** 编译器配置项 */
data class TCOptions(
    @SerializedName("packageName")
        val packageName: String? = "com.example.app", // 默认包名，建议设置为项目包名
    @SerializedName("outputDir") val outputDir: String, // 编译输出目录 (必选)
    @SerializedName("lineMapPath") val lineMapPath: String? = null, // 行号映射表输出路径
    @SerializedName("debug") val debug: Boolean = false, // 是否为 debug 模式
    @SerializedName("ideMode") val ideMode: Boolean = false, // IDE 模式
    @SerializedName("optimizeLevel") val optimizeLevel: Int = 0, // 编译优化级别 0-3
    @SerializedName("lintDisable") val lintDisable: List<String> = emptyList(), // 要屏蔽的 Lint 检查
    @SerializedName("logLevel") val logLevel: Int = 1, // 日志等级
    @SerializedName("platform") val platform: Int = TCPlatform.UNDEFINED, // 目标输出平台 (1:ANDROID, 2:HARMONY, etc.)
    @SerializedName("emitNamesPath") val emitNamesPath: String? = null, // 稳定名称映射表输出路径
    @SerializedName("stableNamesPath") val stableNamesPath: String? = null, // 稳定名称映射表读取路径
    @SerializedName("searchPrefixes") val searchPrefixes: TCSearchPrefixes? = null, // 附加搜寻路径
    @SerializedName("android") val android: TCAndroidConfig? = null // Android 特殊配置
)

/** 附加搜寻路径 */
data class TCSearchPrefixes(
        @SerializedName("lib") val lib: List<String> = emptyList(), // 外部依赖库
        @SerializedName("source") val source: List<String> = emptyList(), // 外部源文件
        @SerializedName("res") val res: List<String> = emptyList(), // 安卓资源
        @SerializedName("assets") val assets: List<String> = emptyList() // 资产文件
)

/** Android 专用配置 */
data class TCAndroidConfig(
        @SerializedName("appConfig") val appConfig: TCAppConfig?,
        @SerializedName("gradle") val gradle: Boolean = false, // 是否以 gradle 工程格式输出
        @SerializedName("foundationLibPath") val foundationLibPath: String? = null // 安卓基本库路径
)

/** Android App 详情配置 */
data class TCAppConfig(
        @SerializedName("appName") val appName: String,
        @SerializedName("appIcon") val appIcon: String,
        @SerializedName("minSdk") val minSdk: Int,
        @SerializedName("targetSdk") val targetSdk: Int,
        @SerializedName("versionCode") val versionCode: Int,
        @SerializedName("versionName") val versionName: String
)

/** 代码补全请求参数 */
data class TCCompletionParams(
        @SerializedName("uri") val uri: String,
        @SerializedName("position") val position: TCPosition,
        @SerializedName("lineText") val lineText: String? = null,
        @SerializedName("partial") val partial: String, // 触发补全的前缀文本
        @SerializedName("triggerChar") val triggerChar: String? = null // 触发补全的字符
)

/** 光标位置相关请求参数 */
data class TCCursorParams(
        @SerializedName("uri") val uri: String,
        @SerializedName("position") val position: TCPosition,
        @SerializedName("lineText") val lineText: String? = null
)

/** 代码查错 (Lint) 结果 */
data class TCLintResult(@SerializedName("diagnostics") val diagnostics: List<TCDiagnostic>)

data class TCDiagnostic(
        @SerializedName("uri") val uri: String,
        @SerializedName("range") val range: TCRange,
        @SerializedName("key") val key: String?, // 诊断信息 key，可用于 QuickFix
        @SerializedName("message") val message: String,
        @SerializedName("level") val level: Int // 日志等级
)

/** 代码补全结果 */
data class TCCompletionResult(@SerializedName("items") val items: List<TCCompletionItem>)

data class TCCompletionItem(
        @SerializedName("kind") val kind: Int,
        @SerializedName("label") val label: String, // 符号名称
        @SerializedName("detail") val detail: String?, // 符号详细描述 (如方法签名)
        @SerializedName("sortKey") val sortKey: String?, // 用于排序的 key
        @SerializedName("symbolName") val symbolName: String?, // 符号原名
        @SerializedName("insertText") val insertText: String, // 实际插入内容
        @SerializedName("extraEdits") val extraEdits: List<TCTextEdit>? = null
)

/** 文本编辑操作 */
data class TCTextEdit(
        @SerializedName("range") val range: TCRange,
        @SerializedName("newText") val newText: String
)

/** 悬停 Markup 内容 */
data class TCMarkupContent(
        @SerializedName("kind") val kind: Int,
        @SerializedName("text") val text: String
)

/** 高亮结果 */
data class TCHighlightResult(@SerializedName("highlights") val highlights: List<TCHighlightItem>)

data class TCHighlightItem(
        @SerializedName("range") val range: TCRange,
        @SerializedName("kind") val kind: Int,
        @SerializedName("isStatic") val isStatic: Boolean
)

/** 源码位置描述 */
data class TCLocation(
        @SerializedName("uri") val uri: String,
        @SerializedName("range") val range: TCRange
)

data class TCReferenceResult(@SerializedName("locations") val locations: List<TCLocation>)

/** 重命名准备阶段信息 */
data class TCRenameSymbolInfo(
        @SerializedName("name") val name: String,
        @SerializedName("range") val range: TCRange,
        @SerializedName("kind") val kind: Int
)

/** 重命名结果 (跨文件编辑) */
data class TCRenameResult(
        @SerializedName("projectEdit") val projectEdit: Map<String, List<TCTextEdit>>
)

/** 映射回原始源码的位置 */
data class TCSourceLocation(
        @SerializedName("path") val path: String,
        @SerializedName("line") val line: Int
)

/** UI 布局绑定节点 (JSON 格式) */
data class TCUIBindingNode(
        @SerializedName("class") val clazz: TCClassName,
        @SerializedName("nameProp") val nameProp: TCUIProp?, // 名称属性
        @SerializedName("properties") val properties: List<TCUIProp>?, // 其它属性列表
        @SerializedName("children") val children: List<TCUIBindingNode>? // 子组件
)

data class TCClassName(@SerializedName("className") val className: String)

data class TCUIProp(
        @SerializedName("propName") val propName: TCUIPropName,
        @SerializedName("propValue") val propValue: TCUIPropValue
)

data class TCUIPropName(@SerializedName("name") val name: String)

data class TCUIPropValue(@SerializedName("value") val value: JsonElement?)

/** UI 编辑后的代码变更 */
data class TCUIBindingEditResult(@SerializedName("edits") val edits: List<TCTextEdit>)
