package com.duckweed.toolkit

import android.annotation.SuppressLint
import android.os.Bundle
import android.util.Log
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.appcompat.app.AppCompatActivity
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.net.Socket
import kotlin.concurrent.thread

class MainActivity : AppCompatActivity() {

    private var serverProcess: Process? = null
    private lateinit var webView: WebView
    private val serverPort = 8000
    private val TAG = "DuckweedWrapper"

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Initialize WebView
        webView = WebView(this)
        setContentView(webView)

        webView.settings.apply {
            javaScriptEnabled = true
            domStorageEnabled = true
            mixedContentMode = WebSettings.MIXED_CONTENT_ALWAYS_ALLOW
            loadWithOverviewMode = true
            useWideViewPort = true
        }

        webView.webViewClient = object : WebViewClient() {
            override fun shouldOverrideUrlLoading(view: WebView?, url: String?): Boolean {
                return false // Handle all navigation in WebView itself
            }
        }

        // Start server and load web view
        startServerAndLoadUI()
    }

    private fun startServerAndLoadUI() {
        thread(start = true) {
            try {
                val binaryFile = prepareServerBinary()
                if (binaryFile == null) {
                    Log.e(TAG, "Failed to copy and prepare server binary")
                    return@thread
                }

                // Start process
                Log.i(TAG, "Launching server binary at ${binaryFile.absolutePath}")
                val processBuilder = ProcessBuilder(
                    binaryFile.absolutePath,
                    "--port", serverPort.toString(),
                    "--data-dir", filesDir.absolutePath
                )
                
                // Set working directory
                processBuilder.directory(filesDir)
                serverProcess = processBuilder.start()

                // Spawning threads to empty stdout/stderr streams to prevent process from hanging
                thread {
                    serverProcess?.inputStream?.bufferedReader()?.use { reader ->
                        var line: String?
                        while (reader.readLine().also { line = it } != null) {
                            Log.d(TAG, "[Server STDOUT] $line")
                        }
                    }
                }
                thread {
                    serverProcess?.errorStream?.bufferedReader()?.use { reader ->
                        var line: String?
                        while (reader.readLine().also { line = it } != null) {
                            Log.e(TAG, "[Server STDERR] $line")
                        }
                    }
                }

                // Wait for the local server port to become available
                waitForServer(serverPort)

                // Load UI on the main thread
                runOnUiThread {
                    Log.i(TAG, "Server is up. Loading WebView UI.")
                    webView.loadUrl("http://127.0.0.1:$serverPort")
                }

            } catch (e: Exception) {
                Log.e(TAG, "Error running background server", e)
            }
        }
    }

    private fun prepareServerBinary(): File? {
        val destFile = File(filesDir, "duckweed-server")
        
        try {
            // Copy from assets to filesDir
            assets.open("duckweed-server").use { inputStream ->
                FileOutputStream(destFile).use { outputStream ->
                    inputStream.copyTo(outputStream)
                }
            }
            
            // Set executable permission
            destFile.setExecutable(true, true)
            Log.i(TAG, "Copied binary to: ${destFile.absolutePath}")
            return destFile
        } catch (e: IOException) {
            Log.e(TAG, "Error copying asset binary", e)
        }
        return null
    }

    private fun waitForServer(port: Int) {
        var attempts = 0
        val maxAttempts = 30
        while (attempts < maxAttempts) {
            try {
                Socket("127.0.0.1", port).use {
                    Log.i(TAG, "Successfully connected to server port $port after $attempts attempts.")
                    return
                }
            } catch (e: IOException) {
                attempts++
                Thread.sleep(500)
            }
        }
        Log.w(TAG, "Server port $port did not become active in time.")
    }

    override fun onDestroy() {
        super.onDestroy()
        // Gracefully kill server process on app close
        Log.i(TAG, "App closing. Stopping backend server process.")
        serverProcess?.destroy()
        serverProcess = null
    }
}
