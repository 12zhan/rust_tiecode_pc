pub const CPP_GRAMMAR: &str = r##"{
  "name": "CPP",
  "fileExtensions": [".cpp", ".h", ".hpp", ".cc", ".c", ".hh"],
  "styles": [
    { "name": "keyword", "foreground": "#569cd6", "tags": ["bold"] },
    { "name": "string", "foreground": "#ce9178" },
    { "name": "comment", "foreground": "#6a9955" },
    { "name": "number", "foreground": "#b5cea8" },
    { "name": "type", "foreground": "#4ec9b0" },
    { "name": "preprocessor", "foreground": "#c586c0" },
    { "name": "function", "foreground": "#dcdcaa" }
  ],
  "states": {
    "default": [
      { "pattern": "//.*", "style": "comment" },
      { "pattern": "/\\*", "state": "block_comment", "style": "comment" },
      { "pattern": "\"(?:[^\"\\\\]|\\\\.)*\"", "style": "string" },
      { "pattern": "'(?:[^'\\\\]|\\\\.)*'", "style": "string" },
      { "pattern": "#\\s*\\w+", "style": "preprocessor" },
      { "pattern": "\\b(?:if|else|while|for|return|class|struct|public|private|protected|virtual|override|namespace|using|template|typename|void|int|float|double|bool|const|static|auto|friend|explicit|constexpr|nullptr|true|false|switch|case|default|break|continue|do|try|catch|throw|sizeof|alignof|alignas|decltype|noexcept|static_assert|static_cast|dynamic_cast|const_cast|reinterpret_cast|new|delete|operator|this|enum|union|typedef|char|short|long|unsigned|signed)\\b", "style": "keyword" },
      { "pattern": "\\b[0-9]+(?:\\.[0-9]*)?f?\\b", "style": "number" },
      { "pattern": "\\b0x[0-9a-fA-F]+\\b", "style": "number" },
      { "pattern": "\\b[A-Z][a-zA-Z0-9_]*\\b", "style": "type" },
      { "pattern": "\\b[a-zA-Z_][a-zA-Z0-9_]*(?=\\()", "style": "function" }
    ],
    "block_comment": [
      { "pattern": "\\*/", "state": "default", "style": "comment" },
      { "pattern": ".", "style": "comment" }
    ]
  }
}"##;

pub const RUST_GRAMMAR: &str = r##"{
  "name": "Rust",
  "fileExtensions": [".rs"],
  "styles": [
    { "name": "keyword", "foreground": "#569cd6", "tags": ["bold"] },
    { "name": "string", "foreground": "#ce9178" },
    { "name": "comment", "foreground": "#6a9955" },
    { "name": "number", "foreground": "#b5cea8" },
    { "name": "type", "foreground": "#4ec9b0" },
    { "name": "macro", "foreground": "#dcdcaa" },
    { "name": "attribute", "foreground": "#c586c0" }
  ],
  "states": {
    "default": [
      { "pattern": "//.*", "style": "comment" },
      { "pattern": "/\\*", "state": "block_comment", "style": "comment" },
      { "pattern": "r#*\\\"(?:[^\\\"\\\\]|\\\\.)*\\\"#*", "style": "string" },
      { "pattern": "\\\"(?:[^\\\"\\\\]|\\\\.)*\\\"", "style": "string" },
      { "pattern": "'(?:[^'\\\\]|\\\\.)'", "style": "string" },
      { "pattern": "#\\!?\\[[^\\]]+\\]", "style": "attribute" },
      { "pattern": "\\b[a-zA-Z_][a-zA-Z0-9_]*!\\b", "style": "macro" },
      { "pattern": "\\b0x[0-9a-fA-F_]+\\b", "style": "number" },
      { "pattern": "\\b[0-9][0-9_]*(?:\\.[0-9_]+)?(?:[eE][+-]?[0-9_]+)?\\b", "style": "number" },
      { "pattern": "\\b(?:let|fn|struct|enum|impl|trait|pub|use|mod|crate|super|self|Self|mut|ref|const|static|unsafe|async|await|move|match|if|else|while|for|loop|break|continue|return|where|type|as|in|dyn|union|extern|yield|macro_rules)\\b", "style": "keyword" },
      { "pattern": "\\b[A-Z][a-zA-Z0-9_]*\\b", "style": "type" }
    ],
    "block_comment": [
      { "pattern": "\\*/", "state": "default", "style": "comment" },
      { "pattern": ".", "style": "comment" }
    ]
  }
}"##;

