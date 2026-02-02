# JFLSP 客户端集成指南

本文档旨在帮助编辑器开发者将 JFLSP 集成到自己的编辑器或 IDE 中。

## 1. 启动服务

JFLSP 是一个标准的语言服务器，通过标准输入/输出 (Stdio) 进行通信。

*   **可执行文件**: `jflsp` (Windows下为 `jflsp.exe`)
*   **通信方式**: Stdio (Standard Input / Standard Output)
*   **编码**: UTF-8 (Header `Content-Length: ...\r\n\r\n` + JSON Body)

### 命令行示例

```bash
D:\project\jflsp\target\release\jflsp.exe
```

无额外命令行参数。

## 2. 初始化 (Initialize)

在发送 `initialize` 请求时，客户端可以通过 `initializationOptions` 传递配置参数。

### 请求参数示例

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "processId": 1234,
    "rootUri": "file:///path/to/project", //项目目录
    "capabilities": { ... },
    "initializationOptions": {
      "jars": [
        "c:/path/to/lib/dependency.jar", //jar资源
      ],
      "javaSources": [
        "c:/path/to/project/src/main/java" //java文件资源
      ]
    }
  }
}
```

### InitializationOptions 字段说明

| 字段名 | 类型 | 必填 | 说明 |
| :--- | :--- | :--- | :--- |
| `jars` | `string[]` | 否 | 项目依赖的 `.jar` 文件绝对路径列表。用于构建类型索引。 |
| `javaSources` | `string[]` | 否 | Java 源码根目录绝对路径列表。用于索引源码中的类型。 |

> **注意**: 如果未提供 `jars` 或 `javaSources`，JFLSP 默认只会索引打开的文件，补全能力将受限。

## 3. 支持的 LSP 能力

JFLSP 实现了以下 LSP 协议功能：

| 功能 | 方法 | 说明 |
| :--- | :--- | :--- |
| **文档同步** | `textDocument/didOpen`<br>`textDocument/didChange`<br>`textDocument/didSave`<br>`textDocument/didClose` | 支持增量同步 (`Incremental`)。 |
| **代码补全** | `textDocument/completion` | 触发字符: `.`<br>支持类名、成员、关键字补全。<br>支持匿名内部类方法重写补全。 |
| **悬停提示** | `textDocument/hover` | 显示类或方法的签名及文档。 |
| **定义跳转** | `textDocument/definition` | 跳转到类或方法的定义处。 |
| **参数提示** | `textDocument/signatureHelp` | 触发字符: `(`, `,`<br>显示当前方法的参数列表和高亮参数。 |
| **诊断信息** | `textDocument/publishDiagnostics` | (Server -> Client 通知)<br>实时推送语法错误和语义警告。 |

## 4. 文件类型处理

JFLSP 特别针对混合 Java 模板文件 (`.t`) 进行了优化：

1.  **模板语法兼容**:
    *   在 `.t` 文件中，JFLSP 会自动忽略包含 `#` 字符的行（视为模板指令）。
    *   客户端无需特殊处理，只需将 `.t` 文件的内容完整发送给 Server。

2.  **代码块识别**:
    *   JFLSP 会解析 `.t` 文件中的 `@导入Java`、`code` 行及 `@code...@end` 块。
    *   在非 Java 代码区域（如纯文本或 HTML），LSP 不会返回补全或报错。

## 5. 集成建议

1.  **索引构建**:
    *   Server 启动并收到 `initialize` 后，会在后台异步构建索引。
    *   建议客户端在状态栏显示 "JFLSP: Indexing..." 状态（Server 会发送 `window/logMessage` 通知进度）。

2.  **诊断过滤**:
    *   JFLSP 已经内置了对模板语法的过滤逻辑，客户端直接展示 `publishDiagnostics` 推送的错误即可。

3.  **调试**:
    *   可以监控 Stderr 输出以查看 Server 的内部日志（如果 Server 有 panic 或 eprintln 输出）。
    *   主要日志通过 `window/logMessage` 发送。

## 6. 示例流程

1.  **Client** 启动 `jflsp` 进程。
2.  **Client** 发送 `initialize`，带上 `jars` 路径。
3.  **Server** 回复 `capabilities`。
4.  **Client** 发送 `textDocument/didOpen` (打开 `test.t`)。
5.  **Server** 推送 `textDocument/publishDiagnostics`。
6.  **User** 输入 `list.`。
7.  **Client** 发送 `textDocument/completion`。
8.  **Server** 返回 `add`, `get`, `size` 等方法列表。
