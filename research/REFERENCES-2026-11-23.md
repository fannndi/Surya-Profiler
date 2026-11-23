# Audit repo referensi & adaptasi — 2026-11-23

Empat repo referensi di `/home/fannndi/Surya-Profiler/Reference/` diaudit
(subagent, source-level) dan diverifikasi live di device surya (MIUI
14 V14.0.2.0.SJGIDXM, kernel 4.14.190, kondisi stock: service OFF,
`/data/vendor/thermal/config` kosong, `sconfig=0`). Surya-Profiler lisensinya
Apache-2.0; perhatian lisensi per repo di bawah.

## 1. Referensi yang diaudit

| Repo | Lisensi | Verdict |
|---|---|---|
| `vtools_en` (Scene 5.3) | **GPLv3** | Sumber ide terbanyak — dianulir hanya di level spesifikasi/angka, **tidak menyalin kode** (ide/fakta teknis tidak berhak cipta; kode Kotlin/shell-nya GPL) |
| `packages_apps_XiaomiParts` (AOSPA fork) | Apache-2.0 | Bukan sumber node performa (display/audio/haptic); 1 temuan telemetri |
| `mi-thermal-crypt` | Apache-2.0 | Sudah tercakup — `core/src/engine/crypt.rs` terbukti identik |
| `android_packages_apps_GameSpace` | Apache-2.0 | Tidak layak untuk MIUI stock (butuh patch framework + privapp) |

## 2. Fakta live device (2026-11-23)

| Cek | Hasil |
|---|---|
| `/proc/sys/kernel/sched_boost_top_app` | **ABSENT** (hanya `sched_boost`, nilai 0, perf-HAL-owned) |
| `/proc/sys/kernel/sched_prefer_sync_wakee_to_waker` | **ABSENT** (node 5.x) |
| `/sys/devices/platform/soc/1d84000.ufshc/{clkscale,clkgate,hibern8}_enable` | Ada, root:root 0644, nilai 1/1/1, mtime segar → **writer aktif = vendor init** |
| Owner ufshc | `/vendor/etc/init/hw/init.qcom.rc` menulis `clkscale_enable` 0/1; perf HAL `vendor.qti.hardware.perf@2.2-service` berjalan |
| `thermal-engine` binary | **Tidak ada** di MIUI 14 — daemon-nya `mi_thermald` (running, root). Rencana "thermal-engine -o doctor dump" dianulir: `thermal.dump`/`last_thermal.dump` + `decrypt.txt` sudah mencakup |
| `/sys/class/drm/sde-crtc-0/measured_fps` | Ada dan terbaca via su (`fps: 0.0 duration:1000000 frame_count:0` saat screen off) — sampler kita **sudah memakai path ini** (`screen_fps`); perlu verifikasi nilai live (60/90) saat E2E |
| prop `sys.thermal.data.path` | `/data/vendor/thermal/` (dasar scanner file asing) |
| `/data/adb/modules` | Tidak ada (APatch tanpa module) — scan tetap defensif |
| Su context | `u:r:magisk:s0` (APatch/FolkPatch) |

## 3. Yang diadaptasi (implementasi 0.7.x)

1. **`forbidden.rs`**: + `sched_boost_top_app`, `sched_prefer_sync_wakee_to_waker`
   (absent-on-kernel, pola sched_coloc_*), + ufshc `clkscale/clkgate/hibern8`
   (vendor init-owned, bukti di atas).
2. **`crypt.rs`**: komentar test dikoreksi ke CLI yang valid (`-i`/`-o` wajib);
   differential vectors dari binary referensi (n=0/1/15/16/17 full-hex,
   4096/4097/8192 sha256, pola non-ASCII 65 byte) — semua terverifikasi
   round-trip; helper `is_encrypted` + `decrypt_or_plain`.
   **Catatan oracle**: binary `mi-thermal-crypt` benar PKCS7 standar (fixture
   vendor 291→304 byte round-trip identik); kebingungan awal audit disebabkan
   lupa flag `-e` (default = decrypt). `decrypt()` kita melakukan CBC penuh
   dulu, jadi kekhawatiran "trailing \n\n lolos unpad" tidak berlaku.
3. **`decrypt_or_plain` dipasang di `vendor_map_entries_at`** — build MIUI lain
   bisa memuat map plaintext (temuan Scene); tanpa ini map akan diam-diam
   fallback ke tabel hardcoded.
4. **Doctor: scanner file thermal asing** (`bridge/thermal/files.rs` +
   `diagnose/thermal_files.rs` + `doctor --fix-thermal`): klasifikasi
   owned/encrypted-map/encrypted-conf/plaintext-map/plaintext-conf/unknown,
   scan module (`/data/adb/modules/*/system*`), prop path, repair
   `chattr -R -i` → hapus asing → `restorecon -DFR` (hanya via CLI saat
   daemon berhenti). Motivasi: file asing mem-bayangi config vendor saat
   `sconfig=0` sehingga "Service OFF = stock" tidak lagi benar.
5. **`measured_fps`**: sudah tercakup sampler (`screen_fps`) — tidak ada
   perubahan kode; verifikasi nilai live di E2E.

## 3b. E2E device scanner (2026-11-23, surya)

