# IDE服务
结绳编译器内置了IDE服务（包含Completion、Lint、SemanticHighLight、Format、Elements、Hover、Rename、FindDefinition、FindReferences等），
如果您需要使用非C++的方式集成IDE服务，请参见[非C++环境集成结绳编译器]()
## 必要流程
### 创建IDE服务并预编译项目源文件
首先按照创建Compiler的方式创建IDE服务，然后扫描项目中所有源文件(.t)进行预编译
```c++
// 先创建编译器上下文
SharedPtr<Context> context = makeSharedPtr<Context>();
// 注册编译选项
SharedPtr<Options> options = makeSharedPtr<Options>();
// 指定为debug模式
options->platform = TargetPlatform::kWindows // 指定目标平台为Windows
// 向Context中注册Options
context->addComponent(options);

// 创建IDEService指针
SharedPtr<IDEService> service = IDEServiceFactory::makeIDEService(context);
// 预编译项目中所有源文件
List<Source> sources;
for (SharedPtr<Source>& source : 项目中所有源文件) {
  sources.add(source);
}
service->compile(sources);
```
### 将IDE中的变动同步到IDEService
如果IDE中编辑了某个源文件，应该将其变动同步到`IDEService`，支持全量和增量同步
```c++
// 全量同步指定源文件编辑后的代码
didChangeSource(const Uri& source_uri, String&& new_text)
// 增量同步指定源文件编辑后的代码
didChangeSourceIncremental(const Uri& source_uri, const TextChange& change) const;
```
如果IDE中创建/删除/重命名了源文件，应该通知`IDEService`
```c++
/// 通知环境中有新文件创建
/// @param source_uri 代码文件Uri
/// @param initial_text 初始内容
void didCreateSource(const Uri& source_uri, String&& initial_text) const;
    
/// 通知环境中有文件被删除
/// @param source_uri 代码文件Uri
void didDeleteSource(const Uri& source_uri) const;

/// 通知环境中文件被重命名
/// @param old_uri 原来的Uri
/// @param new_uri 新的Uri
void didRenameSource(const Uri& old_uri, const Uri& new_uri) const;
```

## 使用代码补全
首先将IDE中光标信息封装为`CompletionParams`
```c++
CompletionParams params;
params.uri = File("C:\\a.t").toUri();
params.position = {1, 3} // 第2行第4列
params.line_text = "..." // 光标所处行的整行文本内容, 可不填写
params.partial = "启动" // 触发代码补全的前缀单词
params.trigger_char = "动" // 触发代码补全的字符
```
然后获取代码补全结果
```c++
CompletionResult result;
service->complete(params, result);
// 将result呈现到UI
result ....
```

## 获取鼠标悬停处符号信息
首先将IDE中光标信息封装为`CursorParams`
```c++
CursorParams params;
params.uri = File("C:\\a.t").toUri();
params.position = {1, 3} // 第2行第4列
```
然后获取光标处符号文档
```c++
MarkupContent result;
service->hover(params, result);
// 将result呈现到UI
result ....
```