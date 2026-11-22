#!/usr/bin/env bash
# rnd_pair.sh PREFIX DUR SCENARIO PROFILE_A PROFILE_B [LAYER]
# Runs an A/B/B/A sequence (linear thermal drift cancels) with short runs.
#   env ORDER=ABBA (default) or ABAB
#   env VARIANT=/host/file.json (applies to both profiles)
#   env COOL=1 -> cool to <39 C before each run (slow, optional)
set -euo pipefail
PREFIX=$1
DUR=$2
SC=$3
A=$4
B=$5
LAYER=${6:-}
ORDER=${ORDER:-ABBA}
COOL=${COOL:-0}

cool() {
  [ "$COOL" = "1" ] || return 0
  local ADB=${ADB:-/home/fannndi/Android/Sdk/platform-tools/adb}
  "$ADB" shell input keyevent 26 >/dev/null 2>&1 || true
  local end=$(( $(date +%s) + 240 ))
  while [ "$(date +%s)" -lt "$end" ]; do
    local z t
    z=$("$ADB" shell "su -c 'grep -l quiet-therm-adc /sys/class/thermal/thermal_zone*/type'" 2>/dev/null | tr -d '\r' | head -1)
    if [ -n "$z" ]; then
      t=$("$ADB" shell "su -c 'cat ${z%/type}/temp'" 2>/dev/null | tr -d '\r')
      echo "  cool quiet=$t"
      [ -n "$t" ] && [ "$t" -lt 39000 ] 2>/dev/null && break
    fi
    sleep 20
  done
  "$ADB" shell input keyevent 224 >/dev/null 2>&1 || true
  "$ADB" shell wm dismiss-keyguard >/dev/null 2>&1 || true
}

i=0
for X in $(printf '%s' "$ORDER" | fold -w1); do
  i=$((i + 1))
  case "$X" in
    A) P=$A; V=${VARIANT_A:-${VARIANT:-}} ;;
    B) P=$B; V=${VARIANT_B:-${VARIANT:-}} ;;
    *) echo "bad ORDER=$ORDER"; exit 2 ;;
  esac
  cool
  echo "==== $PREFIX $X$i profile=$P variant=$V ===="
  VARIANT="$V" DUR="$DUR" research/scripts/rnd_run.sh "$PREFIX-$X$i" "$P" "$DUR" "$SC" "$LAYER"
  echo
done

python3 research/scripts/rnd_compare.py "$PREFIX"
