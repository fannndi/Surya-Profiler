package com.suryaprofiler.ui

import android.content.Context
import android.util.LruCache
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.core.graphics.drawable.toBitmap

/**
 * Small LRU of app icons. Icons are loaded lazily per visible row instead of
 * decoding every installed app at once (~400 icons × 96² px is ~14 MB).
 */
internal object AppIconCache {
    private val cache = LruCache<String, ImageBitmap>(96)

    fun get(pkg: String): ImageBitmap? = synchronized(cache) { cache.get(pkg) }

    fun load(context: Context, pkg: String): ImageBitmap? {
        get(pkg)?.let { return it }
        val bitmap = runCatching {
            context.packageManager.getApplicationIcon(pkg)
                .toBitmap(96, 96)
                .asImageBitmap()
        }.getOrNull() ?: return null
        synchronized(cache) { cache.put(pkg, bitmap) }
        return bitmap
    }
}
