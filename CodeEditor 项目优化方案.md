# CodeEditor 项目优化方案

## 一、架构设计优化

### 1. 模块化重构
```rust
// 建议的模块结构
src/
├── core/
│   ├── buffer.rs      // 文本缓冲区
│   ├── cursor.rs      // 光标管理
│   ├── selection.rs   // 选区管理
│   └── history.rs     // 撤消/重做
├── syntax/
│   ├── highlight.rs   // 语法高亮
│   ├── folding.rs     // 代码折叠
│   └── indentation.rs // 缩进处理
├── completion/
│   ├── provider.rs    // 补全提供者
│   ├── lsp.rs         // LSP补全
│   └── keyword.rs     // 关键字补全
├── lsp/
│   ├── client.rs      // LSP客户端
│   ├── handler.rs     // LSP消息处理
│   └── capabilities.rs // 能力协商
├── ui/
│   ├── render.rs      // 渲染引擎
│   ├── layout.rs      // 布局计算
│   └── theme.rs       // 主题系统
├── config/
│   ├── settings.rs    // 编辑器设置
│   └── keymap.rs      // 快捷键映射
└── editor.rs          // 主编辑器
```

### 2. 配置系统设计
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorSettings {
    // 显示设置
    pub font: FontSettings,
    pub theme: String,
    pub show_line_numbers: bool,
    pub show_minimap: bool,
    pub line_height: f32,
    
    // 编辑设置
    pub tab_size: usize,
    pub insert_spaces: bool,
    pub auto_indent: bool,
    pub word_wrap: bool,
    
    // 功能设置
    pub enable_completion: bool,
    pub enable_lsp: bool,
    pub enable_format_on_save: bool,
    
    // 性能设置
    pub highlight_cache_size: usize,
    pub render_batch_size: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FontSettings {
    pub family: String,
    pub size: f32,
    pub ligatures: bool,
    pub antialiasing: bool,
}
```

## 二、性能优化策略

### 1. 增量渲染系统
```rust
struct DirtyTracker {
    dirty_lines: BitSet,
    full_redraw: bool,
    last_render_time: Instant,
}

impl DirtyTracker {
    fn mark_line_dirty(&mut self, line: usize) {
        self.dirty_lines.insert(line);
    }
    
    fn mark_range_dirty(&mut self, range: Range<usize>) {
        // 将范围内的所有行标记为脏
    }
    
    fn clear(&mut self) {
        self.dirty_lines.clear();
        self.full_redraw = false;
    }
}

struct RenderPipeline {
    stages: Vec<RenderStage>,
    cache: RenderCache,
}

enum RenderStage {
    Background,    // 绘制背景
    Text,         // 绘制文本
    Selection,    // 绘制选区
    Overlay,      // 绘制叠加层（光标、错误等）
}
```

### 2. 智能缓存机制
```rust
struct HighlightCache {
    cache: LruCache<CacheKey, Vec<HighlightSpan>>,
    invalidator: CacheInvalidator,
}

struct CacheKey {
    line_text: String,
    language: String,
    theme_hash: u64,
}

impl HighlightCache {
    fn get_or_compute(
        &mut self,
        line: usize,
        text: &str,
        language: &str,
        theme: &Theme
    ) -> &[HighlightSpan] {
        let key = CacheKey {
            line_text: text.to_string(),
            language: language.to_string(),
            theme_hash: theme.hash(),
        };
        
        self.cache.get_or_insert_with(key, || {
            syntax::highlight_line(text, language)
        })
    }
}
```

### 3. 文本缓冲区优化
```rust
use ropey::Rope;
use std::sync::Arc;

struct TextBuffer {
    rope: Rope,
    lines: LineCache,
    revision: u64,
    change_listeners: Vec<Box<dyn Fn(TextChange)>>,
}

struct LineCache {
    line_ends: Vec<usize>,
    line_lengths: Vec<usize>,
    dirty: bool,
}

impl TextBuffer {
    fn insert(&mut self, position: usize, text: &str) -> TextChange {
        let old_length = self.rope.len_bytes();
        self.rope.insert(position, text);
        let new_length = self.rope.len_bytes();
        self.revision += 1;
        
        let change = TextChange {
            position,
            deleted_length: 0,
            inserted_text: text.to_string(),
            revision: self.revision,
        };
        
        self.notify_listeners(&change);
        self.lines.invalidate_from(position);
        change
    }
}
```

## 三、功能增强实现

### 1. 多光标系统
```rust
struct MultiCursor {
    cursors: Vec<Cursor>,
    primary_index: usize,
    selections: Vec<Selection>,
}

struct Cursor {
    position: usize,
    preferred_column: Option<usize>,
    selection_anchor: Option<usize>,
}

impl MultiCursor {
    fn add_cursor(&mut self, position: usize) {
        let cursor = Cursor::new(position);
        self.cursors.push(cursor);
    }
    
    fn sync_movement(&mut self, movement: CursorMovement) {
        for cursor in &mut self.cursors {
            cursor.apply_movement(&movement);
        }
    }
    
    fn apply_edit(&mut self, edit: &Edit) -> Vec<Edit> {
        // 为每个光标生成编辑操作
        self.cursors.iter()
            .map(|cursor| edit.translate(cursor.position))
            .collect()
    }
}
```

### 2. 撤消/重做系统
```rust
#[derive(Clone)]
enum EditOperation {
    Insert { position: usize, text: String },
    Delete { position: usize, text: String },
    Composite { operations: Vec<EditOperation> },
}

struct UndoHistory {
    undo_stack: Vec<EditGroup>,
    redo_stack: Vec<EditGroup>,
    current_group: Option<EditGroup>,
    grouping_depth: usize,
}

struct EditGroup {
    operations: Vec<EditOperation>,
    timestamp: Instant,
    selection_before: SelectionState,
    selection_after: SelectionState,
}

impl UndoHistory {
    fn begin_group(&mut self) {
        self.grouping_depth += 1;
        if self.grouping_depth == 1 {
            self.current_group = Some(EditGroup::new());
        }
    }
    
    fn end_group(&mut self) {
        self.grouping_depth = self.grouping_depth.saturating_sub(1);
        if self.grouping_depth == 0 {
            if let Some(group) = self.current_group.take() {
                self.undo_stack.push(group);
                self.redo_stack.clear();
            }
        }
    }
}
```

### 3. 查找替换功能
```rust
struct SearchEngine {
    query: String,
    options: SearchOptions,
    matches: Vec<SearchMatch>,
    current_match: usize,
    highlighter: MatchHighlighter,
}

struct SearchOptions {
    case_sensitive: bool,
    whole_word: bool,
    regex: bool,
    wrap_around: bool,
    incremental: bool,
}

struct SearchMatch {
    range: Range<usize>,
    capture_groups: Vec<Option<Range<usize>>>,
}

impl SearchEngine {
    fn find_all(&mut self, text: &str) -> Vec<SearchMatch> {
        if self.options.regex {
            self.find_regex(text)
        } else {
            self.find_literal(text)
        }
    }
    
    fn replace(&mut self, text: &str, replacement: &str) -> String {
        // 执行替换操作
    }
}
```

## 四、LSP集成深度优化

### 1. 完整的LSP功能支持
```rust
struct LspIntegration {
    client: LspClient,
    capabilities: ServerCapabilities,
    
    // 支持的功能
    diagnostics: DiagnosticsManager,
    symbols: SymbolManager,
    completions: CompletionProvider,
    hover: HoverProvider,
    signatures: SignatureHelpProvider,
    formatting: FormattingProvider,
    code_actions: CodeActionProvider,
    references: ReferenceProvider,
}

struct DiagnosticsManager {
    diagnostics: HashMap<String, Vec<Diagnostic>>,
    renderer: DiagnosticRenderer,
}

impl LspIntegration {
    async fn initialize(&mut self) -> Result<()> {
        // 协商能力
        let capabilities = self.client.initialize().await?;
        self.update_capabilities(capabilities);
        
        // 注册感兴趣的通知
        self.register_handlers();
        
        Ok(())
    }
    
    fn register_handlers(&mut self) {
        self.client.on_notification(
            "textDocument/publishDiagnostics",
            |params| self.handle_diagnostics(params)
        );
        
        self.client.on_notification(
            "window/showMessage",
            |params| self.handle_message(params)
        );
    }
}
```

### 2. 异步请求管理系统
```rust
struct LspRequestManager {
    pending_requests: HashMap<u64, PendingRequest>,
    request_counter: u64,
    timeout: Duration,
    executor: TaskExecutor,
}

struct PendingRequest {
    method: String,
    sent_at: Instant,
    sender: oneshot::Sender<JsonRpcResponse>,
    timeout_task: AbortHandle,
}

impl LspRequestManager {
    async fn send_request(
        &mut self,
        method: &str,
        params: Value
    ) -> Result<Value> {
        let id = self.request_counter;
        self.request_counter += 1;
        
        let (tx, rx) = oneshot::channel();
        
        let request = PendingRequest {
            method: method.to_string(),
            sent_at: Instant::now(),
            sender: tx,
            timeout_task: self.schedule_timeout(id),
        };
        
        self.pending_requests.insert(id, request);
        self.client.send_request(id, method, params).await?;
        
        // 等待响应或超时
        tokio::time::timeout(self.timeout, rx).await?
    }
}
```

### 3. 增量文档同步
```rust
struct DocumentSyncManager {
    version: i32,
    dirty: bool,
    sync_kind: TextDocumentSyncKind,
    change_delay: Duration,
    pending_changes: Vec<TextDocumentContentChangeEvent>,
}

impl DocumentSyncManager {
    fn record_change(&mut self, change: TextChange) {
        match self.sync_kind {
            TextDocumentSyncKind::None => return,
            TextDocumentSyncKind::Full => {
                self.dirty = true;
                self.pending_changes.clear();
            },
            TextDocumentSyncKind::Incremental => {
                self.pending_changes.push(change.to_lsp_format());
                self.dirty = true;
            },
        }
        
        self.schedule_sync();
    }
    
    fn schedule_sync(&mut self) {
        // 防抖延迟发送
        if let Some(task) = self.sync_task.take() {
            task.abort();
        }
        
        let delay = self.change_delay;
        self.sync_task = Some(tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            self.sync_changes().await;
        }));
    }
}
```

## 五、用户体验优化

### 1. 智能补全系统
```rust
struct SmartCompletion {
    items: Vec<CompletionItem>,
    filtered: Vec<CompletionItem>,
    context: CompletionContext,
    scoring: ScoringEngine,
}

