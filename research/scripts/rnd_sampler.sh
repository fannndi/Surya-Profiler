#!/system/bin/sh
# rnd_sampler.sh DUR_SECONDS OUT_CSV
# READ-ONLY 1 Hz sampler for battery/perf A/B runs (research R&D).
# STEP env = seconds between samples (default 1; use 5 for screen-off runs).
# Fields: epoch_s, batt_uA (+ = discharge), batt_uV, cap, batt_temp_dC,
# charge_counter_uAh, quiet_mC, nvm_mC, p0_cur, p0_max, p6_cur, p6_max,
# p0_active, p6_active, gpu_busy, gpu_freq, bypass, msm_min, sched_boost
DUR=$1
OUT=$2
P0=/sys/devices/system/cpu/cpufreq/policy0
P6=/sys/devices/system/cpu/cpufreq/policy6
BAT=/sys/class/power_supply/battery
GPU=/sys/class/kgsl/kgsl-3d0
GPUD=/sys/class/devfreq/5000000.qcom,kgsl-3d0
CC0=/sys/devices/system/cpu/cpu0/core_ctl
CC6=/sys/devices/system/cpu/cpu6/core_ctl
QZ=""; NZ=""
for z in /sys/class/thermal/thermal_zone*; do
  t=$(cat "$z/type" 2>/dev/null)
  case "$t" in
    quiet-therm-adc) QZ=$z ;;
    nvm-therm-adc) NZ=$z ;;
  esac
done
STEP=${STEP:-1}
MSM=/sys/module/msm_performance/parameters/cpu_min_freq
echo "t,batt_uA,batt_uV,cap,batt_temp_dC,charge_counter_uAh,quiet_mC,nvm_mC,p0_cur,p0_max,p6_cur,p6_max,p0_active,p6_active,gpu_busy,gpu_freq,bypass,msm_min,sched_boost" > "$OUT"
END=$(($(date +%s) + DUR))
while [ "$(date +%s)" -lt "$END" ]; do
  t=$(date +%s)
  echo "$t,$(cat $BAT/current_now),$(cat $BAT/voltage_now),$(cat $BAT/capacity),$(cat $BAT/temp),$(cat $BAT/charge_counter 2>/dev/null || echo 0),$(cat $QZ/temp),$(cat $NZ/temp),$(cat $P0/scaling_cur_freq),$(cat $P0/scaling_max_freq),$(cat $P6/scaling_cur_freq),$(cat $P6/scaling_max_freq),$(cat $CC0/active_cpus),$(cat $CC6/active_cpus),$(cat $GPU/gpu_busy_percentage),$(cat $GPUD/cur_freq),$(cat $BAT/input_suspend),$(cat $MSM 2>/dev/null),$(cat /proc/sys/kernel/sched_boost 2>/dev/null)" >> "$OUT"
  sleep "$STEP"
done
