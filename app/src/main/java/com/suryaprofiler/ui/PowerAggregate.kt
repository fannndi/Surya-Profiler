package com.suryaprofiler.ui

import com.suryaprofiler.protocol.PowerBucket
import com.suryaprofiler.protocol.PowerPeak
import kotlin.math.abs
import kotlin.math.max
import kotlin.math.min

/** Pure power aggregation for the chart, per-app/profile rows and sessions. */

internal data class PowerPoint(val tMs: Long, val mw: Double, val screenOn: Boolean)

internal data class PowerRange(val minMw: Double, val maxMw: Double)

internal fun bucketMw(b: PowerBucket): Double? {
    if (b.avgUv <= 0) return null
    return b.avgUa.toDouble() * b.avgUv.toDouble() / 1_000_000_000.0
}

/** History + live buckets, merged by minute for legacy rows or by segment. */
internal fun powerPoints(
    history: List<PowerBucket>,
    live: List<PowerBucket>,
    windowS: Long,
    nowMs: Long,
): List<PowerPoint> {
    val cutoff = nowMs - windowS.coerceAtLeast(1) * 1000
    return mergeBuckets(history, live)
        .asSequence()
        .filter { it.tMs >= cutoff }
        .mapNotNull { b -> bucketMw(b)?.let { PowerPoint(b.tMs, it, b.screenOn) } }
        .toList()
}

/** Data-driven y-range that always includes zero and pads 10 %. */
internal fun chartRange(points: List<PowerPoint>): PowerRange {
    if (points.isEmpty()) return PowerRange(-1_000.0, 1_000.0)
    var lo = min(0.0, points.minOf { it.mw })
    var hi = max(0.0, points.maxOf { it.mw })
    if (hi - lo < 100.0) {
        lo -= 50.0
        hi += 50.0
    }
    val pad = (hi - lo) * 0.1
    return PowerRange(lo - pad, hi + pad)
}

internal data class AppPowerStat(
    val pkg: String,
    val durMs: Long,
    val avgUa: Double,
    val avgMw: Double,
    val avgTempC: Double?,
    val maxTempC: Float?,
)

private class Acc {
    var durMs = 0L
    var uaWeighted = 0.0
    var mwWeighted = 0.0
    var tempWeighted = 0.0
    var tempDur = 0L
    var maxTemp: Float? = null
}

/** Screen-on foreground attribution (the daemon stores the dominant pkg). */
internal fun appStats(buckets: List<PowerBucket>): List<AppPowerStat> {
    val acc = HashMap<String, Acc>()
    for (b in buckets) {
        if (!b.screenOn) continue
        val pkg = b.pkg ?: continue
        val mw = bucketMw(b) ?: continue
        val dur = b.durMs.coerceAtLeast(1)
        val a = acc.getOrPut(pkg) { Acc() }
        a.durMs += dur
        a.uaWeighted += b.avgUa.toDouble() * dur
        a.mwWeighted += mw * dur
        b.avgTempC?.let {
            a.tempWeighted += it * dur
            a.tempDur += dur
        }
        b.maxTempC?.let { a.maxTemp = max(a.maxTemp ?: it, it) }
    }
    return acc.map { (pkg, a) ->
        AppPowerStat(
            pkg = pkg,
            durMs = a.durMs,
            avgUa = if (a.durMs > 0) a.uaWeighted / a.durMs else 0.0,
            avgMw = if (a.durMs > 0) a.mwWeighted / a.durMs else 0.0,
            avgTempC = if (a.tempDur > 0) a.tempWeighted / a.tempDur else null,
            maxTempC = a.maxTemp,
        )
    }.sortedWith(compareByDescending<AppPowerStat> { it.avgMw }.thenByDescending { it.durMs })
}

internal data class ProfilePowerStat(
    val profile: String,
    val durMs: Long,
    val avgMw: Double,
    val avgUa: Double,
)

/** Every bucket, including screen-off `sleep`; shows which profile costs more. */
internal fun profileStats(buckets: List<PowerBucket>): List<ProfilePowerStat> {
    val acc = HashMap<String, Acc>()
    for (b in buckets) {
        val profile = b.profile ?: continue
        val mw = bucketMw(b) ?: continue
        val dur = b.durMs.coerceAtLeast(1)
        val a = acc.getOrPut(profile) { Acc() }
        a.durMs += dur
        a.uaWeighted += b.avgUa.toDouble() * dur
        a.mwWeighted += mw * dur
    }
    return acc.map { (profile, a) ->
        ProfilePowerStat(
            profile = profile,
            durMs = a.durMs,
            avgMw = if (a.durMs > 0) a.mwWeighted / a.durMs else 0.0,
            avgUa = if (a.durMs > 0) a.uaWeighted / a.durMs else 0.0,
        )
    }.sortedByDescending { it.avgMw }
}

internal data class ChargeSession(
    val startMs: Long,
    val endMs: Long,
    val startPct: Int?,
    val endPct: Int?,
    val addedMah: Double,
    val peakMw: Double,
    val durMs: Long,
    val ongoing: Boolean,
)

private class SessionAcc {
    var startMs = 0L
    var endMs = 0L
    var startPct: Int? = null
    var endPct: Int? = null
    var addedMah = 0.0
    var peakMw = 0.0
    var durMs = 0L

    fun add(b: PowerBucket) {
        val dur = b.durMs.coerceAtLeast(1)
        if (durMs == 0L) {
            startMs = b.tMs
            startPct = b.pct
        }
        endMs = b.endMs ?: (b.tMs + dur)
        durMs += dur
        b.pct?.let { endPct = it }
        addedMah += (-b.avgUa / 1000.0) * (dur / 3_600_000.0)
        if (b.avgUv > 0) {
            peakMw = max(peakMw, -b.minUa * b.avgUv / 1_000_000_000.0)
        }
    }

    fun finish(ongoing: Boolean) = ChargeSession(
        startMs = startMs,
        endMs = endMs,
        startPct = startPct,
        endPct = endPct,
        addedMah = addedMah,
        peakMw = peakMw,
        durMs = durMs,
        ongoing = ongoing,
    )
}

/** Charge sessions from contiguous buckets with net current into the battery. */
internal fun chargeSessions(buckets: List<PowerBucket>): List<ChargeSession> {
    val sorted = buckets.sortedBy { it.tMs }
    val out = ArrayList<ChargeSession>()
    var cur: SessionAcc? = null
    var lastWasCharging = false
    for (b in sorted) {
        val charging = b.avgUa < -20_000 && b.avgUv > 0
        if (!charging) {
            cur?.let { out += it.finish(ongoing = false) }
            cur = null
            lastWasCharging = false
            continue
        }
        val prevEnd = cur?.endMs
        if (cur != null && prevEnd != null && b.tMs - prevEnd > 3 * 60_000) {
            out += cur.finish(ongoing = false)
            cur = null
        }
        val acc = cur ?: SessionAcc().also { cur = it }
        acc.add(b)
        lastWasCharging = true
    }
    cur?.let { out += it.finish(ongoing = lastWasCharging) }
    return out
}

internal fun latestChargeSession(buckets: List<PowerBucket>): ChargeSession? =
    chargeSessions(buckets).lastOrNull()

internal fun peakPowerMw(p: PowerPeak?): Double? {
    val peak = p ?: return null
    if (peak.uv <= 0) return null
    return abs(peak.ua).toDouble() * peak.uv.toDouble() / 1_000_000_000.0
}