pub const JSON_GRAMMAR: &str = r##"{
  "name": "JSON",
  "fileExtensions": [".json", ".jsonc"],
  "styles": [
    { "name": "keyword", "foreground": "#569cd6", "tags": ["bold"] },
    { "name": "string", "foreground": "#ce9178" },
    { "name": "number", "foreground": "#b5cea8" },
    { "name": "punctuation", "foreground": "#d4d4d4" }
  ],
  "states": {
    "default": [
      { "pattern": "\\\"(?:[^\\\"\\\\]|\\\\.)*\\\"", "style": "string" },
      { "pattern": "\\b(?:true|false|null)\\b", "style": "keyword" },
      { "pattern": "-?(?:0|[1-9][0-9]*)(?:\\.[0-9]+)?(?:[eE][+-]?[0-9]+)?", "style": "number" },
      { "pattern": "[\\{\\}\\[\\]\\:,]", "style": "punctuation" }
    ]
  }
}"##;

pub const CMAKE_GRAMMAR: &str = r##"{
  "name": "CMake",
  "fileExtensions": [".cmake", "CMakeLists.txt", "cmakelists.txt"],
  "styles": [
    { "name": "keyword", "foreground": "#569cd6", "tags": ["bold"] },
    { "name": "string", "foreground": "#ce9178" },
    { "name": "comment", "foreground": "#6a9955" },
    { "name": "number", "foreground": "#b5cea8" },
    { "name": "variable", "foreground": "#9cdcfe" }
  ],
  "states": {
    "default": [
      { "pattern": "#.*", "style": "comment" },
      { "pattern": "\\\"(?:[^\\\"\\\\]|\\\\.)*\\\"", "style": "string" },
      { "pattern": "'(?:[^'\\\\]|\\\\.)*'", "style": "string" },
      { "pattern": "\\$\\{[A-Za-z0-9_]+\\}", "style": "variable" },
      { "pattern": "\\b(?:cmake_minimum_required|project|add_executable|add_library|add_subdirectory|include|find_package|set|unset|option|message|file|list|string|if|elseif|else|endif|foreach|endforeach|while|endwhile|function|endfunction|macro|endmacro|return|break|continue|install|configure_file|target_link_libraries|target_include_directories|target_compile_definitions|target_compile_options|add_custom_command|add_custom_target)\\b", "style": "keyword" },
      { "pattern": "\\b[0-9]+\\b", "style": "number" }
    ]
  }
}"##;

pub const TOML_GRAMMAR: &str = r##"{
  "name": "TOML",
  "fileExtensions": [".toml"],
  "styles": [
    { "name": "key", "foreground": "#9cdcfe" },
    { "name": "string", "foreground": "#ce9178" },
    { "name": "number", "foreground": "#b5cea8" },
    { "name": "keyword", "foreground": "#569cd6", "tags": ["bold"] },
    { "name": "comment", "foreground": "#6a9955" },
    { "name": "punctuation", "foreground": "#d4d4d4" }
  ],
  "states": {
    "default": [
      { "pattern": "#.*", "style": "comment" },
      { "pattern": "\\b[A-Za-z0-9_-]+(?=\\s*=)", "style": "key" },
      { "pattern": "\\\"(?:[^\\\"\\\\]|\\\\.)*\\\"", "style": "string" },
      { "pattern": "'(?:[^'\\\\]|\\\\.)*'", "style": "string" },
      { "pattern": "\\b(?:true|false)\\b", "style": "keyword" },
      { "pattern": "\\b[0-9]+(?:\\.[0-9]+)?\\b", "style": "number" },
      { "pattern": "[\\[\\]\\{\\}=,\\.]", "style": "punctuation" }
    ]
  }
}"##;

