#!/system/bin/sh
# rnd_frames.sh LAYER_SUBSTR DUR_SECONDS OUT_FILE
# Samples `dumpsys SurfaceFlinger --latency` once per second (128-frame buffer
# at 60 fps covers >2 s, so 1 s polling is lossless).
SUB="$1"
DUR="$2"
OUT="$3"
L=""
END=$(( $(date +%s) + DUR ))
: > "$OUT"
while [ "$(date +%s)" -lt "$END" ]; do
  if [ -z "$L" ]; then
    L=$(dumpsys SurfaceFlinger --list 2>/dev/null | tr -d '\r' | grep -F "$SUB" | grep -v -E 'ActivityRecord|Background for|Bounds for' | head -1)
    echo "# layer=$L" >> "$OUT"
  fi
  if [ -n "$L" ]; then
    dumpsys SurfaceFlinger --latency "$L" 2>/dev/null | tail -n +2 >> "$OUT"
  fi
  sleep 1
done
