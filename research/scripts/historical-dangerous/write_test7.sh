#!/system/bin/sh
P=/sys/devices/system/cpu/cpufreq/policy6/scaling_max_freq
echo "before=[$(cat $P)] min=[$(cat /sys/devices/system/cpu/cpufreq/policy6/scaling_min_freq)]"
echo 1324800 > "$P"
echo "rc=$?"
echo "after=[$(cat $P)]"
echo 2304000 > "$P"
echo "restored=[$(cat $P)]"
