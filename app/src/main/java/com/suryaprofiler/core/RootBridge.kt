package com.suryaprofiler.core

import android.content.Context
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext

/**
 * Root shell access + first-run deployment of the Rust core binary.
 *
 * Layout on device:
 *   /data/local/tmp/suryaprofiler/surya-prof   executable (W^X safe: app data is not)
 *   /data/adb/suryaprofiler/                state (profiles.json, state.json)
 *
 * Responsibility: process execution, asset deployment.
 * Non-goals: parsing surya-prof output (FtClient), any tuning decision.
 */
class RootBridge {

    data class Sh(val code: Int, val out: String) {
        val ok: Boolean get() = code == 0
    }

    /**
     * Absolute su path: apps don't inherit the shell PATH, so `su` by name
     * fails with ENOENT on some setups even though the binary exists.
     */
    private val suBin: String by lazy {
        listOf("/system/bin/su", "/sbin/su", "/system/xbin/su", "/data/adb/ap/bin/su")
            .firstOrNull { File(it).exists() } ?: "su"
    }

    /** Runs `<su> -c <cmd>`; stdout+stderr merged, bounded by [timeoutMs]. */
    fun sh(cmd: String, timeoutMs: Long = 30_000): Sh {
        val process = ProcessBuilder(suBin, "-c", cmd)
            .redirectErrorStream(true)
            // Never let su inherit the app's stdin: APatch/FolkPatch may
            // block waiting for a prompt when stdin is an open pipe.
            .redirectInput(File("/dev/null"))
            .start()
        // Drain stdout concurrently. Waiting before reading deadlocks as soon
        // as the child fills the pipe buffer (e.g. `probe`/`catalog` JSON).
        val out = StringBuilder()
        val reader = kotlin.concurrent.thread(name = "su-stdout") {
            runCatching {
                process.inputStream.bufferedReader().use { r ->
                    val buf = CharArray(8192)
                    while (true) {
                        val n = r.read(buf)
                        if (n < 0) break
                        synchronized(out) { out.append(buf, 0, n) }
                    }
                }
            }
        }
        val finished = process.waitFor(timeoutMs, java.util.concurrent.TimeUnit.MILLISECONDS)
        if (!finished) {
            process.destroyForcibly()
            runCatching { reader.join(500) }
            val partial = synchronized(out) { out.toString() }
            return Sh(124, "$partial\n[timeout after ${timeoutMs}ms]")
        }
        runCatching { reader.join(2_000) }
        return Sh(process.exitValue(), synchronized(out) { out.toString() })
    }

    /**
     * Starts `<su> -c <cmd>` as a long-lived process with stdout/stderr
     * separate (protocol on stdout, logs on stderr). Used by the daemon
     * client; the caller owns the process lifecycle.
     */
    fun spawn(cmd: String): Process = ProcessBuilder(suBin, "-c", cmd)
        .redirectErrorStream(false)
        .start()

    /** True when `su` gives us uid 0. */
    fun isRoot(): Boolean = sh("id -u 2>/dev/null").out.trim() == "0"

    /**
     * Deploy binary + bundled profiles. Returns null on success or an error
     * description (never throws) so the UI can show a precise reason.
     */
    private val deployLock = Mutex()

    suspend fun deploy(context: Context): String? = deployLock.withLock {
        withContext(Dispatchers.IO) {
            runCatching {
                val binBytes = context.assets.open(ASSET_BIN).use { it.readBytes() }
                val target = File(BIN)
                // Content-aware deploy: a same-size version bump must still
                // copy (cmp, not length). tmp+mv keeps the target executable
                // from ever being truncated by a crash/race.
                val tmp = File(context.cacheDir, ASSET_BIN)
                tmp.writeBytes(binBytes)
                val q: (String) -> String = { p -> "'" + p.replace("'", "'\\''") + "'" }
                val sameSize = target.exists() && target.length() == binBytes.size.toLong()
                val sameContent = sameSize && sh(
                    "cmp -s ${q(tmp.absolutePath)} ${q(BIN)} && echo SAME || echo DIFF"
                ).out.trim() == "SAME"
                if (!sameContent) {
                    val staged = "$BIN.new"
                    val r = sh(
                        "mkdir -p ${q(target.parentFile!!.absolutePath)} && " +
                            "cp ${q(tmp.absolutePath)} ${q(staged)} && " +
                            "chmod 755 ${q(staged)} && mv ${q(staged)} ${q(BIN)}"
                    )
                    if (!r.ok || !target.exists()) {
                        return@runCatching "Binary deploy failed (code ${r.code}): ${r.out.trim()}"
                    }
                }

                val profBytes = context.assets.open(ASSET_PROFILES).use { it.readBytes() }
                val tmpP = File(context.cacheDir, ASSET_PROFILES)
                tmpP.writeBytes(profBytes)
                // keep a user-edited copy untouched; sync only when content
                // differs. Atomic tmp+mv: a concurrent read must never see a
                // 0-byte profiles.json (the daemon reads it on startup).
                val r2 = sh(
                    "mkdir -p ${q(STATE_DIR)} && chmod 700 ${q(STATE_DIR)}; " +
                        "if ! cmp -s ${q(tmpP.absolutePath)} ${q("$STATE_DIR/$ASSET_PROFILES")}; then " +
                        "cp ${q(tmpP.absolutePath)} ${q("$STATE_DIR/$ASSET_PROFILES.tmp")} && " +
                        "chmod 600 ${q("$STATE_DIR/$ASSET_PROFILES.tmp")} && " +
                        "mv ${q("$STATE_DIR/$ASSET_PROFILES.tmp")} ${q("$STATE_DIR/$ASSET_PROFILES")}; fi"
                )
                if (!r2.ok) {
                    return@runCatching "Profiles deploy failed (code ${r2.code}): ${r2.out.trim()}"
                }
                // Obsolete state from removed features. holds.json stays:
                // the slim bridge still persists saver/R&D holds.
                sh(
                    "rm -f ${q("$STATE_DIR/maintenance.json")} " +
                        "${q("$STATE_DIR/refresh_fps.dex")}"
                )
                tmp.delete()
                tmpP.delete()
                null
            }.getOrElse { "Deploy error: ${it.message ?: it}" }
        }
    }

    companion object {
        const val BIN = "/data/local/tmp/suryaprofiler/surya-prof"
        const val STATE_DIR = "/data/adb/suryaprofiler"
        const val ASSET_BIN = "surya-prof"
        const val ASSET_PROFILES = "profiles.json"
    }
}
