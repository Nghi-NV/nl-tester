package dev.lm.tester.network

import android.util.Log
import java.net.ServerSocket
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

class CommandServer(private val port: Int) {
    private val executor = Executors.newCachedThreadPool()
    @Volatile
    private var isRunning = false
    private var serverSocket: ServerSocket? = null

    fun start() {
        Thread {
            try {
                serverSocket = ServerSocket(port)
                isRunning = true
                Log.d("NL_MIRROR", "Command server started on port $port")
                while (isRunning) {
                    try {
                        val client = serverSocket?.accept() ?: break
                        executor.execute {
                            try {
                                val inputStream = client.getInputStream()
                                val outputStream = client.getOutputStream()
                                val reader = inputStream.bufferedReader()
                                Log.d("NL_MIRROR", "Command client connected: ${client.inetAddress.hostAddress}")
                                while (!client.isClosed && isRunning) {
                                    val response = CommandHandler.handleCommand(reader)
                                    if (response == null) {
                                        Log.d("NL_MIRROR", "Command client disconnected (EOF): ${client.inetAddress.hostAddress}")
                                        break
                                    }
                                    outputStream.write((response + "\n").toByteArray())
                                    outputStream.flush()
                                }
                            } catch (e: Exception) {
                                Log.d("NL_MIRROR", "Command client error: ${e.message}")
                            } finally {
                                try { client.close() } catch (_: Exception) {}
                                Log.d("NL_MIRROR", "Command client disconnected")
                            }
                        }
                    } catch (e: Exception) {
                        if (!isRunning) break
                        Log.e("NL_MIRROR", "Command server accept error: ${e.message}")
                    }
                }
            } catch (e: Exception) {
                Log.e("NL_MIRROR", "Command server failed to start or crashed: ${e.message}", e)
            } finally {
                try { serverSocket?.close() } catch (_: Exception) {}
                Log.d("NL_MIRROR", "Command server stopped.")
            }
        }.start()
    }

    fun stop() {
        isRunning = false
        try { serverSocket?.close() } catch (_: Exception) {}
        executor.shutdown()
        try {
            if (!executor.awaitTermination(2, TimeUnit.SECONDS)) {
                executor.shutdownNow()
            }
        } catch (_: Exception) {
            executor.shutdownNow()
        }
    }
}
