#!/system/bin/sh
# MIUI 14 stack inventory: packages, processes, props, settings, firmware files.

echo "=== MIUI PACKAGES ==="
pm list packages 2>&1 | grep -i -E 'powerkeeper|joyose|game|misettings|mifinetune|securitycenter|battery|thermal|perf|mcd|millet' 2>&1
echo "=== PACKAGE VERSIONS ==="
for p in com.miui.powerkeeper com.xiaomi.joyose com.xiaomi.gamecenter com.xiaomi.glgm com.xiaomi.misettings com.miui.securitycenter com.android.settings; do
  echo "--- $p"
  dumpsys package "$p" 2>&1 | grep -E 'versionName|versionCode|firstInstallTime|lastUpdateTime' | head -5
done

echo "=== PROCESSES ==="
ps -A 2>&1 | grep -i -E 'powerkeeper|joyose|game|thermal|perf|mcd|millet|miui|surfaceflinger' 2>&1
echo "=== INIT SERVICES ==="
getprop 2>&1 | grep -i -E 'init.svc.(perf|thermal|mcd|millet|power|joyose|game|miui)' 2>&1

echo "=== POWER/PERF PROPS ==="
getprop 2>&1 | grep -i -E 'perf|thermal|power|dfps|refresh|game|miui|aries|millet|mcd' 2>&1

echo "=== SETTINGS SYSTEM ==="
settings list system 2>&1
echo "=== SETTINGS GLOBAL ==="
settings list global 2>&1
echo "=== SETTINGS SECURE ==="
settings list secure 2>&1

echo "=== VENDOR INIT FILES ==="
ls -la /vendor/etc/init 2>&1
echo "=== VENDOR BIN POST BOOT ==="
ls -la /vendor/bin/init.qcom.post_boot.sh /system/bin/init.qcom.post_boot.sh /vendor/etc/init.qcom.post_boot.sh 2>&1
echo "=== PERF XML ==="
ls -la /vendor/etc/perf 2>&1
echo "=== POWERHINT ==="
ls -la /vendor/etc/powerhint.xml /vendor/etc/perf/powerhint.xml 2>&1
echo "=== THERMAL FILES ==="
ls -la /vendor/etc/thermal* 2>&1
echo "=== MCD ==="
ls -la /system/bin/mcd /system/etc/mcd_default.conf /system/bin/millet 2>&1
echo "=== PERFD DEFAULT VALUES ==="
ls -la /data/vendor/perfd 2>&1
head -100 /data/vendor/perfd/default_values 2>&1
echo "=== THERMAL LIVE ==="
ls -la /data/vendor/thermal 2>&1
for f in /data/vendor/thermal/*; do echo "--- $f"; head -50 "$f" 2>&1; done

echo "=== DMESG AVC ==="
dmesg 2>&1 | grep -i avc | tail -50

echo "=== DONE ==="