struct CompletionContext {
    prefix: String,
    line_prefix: String,
    line_suffix: String,
    position: Position,
    language: String,
    trigger_kind: CompletionTriggerKind,
}

struct ScoringEngine {
    weights: ScoringWeights,
    cache: ScoringCache,
}

impl SmartCompletion {
    fn filter_and_sort(&mut self) {
        // 1. 过滤不匹配的项目
        self.filtered = self.items.iter()
            .filter(|item| self.matches_prefix(item))
            .cloned()
            .collect();
        
        // 2. 计算分数
        for item in &mut self.filtered {
            item.score = self.scoring.calculate_score(item, &self.context);
        }
        
        // 3. 排序
        self.filtered.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal)
        });
    }
}
```

### 2. 语法错误和警告
```rust
struct DiagnosticRenderer {
    diagnostics: Vec<Diagnostic>,
    severity_styles: HashMap<DiagnosticSeverity, DiagnosticStyle>,
}

struct DiagnosticStyle {
    underline_color: Hsla,
    underline_style: UnderlineStyle,
    gutter_color: Hsla,
    hover_background: Hsla,
}

impl DiagnosticRenderer {
    fn render(&self, window: &Window, bounds: Bounds<Pixels>) {
        for diagnostic in &self.diagnostics {
            // 绘制波浪线
            self.render_squiggly(diagnostic.range, diagnostic.severity);
            
            // 绘制行号标记
            self.render_gutter_marker(diagnostic);
            
            // 绘制悬停提示
            if is_hovered(diagnostic.range) {
                self.render_hover_tooltip(diagnostic);
            }
        }
    }
    
