package dev.lm.tester.util

import dev.lm.tester.config.LogConfig

/**
 * Centralized logging utility.
 * Logs are only printed when corresponding config flags are enabled.
 */
object Logger {
    /**
     * Debug log - only prints when debug mode is enabled
     */
    fun d(tag: String, msg: String) {
        if (LogConfig.debug) {
            println("[DEBUG][$tag] $msg")
        }
    }
    
    /**
     * Verbose/Info log - only prints when verbose mode is enabled
     */
    fun v(tag: String, msg: String) {
        if (LogConfig.verbose) {
            println("[$tag] $msg")
        }
    }
    
    /**
     * Info log - always prints (important milestones)
     */
    fun i(tag: String, msg: String) {
        println("[$tag] $msg")
    }
    
    /**
     * Warning log - always prints with WARN prefix
     */
    fun w(tag: String, msg: String) {
        println("[WARN][$tag] $msg")
    }
    
    /**
     * Error log - always prints with ERROR prefix
     */
    fun e(tag: String, msg: String) {
        println("[ERROR][$tag] $msg")
    }
    
    /**
     * Error log with exception
     */
    fun e(tag: String, msg: String, e: Throwable) {
        println("[ERROR][$tag] $msg: ${e.message}")
        if (LogConfig.debug) {
            e.printStackTrace()
        }
    }
}
