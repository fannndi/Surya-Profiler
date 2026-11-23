#!/system/bin/sh
# Sample scheduler/power state once per second.
N=${1:-60}
echo "t,screen,status,cap,min0,max0,cur0,min6,max6,cur6,sched_boost,cc0_min,cc0_max,cc0_active,cc6_min,cc6_max,cc6_active,gpu_pwr,gpu_freq,hs0,hl0,pl0,hs6,hl6,pl6,boost0,boost6"
i=0
while [ "$i" -lt "$N" ]; do
  t=$(date +%s)
  sc=$(cat /sys/class/thermal/thermal_message/screen_state 2>/dev/null)
  st=$(cat /sys/class/power_supply/battery/status 2>/dev/null)
  cap=$(cat /sys/class/power_supply/battery/capacity 2>/dev/null)
  min0=$(cat /sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq 2>/dev/null)
  max0=$(cat /sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq 2>/dev/null)
  cur0=$(cat /sys/devices/system/cpu/cpufreq/policy0/scaling_cur_freq 2>/dev/null)
  min6=$(cat /sys/devices/system/cpu/cpufreq/policy6/scaling_min_freq 2>/dev/null)
  max6=$(cat /sys/devices/system/cpu/cpufreq/policy6/scaling_max_freq 2>/dev/null)
  cur6=$(cat /sys/devices/system/cpu/cpufreq/policy6/scaling_cur_freq 2>/dev/null)
  sb=$(cat /proc/sys/kernel/sched_boost 2>/dev/null)
  c0min=$(cat /sys/devices/system/cpu/cpu0/core_ctl/min_cpus 2>/dev/null)
  c0max=$(cat /sys/devices/system/cpu/cpu0/core_ctl/max_cpus 2>/dev/null)
  c0act=$(cat /sys/devices/system/cpu/cpu0/core_ctl/active_cpus 2>/dev/null)
  c6min=$(cat /sys/devices/system/cpu/cpu6/core_ctl/min_cpus 2>/dev/null)
  c6max=$(cat /sys/devices/system/cpu/cpu6/core_ctl/max_cpus 2>/dev/null)
  c6act=$(cat /sys/devices/system/cpu/cpu6/core_ctl/active_cpus 2>/dev/null)
  gp=$(cat /sys/class/kgsl/kgsl-3d0/max_pwrlevel 2>/dev/null)
  gf=$(cat /sys/class/kgsl/kgsl-3d0/gpuclk 2>/dev/null)
  hs0=$(cat /sys/devices/system/cpu/cpufreq/policy0/schedutil/hispeed_freq 2>/dev/null)
  hl0=$(cat /sys/devices/system/cpu/cpufreq/policy0/schedutil/hispeed_load 2>/dev/null)
  pl0=$(cat /sys/devices/system/cpu/cpufreq/policy0/schedutil/pl 2>/dev/null)
  hs6=$(cat /sys/devices/system/cpu/cpufreq/policy6/schedutil/hispeed_freq 2>/dev/null)
  hl6=$(cat /sys/devices/system/cpu/cpufreq/policy6/schedutil/hispeed_load 2>/dev/null)
  pl6=$(cat /sys/devices/system/cpu/cpufreq/policy6/schedutil/pl 2>/dev/null)
  b0=$(cat /sys/devices/system/cpu/cpu0/sched_load_boost 2>/dev/null)
  b6=$(cat /sys/devices/system/cpu/cpu6/sched_load_boost 2>/dev/null)
  echo "$t,$sc,$st,$cap,$min0,$max0,$cur0,$min6,$max6,$cur6,$sb,$c0min,$c0max,$c0act,$c6min,$c6max,$c6act,$gp,$gf,$hs0,$hl0,$pl0,$hs6,$hl6,$pl6,$b0,$b6"
  sleep 1
  i=$((i+1))
done