    fn render_squiggly(&self, range: Range<usize>, severity: DiagnosticSeverity) {
        let style = &self.severity_styles[&severity];
        
        // 绘制波浪线
        let path = create_squiggly_path(range);
        window.paint_path(&path, style.underline_color, style.underline_width);
    }
}
```

### 3. 代码格式化
```rust
struct CodeFormatter {
    providers: Vec<Box<dyn FormatProvider>>,
    fallback: Box<dyn FormatProvider>,
}

trait FormatProvider {
    fn supports(&self, language: &str) -> bool;
    fn format_range(
        &self,
        text: &str,
        range: Range<usize>,
        options: FormatOptions
    ) -> Result<String>;
    fn format_document(
        &self,
        text: &str,
        options: FormatOptions
    ) -> Result<String>;
}

impl CodeFormatter {
    async fn format(&self, request: FormatRequest) -> Result<String> {
        let provider = self.find_provider(&request.language)
            .unwrap_or(&*self.fallback);
        
        match request.range {
            Some(range) => provider.format_range(&request.text, range, request.options),
            None => provider.format_document(&request.text, request.options),
        }
    }
}
```

### 4. 主题系统
```rust
#[derive(Clone, Debug)]
pub struct Theme {
    name: String,
    author: String,
    version: String,
    colors: ThemeColors,
    tokens: TokenColors,
    ui: UiColors,
}

