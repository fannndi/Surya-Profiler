#!/usr/bin/env bash
# rnd_run.sh TAG PROFILE DUR SCENARIO [LAYER_SUBSTR]
# One short A/B run (3-5 min) with power + SurfaceFlinger frame sampling.
#   PROFILE "-" = stock (no apply/restore)
#   SCENARIO chrome|game|video|idle
#   LAYER_SUBSTR default: ChromeChildSurface / SurfaceView[com.stove.epic7
#   env BUS=off|medium|high  direct bus-DCVS override (R&D only)
#   env VARIANT=/host/file.json  use a variant profile pack
#   env WARMUP=5  settle seconds before sampling
set -euo pipefail
ADB=${ADB:-/home/<user>/Android/Sdk/platform-tools/adb}
BIN=/data/local/tmp/suryaprofiler/surya-prof
DIR=/data/local/tmp/surya_rnd
RAW=research/raw/rnd
TAG=$1
PROFILE=$2
DUR=$3
SC=$4
LAYER=${5:-}
BUS=${BUS:-off}
WARMUP=${WARMUP:-5}
mkdir -p "$RAW"

cmd() { "$ADB" shell "su -c '$1'"; }

case "$SC" in
  chrome) [ -n "$LAYER" ] || LAYER="ChromeChildSurface" ;;
  stress) [ -n "$LAYER" ] || LAYER="ChromeChildSurface" ;;
  game)   [ -n "$LAYER" ] || LAYER="SurfaceView[com.stove.epic7.google/kr.supercreative.epic7.AppActivity](BLAST)" ;;
  video)  [ -n "$LAYER" ] || LAYER="" ;;
  idle)   [ -n "$LAYER" ] || LAYER="" ;;
  *) echo "unknown scenario $SC"; exit 2 ;;
esac

echo "[$TAG] prep screen"
"$ADB" shell input keyevent 224 >/dev/null 2>&1 || true
"$ADB" shell wm dismiss-keyguard >/dev/null 2>&1 || true
"$ADB" shell settings put system screen_brightness 400
"$ADB" shell settings put system screen_brightness_mode 0
"$ADB" shell settings put system user_refresh_rate 60
"$ADB" shell settings put system peak_refresh_rate 60
"$ADB" shell settings put system screen_off_timeout 1800000
cmd "service call SurfaceFlinger 1035 i32 2" >/dev/null 2>&1 || true
cmd "sh $DIR/rnd_keepawake.sh on" >/dev/null 2>&1 || true

if [ "$PROFILE" != "-" ]; then
  PF=""
  if [ -n "${VARIANT:-}" ]; then
    "$ADB" push -q "$VARIANT" "$DIR/profiles_variant.json"
    PF="--profiles $DIR/profiles_variant.json"
  fi
  echo "[$TAG] apply $PROFILE $PF"
  apply_out=$(cmd "$BIN apply $PROFILE $PF --json" || true)
  if ! printf '%s' "$apply_out" | grep -q '"ok": *true'; then
    echo "[$TAG] APPLY FAILED"
    printf '%s\n' "$apply_out" | tail -5
    cmd "$BIN restore --json" >/dev/null 2>&1 || true
    exit 1
  fi
fi

if [ "$BUS" != "off" ]; then
  echo "[$TAG] bus=$BUS"
  cmd "sh $DIR/rnd_bus.sh $BUS" | tail -3
fi

echo "[$TAG] launch $SC"
case "$SC" in
  chrome)
    "$ADB" reverse tcp:8000 tcp:8000 >/dev/null
    "$ADB" shell am start -a android.intent.action.VIEW \
      -d "http://127.0.0.1:8000/scroll.html" \
      -n com.android.chrome/com.google.android.apps.chrome.Main >/dev/null 2>&1 || true
    ;;
  stress)
    "$ADB" reverse tcp:8000 tcp:8000 >/dev/null
    "$ADB" shell am start -a android.intent.action.VIEW \
      -d "http://127.0.0.1:8000/stress.html?n=${STRESS_N:-1800}" \
      -n com.android.chrome/com.google.android.apps.chrome.Main >/dev/null 2>&1 || true
    ;;
  game)
    "$ADB" shell am force-stop com.stove.epic7.google >/dev/null 2>&1 || true
    "$ADB" shell monkey -p com.stove.epic7.google -c android.intent.category.LAUNCHER 1 >/dev/null 2>&1 || true
    ;;
  video)
    "$ADB" shell am start -n com.mxtech.videoplayer.ad/.ActivityScreen \
      -d "content://media/external/video/media/1000002255" -t "video/*" >/dev/null 2>&1 || true
    ;;
  idle)
    "$ADB" shell input keyevent 3 >/dev/null 2>&1 || true
    ;;
esac
sleep "$WARMUP"

# A game launch can change the panel mode; force 60 Hz after it settles.
cmd "service call SurfaceFlinger 1035 i32 2" >/dev/null 2>&1 || true
"$ADB" shell settings put system user_refresh_rate 60
"$ADB" shell settings put system peak_refresh_rate 60
sleep 1

echo "[$TAG] sample ${DUR}s"
"$ADB" shell "su -c 'echo 1 > /sys/class/power_supply/battery/input_suspend'"
if [ -n "$LAYER" ]; then
  "$ADB" shell "su -c 'sh $DIR/rnd_frames.sh \"$LAYER\" $DUR $DIR/$TAG.frames'" >/dev/null 2>&1 &
  FRAMES_PID=$!
else
  FRAMES_PID=""
fi
cmd "sh $DIR/rnd_sampler.sh $DUR $DIR/$TAG.csv" >/dev/null 2>&1 || true
[ -n "$FRAMES_PID" ] && wait "$FRAMES_PID" || true
cmd "echo 0 > /sys/class/power_supply/battery/input_suspend" >/dev/null 2>&1 || true
sleep 1

if [ -n "$LAYER" ]; then
  cmd "cat $DIR/$TAG.frames" > "$RAW/$TAG.frames" 2>/dev/null || true
  cmd "rm -f $DIR/$TAG.frames" >/dev/null 2>&1 || true
fi
cmd "cat $DIR/$TAG.csv" > "$RAW/$TAG.csv" 2>/dev/null || true
cmd "rm -f $DIR/$TAG.csv" >/dev/null 2>&1 || true

if [ "$BUS" != "off" ]; then
  cmd "sh $DIR/rnd_bus.sh off" >/dev/null 2>&1 || true
fi
if [ "$PROFILE" != "-" ]; then
  echo "[$TAG] restore"
  restore_out=$(cmd "$BIN restore --json" || true)
  printf '%s' "$restore_out" | grep -q '"ok": *true' || echo "[$TAG] RESTORE NOT OK"
fi

echo "[$TAG] done"
python3 research/scripts/rnd_report.py "$RAW/$TAG.csv" 2>/dev/null || true
if [ -n "$LAYER" ]; then
  python3 research/scripts/rnd_frames_report.py "$RAW/$TAG.frames" 2>/dev/null || true
fi
