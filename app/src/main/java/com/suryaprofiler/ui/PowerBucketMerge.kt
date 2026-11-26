package com.suryaprofiler.ui

import com.suryaprofiler.protocol.PowerBucket
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToLong

/** Duration-weighted merging for legacy minute rows, plus segment-aware live updates. */

private data class BucketKey(val tMs: Long, val segmentId: String?)

internal fun mergeBuckets(
    history: List<PowerBucket>,
    live: List<PowerBucket>,
): List<PowerBucket> {
    val map = LinkedHashMap<BucketKey, PowerBucket>()
    fun key(b: PowerBucket) = BucketKey(b.tMs, b.segmentId)
    history.forEach { b ->
        val k = key(b)
        map[k] = map[k]?.let { mergePowerBuckets(it, b) } ?: b
    }
    live.forEach { b ->
        val k = key(b)
        val previous = map[k]
        map[k] = when {
            previous == null -> b
            // A live bucket is cumulative for its segment, not another segment.
            previous.segmentId != null && previous.segmentId == b.segmentId -> b
            else -> mergePowerBuckets(previous, b)
        }
    }
    return map.values.sortedWith(
        compareBy<PowerBucket> { it.tMs }.thenBy { it.startMs ?: it.tMs },
    )
}

internal fun mergePowerBuckets(a: PowerBucket, b: PowerBucket): PowerBucket {
    val wa = a.durMs.coerceAtLeast(1).toDouble()
    val wb = b.durMs.coerceAtLeast(1).toDouble()
    val weight = wa + wb
    val durMs = (a.durMs + b.durMs).coerceAtLeast(0)
    val onMs = (if (a.screenOn) a.durMs else 0) + (if (b.screenOn) b.durMs else 0)
    val screenOn = durMs > 0 && onMs * 2 >= durMs
    val avgUa = ((a.avgUa * wa + b.avgUa * wb) / weight).roundToLong()
    val avgUv = ((a.avgUv * wa + b.avgUv * wb) / weight).roundToLong()
    val avgTempC = when {
        a.avgTempC != null && b.avgTempC != null ->
            ((a.avgTempC * wa + b.avgTempC * wb) / weight).toFloat()
        else -> a.avgTempC ?: b.avgTempC
    }
    val startMs = listOfNotNull(a.startMs, b.startMs).minOrNull()
    val endMs = listOfNotNull(a.endMs, b.endMs).maxOrNull()
    val energyMwh = when {
        a.energyMwh != null && b.energyMwh != null -> a.energyMwh + b.energyMwh
        a.energyMwh != null || b.energyMwh != null ->
            avgUa.toDouble() * avgUv / 1_000_000_000.0 * (durMs / 3_600_000.0)
        else -> null
    }
    val pkgUnknownMs = (if (a.screenOn && a.pkg == null) a.durMs else 0) +
        (if (b.screenOn && b.pkg == null) b.durMs else 0)
    val profileUnknownMs = (if (a.profile == null) a.durMs else 0) +
        (if (b.profile == null) b.durMs else 0)
    return PowerBucket(
        tMs = a.tMs,
        pct = b.pct ?: a.pct,
        avgUa = avgUa,
        minUa = min(a.minUa, b.minUa),
        maxUa = max(a.maxUa, b.maxUa),
        avgUv = avgUv,
        avgTempC = avgTempC,
        maxTempC = listOfNotNull(a.maxTempC, b.maxTempC).maxOrNull(),
        screenOn = screenOn,
        durMs = durMs,
        samples = (a.samples.toLong() + b.samples).coerceAtMost(Int.MAX_VALUE.toLong()).toInt(),
        pkg = if (screenOn) dominant(
            pkgUnknownMs,
            a.pkg?.takeIf { a.screenOn }?.let { it to a.durMs },
            b.pkg?.takeIf { b.screenOn }?.let { it to b.durMs },
        ) else null,
        profile = dominant(
            profileUnknownMs,
            a.profile?.let { it to a.durMs },
            b.profile?.let { it to b.durMs },
        ),
        startMs = startMs,
        endMs = endMs,
        energyMwh = energyMwh,
        segmentId = a.segmentId?.takeIf { it == b.segmentId },
    )
}

private fun dominant(unknownMs: Long, vararg rows: Pair<String, Long>?): String? =
    rows.filterNotNull().maxWithOrNull(
        compareBy<Pair<String, Long>> { it.second }.thenByDescending { it.first },
    )?.takeIf { it.second > unknownMs }?.first