pub const YAML_GRAMMAR: &str = r##"{
  "name": "YAML",
  "fileExtensions": [".yml", ".yaml"],
  "styles": [
    { "name": "key", "foreground": "#9cdcfe" },
    { "name": "string", "foreground": "#ce9178" },
    { "name": "number", "foreground": "#b5cea8" },
    { "name": "keyword", "foreground": "#569cd6", "tags": ["bold"] },
    { "name": "comment", "foreground": "#6a9955" },
    { "name": "punctuation", "foreground": "#d4d4d4" }
  ],
  "states": {
    "default": [
      { "pattern": "#.*", "style": "comment" },
      { "pattern": "\\b[A-Za-z0-9_-]+(?=\\s*:)", "style": "key" },
      { "pattern": "\\\"(?:[^\\\"\\\\]|\\\\.)*\\\"", "style": "string" },
      { "pattern": "'(?:[^'\\\\]|\\\\.)*'", "style": "string" },
      { "pattern": "\\b(?:true|false|null)\\b", "style": "keyword" },
      { "pattern": "\\b[0-9]+(?:\\.[0-9]+)?\\b", "style": "number" },
      { "pattern": "[-:\\[\\]\\{\\},]", "style": "punctuation" }
    ]
  }
}"##;

pub const PYTHON_GRAMMAR: &str = r##"{
  "name": "Python",
  "fileExtensions": [".py"],
  "styles": [
    { "name": "keyword", "foreground": "#569cd6", "tags": ["bold"] },
    { "name": "string", "foreground": "#ce9178" },
    { "name": "comment", "foreground": "#6a9955" },
    { "name": "number", "foreground": "#b5cea8" }
  ],
  "states": {
    "default": [
      { "pattern": "#.*", "style": "comment" },
      { "pattern": "\\\"\\\"\\\"", "style": "string", "state": "triple_double" },
      { "pattern": "'''", "style": "string", "state": "triple_single" },
      { "pattern": "\\\"(?:[^\\\"\\\\]|\\\\.)*\\\"", "style": "string" },
      { "pattern": "'(?:[^'\\\\]|\\\\.)*'", "style": "string" },
      { "pattern": "\\b(?:def|class|import|from|as|if|elif|else|for|while|break|continue|return|yield|try|except|finally|with|lambda|pass|raise|global|nonlocal|assert|True|False|None)\\b", "style": "keyword" },
      { "pattern": "\\b[0-9]+(?:\\.[0-9]+)?\\b", "style": "number" }
    ],
    "triple_double": [
      { "pattern": "\\\"\\\"\\\"", "style": "string", "state": "default" },
      { "pattern": ".", "style": "string" }
    ],
    "triple_single": [
      { "pattern": "'''", "style": "string", "state": "default" },
      { "pattern": ".", "style": "string" }
    ]
  }
}"##;

pub const JAVASCRIPT_GRAMMAR: &str = r##"{
  "name": "JavaScript",
  "fileExtensions": [".js", ".mjs", ".cjs", ".jsx"],
  "styles": [
    { "name": "keyword", "foreground": "#569cd6", "tags": ["bold"] },
    { "name": "string", "foreground": "#ce9178" },
    { "name": "comment", "foreground": "#6a9955" },
    { "name": "number", "foreground": "#b5cea8" },
    { "name": "function", "foreground": "#dcdcaa" }
  ],
  "states": {
    "default": [
      { "pattern": "//.*", "style": "comment" },
      { "pattern": "/\\*", "state": "block_comment", "style": "comment" },
      { "pattern": "`(?:[^`\\\\]|\\\\.)*`", "style": "string" },
      { "pattern": "\\\"(?:[^\\\"\\\\]|\\\\.)*\\\"", "style": "string" },
      { "pattern": "'(?:[^'\\\\]|\\\\.)*'", "style": "string" },
      { "pattern": "\\b(?:function|class|const|let|var|if|else|for|while|do|return|break|continue|switch|case|default|try|catch|finally|throw|new|this|super|extends|import|from|export|async|await|yield|typeof|instanceof|in|of|true|false|null|undefined)\\b", "style": "keyword" },
      { "pattern": "\\b[0-9]+(?:\\.[0-9]+)?\\b", "style": "number" },
      { "pattern": "\\b[a-zA-Z_][a-zA-Z0-9_]*(?=\\()", "style": "function" }
    ],
    "block_comment": [
      { "pattern": "\\*/", "state": "default", "style": "comment" },
      { "pattern": ".", "style": "comment" }
    ]
  }
}"##;

