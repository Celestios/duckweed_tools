package com.duckweed.toolkit

import android.annotation.SuppressLint
import android.content.Intent
import android.os.Bundle
import android.provider.DocumentsContract
import android.util.Log
import android.webkit.JavascriptInterface
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.net.HttpURLConnection
import java.net.Socket
import java.net.URL
import kotlin.concurrent.thread

class MainActivity : AppCompatActivity() {

    private var serverProcess: Process? = null
    private lateinit var webView: WebView
    private val serverPort = 8000
    private val TAG = "DuckweedWrapper"

    private val CREATE_FILE_REQUEST = 1001
    private val PICK_FILE_REQUEST = 1002
    private val PICK_IMAGE_REQUEST = 1003
    private val EXPORT_IMAGES_REQUEST = 1004
    private var pendingExportData: String? = null
    private var pendingExportMimeType: String? = null
    private var pendingExportFilename: String? = null
    private var pendingImageCallback: String? = null

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

        // Expose Native SAF File Interface to JS
        webView.addJavascriptInterface(AndroidInterface(), "AndroidInterface")

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

                // Use external files directory so database is accessible to user file managers.
                // Fallback to internal filesDir if external storage is not mounted.
                val dataDir = getExternalFilesDir(null) ?: filesDir

                // Start process
                Log.i(TAG, "Launching server binary at ${binaryFile.absolutePath}")
                val processBuilder = ProcessBuilder(
                    binaryFile.absolutePath,
                    "--port", serverPort.toString(),
                    "--data-dir", dataDir.absolutePath
                )
                
                // Set working directory
                processBuilder.directory(dataDir)
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
        val nativeDir = applicationInfo.nativeLibraryDir
        val binaryFile = File(nativeDir, "libduckweed-server.so")
        
        if (binaryFile.exists()) {
            // Android package installer extracts library files with executable permissions by default.
            // We verify or set it just in case.
            binaryFile.setExecutable(true, false)
            Log.i(TAG, "Located native server binary at: ${binaryFile.absolutePath}")
            return binaryFile
        } else {
            Log.e(TAG, "Native server binary libduckweed-server.so not found in $nativeDir")
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

    private fun triggerAndroidExport(data: String, mimeType: String, filename: String) {
        pendingExportData = data
        pendingExportMimeType = mimeType
        pendingExportFilename = filename
        val intent = Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = mimeType
            putExtra(Intent.EXTRA_TITLE, filename)
        }
        startActivityForResult(intent, CREATE_FILE_REQUEST)
    }

    private fun triggerAndroidImport() {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = "application/json"
        }
        startActivityForResult(intent, PICK_FILE_REQUEST)
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (resultCode != RESULT_OK || data == null) return

        val uri = data.data ?: return