#[derive(Clone, Debug)]
pub struct ThemeColors {
    background: Hsla,
    foreground: Hsla,
    selection: Hsla,
    cursor: Hsla,
    
    // 语义颜色
    keyword: Hsla,
    string: Hsla,
    number: Hsla,
    comment: Hsla,
    function: Hsla,
    class: Hsla,
    variable: Hsla,
    
    // UI颜色
    gutter: Hsla,
    gutter_foreground: Hsla,
    line_highlight: Hsla,
    border: Hsla,
}

impl Theme {
    fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(Into::into)
    }
    
    fn apply(&self, editor: &mut CodeEditor) {
        editor.set_background_color(self.colors.background);
        editor.set_foreground_color(self.colors.foreground);
        // ... 应用所有颜色
    }
}
```

## 六、性能监控与调试

### 1. 性能指标收集
```rust
#[derive(Debug, Clone)]
struct PerformanceMetrics {
    // 渲染性能
    frame_time: Duration,
    frame_rate: f32,
    render_time: Duration,
    layout_time: Duration,
    
    // 编辑性能
    edit_latency: Duration,
    completion_latency: Duration,
    highlight_time: Duration,
    
    // 内存使用
    memory_usage: usize,
    cache_hit_rate: f32,
    buffer_size: usize,
    
    // LSP性能
    lsp_request_latency: Duration,
    lsp_notification_rate: f32,
}

struct PerformanceMonitor {
    metrics: Arc<Mutex<PerformanceMetrics>>,
    sampler: PerformanceSampler,
    reporter: MetricsReporter,
}

impl PerformanceMonitor {
    fn start_frame(&mut self) {
        self.sampler.start_frame();
    }
    
    fn end_frame(&mut self) {
        let metrics = self.sampler.end_frame();
        self.metrics.lock().unwrap().update(metrics);
        
        if self.reporter.should_report() {
            self.reporter.report(&self.metrics.lock().unwrap());
        }
    }
}
```

### 2. 调试工具
```rust
struct DebugOverlay {
    enabled: bool,
    panels: Vec<DebugPanel>,
    hotkeys: DebugHotkeys,
}

enum DebugPanel {
    Performance,
    Memory,
    Render,
    Cache,
    Lsp,
}

impl DebugOverlay {
    fn toggle(&mut self, panel: DebugPanel) {
        if let Some(index) = self.panels.iter().position(|p| *p == panel) {
            self.panels.remove(index);
        } else {
            self.panels.push(panel);
        }
    }
    
    fn render(&self, window: &Window, cx: &mut Context) {
        if !self.enabled {
            return;
        }
        
        for panel in &self.panels {
            match panel {
                DebugPanel::Performance => self.render_performance_panel(window, cx),
                DebugPanel::Memory => self.render_memory_panel(window, cx),
                DebugPanel::Render => self.render_render_panel(window, cx),
                DebugPanel::Cache => self.render_cache_panel(window, cx),
                DebugPanel::Lsp => self.render_lsp_panel(window, cx),
            }
        }
    }
    
