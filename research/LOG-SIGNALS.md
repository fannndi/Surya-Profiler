# Logcat signals — MIUI 14 / Android 12

Semua contoh dari device (epoch seconds, `-v epoch`). Untuk daemon, gunakan
stream terpisah agar parsing sederhana dan tidak membanjiri buffer.

## 1. Foreground: events buffer (`wm_*`)

**Tag yang terbukti bekerja:**

```text
wm_resume_activity: [0,<activityId>,<taskId>,<pkg>/<activity>]
wm_set_resumed_activity: [0,<pkg>/<activity>,<reason>]
wm_task_to_front: [0,<taskId>]
wm_focused_root_task: [0,0,<rootTaskId>,<lastRootTaskId>,<reason>]
```

Contoh nyata:

```text
1795100618.869  1732  2942 I wm_resume_activity: [0,117144395,177,com.whatsapp/.home.ui.HomeActivity]
1795100621.031  1732  1767 I wm_set_resumed_activity: [0,com.miui.home/.launcher.Launcher,bringingFoundTaskToFront]
```

- Launcher **hanya** muncul di `wm_set_resumed_activity` pada kasus ini; karena
  itu parser harus mendukung kedua tag (seperti referensi).
- Package = token sebelum `/` pada field package/activity.
- Filter server-side: `logcat -b events -v epoch -s wm_resume_activity:I wm_set_resumed_activity:I`
  (format `TAG:LEVEL`; gunakan `*:S` untuk silence tag lain bila perlu).

## 2. Foreground rich info: main buffer (`ProcessMonitor`)

```text
ProcessMonitor: onForegroundInfoChanged: ForegroundInfo{
  mForegroundPackageName='com.whatsapp',
  mForegroundUid=10186, mForegroundPid=10736,
  mLastForegroundPackageName='com.android.settings',
  mLastForegroundUid=1000, mLastForegroundPid=6417,
  mMultiWindowForegroundPackageName='com.whatsapp',
  mMultiWindowForegroundUid=10186,
  mFlags=0
}
```

- Muncul saat app berubah (WhatsApp -> Chrome -> Settings).
- **Tidak muncul** saat pindah ke launcher pada uji `am start HOME`.
- `mMultiWindowForegroundPackageName` masih sama dengan foreground pada split
  screen sintetis; belum bisa diandalkan sebagai sinyal multi-window.
- Tag duplikat: `PerfEngineController` (PowerKeeper) dan `AppObserver`
  (launcher) mencetak objek yang sama.
- Berguna sebagai sinyal sekunder / label, bukan pengganti `wm_*`.

## 3. Multi-window

Belum ada sinyal yang terbukti.

- `MultiScreenObserver: onTaskStackChanged, splitScreenMode: false isCameraFloat: false isSmallScreen: false`
  muncul saat split screen sintetis dibuat (Chrome primary + Settings
  secondary, keduanya `visible=true` di `dumpsys activity activities`).
- `ProcessMonitor` tidak berubah pada split tersebut.
- `GameBoosterService` / `mMultiWindowForegroundPackageName` ala MIUI 12 tidak
  ada (package Joyose/GameBooster tidak terpasang).
- Kandidat fallback: polling `dumpsys activity activities` dan cari
  `mode=split-screen` + `visible=true`. Biaya perlu diukur; root `dumpsys`
  bekerja.
- Kandidat lain: `dumpsys window displays` / `dumpsys activity` events; belum
  diverifikasi.

## 4. Jank

- `Choreographer` masih mencetak `Frame time is ... in the future!` pada MIUI 14.
- Uji swipe di Chrome tidak menghasilkan `Skipped N frames!`; sinyal jank
  `Choreographer: Skipped N frames!` belum terbukti.
- Alternatif: `dumpsys gfxinfo` (janky frames) atau frame stats; perlu
  eksperimen dengan app/game yang benar-benar jank.
- Untuk v1, jank boost sebaiknya **opsional/experimental** dan tidak menjadi
  blocker.

## 5. Screen / power / thermal (dari app)

- App dapat memakai broadcast `ACTION_SCREEN_ON/OFF` + `KeyguardManager`.
- `low_power` bisa dibaca via `settings get global low_power`.
- `thermal_message/board_sensor_temp` bisa dibaca langsung (root) atau via
  `PowerManager` thermal API untuk app.
- `DisplayFeatureHal` logs (`HandleFpsSwitch`, `HandleDisplayState`) berguna
  untuk debug refresh; jangan dijadikan input keputusan.

## 6. Contoh filter daemon yang disarankan

```text
# foreground (events)
logcat -b events -v epoch -s wm_resume_activity:I wm_set_resumed_activity:I *:S

# rich foreground (main)
logcat -b main -v epoch -s ProcessMonitor:I *:S

# jank (main; experimental)
logcat -b main -v epoch -s Choreographer:W *:S
```

Catatan: `logcat -s` dengan `*:S` harus diuji di device; jika tidak didukung
oleh toybox logcat versi ini, gunakan `logcat -b events -v epoch` lalu filter
di proses daemon. Stream mentah events buffer relatif kecil.
