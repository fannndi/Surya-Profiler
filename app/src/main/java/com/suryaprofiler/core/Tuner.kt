package com.suryaprofiler.core

import android.content.Context
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext

/**
 * Single owner of every `surya-prof` engine call, shared by the UI and the
 * dynamic profile service.
 *
 * Responsibility: serializing engine access (one apply/restore at a time).
 * Non-goals: deciding *which* profile to apply (Rust daemon / UI do that).
 */
object Tuner {

    val bridge = RootBridge()
    val client = FtClient(bridge)

    /** Serializes apply/restore across UI + service. */
    private val mutex = Mutex()

    suspend fun deploy(context: Context): String? = bridge.deploy(context)

    suspend fun status(): Status = withContext(Dispatchers.IO) { client.status() }

    suspend fun plan(profileId: String): Plan = withContext(Dispatchers.IO) { client.plan(profileId) }

    suspend fun apply(profileId: String): ApplyReport =
        mutex.withLock { withContext(Dispatchers.IO) { client.apply(profileId) } }

    /** Explicit stock restore (service-off fallback / CLI path). */
    suspend fun restore(): ApplyReport =
        mutex.withLock { withContext(Dispatchers.IO) { client.restore() } }

    /** Releases every persisted hold (saver/input-suspend/thermal). */
    suspend fun clean(): Boolean =
        mutex.withLock { withContext(Dispatchers.IO) { client.clean() } }
}
