# JFLSP 交互协议参考手册

本文档详细描述了 JFLSP (Java-Frontend LSP) 支持的 LSP 协议接口、数据结构及 JSON 交互示例。适用于需要自行实现 LSP 客户端的开发者。

## 1. 基础协议格式

JFLSP 基于 [Language Server Protocol (LSP) 3.16+](https://microsoft.github.io/language-server-protocol/)。
通信采用标准输入/输出 (Stdio)。

### 消息封装 (Base Protocol)

所有消息必须包含 `Content-Length` 头。

```http
Content-Length: <length>\r\n
\r\n
{
    "jsonrpc": "2.0",
    ...
}
```

---

## 2. 生命周期接口

### 2.1 初始化 (initialize)

客户端发送的第一个请求，用于配置 JFLSP。

**请求 (Request):**

*   `method`: `initialize`
*   `params`: `InitializeParams`

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "processId": 12345,
    "rootUri": "file:///c:/Projects/MyProject",
    "capabilities": {
      "textDocument": {
        "completion": { ... },
        "hover": { ... }
      }
    },
    "initializationOptions": {
      "jars": [
        "c:/Projects/MyProject/lib/dependency.jar"
      ],
      "javaSources": [
        "c:/Projects/MyProject/src/main/java"
      ]
    }
  }
}
```

**响应 (Response):**

*   `result`: `InitializeResult`

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "capabilities": {
      "textDocumentSync": {
        "openClose": true,
        "change": 2, // Incremental
        "save": true
      },
      "completionProvider": {
        "triggerCharacters": ["."],
        "resolveProvider": false
      },
      "hoverProvider": true,
      "definitionProvider": true,
      "signatureHelpProvider": {
        "triggerCharacters": ["(", ","]
      }
    },
    "serverInfo": {
      "name": "jflsp",
      "version": "0.1.0"
    }
  }
}
```

---

## 3. 文档同步接口

### 3.1 打开文档 (textDocument/didOpen)

**通知 (Notification):**

```json
{
  "jsonrpc": "2.0",
  "method": "textDocument/didOpen",
  "params": {
    "textDocument": {
      "uri": "file:///c:/Projects/MyProject/test.t",
      "languageId": "java", // 即使是 .t 文件，建议标记为 java 或 plain text
      "version": 1,
      "text": "@导入Java(\"java.util.*\")\ncode List<String> l = new ArrayList<>();"
    }
  }
}
```

### 3.2 修改文档 (textDocument/didChange)

**通知 (Notification):**

```json
{
  "jsonrpc": "2.0",
  "method": "textDocument/didChange",
  "params": {
    "textDocument": {
      "uri": "file:///c:/Projects/MyProject/test.t",
      "version": 2
    },
    "contentChanges": [
      {
        "range": {
          "start": { "line": 1, "character": 36 },
          "end": { "line": 1, "character": 36 }
        },
        "rangeLength": 0,
        "text": "."
      }
    ]
  }
}
```

---

## 4. 语言特性接口

### 4.1 代码补全 (textDocument/completion)

当用户输入触发字符（如 `.`）时发送。

**请求 (Request):**

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "textDocument/completion",
  "params": {
    "textDocument": {
      "uri": "file:///c:/Projects/MyProject/test.t"
    },
    "position": {
      "line": 1,
      "character": 37
    },
    "context": {
      "triggerKind": 2, // 2 = TriggerCharacter
      "triggerCharacter": "."
    }
  }
}
```

**响应 (Response):**

*   `result`: `CompletionItem[]`

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": [
    {
      "label": "add",
      "kind": 2, // Method
      "detail": "boolean add(E e)",
      "insertText": "add()",
      "insertTextFormat": 1 // PlainText
    },
    {
      "label": "size",
      "kind": 2,
      "detail": "int size()",
      "insertText": "size()",
      "insertTextFormat": 1
    }
  ]
}
```

### 4.2 悬停提示 (textDocument/hover)

**请求 (Request):**

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "textDocument/hover",
  "params": {
    "textDocument": { "uri": "file:///c:/Projects/MyProject/test.t" },
    "position": { "line": 1, "character": 10 }
  }
}
```

**响应 (Response):**

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "contents": {
      "kind": "markdown",
      "value": "```java\njava.util.List\n```\n\n```java\nAn ordered collection (also known as a sequence)...\n```"
    }
  }
}
```

### 4.3 参数提示 (textDocument/signatureHelp)

**请求 (Request):**

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "textDocument/signatureHelp",
  "params": {
    "textDocument": { "uri": "file:///c:/Projects/MyProject/test.t" },
    "position": { "line": 1, "character": 41 }
  }
}
```

**响应 (Response):**

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": {
    "signatures": [
      {
        "label": "boolean add(E e)",
        "parameters": [
          { "label": "E e" }
        ]
      },
      {
        "label": "void add(int index, E element)",
        "parameters": [
          { "label": "int index" },
          { "label": "E element" }
        ]
      }
    ],
    "activeSignature": 0,
    "activeParameter": 0
  }
}
```

---

## 5. 服务端推送

### 5.1 诊断发布 (textDocument/publishDiagnostics)

服务端主动推送的语法错误或警告。

**通知 (Notification):**

```json
{
  "jsonrpc": "2.0",
  "method": "textDocument/publishDiagnostics",
  "params": {
    "uri": "file:///c:/Projects/MyProject/test.t",
    "diagnostics": [
      {
        "range": {
          "start": { "line": 1, "character": 5 },
          "end": { "line": 1, "character": 10 }
        },
        "severity": 1, // Error
        "source": "jflsp",
        "message": "类型不匹配: 无法将 String 转换为 int"
      }
    ]
  }
}
```

### 5.2 日志消息 (window/logMessage)

服务端推送的日志，用于调试或展示进度。

**通知 (Notification):**

```json
{
  "jsonrpc": "2.0",
  "method": "window/logMessage",
  "params": {
    "type": 3, // Info
    "message": "JavaIndex rebuilt"
  }
}
```
