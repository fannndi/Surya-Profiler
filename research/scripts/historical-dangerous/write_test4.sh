#!/system/bin/sh
P=/sys/devices/system/cpu/cpufreq/policy0
echo "screen=$(cat /sys/class/thermal/thermal_message/screen_state)"
echo "before_min=$(cat "$P/scaling_min_freq") before_max=$(cat "$P/scaling_max_freq")"
echo 576000 > "$P/scaling_min_freq"
rc=$?
echo "min_write_rc=$rc"
echo "after_min=$(cat "$P/scaling_min_freq")"
sleep 1
echo "after_1s_min=$(cat "$P/scaling_min_freq")"
echo 1804800 > "$P/scaling_max_freq"
rc=$?
echo "max_write_rc=$rc"
echo "after_max=$(cat "$P/scaling_max_freq")"
echo 576000 > "$P/scaling_min_freq"
rc=$?
echo "min2_write_rc=$rc"
echo "after_min2=$(cat "$P/scaling_min_freq")"
