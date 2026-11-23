#!/system/bin/sh
# Read-only MIUI 14 / Android 12 inventory: scheduler, QoS, thermal, power, display.

echo "=== POLICY0 TREE ==="
ls -la /sys/devices/system/cpu/cpufreq/policy0 2>&1
echo "=== POLICY6 TREE ==="
ls -la /sys/devices/system/cpu/cpufreq/policy6 2>&1
echo "=== POLICY0/SCHEDUTIL ==="
ls -la /sys/devices/system/cpu/cpufreq/policy0/schedutil 2>&1
for f in /sys/devices/system/cpu/cpufreq/policy0/schedutil/*; do echo "$f: $(cat "$f" 2>&1)"; done
echo "=== POLICY6/SCHEDUTIL ==="
ls -la /sys/devices/system/cpu/cpufreq/policy6/schedutil 2>&1
for f in /sys/devices/system/cpu/cpufreq/policy6/schedutil/*; do echo "$f: $(cat "$f" 2>&1)"; done
echo "=== CORE_CTL LOCATIONS ==="
find /sys/devices/system/cpu -name '*core_ctl*' -o -name 'core_ctl' 2>&1
ls -la /sys/devices/system/cpu/cpu0/core_ctl 2>&1
ls -la /sys/devices/system/cpu/cpu6/core_ctl 2>&1

echo "=== MSM_PERFORMANCE ==="
ls -la /sys/module/msm_performance/parameters 2>&1
for f in /sys/module/msm_performance/parameters/*; do echo "$f: $(cat "$f" 2>&1)"; done

echo "=== SCHED TUNING ==="
for f in /proc/sys/kernel/sched_upmigrate /proc/sys/kernel/sched_downmigrate /proc/sys/kernel/sched_group_upmigrate /proc/sys/kernel/sched_group_downmigrate /proc/sys/kernel/sched_little_cluster_coloc_fmin_khz /proc/sys/kernel/sched_walt_rotate_big_tasks /proc/sys/kernel/sched_min_task_util_for_boost /proc/sys/kernel/sched_min_task_util_for_colocation /proc/sys/kernel/sched_many_wakeup_threshold /proc/sys/kernel/sched_sync_hint_enable /proc/sys/kernel/sched_conservative_pl /proc/sys/kernel/sched_boost /proc/sys/kernel/sched_migration_cost_ns /proc/sys/kernel/sched_latency_ns /proc/sys/kernel/sched_min_granularity_ns /proc/sys/kernel/sched_wakeup_granularity_ns /proc/sys/kernel/sched_nr_migrate /proc/sys/kernel/timer_migration /proc/sys/kernel/sched_rr_timeslice_ms; do
  if [ -e "$f" ]; then echo "$f: $(cat "$f" 2>&1)"; else echo "$f: MISSING"; fi
done

echo "=== THERMAL ZONES ==="
for z in /sys/class/thermal/thermal_zone*; do
  echo "--- $z"
  echo "type: $(cat "$z/type" 2>&1)"
  echo "temp: $(cat "$z/temp" 2>&1)"
  echo "mode: $(cat "$z/mode" 2>&1)"
done
echo "=== COOLING DEVICES ==="
for c in /sys/class/thermal/cooling_device*; do
  echo "--- $c"
  echo "type: $(cat "$c/type" 2>&1)"
  echo "cur_state: $(cat "$c/cur_state" 2>&1)"
  echo "max_state: $(cat "$c/max_state" 2>&1)"
done
echo "=== THERMAL MESSAGE ==="
ls -la /sys/class/thermal/thermal_message 2>&1
for f in /sys/class/thermal/thermal_message/*; do echo "$f: $(cat "$f" 2>&1)"; done

echo "=== POWER SUPPLY ==="
for ps in /sys/class/power_supply/*; do
  echo "--- $ps"
  for f in type status present online current_now voltage_now capacity charge_full charge_full_design cycle_count health temp input_suspend battery_charging_enabled charging_enabled charge_control_limit charge_control_limit_max constant_charge_current constant_charge_current_max charge_term_current force_recharge parallel_disable; do
    if [ -e "$ps/$f" ]; then echo "$f: $(cat "$ps/$f" 2>&1)"; fi
  done
done

echo "=== BATTERY STATS ==="
dumpsys battery 2>&1

echo "=== POWER SETTINGS ==="
settings get global low_power 2>&1
settings get global low_power_sticky 2>&1
settings get system power_mode 2>&1
settings get system user_refresh_rate 2>&1
settings get system peak_refresh_rate 2>&1
settings get system min_refresh_rate 2>&1
getprop persist.sys.aries.power_profile 2>&1
getprop persist.vendor.dfps.level 2>&1
getprop ro.vendor.dfps.level 2>&1

echo "=== DISPLAY ==="
dumpsys display 2>&1 | head -200
echo "=== SURFACEFLINGER DISPLAY ==="
dumpsys SurfaceFlinger --display-id 2>&1
dumpsys SurfaceFlinger 2>&1 | grep -i -E 'refresh|fps|mode|activeConfig' | head -100

echo "=== DONE ==="
