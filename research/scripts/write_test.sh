#!/system/bin/sh
# Controlled write test: policy0 scaling_min_freq (safe, restore after).
P=/sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq
old=$(cat "$P")
echo "old=$old"
echo 576000 > "$P" 2>&1
echo "write_rc=$?"
new=$(cat "$P")
echo "new=$new"
echo "$old" > "$P" 2>&1
echo "restore_rc=$?"
echo "restored=$(cat "$P")"
# A pure sysctl write test
V=/proc/sys/vm/stat_interval
oldv=$(cat "$V")
echo "stat_old=$oldv"
echo 2 > "$V" 2>&1
echo "stat_write_rc=$?"
echo "stat_new=$(cat "$V")"
echo "$oldv" > "$V" 2>&1
echo "stat_restored=$(cat "$V")"
