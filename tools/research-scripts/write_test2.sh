#!/system/bin/sh
# Controlled write tests for freq caps + governor on policy0/policy6.
for P in /sys/devices/system/cpu/cpufreq/policy0 /sys/devices/system/cpu/cpufreq/policy6; do
  echo "=== $P ==="
  oldmax=$(cat "$P/scaling_max_freq")
  oldmin=$(cat "$P/scaling_min_freq")
  oldgov=$(cat "$P/scaling_governor")
  echo "old governor=$oldgov min=$oldmin max=$oldmax cur=$(cat "$P/scaling_cur_freq")"
  case "$P" in
    *policy0) want_max=1248000 ;;
    *policy6) want_max=1324800 ;;
  esac
  echo "$want_max" > "$P/scaling_max_freq" 2>&1
  echo "max_write_rc=$?"
  echo "max_after=$(cat "$P/scaling_max_freq")"
  echo "cur_after_max=$(cat "$P/scaling_cur_freq")"
  echo "$oldmax" > "$P/scaling_max_freq" 2>&1
  echo "max_restored=$(cat "$P/scaling_max_freq")"
  echo "powersave" > "$P/scaling_governor" 2>&1
  echo "gov_write_rc=$?"
  sleep 2
  echo "gov_after=$(cat "$P/scaling_governor")"
  echo "cur_powersave=$(cat "$P/scaling_cur_freq")"
  echo "$oldgov" > "$P/scaling_governor" 2>&1
  echo "gov_restored=$(cat "$P/scaling_governor")"
  sleep 1
  echo "cur_final=$(cat "$P/scaling_cur_freq")"
done
