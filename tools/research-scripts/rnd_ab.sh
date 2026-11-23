#!/usr/bin/env bash
# rnd_ab.sh TAG PROFILE DUR SCENARIO [PKG]
# One measured A/B run: apply profile (CLI, service off), run stimulus +
# 1 Hz sampler on battery bypass, capture gfxinfo, restore.
#   PROFILE "-" = stock (nothing applied, no restore)
#   env VARIANT=/host/file.json  -> push as variant profiles file, apply with it
#   env NOCOOLDOWN=1             -> skip pre-run temp wait
set -euo pipefail
ADB=${ADB:-/home/<user>/Android/Sdk/platform-tools/adb}
TAG=$1; PROFILE=$2; DUR=$3; SC=$4; PKG=${5:-}
DIR=/data/local/tmp/surya_rnd
BIN=/data/local/tmp/suryaprofiler/surya-prof
RAW=research/raw/rnd
mkdir -p "$RAW"
S=/home/<user>/Documents/Surya-Profiler/research/scripts

sh() { "$ADB" shell su -c sh; }
# NOTE: adb joins args, so the command string must be single-quoted on the
# remote side for su to receive it whole (redirections then run as root).
cmd() { "$ADB" shell "su -c '$1'"; }

# 0. ensure harness on device
$ADB push -q "$S/rnd_sampler.sh" "$DIR/rnd_sampler.sh" >/dev/null
$ADB push -q "$S/rnd_stim.sh" "$DIR/rnd_stim.sh" >/dev/null
cmd "chmod 755 $DIR/rnd_sampler.sh $DIR/rnd_stim.sh"

# 1. (screen-on scenarios wake inside the stimulus script, after apply —
#     waking before the cool-down used to re-lock the device mid-run)

# 2. optional cool-down to < 35C quiet-therm (Awake-guarded toggle; the
#    previous unconditional keyevent 26 woke an already-sleeping device)
if [ "${NOCOOLDOWN:-0}" != "1" ]; then
  echo "[$TAG] cooling to <35000 mC quiet..."
  $ADB shell su -c sh <<EOF
END=\$(( \$(date +%s) + 300 ))
while [ \$(date +%s) -lt \$END ]; do
  Q=\$(for z in /sys/class/thermal/thermal_zone*; do t=\$(cat \$z/type 2>/dev/null); [ "\$t" = quiet-therm-adc ] && cat \$z/temp; done | head -1)
  echo "quiet=\$Q"
  [ "\$Q" -lt 35000 ] 2>/dev/null && break
  AW=\$(dumpsys power | grep -m1 mWakefulness | grep -c Awake)
  if [ "\$AW" != "0" ]; then input keyevent 26; fi
  sleep 30
done
if [ "$SC" != "sleep" ]; then input keyevent 224; fi
EOF
fi

# 3. apply profile (single writer: daemon off)
if [ "$PROFILE" != "-" ]; then
  PF=""
  if [ -n "${VARIANT:-}" ]; then
    $ADB push -q "$VARIANT" "$DIR/profiles_variant.json" >/dev/null
    PF="--profiles $DIR/profiles_variant.json"
  fi
  echo "[$TAG] apply $PROFILE $PF"
  cmd "$BIN apply $PROFILE $PF --json" | tail -1
fi

# 4. start stimulus (except sleep: screen off instead)
case "$SC" in
  sleep) $ADB shell su -c "$DIR/rnd_stim.sh off" ;;
  *)
    $ADB shell su -c "nohup $DIR/rnd_stim.sh $SC $DUR >/dev/null 2>&1 &"
    sleep "${SETTLE:-10}"
    # ensure actually unlocked (keyguard must not sit on top of the app)
    $ADB shell su -c sh <<EOF
F=\$(dumpsys window | grep mCurrentFocus)
case "\$F" in *NotificationShade*) input keyevent 224; sleep 1; wm dismiss-keyguard; sleep 1; echo "re-unlocked";; esac
EOF
    ;;
esac

# 5. gfxinfo baseline
if [ -n "$PKG" ]; then
  $ADB shell su -c "dumpsys gfxinfo $PKG reset" > "$RAW/$TAG.gfxinfo.reset" 2>&1 || true
fi

# 6. bypass charging + sample (STEP=5 for screen-off runs: fewer wake pokes)
STEP_V=${STEP:-}
[ -z "$STEP_V" ] && [ "$SC" = "sleep" ] && STEP_V=5
STEP_V=${STEP_V:-1}
cmd "echo 1 > /sys/class/power_supply/battery/input_suspend"
cmd "STEP=$STEP_V $DIR/rnd_sampler.sh $DUR $DIR/$TAG.csv"
cmd "echo 0 > /sys/class/power_supply/battery/input_suspend"

# 7. gfxinfo capture
if [ -n "$PKG" ]; then
  $ADB shell su -c "dumpsys gfxinfo $PKG" > "$RAW/$TAG.gfxinfo" 2>&1 || true
fi

# 8. collect
cmd "cat $DIR/$TAG.csv" > "$RAW/$TAG.csv"
$ADB shell su -c "rm -f $DIR/$TAG.csv"
if [ "$PROFILE" != "-" ]; then
  cmd "$BIN restore --json" | tail -1
fi
case "$SC" in
  sleep) ;;
  *) $ADB shell su -c "$DIR/rnd_stim.sh stop" ;;
esac
echo "[$TAG] done -> $RAW/$TAG.csv"
