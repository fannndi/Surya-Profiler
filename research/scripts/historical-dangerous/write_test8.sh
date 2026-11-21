#!/system/bin/sh
P=/sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq
echo "before=[$(cat $P)] min=[$(cat /sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq)]"
echo 1248000 > "$P"
echo "rc=$?"
echo "after=[$(cat $P)]"
echo 1804800 > "$P"
echo "restored=[$(cat $P)]"