| Step | Hasil |
|---|---|
| `doctor` scan, kondisi takeover aktif (owned 6 file) | `thermal_foreign_files pass=true, clean=true`, base ter-resolve dari prop |
| Tanam 3 file asing (`thermal-foreign.conf` blob, `thermal-map.conf.alt` plaintext map, `leftover.bin`) | scan: `pass=false`; klasifikasi tepat — `encrypted-conf`, `plaintext-map` (entries `[0, 9]`), `unknown-binary`; owned tidak terflag |
| Force-stop app (path stdin-EOF 0.6.0) | exit restore daemon: `sconfig=0`, 6 file owned terhapus, **3 file asing bertahan** (bukti masalah nyata) |
| `doctor --fix-thermal` (daemon off) | `removed: [thermal-foreign.conf, thermal-map.conf.alt]`, `failed: []`, `chattr_ok=true`, `restorecon_ok=true`, re-scan `clean=true`, overall `ok=true` |
| Round-trip takeover | deploy→restart daemon: `sconfig=60` + decrypt.txt match; force-stop: `sconfig=0` + config dir bersih (2× konsisten) |
| `screen_fps` live (screen on, swipe) | `fps: 14.9` → `21.9` → `34.9` — telemetri nyata terbaca via `measured_fps` |
| Guard test baru | `guard_path` menolak 5 node baru (sched_boost_top_app, prefer_sync_wakee, ufshc ×3) — unit test `forbidden_paths_rejected` |

## 4. Yang TIDAK diadaptasi (melanggar hard rule / out of scope)

- `msm_performance` unlock 2.5/2.75 GHz (Scene powercfg) — no-op clamp kernel,
  hard rule #1.
- `cpu_boost` powerkey/sched_boost_on_input, `sched_boost(_top_app)` write,
  `sched_load_boost` — forbidden.
- bus-DCVS `llcc-bw`/`llcc-ddr-bw`, devfreq `1d84000.ufshc`, lpm/lmk/vm-swap —
  forbidden / tanpa win terukur (RND-SOC-2026-11-22).
- Remount `/vendor`, replace perfd/perfboostsconfig, uninstall
  `com.miui.daemon`/joyose, stop mi_thermald — bertabrakan coexist/harmony.
- Scene: accessibility foreground (kita: logcat watcher lebih baik), biner
  `scene-scheduler` (tertutup, tidak bisa diaudit, melanggar single-writer),
  swap-controller, xposed addin, freeze/suspend app.
- GameSpace: GameMode API (kosong di MIUI stock, tanpa listener publik),
  `sys.perf_profile` (tidak ada konsumen), FPS via `registerTaskFpsCallback`
  (hidden/privileged — jalur benar: `measured_fps`/`dumpsys gfxinfo`).
- XiaomiParts: KCAL/HBM/CABC/Dirac/haptic/refresh-rate `Settings.System`
  (framework-owned; padanan `persist.sys.aries.*` kita).

## 5. Referensi tuning per-app sm6150 (angka murni, dari Scene
   `powercfg/sm6150/powercfg-utils.sh` `adjustment_by_top_app` + tabel 730g)

Nilai stock-max kita: little 1804800, big 2208000 (setelah unlock constraint
kita: **tanpa msm_performance**, sesuai 0.7.0). Angka Scene yang melebihi itu
di-clamp; hanya dipakai sebagai kandidat untuk iterasi profil per-app
berikutnya, bukan katalog:

| App/kelas | Pengaturan Scene | Adaptasi clamp ke stock |
|---|---|---|
| Genshin (balance) | `sched_config` top-app 50/68 (bg 67/80), `stune.top-app.boost=20`, `gpu_pl_up=2`, freq 1804800/2500000/1939200/2750000 | sched_config & stune dalam range kita; freq big → 2208000 |
| douyin/bilibili (streaming) | `core_ctl` cpu0/cpu7 on, cpuset.foreground 0-3 | kandidat profil "video": core_ctl little on + cpuset |
| game umum (fast) | little max 1804800, big 2208000, schedutil hispeed 1804800/2304000, gpu min_pwrlevel naik 1 step | identik dengan nilai kita saat ini |

Frame-time harness kita (RND-SOC-2026-11-22) adalah alat validasi; angka di
atas hanya memasuki backlog eksperimen, tidak mengubah katalog tanpa A/B.

## 6. Desain tertunda: thread-affinity per-app (Fase 6)

Desain lengkap terkunci 2026-11-23 (diskusi): modul `bridge/affinity/`,
config per-app `affinity: [{comm, mask}]` di `config.json` (schema-2),
eksekusi `taskset -pc <tid>` idempoten (baca `Cpus_allowed_list` dulu),
capture/restore per-tid ala holds.json, guard uid target + mask ⊆ cpuset
profil aktif, opt-in default off, polling 1 s saat game foreground, host
test via fake `/proc` (env var baru `SURYAPROFILER_PROC_ROOT`). Belum
diimplementasi — menunggu doctor/guard stabil.

## 7. Lisensi & atribusi

- Scene/vtools: GPLv3 — semua peminjaman di atas adalah fakta teknis
  (path node, angka, format config, perilaku daemon) atau penulisan-ulang
  dari spesifikasi; tidak ada kode Scene disalin. Biner `scene-scheduler`
  tidak didistribusikan.
- XiaomiParts/GameSpace/mi-thermal-crypt: Apache-2.0 — atribusi pola tetap
  dicatat di sini sesuai etika.
