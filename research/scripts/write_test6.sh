#!/system/bin/sh
for P in /sys/devices/system/cpu/cpufreq/policy0 /sys/devices/system/cpu/cpufreq/policy6; do
  echo "=== $P ==="
  oldmax=$(cat "$P/scaling_max_freq")
  oldmin=$(cat "$P/scaling_min_freq")
  echo "min=$oldmin max=$oldmax"
  case "$P" in
    *policy0) want=1612800 ;;
    *policy6) want=1843200 ;;
  esac
  echo "$want" > "$P/scaling_max_freq"
  echo "rc=$? after=$(cat "$P/scaling_max_freq") cur=$(cat "$P/scaling_cur_freq")"
  echo "$oldmax" > "$P/scaling_max_freq"
  echo "restored=$(cat "$P/scaling_max_freq")"
done
