#ifndef TIEC_C_API_H
#define TIEC_C_API_H
#include <cstddef>
#include <cstdint>

#if defined(WINDOWS) || defined(_WIN32) || defined(_WIN64)
#ifdef TIEC_EXPORT
#define TIEC_API __declspec(dllexport)
#else
#define TIEC_API __declspec(dllimport)
#endif
#else
#define TIEC_API __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C"
{
#endif

    /// 错误码
    typedef enum tc_error
    {
        TC_OK = 0,             // 没有错误
        TC_HANDLE_INVALID = 1, // 句柄不合法
        TC_COMPILE_FAILED = 2, // 编译失败
        TC_IO_ERR = 3,         // 文件IO错误
    } tc_error_t;

    /// TaskKind枚举
    typedef enum tc_task_kind
    {
        TC_PARSE = 0, // 解析语法树阶段
        TC_ENTER,     // 符号表填充阶段
        TC_ATTRIBUTE, // 语法树标注阶段
        TC_LOWER,     // 语法树低级化阶段
        TC_FINAL      // 最终输出阶段
    } tc_task_kind_t;

    /// Source::getName
    typedef const char *(*tc_source_get_name)();
    /// Source::lastModified
    typedef uint64_t (*tc_source_last_modified)();
    /// Source::readContent
    typedef const char *(*tc_source_read_content)();
    /// Source::getUri
    typedef const char *(*tc_source_get_uri)();
    /// Source::getPath
    typedef const char *(*tc_source_get_path)();
    /// TaskListener::onTaskBegin
    typedef void (*tc_task_on_begin)(tc_task_kind_t task_kind);
    /// TaskListener::onTaskEnd
    typedef void (*tc_task_on_end)(tc_task_kind_t task_kind);
    /// DiagnosticHandler::report
    /// 其中 diagnostic_json 格式如下:
    /// @code
    /// {
    ///   "uri": "文件URI",
    ///   "range": {
    ///     "start": {
    ///       "line": 行号,
    ///       "column": 列号
    ///     },
    ///     "end": {
    ///       "line": 行号,
    ///       "column": 列号
    ///     }
    ///   },
    ///   "key": "诊断信息的key，可用于做QuickFix",
    ///   "message": "诊断信息文本",
    ///   "level": 日志等级
    /// }
    /// @endcode
    typedef void (*tc_diagnostic_report)(const char *diagnostic_json);

    /// Source
    typedef struct tc_source
    {
        tc_source_get_name get_name;           // 获取源文件名称
        tc_source_last_modified last_modified; // 获取源文件最后一次修改时间
        tc_source_read_content read_content;   // 读取源文件内容
        tc_source_get_uri get_uri;             // 获取源文件Uri
        tc_source_get_path get_path;           // 获取源文件文件路径
    } tc_source_t;

    /// TaskListener
    typedef struct tc_task_listener
    {
        tc_task_on_begin on_begin; // 阶段任务开始
        tc_task_on_end on_end;     // 阶段任务结束
    } tc_task_listener_t;

    /// DiagnosticHandler
    typedef struct tc_diagnostic_handler
    {
        tc_diagnostic_report report; // 报告一个编译器诊断信息
    } tc_diagnostic_handler_t;

    /// @see {TLYSerializeFormat}
    typedef enum
    {
        TC_TLY_FORMAT = 0,  // tly格式
        TC_JSON_FORMAT = 1, // json格式
    } tc_tly_format_t;

    /// 各平台编程语言文件类型定义
    typedef enum
    {
        TC_DECLARATION_JAVA = 0,       // java文件
        TC_DECLARATION_CPP_HEADER = 1, // c++头文件
        TC_DECLARATION_JS = 2,         // js文件
    } tc_declaration_kind_t;

    /// 传入Options组件的json创建编译器上下文
    /// @param options_json Options序列化json，json格式如下
    /// @code
    /// {
    ///   "packageName": "默认包名", //可选，不过建议设置为项目包名
    ///   "outputDir": "编译输出目录", //编译时必选, IDE服务可选
    ///   "lineMapPath": "行号映射表输出路径", //可选
    ///   "hardMode": true/false, //可选，指定是否为硬输出模式，硬输出模式下会将所有依赖库拷贝到输出目录，而不是提供文件路径引用
    ///   "debug": true/false, //是否为debug模式, 可选
    ///   "enableTopLevelStmt": true/false, //是否开启顶级语句语法特性支持, 可选
    ///   "friendlyName": 符号输出名称模式(整数), //0:RANDOM, 1:PINYIN, 2:ORIGINAL
    ///   "ideMode": IDE模式时传true,
    ///   "profile": 编译部署模式(整数), // 0: 标准模式, 1: 部署设计器模式
    ///   "optimizeLevel": 编译优化级别，0-3, 0为不优化, 可选
    ///   "lintDisable": ["auto.type-cast", "...要屏蔽的Lint检查"], //可选
    ///   "logLevel": 日志等级(整数), 0:DEBUG, 1:INFO, 2:WARNING, 3:ERROR
    ///   "platform": 目标输出平台(整数)，0:UNDEFINED, 1:ANDROID, 2:HARMONY, 3:LINUX, 4:WINDOWS, 5:IOS, 6:APPLE, 7:HTML
    ///   "emitNamesPath": "稳定名称映射表输出路径", //可选
    ///   "stableNamesPath": "稳定名称映射表读取路径", //可选
    ///   "searchPrefixes": { //(可选)附加的文件搜寻路径(外部依赖库/外部源文件等)，默认是基于源代码文件目录进行相对路径搜寻
    ///     "lib": ["/path/dir1", "/path/dir2"], //外部依赖库附加搜寻路径数组
    ///     "source": ["/path/dir1", "/path/dir2"], //外部源文件附加搜寻路径数组
    ///     "res": ["/path/dir1", "/path/dir2"], //安卓资源附加搜寻路径数组
    ///     "assets": ["/path/dir1", "/path/dir2"], //附加资产文件附加搜寻路径数组
    ///   },
    ///   "android": { //目标输出平台为Android时的特殊配置，其他平台无需配置
    ///     "appConfig": {
    ///       "appName": "App的名称",
    ///       "appIcon": "App的启动图标路径",
    ///       "minSdk": minSdkVersion,
    ///       "targetSdk": targetSdkVersion,
    ///       "versionCode": 版本号,
    ///       "versionName": "版本名称"
    ///     },
    ///     "gradle": 是否以gradle工程格式输出, //默认为false
    ///     "foundationLibPath": "安卓基本库路径" //用于兼容安卓基本库androidx依赖库版本混乱问题
    ///   }
    /// }
    /// @endcode
    /// @return 编译器上下文句柄
    TIEC_API intptr_t tc_create_context(const char *options_json);

    /// 销毁编译器上下文实例
    /// @param context_handle 编译器上下文句柄
    /// @return 返回错误码，销毁成功返回TC_OK
    TIEC_API tc_error_t tc_free_context(intptr_t context_handle);

    /// 传入编译器上下文句柄创建编译器
    /// @param context_handle 编译器上下文句柄
    /// @return 编译器句柄
    TIEC_API intptr_t tc_create_compiler(intptr_t context_handle);

    /// 设置编译器自定义处理代码报错的逻辑
    /// @param compiler_handle 编译器句柄
    /// @param diagnostic_handler tc_diagnostic_handler
    /// @return 错误码
    TIEC_API tc_error_t tc_compiler_set_diagnostic_handler(intptr_t compiler_handle, tc_diagnostic_handler_t diagnostic_handler);

    /// 设置编译器自定义Task监听逻辑
    /// @param compiler_handle 编译器句柄
    /// @param task_listener tc_task_listener
    /// @return 错误码
    TIEC_API tc_error_t tc_compiler_add_task_listener(intptr_t compiler_handle, tc_task_listener_t task_listener);

    /// 编译指定源文件
    /// @param compiler_handle 编译器句柄
    /// @param file_count 源文件数量
    /// @param files 源文件路径数组
    /// @return 错误码
    TIEC_API tc_error_t tc_compiler_compile_files(intptr_t compiler_handle, size_t file_count, const char *const *files);

    /// 编译指定源文件（自定义Source）
    /// @param compiler_handle 编译器句柄
    /// @param source_count 源文件数量
    /// @param sources 源文件数组
    /// @return 错误码
    TIEC_API tc_error_t tc_compiler_compile_sources(intptr_t compiler_handle, size_t source_count, tc_source_t *sources);

    /// 销毁编译器实例
    /// @param compiler_handle 编译器句柄
    /// @return 返回错误码，销毁成功返回TC_OK
    TIEC_API tc_error_t tc_free_compiler(intptr_t compiler_handle);

    /// 传入编译器上下文句柄创建IDE服务
    /// @param context_handle 编译器上下文句柄
    /// @return IDEService 句柄
    TIEC_API intptr_t tc_create_ide_service(intptr_t context_handle);

    /// 为IDE服务预编译所有源文件，首次打开项目时必须调用编译项目中所有源文件，该函数与 @see {tc_ide_service_compile_sources} 二选一
    /// @param ide_service_handle IDEService 句柄
    /// @param file_count 源文件数量
    /// @param files 源文件路径
    /// @return 错误码
    TIEC_API tc_error_t tc_ide_service_compile_files(intptr_t ide_service_handle, size_t file_count, const char *const *files);

    /// 为IDE服务预编译所有源文件（自定义Source），首次打开项目时必须调用编译项目中所有源文件，该函数与 @see {tc_ide_service_compile_files} 二选一
    /// @param ide_service_handle IDEService 句柄
    /// @param source_count 源文件数量
    /// @param sources 源文件数组
    /// @return 错误码
    TIEC_API tc_error_t tc_ide_service_compile_sources(intptr_t ide_service_handle, size_t source_count, tc_source_t *sources);

    /// 通知IDE服务某个源文件内容发生变化（全量更新），等同于 IDEService::didChangeSource
    /// @param ide_service_handle IDEService句柄
    /// @param uri 源文件的Uri
    /// @param new_text 源文件改变后的内容
    /// @return 错误码
    TIEC_API tc_error_t tc_ide_service_edit_source(intptr_t ide_service_handle, const char *uri, const char *new_text);

    /// 通知IDE服务某个源文件内容发生变化（增量更新），等同于 IDEService::didChangeSourceIncremental
    /// @param ide_service_handle IDEService句柄
    /// @param uri 源文件的Uri
    /// @param change_json 增量变更的数据(TextChange序列化后的json),json格式如下
    /// @code
    /// {
    ///   "range": { // 变更区域
    ///     "start": {
    ///       "line": 行号,
    ///       "column": 列号
    ///     },
    ///     "end": {
    ///       "line": 行号,
    ///       "column": 列号
    ///     }
    ///   },
    ///   "newText": "变更后的文本"
    /// }
    /// @endcode
    /// @return 错误码
    TIEC_API tc_error_t tc_ide_service_edit_source_incremental(intptr_t ide_service_handle, const char *uri, const char *change_json);

    /// 通知IDE服务有新文件创建，等同于 IDEService::didCreateSource
    /// @param ide_service_handle IDEService句柄
    /// @param uri 源文件的Uri
    /// @param initial_text 源文件初始内容
    /// @return 错误码
    TIEC_API tc_error_t tc_ide_service_create_source(intptr_t ide_service_handle, const char *uri, const char *initial_text);

    /// 通知IDE服务有新文件删除，等同于 IDEService::didDeleteSource
    /// @param ide_service_handle IDEService句柄
    /// @param uri 源文件的Uri
    /// @return 错误码
    TIEC_API tc_error_t tc_ide_service_delete_source(intptr_t ide_service_handle, const char *uri);

    /// 通知IDE服务有文件被重命名，等同于 IDEService::didRenameSource
    /// @param ide_service_handle IDEService句柄
    /// @param uri 源文件的Uri
    /// @param new_uri 新的Uri
    /// @return 错误码
    TIEC_API tc_error_t tc_ide_service_rename_source(intptr_t ide_service_handle, const char *uri, const char *new_uri);

    /// 请求代码补全，等同于 IDEService::complete
    /// @param ide_service_handle IDEService句柄
    /// @param params_json CompletionParams序列化后的json， json格式如下
    /// @code
    /// {
    ///   "uri": "文件URI",
    ///   "position": {
    ///     "line": 光标所处行,
    ///     "column": 光标所处列
    ///   },
    ///   "lineText": "当前行文本", //一般用不到，可以不传
    ///   "partial": "当前触发代码补全的前缀文本",
    ///   "triggerChar": "当前触发代码补全的字符"
    /// }
    /// @endcode
    /// @return 代码补全结果(CompletionResult序列化后的json)，json格式如下
    /// @code
    /// {
    ///   "items": [
    ///     {
    ///       "kind": CompletionItemKind,
    ///       "label": "符号名称",
    ///       "detail": "符号详细描述(如方法签名)",
    ///       "sortKey": "用于排序的key",
    ///       "symbolName": "符号名称，用于IDE统计符号使用频率，智能排序",
    ///       "insertText": "实际要插入到IDE编辑器中的内容",
    ///       "extraEdits": [ // 该字段不一定有
    ///         {
    ///           "range": {
    ///             "start": {
    ///               "line": 行号,
    ///               "column": 列号
    ///             },
    ///             "end": {
    ///               "line": 行号,
    ///               "column": 列号
    ///             }
    ///           },
    ///           "newText": "替换后文本"
    ///         }
    ///         ...
    ///       ]
    ///     }
    ///     ...
    ///   ]
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_complete(intptr_t ide_service_handle, const char *params_json);

    /// 请求光标悬停信息，等同于 IDEService::hover
    /// @param ide_service_handle IDEService句柄
    /// @param params_json CursorParams序列化后的json, json格式如下
    /// {
    ///   "uri": "文件URI",
    ///   "position": {
    ///     "line": 光标所处行,
    ///     "column": 光标所处列
    ///   },
    ///   "lineText": "当前行文本", //一般用不到，可以不传
    /// }
    /// @return 光标处符号信息结果(MarkupContent序列化后的json),json格式如下
    /// @code
    /// {
    ///   "kind": MarkupKind,
    ///   "text": markdown文本或纯文本（与kind相关）
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_hover(intptr_t ide_service_handle, const char *params_json);

    /// 请求代码查错，等同于 IDEService::lintFile
    /// @param ide_service_handle IDEService句柄
    /// @param uri 文件Uri
    /// @return 代码查错结果(LintResult序列化后的json),json格式如下
    /// @code
    /// {
    ///   "diagnostics": [
    ///     {
    ///       "uri": "文件URI",
    ///       "range": {
    ///         "start": {
    ///           "line": 行号,
    ///           "column": 列号
    ///         },
    ///         "end": {
    ///           "line": 行号,
    ///           "column": 列号
    ///         }
    ///       },
    ///       "key": "编译器错误的key",
    ///       "message": "错误信息",
    ///       "level": LogLevel
    ///     }
    ///     ...
    ///   ]
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_lint_file(intptr_t ide_service_handle, const char *uri);

    /// 请求代码查错，等同于 IDEService::lintAll
    /// @param ide_service_handle IDEService句柄
    /// @return 代码查错结果(LintResult序列化后的json),json格式与 @see {tc_ide_service_lint_file} 格式一致
    TIEC_API const char *tc_ide_service_lint_all(intptr_t ide_service_handle);

    /// 请求语义高亮，等同于 IDEService::semanticHighlight
    /// @param ide_service_handle IDEService句柄
    /// @param uri 文件Uri
    /// @return 语义高亮结果(HighlightResult序列化后的json), json格式如下
    /// @code
    /// {
    ///   "highlights": [
    ///     {
    ///       "range": {
    ///         "start": {
    ///           "line": 行号,
    ///           "column": 列号
    ///         },
    ///         "end": {
    ///           "line": 行号,
    ///           "column": 列号
    ///         }
    ///       },
    ///       "kind": ElementKind,
    ///       "tags": [1, 2] // 高亮符号的tag（附加属性），如静态、废弃等，参见 ide_service.h->ElementTag
    ///     }
    ///     ...
    ///   ]
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_highlight(intptr_t ide_service_handle, const char *uri);

    /// 请求对指定文件增量格式化，等同于 IDEService::format
    /// @param ide_service_handle IDEService句柄
    /// @param uri 文件Uri
    /// @return 格式化结果(FormattingResult 序列化后的json), json格式如下
    /// @code
    /// {
    ///   "edits": [
    ///     {
    ///       "range": {
    ///         "start": {
    ///           "line": 行号,
    ///           "column": 列号
    ///         },
    ///         "end": {
    ///           "line": 行号,
    ///           "column": 列号
    ///         }
    ///       },
    ///       "newText": "\t\t..." //空格/tab替换为指定数量的tab
    ///     }
    ///     ...
    ///   ]
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_format(intptr_t ide_service_handle, const char *uri);

    /// 请求获取指定文件的符号嵌套结构(类->方法/变量->...)，包含每个符号的详细信息（符号名称、类型、定义位置等），等同于 IDEService::sourceElements
    /// @param ide_service_handle IDEService句柄
    /// @param uri 文件Uri
    /// @return 符号嵌套结果(SourceElementsResult 序列化后的json), json格式如下
    /// @code
    /// {
    ///   "elements": [
    ///     {
    ///       "element": {
    ///         "kind": ElementKind, // 参见ide_service.h -> enum struct ElementKind
    ///         "tags": [ElementTag...], // 参见ide_service.h -> enum struct ElementTag
    ///         "name": "符号名称",
    ///         "detail": "符号详细信息", // 如方法签名、类包名等
    ///         "range": { // 符号定义的完整位置
    ///           "start": {
    ///             "line": 行号,
    ///             "column": 列号
    ///           },
    ///           "end": {
    ///             "line": 行号,
    ///             "column": 列号
    ///           }
    ///         },
    ///         "identifierRange": { // 符号标识符位置
    ///           "start": {
    ///             "line": 行号,
    ///             "column": 列号
    ///           },
    ///           "end": {
    ///             "line": 行号,
    ///             "column": 列号
    ///           }
    ///         }
    ///       },
    ///       "children": [] //子节点，对象结构同elements数组节点
    ///     }
    ///     ...
    ///   ]
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_source_elements(intptr_t ide_service_handle, const char *uri);

    /// 通过关键词搜索整个项目中结绳源代码符号，等同于 IDEService::workspaceElements
    /// @param ide_service_handle IDEService句柄
    /// @param keyword 搜索关键词
    /// @return 包含搜索关键词的所有符号结果信息(WorkspaceElementsResult 序列化后的json), json格式如下
    /// @code
    /// {
    ///   "elements": {
    ///     "file:///xxx/A.t": [ // elements的key为文件Uri
    ///       {
    ///         "kind": ElementKind, // 参见ide_service.h -> enum struct ElementKind
    ///         "tags": [ElementTag...], // 参见ide_service.h -> enum struct ElementTag
    ///         "name": "符号名称",
    ///         "detail": "符号详细信息", // 如方法签名、类包名等
    ///         "range": { // 符号定义的完整位置
    ///           "start": {
    ///             "line": 行号,
    ///             "column": 列号
    ///           },
    ///           "end": {
    ///             "line": 行号,
    ///             "column": 列号
    ///           }
    ///         },
    ///         "identifierRange": { // 符号标识符位置
    ///           "start": {
    ///             "line": 行号,
    ///             "column": 列号
    ///           },
    ///           "end": {
    ///             "line": 行号,
    ///             "column": 列号
    ///           }
    ///         }
    ///       },
    ///       ... // 每个Uri对应多个符号信息
    ///     ],
    ///     "file:///xxx/B.t": [..], //第2个文件的符号信息
    ///     "file:///xxx/...": [..], //第N个文件的符号信息
    ///   }
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_workspace_elements(intptr_t ide_service_handle, const char *keyword);

    /// 请求方法签名帮助信息，等同于 IDEService::signatureHelp
    /// @param ide_service_handle IDEService句柄
    /// @param params_json SignatureHelpParams序列化后的json, json格式如下
    /// @code
    /// {
    ///   "uri": "文件URI",
    ///   "position": {
    ///     "line": 光标所处行,
    ///     "column": 光标所处列
    ///   },
    ///   "triggerChar": "当前触发方法签名帮助的字符" //一般为 '('或','
    /// }
    /// @endcode
    /// @return 方法签名帮助信息(SignatureHelpParams序列化后的json),json格式如下
    /// @code
    /// {
    ///   "signature": "方法签名", //示例: "取参数信息(参数1: 文本, 参数2: 整数): 文本"
    ///   "activeParameter": "当前所处参数签名" //示例: "参数2: 整数", 可通过字符串查找对方法签名中对应参数进行高亮/加粗显示
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_signature_help(intptr_t ide_service_handle, const char *params_json);

    /// 转到定义，等同于 IDEService::findDefinition
    /// @param ide_service_handle IDEService句柄
    /// @param params_json CursorParams序列化后的json，与 @see {tc_ide_service_hover} 的 params_json 一致
    /// @return 符号定义描述信息(Location序列化后的json),json格式如下
    /// @code
    /// {
    ///   "identifierRange": { //光标处标识符位置
    ///     "start": {
    ///       "line": 行号,
    ///       "column": 列号
    ///     },
    ///     "end": {
    ///       "line": 行号,
    ///       "column": 列号
    ///     }
    ///   },
    ///   "location": {
    ///     "uri": "文件URI",
    ///     "range": {
    ///       "start": {
    ///         "line": 行号,
    ///         "column": 列号
    ///       },
    ///       "end": {
    ///         "line": 行号,
    ///         "column": 列号
    ///       }
    ///     }
    ///   }
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_find_definition(intptr_t ide_service_handle, const char *params_json);

    /// 转到定义，等同于 IDEService::findReferences
    /// @param ide_service_handle IDEService句柄
    /// @param params_json CursorParams序列化后的json，与 @see {tc_ide_service_hover} 的 params_json 一致
    /// @return 符号引用描述信息(ReferenceResult序列化后的json),json格式如下
    /// @code
    /// {
    ///   "identifierRange": { //光标处标识符位置
    ///     "start": {
    ///       "line": 行号,
    ///       "column": 列号
    ///     },
    ///     "end": {
    ///       "line": 行号,
    ///       "column": 列号
    ///     }
    ///   },
    ///   "locations": [
    ///     {
    ///       "uri": "文件URI",
    ///       "range": {
    ///         "start": {
    ///           "line": 行号,
    ///           "column": 列号
    ///         },
    ///         "end": {
    ///           "line": 行号,
    ///           "column": 列号
    ///         }
    ///       }
    ///     }
    ///     ...
    ///   ]
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_find_references(intptr_t ide_service_handle, const char *params_json);

    /// 获取光标处要重命名符号的信息，等同于 IDEService::getRenameSymbolInfo
    /// @param ide_service_handle IDEService句柄
    /// @param params_json CursorParams序列化后的json，与 @see {tc_ide_service_hover} 的 params_json 一致
    /// @return 符号重命名信息(RenameSymbolInfo序列化后的json),json格式如下
    /// @code
    /// {
    ///   "name": "光标处符号名称",
    ///   "range": {
    ///     "start": {
    ///       "line": 行号,
    ///       "column": 列号
    ///     },
    ///     "end": {
    ///       "line": 行号,
    ///       "column": 列号
    ///     }
    ///   },
    ///   "kind": ElementKind
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_prepare_rename(intptr_t ide_service_handle, const char *params_json);

    /// 在光标处执行重命名，等同于 IDEService::rename
    /// @param ide_service_handle IDEService句柄
    /// @param params_json CursorParams序列化后的json，与 @see {tc_ide_service_hover} 的 params_json 一致
    /// @param new_name 重命名名称
    /// @return 符号重命名结果(RenameResult序列化后的json),json格式如下
    /// @code
    /// {
    ///   "projectEdit": {
    ///     "文件URI1": [
    ///       {
    ///         "range": {
    ///           "start": {
    ///             "line": 行号,
    ///             "column": 列号
    ///           },
    ///           "end": {
    ///             "line": 行号,
    ///             "column": 列号
    ///           }
    ///         },
    ///         "newText": "替换后文本"
    ///       }
    ///       ...
    ///     ],
    ///     "文件URI2": ...
    ///   }
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_rename(intptr_t ide_service_handle, const char *params_json, const char *new_name);

    /// 获取光标处智能键入信息（如文件选择、常量值选择、switch开关等），等同于 IDEService::smartEnter
    /// @param ide_service_handle IDEService句柄
    /// @param params_json CursorParams序列化后的json，与 @see {tc_ide_service_hover} 的 params_json 一致
    /// @return 智能键入信息(SmartEnterResult 序列化后的json),json格式如下
    /// @code
    /// {
    ///   "kind": SmartEnterKind(整数枚举). 0:未知类型, 1:选择文件, 2:选择枚举常量, 3:真/假开关
    ///   "range": { //要替换/插入文本的位置
    ///     "start": {
    ///       "line": 行号,
    ///       "column": 列号
    ///     },
    ///     "end": {
    ///       "line": 行号,
    ///       "column": 列号
    ///     }
    ///   },
    ///   "replaceFormat": "用于替换的文本格式", //这是一个需要格式化的字符串，会包含 %s 用于表示被替换的内容（路径/常量类型值/布尔值），
    ///     //比如为注解选择文件，也许这个格式为 "@外部依赖库(\"%s\")"，需要格式化后原封不动的去替换range之间的内容
    ///     //再比如为某个属性选择常量类型值，也许这个格式为 "文本1.对齐方式 = %s"，也有可能是 " = %s"或者直接"%s"，都需要格式化后原封不动的去替换range之间的内容
    ///   ”enums": ["枚举值1", "枚举值2"], //仅当 kind 为 2(选择枚举常量)时有该字段
    ///   ”isTrue": true/false, //仅当 kind 为 3(真/假开关)时有该字段
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_smart_enter(intptr_t ide_service_handle, const char *params_json);

    /// 为光标处所处[变量/类]生成[事件/虚拟方法]，等同于 IDEService::generateEvent
    /// @param ide_service_handle IDEService句柄
    /// @param params_json CursorParams序列化后的json，与 @see {tc_ide_service_hover} 的 params_json 一致
    /// @return 插入事件/虚拟方法的信息(CodeActionResult 序列化后的json),json格式如下
    /// @code
    /// {
    ///   "actions": [
    ///     {
    ///       "title": "Action的标题",
    ///       "edits": [
    ///         {
    ///           "range": { //一般 start和end一致，在指定位置处插入事件/虚拟方法
    ///             "start": {
    ///               "line": 行号,
    ///               "column": 列号
    ///             },
    ///             "end": {
    ///               "line": 行号,
    ///               "column": 列号
    ///             }
    ///           },
    ///           "newText": "插入的文本"
    ///         }
    ///         ...
    ///       ]
    ///     }
    ///     ...
    ///   ]
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_generate_event(intptr_t ide_service_handle, const char *params_json);

    /// 判断光标处所有类是否支持组件布局，等同于 IDEService::supportUIBinding
    /// @param ide_service_handle IDEService句柄
    /// @param params_json CursorParams序列化后的json，与 @see {tc_ide_service_hover} 的 params_json 一致
    /// @return 返回当前所处类是否支持组件布局设计的相关信息 (UIBindingSupportInfo 序列化后的json), json格式如下:
    /// @code
    /// {
    ///   "isSupport": true, // 是否支持组件布局设计
    ///   "element": { // 当前所处类信息，结构与 @see {tc_ide_service_source_elements} 中 "element" 格式相同
    ///   }
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_support_ui_binding(intptr_t ide_service_handle, const char *params_json);

    /// 获取光标处所处类的组件布局信息（仅安卓平台可用），等同于 IDEService::getUIBindings
    /// @param ide_service_handle IDEService句柄
    /// @param params_json CursorParams序列化后的json，与 @see {tc_ide_service_hover} 的 params_json 一致
    /// @param format 获取tly的序列化格式(tly格式/json格式)
    /// @return 组件布局信息(tly布局序列化后的结果)
    /// @code
    /// TLY格式如下:
    /// {
    ///   线性布局,
    ///   名称="线性布局1",
    ///   宽度=-1,
    ///   {
    ///     文本框,
    ///     名称="文本框1",
    ///     内容="你好"
    ///   }
    /// }
    /// @endcode
    /// @code
    /// JSON格式如下:
    /// {
    ///   "class": {
    ///     "className": "组件类名"
    ///   },
    ///   "nameProp": { // 名称属性，和其他属性独立
    ///     "propName": {
    ///       "name": "名称"
    ///     },
    ///     "propValue": {
    ///       "value": "组件名称值"
    ///     }
    ///   },
    ///   "properties": [
    ///     {
    ///       "propName": {
    ///         "name": "宽度"
    ///       },
    ///       "propValue": {
    ///         "value": -1
    ///       }
    ///     },
    ///     ...
    ///   ],
    ///   "children": [
    ///     {
    ///       "class": {
    ///         "className": "组件类名"
    ///       },
    ///       "nameProp": { // 名称属性，和其他属性独立
    ///         "propName": {
    ///           "name": "名称"
    ///         },
    ///         "propValue": {
    ///           "value": "组件名称值"
    ///         }
    ///       },
    ///       "properties": [
    ///         {
    ///           "propName": {
    ///             "name": "宽度"
    ///           },
    ///           "propValue": {
    ///             "value": -1
    ///           }
    ///         },
    ///         ...
    ///       ],
    ///     }
    ///     ...
    ///   ]
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_get_ui_bindings(intptr_t ide_service_handle, const char *params_json, tc_tly_format_t format);

    /// 解析TLY布局代码，等同于 IDEService::parseTLYEntity
    /// @param ide_service_handle IDEService句柄
    /// @param tly_text TLY布局代码
    /// @return TLY布局解析结果 (TLYParsingResult 序列化后的json)
    /// @code
    /// JSON格式如下:
    /// {
    ///   "root": , // TLYEntity树，格式同 @see {tc_ide_service_get_ui_bindings} 的json格式
    ///   "diagnostics": [...] // 解析时出现的诊断信息，如果没有则为空，格式同 @see {tc_ide_service_lint_file} 的json格式
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_parse_tly_entity(intptr_t ide_service_handle, const char *tly_text);

    /// 将光标处所处类原有的布局变量删除，并替换为新的TLY布局变量（仅安卓平台可用），等同于 IDEService::getUIBindings
    /// @param ide_service_handle IDEService句柄
    /// @param params_json CursorParams序列化后的json，与 @see {tc_ide_service_hover} 的 params_json 一致
    /// @param new_tly_data 新的TLY布局数据
    /// @param format 传进来的tly的序列化格式(tly格式/json格式)
    /// @return 接收当前代码文件的编辑结果，会将原有布局变量全部删除，然后插入新的布局变量（UIBindingEditResult序列化后的json）
    /// @code
    /// json格式如下:
    /// {
    ///   "edits": [
    ///     {
    ///       "range": {
    ///         "start": {
    ///           "line": 行号,
    ///           "column": 列号
    ///         },
    ///         "end": {
    ///           "line": 行号,
    ///           "column": 列号
    ///         }
    ///       },
    ///       "newText": "替换后文本"
    ///     }
    ///     ...
    ///   ]
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_edit_ui_bindings(intptr_t ide_service_handle, const char *params_json, const char *new_tly_data, tc_tly_format_t format);

    /// 扫描整个编译环境中可视化组件类型信息，用于布局设计器支持设计布局（仅安卓平台可用），等同于 IDEService::scanUIClasses
    /// @param ide_service_handle IDEService句柄
    /// @return 可视化组件类型信息（ViewClassInfoResult 序列化后的json）
    /// @code
    /// json格式如下:
    /// {
    ///   "viewClasses": [
    ///     {
    ///       "name": "结绳.安卓.进度条", // 完整类名
    ///       "mangledName": "js.az.JinDuTiao", // 类名输出名，反射时以该名称为准
    ///       "isContainer": 是否为布局组件,
    ///       "viewProperties": [ // 组件自身的属性，不包含基础属性
    ///         {
    ///           "name": "最大进度", // 属性名称
    ///           "type": "整数", // 属性类型，常见的还有文本、图片资源等
    ///           "mangledName": "setMaxProgress", // 属性名输出名，反射时以该名称为准
    ///         }
    ///         ...
    ///       ]
    ///       "containerProperties": [ // 布局组件为子组件提供的布局属性，仅"isContainer"为true时生效
    ///         {
    ///           "name": "权重",
    ///           "type": "小数",
    ///           "mangledName", "setWeight" // 注意：布局属性反射时第一个参数固定为子组件对象，第二个参数才是属性值
    ///         }
    ///       ]
    ///     }
    ///     ...
    ///   ],
    ///   "basicProperties": [ // 所有可视化组件的基础属性
    ///     {
    ///       "name": "宽度", // 属性名称
    ///       "type": "整数", // 属性类型，常见的还有文本、图片资源等
    ///       "mangledName": "宽度", // 属性名输出名，反射时以该名称为准
    ///     },
    ///     ...
    ///   ]
    /// }
    /// @endcode
    TIEC_API const char *tc_ide_service_scan_ui_classes(intptr_t ide_service_handle);

    /// 取消对IDE服务的请求，等同于 IDEService::cancel
    /// @param ide_service_handle IDEService句柄
    /// @return 返回错误码
    TIEC_API tc_error_t tc_ide_service_cancel(intptr_t ide_service_handle);

    /// 销毁IDEService实例
    /// @param ide_service_handle IDEService句柄
    /// @return 返回错误码，销毁成功返回TC_OK
    TIEC_API tc_error_t tc_free_ide_service(intptr_t ide_service_handle);

    /// 格式化代码文本（不包含任何语义，纯代码解析缩进）
    /// @param doc_text 代码文本内容
    /// @return 格式化之后的代码文本
    TIEC_API const char *tc_ide_service_format_text(const char *doc_text);

    /// 根据代码内容和光标位置获取处换行自动插入内容，如自动插入结束语句
    /// @param doc_text 代码文本
    /// @param line 光标所处行
    /// @param column 光标所处列
    /// @return 换行需要插入的内容，比如 "结束 如果"
    TIEC_API const char *tc_ide_service_newline(const char *doc_text, size_t line, size_t column);

    /// 根据当前行文本解析获取下一行的缩进基数
    /// @param line_text 当前行文本内容
    /// @param column 光标所处列
    /// @return 下一行的缩进基数
    TIEC_API int tc_ide_service_indent_advance(const char *line_text, size_t column);

    /// 根据各平台语言源文件生成结绳类型声明文件(.d.t)
    /// @param kind 语言文件类型，参见 @see {tc_declaration_kind_t}
    /// @param file_count 文件数量
    /// @param files 文件路径数组，大小和文件数量对应
    /// @param output_dir .d.t类型声明文件输出目录
    /// @return 返回错误码
    TIEC_API tc_error_t tc_generate_declarations(tc_declaration_kind_t kind, size_t file_count, const char *const *files, const char *output_dir);

    /// 从行号映射表创建行号表工具
    /// @param mapping_path 行号表路径
    /// @return 行号表句柄
    TIEC_API intptr_t tc_decode_source_mapping(const char *mapping_path);

    /// 从行号表获取输出名对应的结绳符号名称
    /// @param mapping_handle 行号表句柄
    /// @param output_name 输出后的名称
    /// @return 在源代码中的原始名称
    TIEC_API const char *tc_source_mapping_get_name(intptr_t mapping_handle, const char *output_name);

    /// 从行号表获取输出文件行号对应的结绳源代码原始行号
    /// @param mapping_handle 行号表句柄
    /// @param filename 输出的文件名（不是路径）
    /// @param line_number 输出的文件行号
    /// @return 在结绳源代码中的源文件路径和原始行号，以JSON格式返回
    /// @code
    /// json格式如下:
    /// {
    ///   "path": 原始文件路径,
    ///   "line": 行号
    /// }
    /// @endcode
    TIEC_API const char *tc_source_mapping_get_line(intptr_t mapping_handle, const char *filename, size_t line_number);

    /// 销毁行号表实例
    /// @param mapping_handle 行号表句柄
    /// @return 返回错误码，销毁成功返回TC_OK
    TIEC_API tc_error_t tc_free_source_mapping(intptr_t mapping_handle);

    /// 快速计算指定文件的哈希值
    /// @param file_path 文件路径
    /// @return 返回文件内容哈希值，计算失败时返回0
    TIEC_API uint64_t tc_hash_file(const char *file_path);

    /// 快速计算指定文本内容的哈希值
    /// @param text 文本内容
    /// @return 返回文本哈希值，计算失败时返回0
    TIEC_API uint64_t tc_hash_text(const char *text);

#ifdef __cplusplus
}
#endif

#endif // TIEC_C_API_H