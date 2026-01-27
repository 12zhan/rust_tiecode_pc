package org.xiaoa.jna.tiec

import androidx.compose.ui.graphics.Color
import com.google.gson.Gson
import com.sun.jna.Pointer
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.xiaoa.components.codeEdit.*

/** 基于 JNA 的 LSP 服务实现 */
class JnaTiecodeLspService(private val debug: Boolean = false) :
        CodeCompletionProvider,
        CodeDiagnosticsProvider,
        InlineCompletionProvider,
        CodeHoverProvider,
        CodeDefinitionProvider {

    private var rootDir: File? = null
    private val gson = Gson()

    // 编译器句柄
    private var contextHandle: Pointer? = null
    private var ideServiceHandle: Pointer? = null

    private val lock = Any()
    @Volatile private var initialized = false

    // 缓存 Gson 解析的结果类
    private data class CompletionResultWrapper(val items: List<TCCompletionItem>)
    private data class LintResultWrapper(val diagnostics: List<TCDiagnostic>)

    companion object {
        private val GLOBAL_LOCK = Any()
    }

    init {
        System.setProperty("jna.encoding", "UTF-8")
        // Lazy initialize on first request
    }

    fun initialize(newRoot: File) {
        // 使用全局锁防止多实例间原生资源竞争
        synchronized(GLOBAL_LOCK) {
            // If already initialized for this root, skip
            if (initialized && rootDir == newRoot) return

            // If initialized for another root, dispose first
            if (initialized) {
                dispose()
            }

            rootDir = newRoot
            try {
                println("[JnaLsp] Initializing new context for $rootDir...")
                // ... (rest of initialize)

                // 1. 创建 Context
                val options =
                        TCOptions(
                                packageName = "com.example.app", // TODO: 从配置读取
                                outputDir = File(newRoot, "build/bin").absolutePath,
                                debug = debug,
                                ideMode = true
                        )
                contextHandle = TiecodeNative.createContext(options)
                println("[JnaLsp] Context created: $contextHandle")

                if (contextHandle == null) {
                    println("[JnaLsp] Failed to create context")
                    return
                }

                // 2. 创建 IDE Service
                ideServiceHandle = TiecodeNative.createIdeService(contextHandle)
                println("[JnaLsp] IdeService created: $ideServiceHandle")

                if (ideServiceHandle == null) {
                    println("[JnaLsp] Failed to create IDE service")
                    return
                }

                initialized = true
                println("[JnaLsp] Initialized successfully. Root: $rootDir")
            } catch (e: Exception) {
                println("[JnaLsp] Init error: ${e.message}")
                e.printStackTrace()
            }
        }
    }

    /** 预热：扫描项目文件并预编译 */
    suspend fun warmUp() {
        if (!initialized) return
        val currentRoot = rootDir ?: return
        withContext(Dispatchers.IO) {
            synchronized(lock) {
                if (ideServiceHandle == null) {
                    println("[JnaLsp] warmUp skipped, handle is null")
                    return@synchronized
                }
                val files = scanTiecodeFiles(currentRoot)
                println("[JnaLsp] Scanning files in $currentRoot found ${files.size} files")
                if (files.isNotEmpty()) {
                    val filePaths = files.map { it.absolutePath }.toTypedArray()
                    println(
                            "[JnaLsp] Calling ideServiceCompileFiles with ${filePaths.size} files. First: ${filePaths.firstOrNull()}"
                    )
                    try {
                        val ret = TiecodeNative.ideServiceCompileFiles(ideServiceHandle, filePaths)
                        println("[JnaLsp] warmUp compiled ${files.size} files, ret=$ret")
                    } catch (e: Error) {
                        println("[JnaLsp] CRITICAL: Native Error in warmUp: ${e.message}")
                        e.printStackTrace()
                    } catch (e: Exception) {
                        println("[JnaLsp] Exception in warmUp: ${e.message}")
                        e.printStackTrace()
                    }
                }
            }
        }
    }

    private fun scanTiecodeFiles(dir: File): List<File> {
        val result = mutableListOf<File>()
        dir.walkTopDown().forEach {
            if (it.isFile && it.extension == "t") {
                result.add(it)
            }
        }
        return result
    }

    private fun ensureInitialized(filePath: String) {
        val file = File(filePath)
        
        // 如果已经初始化了，检查文件是否属于当前项目
        // 简单的逻辑：如果 initialized 为 true，我们假设已经正确设置了 rootDir (通过 onProjectOpened)
        // 除非我们想支持多项目同时打开（目前架构是一个 rootDir）
        // 如果没有初始化，尝试自动推断（向后兼容）
        
        if (initialized && rootDir != null) {
            // 可以在这里加一个检查，如果文件完全在项目外，是否需要警告？
            // 暂时忽略
            return
        }

        // Fallback: auto detect if not initialized via onProjectOpened
        // Find project root: look for "结绳项目.json" or default to parent dir
        var current = file.parentFile
        var foundRoot: File? = null
        while (current != null) {
            if (File(current, "结绳项目.json").exists()) {
                foundRoot = current
                break
            }
            current = current.parentFile
        }
        val targetRoot = foundRoot ?: file.parentFile ?: File(".")
        
        if (rootDir != targetRoot) {
            initialize(targetRoot)
            // Auto warm up for new root
            // Note: warmUp is suspend, so we can't call it directly here easily unless we launch it
            // For now, we just rely on lazy compilation or explicit warmup calls if possible.
            // But since this is called from suspend functions usually, we might be okay?
            // ensureInitialized is called from suspend functions? No, it's called from ensureFileUpdated which is not suspend.
            // Wait, ensureFileUpdated is called from suspend functions.
        }
    }

    private fun ensureFileUpdated(filePath: String?, text: String) {
        if (filePath == null) return // 忽略无路径文件 (或者可以使用临时路径)
        
        ensureInitialized(filePath)

        if (ideServiceHandle == null) return
        // 注意：这里简单起见，每次请求都全量更新 current file
        // 实际优化可以用 hash 或增量
        TiecodeNative.ideServiceEditSource(ideServiceHandle, filePath, text)
    }

    override suspend fun requestCompletion(request: CodeCompletionRequest): List<CompletionItem> {
        val filePath = request.filePath
        if (filePath == null) return emptyList()

        return withContext(Dispatchers.IO) {
            synchronized(lock) {
                // 1. 更新当前文件内容 (will also initialize if needed)
                ensureFileUpdated(filePath, request.text)

                if (ideServiceHandle == null) return@synchronized emptyList<CompletionItem>()

                // 2. 构造请求参数
                val params =
                        TCCompletionParams(
                                uri = filePath,
                                position = TCPosition(request.lineIndex, request.columnIndex),
                                partial = request.prefix,
                                triggerChar = null
                        )

                // 3. 调用 JNA
                val jsonRes =
                        TiecodeNative.ideServiceComplete(ideServiceHandle, params)
                                ?: return@synchronized emptyList<CompletionItem>()

                // 4. 解析结果
                try {
                    val result = gson.fromJson(jsonRes, TCCompletionResult::class.java)
                    // ANSI Colors: Green for success, Yellow for count
                    println(
                            "\u001B[32m[JnaLsp] Completion result:\u001B[0m count=\u001B[33m${result.items.size}\u001B[0m"
                    )
                    result.items.map { it.toUiModel() }
                } catch (e: Exception) {
                    println(
                            "\u001B[31m[JnaLsp] Completion parse error: ${e.message}, json=$jsonRes\u001B[0m"
                    )
                    emptyList()
                }
            }
        }
    }

    // 内联补全暂未对接，返回 null
    override suspend fun requestInlineCompletion(
            request: InlineCompletionRequest
    ): InlineCompletionResult? {
        return null
    }

    override suspend fun requestHover(request: CodeHoverRequest): HoverResult? {
        val filePath = request.filePath
        if (!initialized || filePath == null) return null
        return withContext(Dispatchers.IO) {
            synchronized(lock) {
                if (ideServiceHandle == null) return@synchronized null
                ensureFileUpdated(filePath, request.text)
                val line = request.lineIndex
                val col = request.columnIndex
                val params =
                    TCCursorParams(
                        uri = filePath,
                        position = TCPosition(line, col),
                        lineText = null
                    )
                val jsonRes = TiecodeNative.ideServiceHover(ideServiceHandle, params) ?: return@synchronized null
                try {
                    val content = gson.fromJson(jsonRes, TCMarkupContent::class.java)
                    if (content.text.isBlank()) return@synchronized null
                    HoverResult(text = content.text, range = null)
                } catch (e: Exception) {
                    println("[JnaLsp] Hover parse error: ${e.message}, json=$jsonRes")
                    null
                }
            }
        }
    }

    override suspend fun findDefinition(request: CodeDefinitionRequest): DefinitionResult? {
        val filePath = request.filePath
        if (!initialized || filePath == null) return null

        return withContext(Dispatchers.IO) {
            synchronized(lock) {
                if (ideServiceHandle == null) return@synchronized null
                ensureFileUpdated(filePath, request.text)

                // 1. 构造请求参数
                val params =
                        TCCursorParams(
                            uri = filePath,
                            position = TCPosition(request.lineIndex, request.columnIndex),
                            lineText = null
                        )

                // 2. 调用 JNA
                val jsonRes =
                        TiecodeNative.ideServiceFindDefinition(ideServiceHandle, params)
                                ?: return@synchronized null
                println("[JnaLsp] findDefinition result: $jsonRes")

                // 3. 解析结果
                try {
                    // Result: { "uri": "file:///...", "range": ... }
                    val location = gson.fromJson(jsonRes, TCLocation::class.java)

                    if (location != null) {
                        // Convert URI to File Path
                        val uriStr = location.uri
                        val path =
                                if (uriStr.startsWith("file://")) {
                                    try {
                                        File(java.net.URI(uriStr)).absolutePath
                                    } catch (e: Exception) {
                                        // Fallback if URI parsing fails
                                        uriStr.removePrefix("file://")
                                    }
                                } else {
                                    uriStr
                                }

                        DefinitionResult(
                                filePath = path,
                                line = location.range.start.line,
                                column = location.range.start.column
                        )
                    } else {
                        null
                    }
                } catch (e: Exception) {
                    println(
                            "\u001B[31m[JnaLsp] Definition parse error: ${e.message}, json=$jsonRes\u001B[0m"
                    )
                    null
                }
            }
        }
    }

    override suspend fun requestDiagnostics(filePath: String?, text: String): List<CodeDiagnostic> {
        if (filePath == null) return emptyList()

        return withContext(Dispatchers.IO) {
            synchronized(lock) {
                ensureFileUpdated(filePath, text)
                if (ideServiceHandle == null) return@synchronized emptyList<CodeDiagnostic>()

                val jsonRes =
                        TiecodeNative.ideServiceLintFile(ideServiceHandle, filePath)
                                ?: return@synchronized emptyList<CodeDiagnostic>()

                try {
                    val result = gson.fromJson(jsonRes, TCLintResult::class.java)

                    // 需要计算 offset
                    val lines = buildLines(text)
                    val lineStarts = buildLineStartOffsets(lines)

                    result.diagnostics.map { diag ->
                        // 1-based -> 0-based (Fixed: now treating as 0-based from native)
                        val startLine = (diag.range.start.line).coerceAtLeast(0)
                        val startCol = (diag.range.start.column).coerceAtLeast(0)
                        val endLine = (diag.range.end.line).coerceAtLeast(0)
                        val endCol = (diag.range.end.column).coerceAtLeast(0)

                        val startOffset = lineColumnToOffset(lines, lineStarts, startLine, startCol)
                        val endOffset =
                                lineColumnToOffset(lines, lineStarts, endLine, endCol)
                                        .coerceAtLeast(startOffset + 1)

                        CodeDiagnostic(
                                startOffset = startOffset,
                                endOffset = endOffset,
                                message = diag.message,
                                color = severityToColor(diag.level)
                        )
                    }
                } catch (e: Exception) {
                    println("[JnaLsp] Lint parse error: ${e.message}")
                    emptyList()
                }
            }
        }
    }

    private fun severityToColor(level: Int): Long {
        // level: 1=Info, 2=Warning, 3=Error (Hypothetically)
        // TCLogLevel: DEBUG(0), INFO(1), WARNING(2), ERROR(3)
        return when (level) {
            3 -> 0xFFEF5350 // Error Red
            2 -> 0xFFFFCA28 // Warning Amber
            else -> 0xFF9E9E9E // Info Grey
        }
    }

    private fun TCCompletionItem.toUiModel(): CompletionItem {
        val kindColor =
                when (kind) {
                    // 根据 kind 决定颜色，这里简单硬编码
                    0,
                    1 -> Color(0xFF64B5F6) // Class/Method
                    2 -> Color(0xFFFFB74D) // Var
                    else -> Color(0xFFE57373)
                }

        return CompletionItem(
                title = label,
                signature = detail ?: "",
                color = kindColor,
                insertText = label
        )
    }

    fun dispose() {
        synchronized(GLOBAL_LOCK) {
            println("[JnaLsp] Disposing service for $rootDir")
            // 同时锁住实例锁，防止 warmUp 还在跑
            synchronized(lock) {
                if (ideServiceHandle != null) {
                    println("[JnaLsp] Freeing ideService: $ideServiceHandle")
                    TiecodeNative.freeIdeService(ideServiceHandle)
                    ideServiceHandle = null
                }
                if (contextHandle != null) {
                    println("[JnaLsp] Freeing context: $contextHandle")
                    TiecodeNative.freeContext(contextHandle)
                    contextHandle = null
                }
                initialized = false
            }
        }
    }
}
