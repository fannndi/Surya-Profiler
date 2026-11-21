# MIUI 14 / Android 12 vs MiFineTune (MIUI 12 / Android 10)

Dokumen ini hanya mencatat perbedaan yang **berdampak pada desain**.
Referensi MiFineTune tetap dipakai untuk filosofi, bukan nilai/path mentah.

## 1. Runtime & framework

| Area | MiFineTune (MIUI 12/A10) | MIUI 14/A12 (device ini) | Dampak |
|---|---|---|---|
| Activity resume tags | `am_resume_activity`, `am_set_resumed_activity` | **`wm_resume_activity`, `wm_set_resumed_activity`** | watcher foreground harus ganti tag |
| Foreground rich info | tidak ada | `ProcessMonitor: onForegroundInfoChanged: ForegroundInfo{...}` (main buffer) | bisa dipakai untuk label + multiwindow, tapi **tidak selalu log saat pindah ke launcher** |
| Multi-window | `GameBoosterService` log `mMultiWindowForegroundPackageName` | `MultiScreenObserver` ada, tetapi log `splitScreenMode:false` pada split sintetis; `ProcessMonitor` punya `mMultiWindowForegroundPackageName` tetapi tidak berubah pada split sintetis | **belum ada sinyal yang terbukti; perlu riset lanjutan / fallback `dumpsys`** |
| FGS | Android 10 | Android 12: restriksi start FGS dari background, `PendingIntent` mutability, notification trampoline | app shell harus menangani `ForegroundServiceStartNotAllowedException`, flag PendingIntent |
| Phantom process killer | tidak ada | Android 12 punya phantom process killer; uji daemon root detached bertahan (PPID 1) | risiko rendah, tetapi harus diuji dengan daemon yang di-spawn app + stdin pipe |
| Cgroup | v1 | hybrid v1 + **cgroup2 freezer** (`/sys/fs/cgroup/frozen|unfrozen`, dibuat `millet_monitor`) | app bisa dibekukan; FGS + whitelist battery optimization penting |
| Root | APatch | APatch; `su` uid 0, transisi `magisk:s0`, tulis sysfs berhasil | tetap single-writer root daemon |

## 2. CPU / scheduler

| Area | MiFineTune | MIUI 14 | Dampak |
|---|---|---|---|
| Screen-on floor | 576000/652800 (bisa turun) | **boost 1612800/1843200** via `msm_performance.cpu_min_freq` (interaction-scoped) | profil tidak bisa menurunkan floor saat boost; `scaling_min_freq` di bawah floor live di-clamp |
| Screen-off floor | 576000/652800 | 576000/652800 | sleep profile tetap bisa cap max |
| `sched_boost` | bisa 0/1; forbidden | screen-on 1, screen-off 0; forbidden | jangan ditulis; framework punya kontrol |
| OPP little | 300k..1804800 (tanpa 1612800/1708800?) | + `1612800`, `1708800` | validator OPP harus pakai list live |
| OPP big | 300k..2304000 | + `1843200`, `1939200`, `2169600`, `2208000` | balance big cap minimal 1843200 saat screen-on |
| core_ctl | `policy0/core_ctl` (referensi) | `cpu0/core_ctl`, `cpu6/core_ctl`; big `enable=0` | path & validasi beda |
| `schedutil` | hispeed 1248000/1324600, load 90/85 | sama; tidak ada `schedutil/conservative_pl` | `kernel.sched_conservative_pl` tetap ada |
| `cpu_boost` | input 1324800@120ms | sama, powerkey 1804800/2208000@400ms | `input_boost_freq` tetap satu-satunya yang diizinkan |
| cpuset stock | 0-3/0-5/0-5/0-7 | **0-2 / 0-3 / 0-2,4-7 / 0-7**; `foreground/boost=4-7` | profile tidak boleh copy cpuset MIUI 12 |

## 3. GPU / IO / VM

| Area | MiFineTune | MIUI 14 | Dampak |
|---|---|---|---|
| GPU levels | 7 level 180..800 | sama | pwrlevel mapping sama |
| GPU stock | max 0/min 6/default 6 | sama | powersave max 5, game min 3 |
| IO scheduler | cfq/deadline | cfq stock, deadline tersedia | sama |
| CFQ stock | beda | quantum 8, fifo 125/250, slice 100/40, low_latency 1 | nilai profile harus pakai baseline MIUI 14 |
| VM stock | dirty 25/10/500/1000 (asumsi) | **20/5/200/500**, stat 1, watermark 1 | jangan pakai angka referensi tanpa cek |
| f2fs | sda16 | sda16 | sama |

## 4. Display / refresh

