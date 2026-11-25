package com.suryaprofiler.ui

import android.graphics.Paint
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp

/**
 * Signed battery power chart: positive (above zero) = usage, negative =
 * charging. A thin strip at the bottom marks screen-off buckets.
 */
@Composable
internal fun PowerChart(points: List<PowerPoint>, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val usage = scheme.primary
    val charge = scheme.tertiary
    val grid = scheme.onSurface.copy(alpha = 0.08f)
    val axis = scheme.onSurfaceVariant
    val offStrip = scheme.onSurface.copy(alpha = 0.18f)
    val density = LocalDensity.current
    val labelPx = with(density) { 9.dp.toPx() }
    val stroke = with(density) { 2.dp.toPx() }
    val padLeft = with(density) { 42.dp.toPx() }
    val padRight = with(density) { 6.dp.toPx() }
    val padTop = with(density) { 8.dp.toPx() }
    val padBottom = with(density) { 22.dp.toPx() }
    val stripH = with(density) { 3.dp.toPx() }

    Canvas(
        modifier
            .fillMaxWidth()
            .height(200.dp),
    ) {
        if (points.isEmpty()) return@Canvas
        val range = chartRange(points)
        val span = (range.maxMw - range.minMw).coerceAtLeast(1.0)
        val start = points.first().tMs
        val end = points.last().tMs.coerceAtLeast(start + 60_000)
        val timeSpan = (end - start).coerceAtLeast(1)
        val left = padLeft
        val right = size.width - padRight
        val top = padTop
        val bottom = size.height - padBottom
        val w = (right - left).coerceAtLeast(1f)
        val h = (bottom - top).coerceAtLeast(1f)

        fun x(t: Long): Float = left + (t - start).toFloat() / timeSpan * w
        fun y(mw: Double): Float = top + ((range.maxMw - mw) / span).toFloat() * h

        // Horizontal grid + labels at max / zero / min.
        val text = Paint().apply {
            isAntiAlias = true
            textSize = labelPx
            color = axis.toArgb()
        }
        for (v in listOf(range.maxMw, 0.0, range.minMw)) {
            val yy = y(v)
            drawLine(
                grid,
                Offset(left, yy),
                Offset(right, yy),
                strokeWidth = 1f,
            )
            drawContext.canvas.nativeCanvas.drawText(
                fmtAxisW(v),
                0f,
                yy + labelPx / 3f,
                text,
            )
        }
        if (range.minMw < 0 && range.maxMw > 0) {
            drawLine(
                axis.copy(alpha = 0.35f),
                Offset(left, y(0.0)),
                Offset(right, y(0.0)),
                strokeWidth = 1.2f,
                pathEffect = PathEffect.dashPathEffect(floatArrayOf(4f, 6f)),
            )
        }

        // Screen-off strip (one small rect per segment).
        for (i in 0 until points.size - 1) {
            val a = points[i]
            val b = points[i + 1]
            if (!a.screenOn) {
                drawRect(
                    offStrip,
                    topLeft = Offset(x(a.tMs), bottom + 4f),
                    size = androidx.compose.ui.geometry.Size(
                        (x(b.tMs) - x(a.tMs)).coerceAtLeast(1f),
                        stripH,
                    ),
                )
            }
        }

        // Power line split by sign so charging and usage are readable.
        for (i in 0 until points.size - 1) {
            val a = points[i]
            val b = points[i + 1]
            if (b.tMs - a.tMs > 3 * 60_000) continue
            val color = if ((a.mw + b.mw) / 2 < 0) charge else usage
            drawLine(
                color,
                Offset(x(a.tMs), y(a.mw)),
                Offset(x(b.tMs), y(b.mw)),
                strokeWidth = stroke,
                cap = StrokeCap.Round,
            )
        }
        val last = points.last()
        drawCircle(
            if (last.mw < 0) charge else usage,
            radius = stroke * 1.6f,
            center = Offset(x(last.tMs), y(last.mw)),
        )

        // X labels: start / mid / end.
        text.color = axis.toArgb()
        val mid = start + timeSpan / 2
        listOf(start to "left", mid to "center", end to "right").forEach { (t, align) ->
            val xx = x(t)
            text.textAlign = when (align) {
                "left" -> Paint.Align.LEFT
                "right" -> Paint.Align.RIGHT
                else -> Paint.Align.CENTER
            }
            drawContext.canvas.nativeCanvas.drawText(
                fmtClock(t),
                xx,
                size.height - 2f,
                text,
            )
        }
    }
}

