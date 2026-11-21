package com.suryaprofiler.dynamic

import android.app.NotificationManager
import android.app.Service
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.core.content.ContextCompat
import com.suryaprofiler.core.ProfileLabels
import com.suryaprofiler.protocol.DaemonCommands
import com.suryaprofiler.protocol.HealsInfo
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import org.json.JSONObject

/**
 * Foreground service: Android lifecycle shell around the Rust daemon.
 *
 * Responsibility: FGS + notification, spawn/restart the daemon, forward
 * Android-only signals (screen, keyguard) as commands, relay daemon events
 * into [DynamicProfileState] + notifications.
 * Non-goals: any decision or tuning logic (all in the Rust daemon).
 */
class DynamicProfileService : Service() {

    companion object {
        private const val TAG = "SuryaProfiler"
        private const val SUPERVISE_MS = 3_000L
        private const val RESTART_BACKOFF_MS = 10_000L

        const val ACTION_START = "com.suryaprofiler.action.START"
        const val ACTION_STOP = "com.suryaprofiler.action.STOP"

        fun start(context: Context): Boolean {
            val i = Intent(context, DynamicProfileService::class.java).setAction(ACTION_START)
            return try {
                if (Build.VERSION.SDK_INT >= 26) context.startForegroundService(i)
                else context.startService(i)
                true
            } catch (e: Exception) {
                // Android 12: background FGS starts can be rejected. The UI
                // path starts from an Activity; boot/automation callers must
                // tolerate this instead of crashing the process.
                Log.w(TAG, "service start failed: $e")
                false
            }
        }

        fun stop(context: Context) {
            context.stopService(Intent(context, DynamicProfileService::class.java))
        }
    }

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val router by lazy { DaemonEventRouter(this) { p, r -> updateNotification(p, r) } }
    private lateinit var config: DynamicProfileConfig
    private lateinit var reader: DeviceContextReader
    @Volatile private var client: DaemonClient? = null
    private val starting = java.util.concurrent.atomic.AtomicBoolean(false)
    @Volatile private var stopping = false
    @Volatile private var acceptingEvents = true
    private var lastClientStart = 0L
    @Volatile private var lastScreen: Boolean? = null
    @Volatile private var lastLocked: Boolean? = null
    private var lastNotified: Pair<String, String>? = null

