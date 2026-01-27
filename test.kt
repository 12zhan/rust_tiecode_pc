package org.xiaoa.jna.tiec

import com.google.gson.Gson
import java.io.File

fun getFilesWithSuffix(path: String, suffix: String): ArrayList<String> {
    val result = ArrayList<String>()
    val dir = File(path)
    if (!dir.exists() || !dir.isDirectory) return result
    dir.listFiles()?.forEach { file ->
        if (file.isDirectory) {
            result.addAll(getFilesWithSuffix(file.absolutePath, suffix))
        } else if (file.name.endsWith(suffix)) {
            result.add(file.absolutePath)
        }
    }
    return result
}

fun main() {
    val gson = Gson()

    // 1. 创建配置
    val option =
            TCOptions(
                    packageName = "结绳.中文",
                    outputDir = "C:\\Users\\xiaoa\\.tiecode\\project\\Demo1\\build",
                    ideMode = true
            )

    // 2. 创建 Context
    val context = TiecodeNative.createContext(option)
    if (context == null) {
        println("Failed to create context")
        return
    }

    // 3. 创建 IDE 服务
    val ideService = TiecodeNative.createIdeService(context)

    // 4. 预编译文件
    val files =
            getFilesWithSuffix("C:\\Users\\xiaoa\\.tiecode\\project\\tie_c", ".t").toTypedArray()
    TiecodeNative.ideServiceCompileFiles(ideService, files)

    // 5. 测试提示补全
    var completionParams =
            TCCompletionParams(
                    uri =
                            File("C:\\Users\\xiaoa\\.tiecode\\project\\tie_c\\源代码\\sample1.t")
                                    .toURI()
                                    .toString(),
                    position = TCPosition(17, 9),
                    partial = "调试",
                    triggerChar = "调试"
            )

    var resultJson = TiecodeNative.ideServiceComplete(ideService, completionParams)
    println("Completion Result: $resultJson")

    // 跳转定义
    val findDefintion =
            TCCursorParams(
                    uri =
                            File("C:\\Users\\xiaoa\\.tiecode\\project\\tie_c\\源代码\\sample1.t")
                                    .toURI()
                                    .toString(),
                    position = TCPosition(1, 13)
            )

    resultJson = TiecodeNative.ideServiceFindDefinition(ideService, findDefintion)
    println(resultJson.toString())

    // 代码差错
    //    resultJson =
    // TiecodeNative.ideServiceLintFile(ideService,File("C:\\Users\\xiaoa\\.tiecode\\project\\Demo1\\源代码\\初始代码.t").toURI().toString())
    //
    //    println(resultJson)
}
