#!/system/bin/sh
# Controlled writes across catalog classes; each is restored immediately.
check() {
  name=$1; path=$2; want=$3
  old=$(cat "$path" 2>/dev/null)
  echo "$name old=[$old]"
  echo "$want" > "$path" 2>/dev/null
  rc=$?
  now=$(cat "$path" 2>/dev/null)
  echo "$name write_rc=$rc now=[$now]"
  echo "$old" > "$path" 2>/dev/null
  echo "$name restored=[$(cat "$path" 2>/dev/null)]"
}
check stune_boost /dev/stune/top-app/schedtune.boost 1
check cpuset_bg /dev/cpuset/background/cpus 0-3
check sched_up /proc/sys/kernel/sched_upmigrate 72
check gpu_max /sys/class/kgsl/kgsl-3d0/max_pwrlevel 1
check core_ctl_busy /sys/devices/system/cpu/cpu0/core_ctl/busy_up_thres 65
check io_sched /sys/block/sda/queue/scheduler deadline