pub const JAVA_GRAMMAR: &str = r##"{
  "name": "Java",
  "fileExtensions": [".java", ".jav"],
  "styles": [
    { "name": "keyword", "foreground": "#569cd6", "tags": ["bold"] },
    { "name": "string", "foreground": "#ce9178" },
    { "name": "comment", "foreground": "#6a9955" },
    { "name": "number", "foreground": "#b5cea8" },
    { "name": "type", "foreground": "#4ec9b0" },
    { "name": "annotation", "foreground": "#dcdcaa" },
    { "name": "function", "foreground": "#dcdcaa" }
  ],
  "states": {
    "default": [
      { "pattern": "//.*", "style": "comment" },
      { "pattern": "/\\*", "state": "block_comment", "style": "comment" },
      { "pattern": "\\\"(?:[^\\\"\\\\]|\\\\.)*\\\"", "style": "string" },
      { "pattern": "'(?:[^'\\\\]|\\\\.)'", "style": "string" },
      { "pattern": "@[a-zA-Z_][a-zA-Z0-9_]*", "style": "annotation" },
      { "pattern": "\\b(?:abstract|assert|boolean|break|byte|case|catch|char|class|const|continue|default|do|double|else|enum|extends|final|finally|float|for|goto|if|implements|import|instanceof|int|interface|long|native|new|package|private|protected|public|return|short|static|strictfp|super|switch|synchronized|this|throw|throws|transient|try|void|volatile|while|true|false|null|var)\\b", "style": "keyword" },
      { "pattern": "\\b[0-9]+(?:\\.[0-9]+)?f?\\b", "style": "number" },
      { "pattern": "\\b0x[0-9a-fA-F]+\\b", "style": "number" },
      { "pattern": "\\b[A-Z][a-zA-Z0-9_]*\\b", "style": "type" },
      { "pattern": "\\b[a-zA-Z_][a-zA-Z0-9_]*(?=\\()", "style": "function" }
    ],
    "block_comment": [
      { "pattern": "\\*/", "state": "default", "style": "comment" },
      { "pattern": ".", "style": "comment" }
    ]
  }
}"##;

pub const TYPESCRIPT_GRAMMAR: &str = r##"{
  "name": "TypeScript",
  "fileExtensions": [".ts", ".tsx"],
  "styles": [
    { "name": "keyword", "foreground": "#569cd6", "tags": ["bold"] },
    { "name": "string", "foreground": "#ce9178" },
    { "name": "comment", "foreground": "#6a9955" },
    { "name": "number", "foreground": "#b5cea8" },
    { "name": "function", "foreground": "#dcdcaa" },
    { "name": "type", "foreground": "#4ec9b0" }
  ],
  "states": {
    "default": [
      { "pattern": "//.*", "style": "comment" },
      { "pattern": "/\\*", "state": "block_comment", "style": "comment" },
      { "pattern": "`(?:[^`\\\\]|\\\\.)*`", "style": "string" },
      { "pattern": "\\\"(?:[^\\\"\\\\]|\\\\.)*\\\"", "style": "string" },
      { "pattern": "'(?:[^'\\\\]|\\\\.)*'", "style": "string" },
      { "pattern": "\\b(?:interface|type|enum|implements|extends|class|const|let|var|if|else|for|while|do|return|break|continue|switch|case|default|try|catch|finally|throw|new|this|super|import|from|export|async|await|yield|typeof|instanceof|in|of|true|false|null|undefined|public|private|protected|readonly|keyof)\\b", "style": "keyword" },
      { "pattern": "\\b[0-9]+(?:\\.[0-9]+)?\\b", "style": "number" },
      { "pattern": "\\b[A-Z][a-zA-Z0-9_]*\\b", "style": "type" },
      { "pattern": "\\b[a-zA-Z_][a-zA-Z0-9_]*(?=\\()", "style": "function" }
    ],
    "block_comment": [
      { "pattern": "\\*/", "state": "default", "style": "comment" },
      { "pattern": ".", "style": "comment" }
    ]
  }
}"##;

