#!/usr/bin/env bash
# Install the APK, restart the service, and print a device health summary.
set -euo pipefail
cd "$(dirname "$0")/.."
ADB=${ADB:-$(command -v adb || echo "$HOME/Android/Sdk/platform-tools/adb")}
APK=app/build/outputs/apk/debug/app-debug.apk

"$ADB" install -r "$APK"
"$ADB" shell su -c 'input keyevent KEYCODE_WAKEUP' >/dev/null 2>&1 || true
"$ADB" shell su -c 'wm dismiss-keyguard' >/dev/null 2>&1 || true
"$ADB" shell am force-stop com.suryaprofiler
"$ADB" shell am start -n com.suryaprofiler/.MainActivity >/dev/null
sleep 6

echo "== binary =="
"$ADB" shell su -c 'md5sum /data/local/tmp/suryaprofiler/surya-prof'
echo "== sconfig =="
"$ADB" shell su -c 'cat /sys/class/thermal/thermal_message/sconfig'
echo "== holds =="
"$ADB" shell su -c 'cat /data/adb/suryaprofiler/holds.json' 2>/dev/null || true
echo "== doctor =="
"$ADB" shell su -c '/data/local/tmp/suryaprofiler/surya-prof doctor --config /data/data/com.suryaprofiler/files/config.json' | head -c 4000 || true
echo