        if (requestCode == CREATE_FILE_REQUEST) {
            try {
                val data = pendingExportData ?: ""
                contentResolver.openOutputStream(uri)?.use { outputStream ->
                    outputStream.write(data.toByteArray())
                }
                pendingExportData = null
                pendingExportMimeType = null
                pendingExportFilename = null
                Toast.makeText(this, "خروجی با موفقیت ایجاد شد", Toast.LENGTH_SHORT).show()
            } catch (e: Exception) {
                Log.e(TAG, "Error writing exported file", e)
                Toast.makeText(this, "خطا در پشتیبان‌گیری", Toast.LENGTH_SHORT).show()
            }
        } else if (requestCode == PICK_FILE_REQUEST) {
            try {
                val jsonStr = contentResolver.openInputStream(uri)?.use { inputStream ->
                    inputStream.bufferedReader().use { it.readText() }
                } ?: ""

                if (jsonStr.contains("container_types") && jsonStr.contains("log")) {
                    thread {
                        try {
                            val url = URL("http://127.0.0.1:$serverPort/api/db/import")
                            val conn = url.openConnection() as HttpURLConnection
                            conn.requestMethod = "POST"
                            conn.setRequestProperty("Content-Type", "application/json")
                            conn.doOutput = true
                            conn.outputStream.use { out ->
                                out.write(jsonStr.toByteArray())
                            }
                            val code = conn.responseCode
                            if (code == 200) {
                                runOnUiThread {
                                    Toast.makeText(this@MainActivity, "پایگاه داده با موفقیت بازیابی شد", Toast.LENGTH_SHORT).show()
                                    webView.reload()
                                }
                            } else {
                                Log.e(TAG, "Import HTTP code: $code")
                                runOnUiThread {
                                    Toast.makeText(this@MainActivity, "خطا در سرور داخلی برای بازیابی داده‌ها", Toast.LENGTH_SHORT).show()
                                }
                            }
                        } catch (e: Exception) {
                            Log.e(TAG, "Error POSTing import", e)
                            runOnUiThread {
                                Toast.makeText(this@MainActivity, "خطا در اتصال به پایگاه داده", Toast.LENGTH_SHORT).show()
                            }
                        }
                    }
                } else {
                    Toast.makeText(this, "فایل معتبر نیست (باید ساختار پشتیبان Duckweed باشد)", Toast.LENGTH_LONG).show()
                }
            } catch (e: Exception) {
                Log.e(TAG, "Error reading import file", e)
                Toast.makeText(this, "خطا در خواندن فایل", Toast.LENGTH_SHORT).show()
            }
        } else if (requestCode == PICK_IMAGE_REQUEST) {
            try {
                val clipData = data.clipData
                val uris = mutableListOf<android.net.Uri>()
                if (clipData != null) {
                    for (i in 0 until clipData.itemCount) {
                        uris.add(clipData.getItemAt(i).uri)
                    }
                } else {
                    data.data?.let { uris.add(it) }
                }

                val imagesDir = File(filesDir, "images")
                imagesDir.mkdirs()

                val savedNames = mutableListOf<String>()
                for (imgUri in uris) {
                    val cursor = contentResolver.query(imgUri, null, null, null, null)
                    val name = cursor?.use {
                        if (it.moveToFirst()) {
                            it.getString(it.getColumnIndexOrThrow(android.provider.OpenableColumns.DISPLAY_NAME))
                        } else "image_${System.currentTimeMillis()}.jpg"
                    } ?: "image_${System.currentTimeMillis()}.jpg"

                    val dest = File(imagesDir, name)
                    contentResolver.openInputStream(imgUri)?.use { input ->
                        FileOutputStream(dest).use { output ->
                            input.copyTo(output)
                        }
                    }
                    savedNames.add(name)
                }

                val callbackId = pendingImageCallback
                pendingImageCallback = null

                if (callbackId != null) {
                    val filename = savedNames.firstOrNull() ?: ""
                    runOnUiThread {
                        webView.evaluateJavascript("window._imagePickCallback('$filename')", null)
                    }
                } else {
                    thread {
                        try {
                            for (name in savedNames) {
                                val url = URL("http://127.0.0.1:$serverPort/api/images/correlate")
                                val conn = url.openConnection() as HttpURLConnection
                                conn.requestMethod = "POST"
                                conn.setRequestProperty("Content-Type", "application/json")
                                conn.doOutput = true
                                conn.outputStream.use { out ->
                                    out.write("""{"filename":"$name"}""".toByteArray())
                                }
                                conn.responseCode
                            }
                            runOnUiThread {
                                Toast.makeText(this@MainActivity, "${savedNames.size} تصویر وارد شد", Toast.LENGTH_SHORT).show()
                                webView.reload()
                            }
                        } catch (e: Exception) {
                            Log.e(TAG, "Error correlating images", e)
                            runOnUiThread {
                                Toast.makeText(this@MainActivity, "تصاویر ذخیره شدند اما همبسته‌سازی ناموفق بود", Toast.LENGTH_SHORT).show()
                            }
                        }
                    }
                }
            } catch (e: Exception) {
                Log.e(TAG, "Error importing images", e)
                Toast.makeText(this, "خطا در وارد کردن تصاویر", Toast.LENGTH_SHORT).show()
            }
        } else if (requestCode == EXPORT_IMAGES_REQUEST) {
            val treeUri = data.data
            if (treeUri != null) {
                try {
                    val jsonStr = pendingExportData ?: "[]"
                    val images = org.json.JSONArray(jsonStr)

                    val imagesSrcDir = File(filesDir, "images")
                    for (i in 0 until images.length()) {
                        val img = images.getJSONObject(i)
                        val filename = img.getString("filename")
                        val src = File(imagesSrcDir, filename)
                        if (src.exists()) {
                            val destUri = DocumentsContract.createDocument(
                                contentResolver,
                                treeUri,
                                "image/*",
                                filename
                            )
                            destUri?.let {
                                contentResolver.openOutputStream(it)?.use { out ->
                                    src.inputStream().use { inp -> inp.copyTo(out) }
                                }
                            }
                        }
                    }
                    pendingExportData = null
                    Toast.makeText(this, "تصاویر با موفقیت خروجی شدند", Toast.LENGTH_SHORT).show()
                } catch (e: Exception) {
                    Log.e(TAG, "Error exporting images", e)
                    Toast.makeText(this, "خطا در خروجی تصاویر", Toast.LENGTH_SHORT).show()
                }
            }
        }
    }

    inner class AndroidInterface {
        @JavascriptInterface
        fun exportDatabase(jsonStr: String) {
            runOnUiThread {
                triggerAndroidExport(jsonStr, "application/json", "duckweed_database.json")
            }
        }

        @JavascriptInterface
        fun exportMarkdown(mdStr: String) {
            runOnUiThread {
                triggerAndroidExport(mdStr, "text/markdown", "cultivation_log.md")
            }
        }

        @JavascriptInterface
        fun exportImages(jsonStr: String) {
            pendingExportData = jsonStr
            val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).apply {
                addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
            }
            startActivityForResult(intent, EXPORT_IMAGES_REQUEST)
        }

        @JavascriptInterface
        fun importImages() {
            pendingImageCallback = null
            val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "image/*"
                putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true)
            }
            startActivityForResult(intent, PICK_IMAGE_REQUEST)
        }

        @JavascriptInterface
        fun pickImageForLog(callbackId: String) {
            pendingImageCallback = callbackId
            val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "image/*"
            }
            startActivityForResult(intent, PICK_IMAGE_REQUEST)
        }

        @JavascriptInterface
        fun importDatabase() {
            runOnUiThread {
                triggerAndroidImport()
            }
        }
    }
}
