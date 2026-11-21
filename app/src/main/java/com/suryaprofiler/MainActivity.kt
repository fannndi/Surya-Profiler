package com.suryaprofiler

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.lifecycle.viewmodel.compose.viewModel
import com.suryaprofiler.ui.HomeScreen
import com.suryaprofiler.ui.HomeViewModel
import com.suryaprofiler.ui.theme.SuryaProfilerTheme

/**
 * Single activity. All logic lives in [HomeViewModel] + the Rust core.
 */
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        maybeRequestNotifications()
        setContent {
            SuryaProfilerTheme {
                val vm: HomeViewModel = viewModel()
                HomeScreen(vm)
            }
        }
    }

    /** The FGS notification is the primary stop/status surface on API 33+. */
    private fun maybeRequestNotifications() {
        if (Build.VERSION.SDK_INT < 33) return
        if (checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED
        ) {
            return
        }
        requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 41)
    }
}