    fn render_performance_panel(&self, window: &Window, cx: &mut Context) {
        let metrics = self.monitor.metrics.lock().unwrap();
        
        // 绘制性能图表
        let chart_data = vec![
            ("Frame Time", metrics.frame_time.as_secs_f32() * 1000.0),
            ("Render Time", metrics.render_time.as_secs_f32() * 1000.0),
            ("FPS", metrics.frame_rate),
        ];
        
        self.draw_chart(window, "Performance", &chart_data);
    }
}
```

## 七、代码质量与可维护性

### 1. 错误处理系统
```rust
#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("LSP error: {0}")]
    Lsp(#[from] LspError),
    
    #[error("Parse error at line {line}, column {column}: {message}")]
    Parse {
        line: usize,
        column: usize,
        message: String,
    },
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Plugin error: {0}")]
    Plugin(String),
    
    #[error("Render error: {0}")]
    Render(String),
}

impl EditorError {
    fn severity(&self) -> ErrorSeverity {
        match self {
            EditorError::Io(_) => ErrorSeverity::Error,
            EditorError::Lsp(_) => ErrorSeverity::Warning,
            EditorError::Parse { .. } => ErrorSeverity::Error,
            EditorError::Config(_) => ErrorSeverity::Warning,
            EditorError::Plugin(_) => ErrorSeverity::Warning,
            EditorError::Render(_) => ErrorSeverity::Error,
        }
    }
}

struct ErrorHandler {
    errors: Vec<EditorError>,
    notifier: ErrorNotifier,
}

impl ErrorHandler {
    fn handle(&mut self, error: EditorError, cx: &mut Context) {
        self.errors.push(error.clone());
        
        match error.severity() {
            ErrorSeverity::Error => {
                self.notifier.show_error(&error, cx);
                log::error!("{}", error);
            },
            ErrorSeverity::Warning => {
                self.notifier.show_warning(&error, cx);
                log::warn!("{}", error);
            },
            ErrorSeverity::Info => {
                log::info!("{}", error);
            },
        }
    }
}
```

### 2. 测试策略
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    mod text_buffer {
        #[test]
        fn test_insert_text() {
            let mut buffer = TextBuffer::new();
            buffer.insert(0, "Hello");
            assert_eq!(buffer.text(), "Hello");
        }
        
        #[test]
        fn test_delete_range() {
            let mut buffer = TextBuffer::from("Hello World");
            buffer.delete(5..11);
            assert_eq!(buffer.text(), "Hello");
        }
    }
    
    mod selection {
        #[test]
        fn test_selection_expansion() {
            let mut selection = Selection::new(0..5);
            selection.expand_to(10);
            assert_eq!(selection.range(), 0..10);
        }
    }
    
    mod completion {
        #[test]
        fn test_completion_filtering() {
            let items = vec![
                CompletionItem::new("hello"),
                CompletionItem::new("world"),
                CompletionItem::new("help"),
            ];
            
            let filtered = filter_completions(&items, "he");
            assert_eq!(filtered.len(), 2);
            assert!(filtered.iter().any(|i| i.label == "hello"));
            assert!(filtered.iter().any(|i| i.label == "help"));
        }
    }
    
    mod integration {
        #[test]
        fn test_editor_integration() {
            let (mut editor, cx) = setup_editor();
            
            // 模拟用户输入
            editor.insert_text("def hello():\n    return 'world'", &mut cx);
            
            // 验证状态
            assert_eq!(editor.line_count(), 2);
            assert!(editor.has_selection() == false);
        }
    }
}
```