pub const HTML_GRAMMAR: &str = r##"{
  "name": "HTML",
  "fileExtensions": [".html", ".htm"],
  "styles": [
    { "name": "tag", "foreground": "#569cd6" },
    { "name": "attr", "foreground": "#9cdcfe" },
    { "name": "string", "foreground": "#ce9178" },
    { "name": "comment", "foreground": "#6a9955" }
  ],
  "states": {
    "default": [
      { "pattern": "<!--", "style": "comment", "state": "comment_block" },
      { "pattern": "<\\/?[a-zA-Z][a-zA-Z0-9-]*", "style": "tag" },
      { "pattern": "\\b[a-zA-Z_:][a-zA-Z0-9_:-]*(?=\\=)", "style": "attr" },
      { "pattern": "\\\"(?:[^\\\"\\\\]|\\\\.)*\\\"", "style": "string" },
      { "pattern": "'(?:[^'\\\\]|\\\\.)*'", "style": "string" }
    ],
    "comment_block": [
      { "pattern": "-->", "style": "comment", "state": "default" },
      { "pattern": ".", "style": "comment" }
    ]
  }
}"##;

pub const CSS_GRAMMAR: &str = r##"{
  "name": "CSS",
  "fileExtensions": [".css", ".scss", ".less"],
  "styles": [
    { "name": "keyword", "foreground": "#569cd6" },
    { "name": "string", "foreground": "#ce9178" },
    { "name": "comment", "foreground": "#6a9955" },
    { "name": "number", "foreground": "#b5cea8" },
    { "name": "selector", "foreground": "#dcdcaa" }
  ],
  "states": {
    "default": [
      { "pattern": "/\\*", "state": "block_comment", "style": "comment" },
      { "pattern": "@[a-zA-Z_-]+", "style": "keyword" },
      { "pattern": "\\.[a-zA-Z0-9_-]+", "style": "selector" },
      { "pattern": "#[a-zA-Z0-9_-]+", "style": "selector" },
      { "pattern": "\\\"(?:[^\\\"\\\\]|\\\\.)*\\\"", "style": "string" },
      { "pattern": "'(?:[^'\\\\]|\\\\.)*'", "style": "string" },
      { "pattern": "\\b[0-9]+(?:\\.[0-9]+)?(?:px|em|rem|%|vh|vw)?\\b", "style": "number" }
    ],
    "block_comment": [
      { "pattern": "\\*/", "state": "default", "style": "comment" },
      { "pattern": ".", "style": "comment" }
    ]
  }
}"##;

pub const MARKDOWN_GRAMMAR: &str = r##"{
  "name": "Markdown",
  "fileExtensions": [".md", ".markdown"],
  "styles": [
    { "name": "heading", "foreground": "#569cd6", "tags": ["bold"] },
    { "name": "code", "foreground": "#ce9178" },
    { "name": "emphasis", "foreground": "#dcdcaa" },
    { "name": "link", "foreground": "#4ec9b0" }
  ],
  "states": {
    "default": [
      { "pattern": "^#{1,6}.*", "style": "heading" },
      { "pattern": "`[^`]+`", "style": "code" },
      { "pattern": "\\*\\*[^*]+\\*\\*", "style": "emphasis" },
      { "pattern": "_[^_]+_", "style": "emphasis" },
      { "pattern": "\\[[^\\]]+\\]\\([^\\)]+\\)", "style": "link" },
      { "pattern": "```", "style": "code", "state": "code_block" }
    ],
    "code_block": [
      { "pattern": "```", "style": "code", "state": "default" },
      { "pattern": ".", "style": "code" }
    ]
  }
}"##;

pub const SHELL_GRAMMAR: &str = r##"{
  "name": "Shell",
  "fileExtensions": [".sh", ".bash", ".zsh"],
  "styles": [
    { "name": "keyword", "foreground": "#569cd6", "tags": ["bold"] },
    { "name": "string", "foreground": "#ce9178" },
    { "name": "comment", "foreground": "#6a9955" },
    { "name": "variable", "foreground": "#9cdcfe" }
  ],
  "states": {
    "default": [
      { "pattern": "#.*", "style": "comment" },
      { "pattern": "\\\"(?:[^\\\"\\\\]|\\\\.)*\\\"", "style": "string" },
      { "pattern": "'(?:[^'\\\\]|\\\\.)*'", "style": "string" },
      { "pattern": "\\$[A-Za-z0-9_]+", "style": "variable" },
      { "pattern": "\\b(?:if|then|else|elif|fi|for|while|do|done|case|esac|function|return|break|continue|export|unset|local|in)\\b", "style": "keyword" }
    ]
  }
}"##;

