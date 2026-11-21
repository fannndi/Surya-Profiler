package com.suryaprofiler.dynamic

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import androidx.core.app.NotificationCompat
import com.suryaprofiler.MainActivity
import com.suryaprofiler.R

/**
 * Notification channels + builders for the foreground service.
 *
 * Responsibility: Android notification surface only.
 * Non-goals: lifecycle, daemon IO.
 *
 * Note: the channel id stays `automation` on purpose — renaming it would
 * orphan the existing channel in system settings.
 */
internal object DaemonNotifications {

    const val CHANNEL_ID = "automation"
    const val NOTIF_ID = 41

    fun createChannels(service: Service) {
        val nm = service.getSystemService(NotificationManager::class.java)
        val ch = NotificationChannel(
            CHANNEL_ID,
            "Service",
            NotificationManager.IMPORTANCE_MIN,
        ).apply {
            description = "Dynamic profile status (silent)"
            setShowBadge(false)
            enableVibration(false)
            setSound(null, null)
        }
        nm.createNotificationChannel(ch)
    }

    fun buildServiceNotification(service: Service, status: String): Notification {
        val openPi = PendingIntent.getActivity(
            service, 0,
            Intent(service, MainActivity::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val stopPi = PendingIntent.getService(
            service, 1,
            Intent(service, DynamicProfileService::class.java)
                .setAction(DynamicProfileService.ACTION_STOP),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        return NotificationCompat.Builder(service, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_app)
            .setContentTitle("Surya Profiler")
            .setContentText(status)
            .setOngoing(true)
            .setShowWhen(false)
            .setContentIntent(openPi)
            .addAction(0, "Turn off", stopPi)
            .setPriority(NotificationCompat.PRIORITY_MIN)
            .build()
    }

}