| Area | MiFineTune | MIUI 14 | Dampak |
|---|---|---|---|
| Settings key | `user_refresh_rate` | `user_refresh_rate`, `peak_refresh_rate`, `miui_refresh_rate` (secure); settings alone **tidak** mengubah panel | tetap butuh jalur SF |
| SF transaction | `1035 i32 <idx>` | **`1035 i32 <idx>` masih bekerja** (0=120,1=90,2=60,3=50,4=30) | bisa reuse |
| MIUI internal code | 1035 | `ScreenEffectService` punya `31035`/`31037`; `service call` 31035 gagal `Not a data message` | gunakan 1035 |
| DFPS list | `[120,90,60,50,30]` | sama, dari panel `nt36672c_huaxing` | mapping sama |
| HAL helper | dex `setScreenEffect(24,hz)` | dex jalan tetapi mencetak error `theme_compatibility.xml`; log HAL `HandleFpsSwitch` tidak terlihat pada uji | SF transaksi jadi sumber utama; helper opsional |
| Panel | 1080x2400 | sama; modes 120/90/60/50/30 | per-app refresh memungkinkan |

## 5. Charging

| Area | MiFineTune | MIUI 14 | Dampak |
|---|---|---|---|
| `input_suspend` | tersedia, 0777 | tersedia, 0777; `1` -> Discharging | per-app bypass tetap bisa |
| `battery_charging_enabled` | tersedia | tersedia; `0` -> current discharge walau status masih Charging | charge guard pakai current sign sebagai verifikasi |
| `charge_full_design` | rusak/negatif | rusak/negatif; bms 5216 mAh | wear estimate pakai 5216 (atau spec 5160) |
| MIUI charge limit UI | tidak ada | tidak terlihat di build ini | fitur charge limit kita tetap relevan |

## 6. Thermal

| Area | MiFineTune | MIUI 14 | Dampak |
|---|---|---|---|
| Config | encrypted | encrypted, tapi `decrypt.txt` plaintext | bisa baca config aktif |
| Normal | big 1209600 @50 °C | sama | balance big cap 1.32 GHz tidak mungkin saat boost floor 1.8432 aktif |
| Sensor | CPU thermal zone by type | banyak `cpu-0-*-usr`/`cpu-1-*-usr`; SS memakai `nvm-therm-adc` (VIRTUAL-SENSOR) | thermal telemetry baca nvm + quiet |
| tgame | ada | ada (encrypted); teruji: cap cpu6 sama, tambah cap cpu0 + LCD awal | bukan upgrade CPU sustained |
| `thermal_message` | ada | `sconfig=0`; game launch tidak mengubahnya | app hanya baca, tidak menulis |
| Override | — | `/data/vendor/thermal/config` (map+config terenkripsi) | R&D only; tidak dipakai app |

## 7. MIUI stack

| Area | MiFineTune | MIUI 14 | Dampak |
|---|---|---|---|
| PowerKeeper | ada | ada 4.2.00; `mPowerMode=2`, `PowerModeHandler` mengatur floor | jangan lawan; monitor saja |
| `mcd` | ada | ada; config plaintext | framework-owned |
| `millet_monitor` | tidak ada | ada (SIG/BINDER/PKG; cgroup2 freezer) | app freezing; FGS/whitelist |
| Joyose | ada (per-game data) | **tidak ada** | jangan bergantung pada Joyose |
| GameTurbo | ada | package tidak ada; game launch tidak memilih `sconfig=9` | game checker lama tidak berlaku |
| perf HAL | 2.0 | **2.2**, 166 resource path | overlap catalog 22 path; baseline/forbidden diperbarui |
| QS tile performance | tidak | `PerformanceModeTileService` (securitycenter) | MIUI-owned, jangan ditulis |

## 8. Ringkasan aturan baru

1. **Jangan pernah** berharap bisa menurunkan CPU floor saat boost aktif; floor itu milik MIUI power policy (`msm_performance`), bukan katalog kita. Boost bersifat interaction-scoped (wake/touch), bukan konstanta screen-on.
2. Validasi `scaling_max_freq >= live_min` (bukan hanya OPP); kalau tidak, tulis gagal.
3. Profile `balance`/`game` tidak boleh memakai cap big < 1843200 saat screen-on.
4. Refresh: **SF transaksi 1035** adalah lever utama; settings key hanya bookkeeping.
5. Foreground watcher: `wm_resume_activity`/`wm_set_resumed_activity`; `ProcessMonitor` sebagai pelengkap.
6. Multi-window: sinyal lama tidak ada; perlu fallback `dumpsys`/riset.
7. Joyose/GameTurbo checker tidak dipakai; bridge fokus ke saver, refresh, DND, charge/bypass.
8. Perf HAL 2.2 overlap 22 node -> Baseline; `msm_performance`, `millet`, `mcd`, thermal, `cpu_boost` prefix tetap forbidden.
