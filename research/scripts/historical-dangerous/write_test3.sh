#!/system/bin/sh
echo "=== context ==="
id
getenforce
echo "=== selinux labels ==="
ls -lZ /sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq /sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq /sys/devices/system/cpu/cpufreq/policy0/scaling_governor /proc/sys/vm/stat_interval 2>&1
echo "=== write min ==="
echo 576000 > /sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq
echo "rc=$?"
cat /sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq
echo "=== write max ==="
echo 1804800 > /sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq
echo "rc=$?"
cat /sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq
echo "=== write governor ==="
echo schedutil > /sys/devices/system/cpu/cpufreq/policy0/scaling_governor
echo "rc=$?"
cat /sys/devices/system/cpu/cpufreq/policy0/scaling_governor
echo "=== dmesg tail ==="
dmesg | tail -30