    private val receiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            Log.d(TAG, "receiver: ${intent.action}")
            when (intent.action) {
                Intent.ACTION_SCREEN_OFF -> sendScreen()
                Intent.ACTION_SCREEN_ON -> sendScreen()
                Intent.ACTION_USER_PRESENT -> {
                    // keyguard dismissed: keep the reconcile dedupe in sync
                    // so the supervisor does not re-send screen and re-peek
                    lastScreen = true
                    lastLocked = false
                    client?.send(DaemonCommands.userPresent())
                }
            }
        }
    }

    override fun onCreate() {
        super.onCreate()
        config = DynamicProfileConfig.get(this)
        reader = DeviceContextReader(this)
        DaemonNotifications.createChannels(this)
        startForeground(
            DaemonNotifications.NOTIF_ID,
            DaemonNotifications.buildServiceNotification(this, "starting…"),
        )
        // A sticky restart after a crash mid-OFF must not resurrect the FGS
        // while the user has the service disabled. Start foreground first so
        // Android's 5 s contract is satisfied, then stop cleanly.
        if (!config.enabled) {
            Log.i(TAG, "service created while disabled; stopping")
            stopSelf()
            return
        }
        DynamicProfileState.running.value = true
        DaemonLink.client = null

        val filter = IntentFilter().apply {
            addAction(Intent.ACTION_SCREEN_OFF)
            addAction(Intent.ACTION_SCREEN_ON)
            addAction(Intent.ACTION_USER_PRESENT)
        }
        ContextCompat.registerReceiver(this, receiver, filter, ContextCompat.RECEIVER_NOT_EXPORTED)

        scope.launch(Dispatchers.IO) { startClient() }
        startSupervisor()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_STOP) {
            // "Turn off" from the tile/notification: restore stock through
            // the daemon (or the CLI), verify, then stop.
            if (stopping) return START_NOT_STICKY
            stopping = true
            scope.launch { restoreThenStop() }
            return START_NOT_STICKY
        }
        if (stopping) {
            // A START arriving while OFF is restoring must not resurrect the
            // service behind the user's back; the user can toggle again.
            Log.i(TAG, "START ignored: Service OFF is in progress")
            return START_NOT_STICKY
        }
        return START_STICKY
    }

    override fun onDestroy() {
        // Stop accepting output before the reader gets a chance to deliver a
        // buffered event into the state flows after the service is gone.
        acceptingEvents = false
        runCatching { unregisterReceiver(receiver) }
        DaemonLink.client = null
        client?.stop()
        client = null
        scope.cancel()
        DynamicProfileState.running.value = false
        DynamicProfileState.reason.value = null
        DynamicProfileState.appliedProfile.value = null
        // Do not leave stale daemon telemetry visible while the service is off.
        DynamicProfileState.diag.value = null
        DynamicProfileState.env.value = null
        DynamicProfileState.stats.value = emptyList()
        DynamicProfileState.heals.value = HealsInfo()
        DynamicProfileState.clearLogs()
        ServiceTile.refresh(this)
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    // --- daemon lifecycle ---------------------------------------------------

    private fun startClient() {
        // Single-flight: the supervisor tick and onCreate must not spawn two
        // `su` daemons while a slow APatch grant is still pending.
        if (!starting.compareAndSet(false, true)) return
        try {
            lastClientStart = System.currentTimeMillis()
            // A failed liveness check can leave the previous child winding
            // down; wait for its exit restore before the PID lock allows a
            // replacement daemon.
            client?.stopAndWait()
            client = null
            val c = DaemonClient(
                configPath = daemonConfigPath(this),
                onEvent = { ev -> handleEvent(ev) },
                onExit = {
                    // restart is the supervisor's job (single restart path)
                    lastClientStart = System.currentTimeMillis()
                    Log.w(TAG, "daemon exited")
                },
                onLog = { line -> DynamicProfileState.pushLog(line) },
            )
            if (!c.start()) {
                Log.w(TAG, "daemon start failed (root?)")
                return
            }
            c.send(DaemonCommands.hello())
            if (!c.awaitHello(5_000)) {
                Log.w(TAG, "daemon did not answer hello; discarding")
                c.stop()
                lastClientStart = System.currentTimeMillis()
                return
            }
            client = c
            DaemonLink.client = c
            // seed the diagnostics screen (on-demand, cheap)
            c.send(DaemonCommands.diag())
            c.send(DaemonCommands.stats())
            // force the full context on every (re)start: the dedupe state
            // belongs to the previous daemon instance
            lastScreen = null
            lastLocked = null
            lastNotified = null
            sendScreen()
        } finally {
            starting.set(false)
        }
    }

    /**
     * 3 s supervisor: screen/keyguard reconcile (MIUI sometimes drops the
     * broadcasts) + daemon restart with backoff.
     */
    private fun startSupervisor() {
        scope.launch {
            while (isActive) {
                delay(SUPERVISE_MS)
                sendScreen()
                val alive = client?.isAlive == true
                if (!alive && !starting.get()) {
                    val now = System.currentTimeMillis()
                    if (now - lastClientStart > RESTART_BACKOFF_MS) {
                        Log.w(TAG, "daemon down — restarting")
                        startClient()
                    }
                }
            }
        }
    }

    /** Forwards the screen/keyguard state only when it changed. */
    private fun sendScreen() {
        val on = reader.screenOn
        val locked = reader.keyguardLocked
        if (on == lastScreen && locked == lastLocked) return
        lastScreen = on
        lastLocked = locked
        client?.send(DaemonCommands.screen(on, locked))
    }

    /** Restore stock (daemon or CLI), verify, then disable + stop. */
    private suspend fun restoreThenStop() {
        val coordinator = ServiceOffCoordinator(
            begin = { config.beginServiceOff() },
            abort = { config.abortServiceOff() },
            confirm = { config.confirmServiceOff() },
            clearSessionFlag = { if (config.rndInputSuspend) config.rndInputSuspend = false },
            restore = { StockRestore.restore() },
        )
        when (val outcome = coordinator.run { true }) {
            is ServiceOffCoordinator.Outcome.Success -> {
                DynamicProfileState.appliedProfile.value = null
                stopSelf()
            }
            ServiceOffCoordinator.Outcome.Refused -> {
                stopping = false
                Log.w(TAG, "stop: could not persist restore_pending; keeping service alive")
            }
            ServiceOffCoordinator.Outcome.Cancelled -> stopping = false
            is ServiceOffCoordinator.Outcome.RestoreFailed -> {
                stopping = false
                Log.w(TAG, "stop: stock restore not confirmed; keeping service alive")
            }
            is ServiceOffCoordinator.Outcome.ConfirmFailed -> {
                config.abortServiceOff()
                stopping = false
                Log.w(TAG, "stop: could not clear restore_pending; keeping service alive")
            }
        }
    }

    private fun handleEvent(ev: JSONObject) {
        if (!acceptingEvents) return
        router.handle(ev)
    }

    private fun updateNotification(profileId: String, reason: String) {
        // state events arrive on every visible change; the notification only
        // needs a re-post when the profile or the reason changed
        val key = profileId to reason
        if (lastNotified == key) return
        lastNotified = key
        getSystemService(NotificationManager::class.java).notify(
            DaemonNotifications.NOTIF_ID,
            DaemonNotifications.buildServiceNotification(
                this,
                "active · ${ProfileLabels.of(profileId)} · $reason",
            ),
        )
        // keep the Quick Settings tile subtitle in step with the profile
        ServiceTile.refresh(this)
    }
}