### 3. 文档系统
```rust
//! # CodeEditor - 现代化的代码编辑器
//!
//! 这是一个功能丰富的代码编辑器，支持：
//! - 语法高亮
//! - 代码补全
//! - LSP集成
//! - 多光标编辑
//! - 代码折叠
//!
//! ## 快速开始
//!
//! ```rust
//! let editor = CodeEditor::new(cx);
//! editor.set_language("rust");
//! editor.load_file("src/main.rs");
//! ```
//!
//! ## 架构概述
//!
//! 编辑器采用分层架构：
//!
//! 1. **核心层**：文本缓冲区、光标管理、历史记录
//! 2. **语法层**：语法高亮、代码折叠、缩进处理
//! 3. **LSP层**：语言服务器协议集成
//! 4. **UI层**：渲染、布局、用户交互

/// 文本缓冲区结构
///
/// 这是编辑器的核心数据结构，负责存储和操作文本内容。
/// 使用Rope数据结构高效支持大文件编辑。
///
/// # 示例
///
/// ```
/// let mut buffer = TextBuffer::new();
/// buffer.insert(0, "Hello, World!");
/// assert_eq!(buffer.len(), 13);
/// ```
pub struct TextBuffer {
    // ...
}

/// 执行文本插入操作
///
/// # 参数
/// - `position`: 插入位置（字节偏移量）
/// - `text`: 要插入的文本
///
/// # 返回值
/// 返回一个`TextChange`对象，描述所做的更改
///
/// # 错误
/// 如果位置超出缓冲区范围，返回`EditorError::OutOfBounds`
///
/// # 性能
/// 时间复杂度：O(log N)，其中N是缓冲区大小
pub fn insert_text(&mut self, position: usize, text: &str) -> Result<TextChange> {
    // ...
}
```

## 八、扩展性与插件系统

### 1. 插件架构设计
```rust
/// 插件接口
pub trait EditorPlugin: Send + Sync {
    /// 插件名称
    fn name(&self) -> &str;
    
    /// 插件版本
    fn version(&self) -> &str;
    
    /// 激活插件
    fn activate(&mut self, context: PluginContext) -> Result<()>;
    
    /// 停用插件
    fn deactivate(&mut self) -> Result<()>;
    
    /// 处理编辑器事件
    fn on_event(&mut self, event: &EditorEvent, cx: &mut Context) -> EventResult;
    
    /// 获取插件命令
    fn commands(&self) -> Vec<Command>;
}

/// 插件管理器
pub struct PluginManager {
    plugins: HashMap<String, Box<dyn EditorPlugin>>,
    context: PluginContext,
    event_bus: EventBus,
}

impl PluginManager {
    /// 加载插件
    pub fn load_plugin<P: EditorPlugin + 'static>(
        &mut self,
        plugin: P
    ) -> Result<()> {
        let name = plugin.name().to_string();
        
        let mut boxed_plugin = Box::new(plugin);
        boxed_plugin.activate(self.context.clone())?;
        
        self.plugins.insert(name, boxed_plugin);
        Ok(())
    }
    
    /// 触发事件
    pub fn dispatch_event(&mut self, event: EditorEvent, cx: &mut Context) {
        for plugin in self.plugins.values_mut() {
            if let EventResult::Handled = plugin.on_event(&event, cx) {
                break;
            }
        }
    }
}
```

### 2. 自定义语言支持
```rust
/// 语言支持接口
pub trait LanguageSupport: Send + Sync {
    /// 语言标识符
    fn language_id(&self) -> &str;
    
    /// 文件扩展名
    fn file_extensions(&self) -> &[&str];
    
    /// 语法高亮规则
    fn highlight_rules(&self) -> HighlightRules;
    
    /// 缩进规则
    fn indentation_rules(&self) -> IndentationRules;
    
    /// 代码折叠规则
    fn folding_rules(&self) -> FoldingRules;
    
    /// 补全触发器
    fn completion_triggers(&self) -> &[char];
    
    /// LSP配置
    fn lsp_config(&self) -> Option<LspConfig>;
}

/// 语言管理器
pub struct LanguageManager {
    languages: HashMap<String, Box<dyn LanguageSupport>>,
    file_associations: HashMap<String, String>, // 扩展名 -> 语言ID
}

