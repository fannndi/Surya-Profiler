#!/system/bin/sh
# Read-only system inventory for Surya-Profiler R&D.
# Run as root: su -c 'sh /data/local/tmp/audit_01_system.sh'

echo "=== IDENTITY ==="
id
getenforce
uname -a
getprop ro.build.display.id
getprop ro.build.version.release
getprop ro.build.version.sdk
getprop ro.miui.ui.version.name
getprop ro.build.fingerprint

echo "=== ROOT / APATCH ==="
ls -la /data/adb 2>&1
ls -la /data/adb/apatch 2>&1
ls -la /data/adb/modules 2>&1
getprop | grep -i -E 'apatch|kernelsu|magisk' 2>&1
ps -A 2>&1 | grep -i -E 'apatch|kpatch|ksud|magisk' 2>&1

echo "=== CGROUP MOUNTS ==="
mount 2>&1 | grep -i cgroup
echo "--- /dev/cpuset"
ls -la /dev/cpuset 2>&1
echo "--- /dev/stune"
ls -la /dev/stune 2>&1
echo "--- /sys/fs/cgroup"
ls -la /sys/fs/cgroup 2>&1
echo "--- /proc/cgroups"
cat /proc/cgroups 2>&1

echo "=== CPU TOPOLOGY ==="
cat /sys/devices/system/cpu/present 2>&1
cat /sys/devices/system/cpu/online 2>&1
cat /sys/devices/system/cpu/possible 2>&1
for c in /sys/devices/system/cpu/cpu[0-9]*; do
  echo "--- $c"
  echo "online: $(cat "$c/online" 2>&1)"
  echo "topology/core_id: $(cat "$c/topology/core_id" 2>&1)"
  echo "topology/physical_package_id: $(cat "$c/topology/physical_package_id" 2>&1)"
  echo "topology/cluster_id: $(cat "$c/topology/cluster_id" 2>&1)"
done

echo "=== CPU POLICIES ==="
for p in /sys/devices/system/cpu/cpufreq/policy*; do
  echo "--- $p"
  for f in affected_cpus related_cpus scaling_driver scaling_governor scaling_available_governors scaling_min_freq scaling_max_freq scaling_cur_freq cpuinfo_min_freq cpuinfo_max_freq scaling_available_frequencies; do
    if [ -e "$p/$f" ]; then
      echo "$f: $(cat "$p/$f" 2>&1)"
    fi
  done
  for f in hispeed_freq hispeed_load up_rate_limit_us down_rate_limit_us pl conservative_pl; do
    if [ -e "$p/schedutil/$f" ]; then
      echo "schedutil/$f: $(cat "$p/schedutil/$f" 2>&1)"
    fi
  done
  for f in min_cpus max_cpus busy_up_thres busy_down_thres offline_delay_ms task_thres not_preferred; do
    if [ -e "$p/core_ctl/$f" ]; then
      echo "core_ctl/$f: $(cat "$p/core_ctl/$f" 2>&1)"
    fi
  done
done

echo "=== CPU BOOST ==="
ls -la /sys/module/cpu_boost/parameters 2>&1
for f in /sys/module/cpu_boost/parameters/*; do
  echo "$f: $(cat "$f" 2>&1)"
done

echo "=== GPU KGSL ==="
ls -la /sys/class/kgsl/kgsl-3d0 2>&1
for f in gpuclk devfreq/cur_freq devfreq/min_freq devfreq/max_freq devfreq/available_frequencies devfreq/governor devfreq/available_governors min_pwrlevel max_pwrlevel default_pwrlevel num_pwrlevels idle_timer force_clk_on force_no_nap force_rail_on bus_split; do
  if [ -e "/sys/class/kgsl/kgsl-3d0/$f" ]; then
    echo "$f: $(cat "/sys/class/kgsl/kgsl-3d0/$f" 2>&1)"
  fi
done

echo "=== DEVFREQ ==="
for d in /sys/class/devfreq/*; do
  echo "--- $d"
  echo "governor: $(cat "$d/governor" 2>&1)"
  echo "available_governors: $(cat "$d/available_governors" 2>&1)"
  echo "cur_freq: $(cat "$d/cur_freq" 2>&1)"
  echo "min_freq: $(cat "$d/min_freq" 2>&1)"
  echo "max_freq: $(cat "$d/max_freq" 2>&1)"
  echo "available_frequencies: $(cat "$d/available_frequencies" 2>&1)"
done

echo "=== DONE ==="
