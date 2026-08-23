package dev.lm.tester.util

import org.json.JSONObject
import java.io.BufferedReader
import java.io.InputStreamReader

/**
 * ViewHierarchyDumper extracts the current UI structure for automation and testing.
 * Since app_process doesn't have access to UiAutomation, we use uiautomator dump command.
 */
object ViewHierarchyDumper {

    /**
     * Dumps the current view hierarchy as a JSON string.
     */
    fun dump(): String {
        return try {
            // Use uiautomator dump command to get hierarchy XML
            val dumpPath = "/data/local/tmp/ui_dump.xml"
            val dumpProcess = Runtime.getRuntime().exec(arrayOf("uiautomator", "dump", dumpPath))
            dumpProcess.waitFor()
            
            // Read the XML file
            val catProcess = Runtime.getRuntime().exec(arrayOf("cat", dumpPath))
            val reader = BufferedReader(InputStreamReader(catProcess.inputStream))
            val xml = reader.readText()
            reader.close()
            
            if (xml.isBlank()) {
                return """{"error": "Empty hierarchy dump"}"""
            }
            
            // Return as JSON with XML data
            JSONObject().apply {
                put("format", "xml")
                put("data", xml)
            }.toString()
        } catch (e: Exception) {
            e.printStackTrace()
            """{"error": "${e.message}"}"""
        }
    }

    /**
     * Gets simplified hierarchy info.
     */
    fun getSimpleInfo(): String {
        return try {
            val process = Runtime.getRuntime().exec(arrayOf("dumpsys", "activity", "top"))
            val reader = BufferedReader(InputStreamReader(process.inputStream))
            val output = StringBuilder()
            var line: String?
            var lineCount = 0
            while (reader.readLine().also { line = it } != null && lineCount < 50) {
                output.appendLine(line)
                lineCount++
            }
            reader.close()

            JSONObject().apply {
                put("format", "text")
                put("data", output.toString())
            }.toString()
        } catch (e: Exception) {
            """{"error": "${e.message}"}"""
        }
    }
}