impl LanguageManager {
    /// 检测文件语言
    pub fn detect_language(&self, path: &Path) -> Option<&str> {
        // 1. 通过扩展名检测
        if let Some(ext) = path.extension() {
            if let Some(lang_id) = self.file_associations.get(ext.to_str()?) {
                return Some(lang_id);
            }
        }
        
        // 2. 通过文件名检测
        if let Some(file_name) = path.file_name() {
            if let Some(lang_id) = self.file_associations.get(file_name.to_str()?) {
                return Some(lang_id);
            }
        }
        
        // 3. 通过文件内容检测
        self.detect_by_content(path)
    }
}
```

## 九、部署与发布

### 1. 构建配置
```toml
# Cargo.toml
[package]
name = "code-editor"
version = "0.1.0"
edition = "2021"

[features]
default = ["lsp", "completion", "syntax-highlight"]
lsp = ["tower-lsp", "serde_json"]
completion = ["fuzzy-matcher"]
syntax-highlight = ["tree-sitter", "syntect"]
plugins = ["libloading", "wasmtime"]

[dependencies]
gpui = "0.1"
ropey = "0.10"
tower-lsp = { version = "0.20", optional = true }
tree-sitter = { version = "0.20", optional = true }

[dev-dependencies]
criterion = "0.5"
proptest = "1.0"
```

### 2. 性能基准测试
```rust
#[cfg(bench)]
mod benches {
    use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
    use super::*;
    
    fn bench_text_insertion(c: &mut Criterion) {
        let mut group = c.benchmark_group("text_insertion");
        
        for size in [100, 1000, 10000].iter() {
            group.bench_with_input(
                BenchmarkId::from_parameter(size),
                size,
                |b, &size| {
                    b.iter(|| {
                        let mut buffer = TextBuffer::new();
                        for i in 0..size {
                            buffer.insert(i, "a");
                        }
                    });
                }
            );
        }
        
        group.finish();
    }
    
    fn bench_syntax_highlight(c: &mut Criterion) {
        let mut group = c.benchmark_group("syntax_highlight");
        
        let test_files = [
            ("small.rs", include_str!("../test_data/small.rs")),
            ("medium.rs", include_str!("../test_data/medium.rs")),
            ("large.rs", include_str!("../test_data/large.rs")),
        ];
        
        for (name, content) in test_files.iter() {
            group.bench_with_input(
                BenchmarkId::new("highlight", name),
                content,
                |b, content| {
                    b.iter(|| {
                        let highlighter = SyntaxHighlighter::new();
                        highlighter.highlight(content, "rust");
                    });
                }
            );
        }
        
        group.finish();
    }
    
    criterion_group!(benches, bench_text_insertion, bench_syntax_highlight);
    criterion_main!(benches);
}
```

## 十、路线图与优先级

### 高优先级（1-2周）
1. **增量渲染系统** - 提升大文件性能
2. **撤消/重做** - 基础编辑功能
3. **错误处理** - 提高稳定性
4. **配置系统** - 用户自定义

### 中优先级（2-4周）
1. **多光标支持** - 现代化编辑体验
2. **代码折叠** - 大文件导航
3. **查找替换** - 核心编辑功能
4. **性能监控** - 开发和调试工具

### 低优先级（1-2月）
1. **插件系统** - 扩展性和生态系统
2. **主题系统** - 个性化定制
3. **高级补全** - AI辅助编程
4. **协作编辑** - 实时协作支持

### 长期目标（3-6月）
1. **WASM支持** - 浏览器部署
2. **移动端适配** - 平板和手机
3. **云同步** - 设置和文件同步
4. **扩展市场** - 插件生态系统

## 总结

这个优化方案为CodeEditor项目提供了全面的改进方向，从架构重构到功能增强，从性能优化到用户体验提升。建议按照优先级逐步实施这些改进，每个阶段都进行充分的测试和验证。

关键的成功因素包括：
1. **保持向后兼容性** - 逐步迁移，不影响现有功能
2. **关注性能指标** - 每个优化都要有可衡量的改进
3. **用户反馈循环** - 与用户保持沟通，优先实现最需要的功能
4. **代码质量标准** - 保持高测试覆盖率和文档完整性

通过系统性的优化，CodeEditor可以成为一个功能强大、性能优异、可扩展性强的现代化代码编辑器。