pub const JIESHENG_GRAMMAR: &str = r##"{
  "name": "tiecode",
  "fileExtensions": [".t"],
  "variables": {
    "identifierStart": "[\\p{Han}\\w_$]+",
    "identifierPart": "[\\p{Han}\\w_$0-9]*",
    "identifier": "${identifierStart}${identifierPart}",
    "whiteSpace": "[ \\t\\f]",
    "any": "[\\S\\s]",
    "classExtends": "(?:${whiteSpace}+(:)${whiteSpace}+${type})?"
  },
  "styles": [
    {
      "name": "keyword",
      "foreground": "#FF569CD6",
      "tags": ["bold", "italic"]
    },
    {
      "name": "string",
      "foreground": "#FFBD63C5"
    },
    {
      "name": "number",
      "foreground": "#FFE4FAD5"
    },
    {
      "name": "comment",
      "foreground": "#FF60AE6F"
    },
    {
      "name": "class",
      "foreground": "#FF4EC9B0"
    },
    {
      "name": "method",
      "foreground": "#FF9CDCFE"
    },
    {
      "name": "variable",
      "foreground": "#FF9B9BC8"
    },
    {
      "name": "punctuation",
      "foreground": "#FFD69D85"
    },
    {
      "name": "annotation",
      "foreground": "#FFFFFD9B"
    }
  ],
  "variables": {
    "identifierStart": "[\\p{Han}\\w_$]+",
    "identifierPart": "[\\p{Han}\\w_$0-9]*",
    "identifier": "${identifierStart}${identifierPart}",
    "whiteSpace": "[ \\t\\f]",
    "any": "[\\S\\s]",
    "classExtends": "(?:${whiteSpace}+(:)${whiteSpace}+${type})?",
    "embedCodeRefThis": "(#)(this)",
    "embedCodeRefClass": "(#)(cls|ncls)(<)(${identifier})(>)"
  },
  "blockPairs": [
    { "start": "类", "end": "结束 类" },
    { "start": "方法", "end": "结束 方法" },
    { "start": "循环", "end": "结束 循环" },
    { "start": "如果", "end": "结束 如果" }
  ],
  "states": {
    "default": [
      {
        "pattern": "\\b(类)\\b${whiteSpace}+(${identifier})",
        "styles": [1, "keyword", 2, "class"],
        "state": "typeDeclare"
      },
      {
        "pattern": "\\b(创建)\\b${whiteSpace}+(${identifier})",
        "styles": [1, "keyword", 2, "class"]
      },
      {
        "pattern": "(变量)${whiteSpace}+(${identifier})(?:${whiteSpace}*(:)${whiteSpace}*)?",
        "styles": [1, "keyword", 2, "variable", 3, "punctuation"],
        "state": "typeDeclare"
      },
      {
        "pattern": "(事件)${whiteSpace}+(${identifier})${whiteSpace}*(:)${whiteSpace}*(${identifier})${whiteSpace}*(\\()",
        "styles": [1, "keyword", 2, "variable", 3, "punctuation", 4, "method", 5, "punctuation"]
      },
      {
        "pattern": "\\b(包名|类|继承|变量|常量|方法|属性写|属性读|属性|定义事件|事件|结束|为|真|假|空|本对象|父对象|变体型)\\b",
        "styles": [1, "keyword"]
      },
      {
        "pattern": "\\b(如果|且|或|则|否则|假如|是|循环|跳过循环|退出循环|订阅事件|属于|返回|创建|等待)\\b",
        "styles": [1, "keyword"]
      },
      {
        "pattern": "\\b(code)\\b",
        "styles": [1, "keyword"],
        "state": "singleLineEmbedCode"
      },
      {
        "pattern": "(@)${whiteSpace}*(code)",
        "styles": [1, "punctuation", 2, "keyword"],
        "state": "multiLineEmbedCode"
      },
      {
        "pattern": "(@)${whiteSpace}*(${identifier})",
        "styles": [1, "punctuation", 2, "annotation"]
      },
      {
        "pattern": "(${identifier})(<)(${identifier})(>)",
        "styles": [1, "class", 2, "punctuation", 3, "class", 4, "punctuation"]
      },
      {
        "pattern": "(${identifier})(<)(${identifier})(,)(${identifier})(>)",
        "styles": [1, "class", 2, "punctuation", 3, "class", 4, "punctuation", 5, "class", 6, "punctuation"]
      },
      {
        "pattern": "(${identifier})(<)(${identifier})(>)(,)${whiteSpace}*(<)(${identifier})(>)",
        "styles": [1, "class", 2, "punctuation", 3, "class", 4, "punctuation", 5, "punctuation", 6, "class", 7, "punctuation"]
      },
      {
        "pattern": "(${identifier})(<)(${identifier})(<)(${identifier})(>)(>)",
        "styles": [1, "class", 2, "punctuation", 3, "class", 4, "punctuation", 5, "class", 6, "punctuation", 7, "punctuation"]
      },
      {
        "pattern": "(${identifier})(<)(${identifier})(>)",
        "styles": [1, "class", 2, "punctuation", 3, "class", 4, "punctuation"]
      },
      {
        "pattern": "(${identifier})${whiteSpace}*(:)${whiteSpace}*(${identifier})${whiteSpace}*([,)])",
        "styles": [1, "variable", 2, "punctuation", 3, "class", 4, "punctuation"]
      },
      {
        "pattern": "(${identifier})${whiteSpace}*(\\()",
        "styles": [1, "method", 2, "punctuation"]
      },
      {
        "pattern": "\"(?:[^\"\\\\]|\\\\.)*\"|'(?:[^'\\\\]|\\\\.)*'",
        "style": "string"
      },
      {
        "pattern": "\\b(?:[0-9]*\\.?[0-9]+(?:[eE][+-]?[0-9]+)?[fFdD]?)\\b",
        "style": "number"
      },
      {
        "pattern": "\\[\\[",
        "style": "string",
        "state": "longString"
      },
      {
        "pattern": "/\\*",
        "style": "comment",
        "state": "longComment"
      },
      {
        "pattern": "//${any}*",
        "style": "comment"
      },
      {
        "pattern": "[.()\\[\\]?@%+\\-*/<>=,{}:]",
        "style": "punctuation"
      }
    ],
    "typeDeclare": [
      {
        "pattern": "[<>,\\[\\]:]",
        "style": "punctuation"
      },
      {
        "pattern": "=",
        "style": "punctuation",
        "state": "default"
      },
      {
        "pattern": "${identifier}",
        "style": "class"
      },
      {
        "onLineEndState": "default"
      }
    ],
    "longString": [
      {
        "pattern": "\\]\\]",
        "style": "string",
        "state": "default"
      },
      {
        "pattern": "${any}",
        "style": "string"
      }
    ],
    "longComment": [
      {
        "pattern": "\\*/",
        "style": "comment",
        "state": "default"
      },
      {
        "pattern": "${any}",
        "style": "comment"
      }
    ],
    "singleLineEmbedCode": {
      "reference": "CPP",
      "rules": [
        {
          "pattern": "${embedCodeRefThis}",
          "styles": [1, "punctuation", 2, "keyword"]
        },
        {
          "pattern": "${embedCodeRefClass}",
          "styles": [1, "punctuation", 2, "keyword", 3, "punctuation", 4, "class", 5, "punctuation"]
        }
      ],
      "onLineEndState": "default"
    },
    "multiLineEmbedCode": {
      "reference": "CPP",
      "rules": [
        {
          "pattern": "${embedCodeRefThis}",
          "styles": [1, "punctuation", 2, "keyword"]
        },
        {
          "pattern": "${embedCodeRefClass}",
          "styles": [1, "punctuation", 2, "keyword", 3, "punctuation", 4, "class", 5, "punctuation"]
        },
        {
          "pattern": "(@)${whiteSpace}*(end)",
          "styles": [1, "punctuation", 2, "keyword"],
          "state": "default"
        }
      ],
      "onLineEndState": "multiLineEmbedCode"
    }
  }
}"##;
