#!/system/bin/sh
echo "KEY|PATH|EXISTS|PERM|OWNER|VALUE"
p="/sys/devices/system/cpu/cpufreq/policy0/scaling_governor"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy0.scaling_governor" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy0.scaling_governor" "$p"; fi
p="/sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy0.scaling_min_freq" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy0.scaling_min_freq" "$p"; fi
p="/sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy0.scaling_max_freq" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy0.scaling_max_freq" "$p"; fi
p="/sys/devices/system/cpu/cpufreq/policy0/schedutil/hispeed_freq"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy0.schedutil.hispeed_freq" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy0.schedutil.hispeed_freq" "$p"; fi
p="/sys/devices/system/cpu/cpufreq/policy0/schedutil/hispeed_load"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy0.schedutil.hispeed_load" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy0.schedutil.hispeed_load" "$p"; fi
p="/sys/devices/system/cpu/cpufreq/policy0/schedutil/up_rate_limit_us"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy0.schedutil.up_rate_limit_us" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy0.schedutil.up_rate_limit_us" "$p"; fi
p="/sys/devices/system/cpu/cpufreq/policy0/schedutil/down_rate_limit_us"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy0.schedutil.down_rate_limit_us" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy0.schedutil.down_rate_limit_us" "$p"; fi
p="/sys/devices/system/cpu/cpufreq/policy0/schedutil/pl"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy0.schedutil.pl" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy0.schedutil.pl" "$p"; fi
p="/sys/devices/system/cpu/cpu0/core_ctl/min_cpus"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy0.core_ctl.min_cpus" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy0.core_ctl.min_cpus" "$p"; fi
p="/sys/devices/system/cpu/cpu0/core_ctl/task_thres"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy0.core_ctl.task_thres" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy0.core_ctl.task_thres" "$p"; fi
p="/sys/devices/system/cpu/cpu0/core_ctl/busy_up_thres"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy0.core_ctl.busy_up_thres" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy0.core_ctl.busy_up_thres" "$p"; fi
p="/sys/devices/system/cpu/cpu0/core_ctl/busy_down_thres"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy0.core_ctl.busy_down_thres" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy0.core_ctl.busy_down_thres" "$p"; fi
p="/sys/devices/system/cpu/cpu0/core_ctl/max_cpus"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy0.core_ctl.max_cpus" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy0.core_ctl.max_cpus" "$p"; fi
p="/sys/devices/system/cpu/cpu0/core_ctl/offline_delay_ms"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy0.core_ctl.offline_delay_ms" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy0.core_ctl.offline_delay_ms" "$p"; fi
p="/sys/devices/system/cpu/cpufreq/policy6/scaling_governor"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy6.scaling_governor" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy6.scaling_governor" "$p"; fi
p="/sys/devices/system/cpu/cpufreq/policy6/scaling_min_freq"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy6.scaling_min_freq" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy6.scaling_min_freq" "$p"; fi
p="/sys/devices/system/cpu/cpufreq/policy6/scaling_max_freq"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy6.scaling_max_freq" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy6.scaling_max_freq" "$p"; fi
p="/sys/devices/system/cpu/cpufreq/policy6/schedutil/hispeed_freq"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy6.schedutil.hispeed_freq" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy6.schedutil.hispeed_freq" "$p"; fi
p="/sys/devices/system/cpu/cpufreq/policy6/schedutil/hispeed_load"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy6.schedutil.hispeed_load" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy6.schedutil.hispeed_load" "$p"; fi
p="/sys/devices/system/cpu/cpufreq/policy6/schedutil/up_rate_limit_us"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy6.schedutil.up_rate_limit_us" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy6.schedutil.up_rate_limit_us" "$p"; fi
p="/sys/devices/system/cpu/cpufreq/policy6/schedutil/down_rate_limit_us"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy6.schedutil.down_rate_limit_us" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy6.schedutil.down_rate_limit_us" "$p"; fi
p="/sys/devices/system/cpu/cpufreq/policy6/schedutil/pl"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy6.schedutil.pl" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy6.schedutil.pl" "$p"; fi
p="/sys/devices/system/cpu/cpu6/core_ctl/min_cpus"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy6.core_ctl.min_cpus" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy6.core_ctl.min_cpus" "$p"; fi
p="/sys/devices/system/cpu/cpu6/core_ctl/task_thres"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "policy6.core_ctl.task_thres" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "policy6.core_ctl.task_thres" "$p"; fi
p="/sys/class/kgsl/kgsl-3d0/devfreq/governor"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "gpu.governor" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "gpu.governor" "$p"; fi
p="/sys/class/kgsl/kgsl-3d0/max_pwrlevel"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "gpu.max_pwrlevel" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "gpu.max_pwrlevel" "$p"; fi
p="/sys/class/kgsl/kgsl-3d0/min_pwrlevel"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "gpu.min_pwrlevel" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "gpu.min_pwrlevel" "$p"; fi
p="/sys/class/kgsl/kgsl-3d0/default_pwrlevel"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "gpu.default_pwrlevel" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "gpu.default_pwrlevel" "$p"; fi
p="/sys/block/sda/queue/scheduler"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.scheduler" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.scheduler" "$p"; fi
p="/sys/block/sda/queue/nr_requests"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.nr_requests" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.nr_requests" "$p"; fi
p="/sys/block/sda/queue/nomerges"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.nomerges" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.nomerges" "$p"; fi
p="/sys/block/sda/queue/rq_affinity"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.rq_affinity" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.rq_affinity" "$p"; fi
p="/sys/block/sda/queue/iostats"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.iostats" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.iostats" "$p"; fi
p="/sys/block/sda/queue/iosched/quantum"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.cfq.quantum" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.cfq.quantum" "$p"; fi
p="/sys/block/sda/queue/iosched/fifo_expire_sync"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.cfq.fifo_expire_sync" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.cfq.fifo_expire_sync" "$p"; fi
p="/sys/block/sda/queue/iosched/fifo_expire_async"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.cfq.fifo_expire_async" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.cfq.fifo_expire_async" "$p"; fi
p="/sys/block/sda/queue/iosched/back_seek_max"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.cfq.back_seek_max" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.cfq.back_seek_max" "$p"; fi
p="/sys/block/sda/queue/iosched/slice_sync"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.cfq.slice_sync" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.cfq.slice_sync" "$p"; fi
p="/sys/block/sda/queue/iosched/slice_async"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.cfq.slice_async" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.cfq.slice_async" "$p"; fi
p="/sys/block/sda/queue/iosched/slice_idle_us"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.cfq.slice_idle_us" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.cfq.slice_idle_us" "$p"; fi
p="/sys/block/sda/queue/iosched/target_latency_us"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.cfq.target_latency_us" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.cfq.target_latency_us" "$p"; fi
p="/sys/block/sda/queue/iosched/group_idle"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.cfq.group_idle" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.cfq.group_idle" "$p"; fi
p="/sys/block/sda/queue/iosched/low_latency"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.cfq.low_latency" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.cfq.low_latency" "$p"; fi
p="/sys/block/sda/queue/read_ahead_kb"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "io.read_ahead_kb" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "io.read_ahead_kb" "$p"; fi
p="/sys/class/devfreq/soc:qcom,cpu0-cpu-l3-lat/min_freq"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "devfreq.cpu0_l3_lat.min_freq" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "devfreq.cpu0_l3_lat.min_freq" "$p"; fi
p="/sys/class/devfreq/soc:qcom,cpu6-cpu-l3-lat/min_freq"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "devfreq.cpu6_l3_lat.min_freq" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "devfreq.cpu6_l3_lat.min_freq" "$p"; fi
p="/sys/fs/f2fs/sda16/gc_urgent"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "f2fs.gc_urgent" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "f2fs.gc_urgent" "$p"; fi
p="/sys/fs/f2fs/sda16/gc_urgent_sleep_time"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "f2fs.gc_urgent_sleep_time" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "f2fs.gc_urgent_sleep_time" "$p"; fi
p="/sys/class/power_supply/battery/battery_charging_enabled"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "charge.battery_charging_enabled" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "charge.battery_charging_enabled" "$p"; fi
p="/sys/class/power_supply/battery/input_suspend"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "charge.input_suspend" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "charge.input_suspend" "$p"; fi
p="/proc/sys/kernel/sched_conservative_pl"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_conservative_pl" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_conservative_pl" "$p"; fi
p="/proc/sys/kernel/sched_latency_ns"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_latency_ns" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_latency_ns" "$p"; fi
p="/proc/sys/kernel/sched_min_granularity_ns"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_min_granularity_ns" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_min_granularity_ns" "$p"; fi
p="/proc/sys/kernel/sched_wakeup_granularity_ns"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_wakeup_granularity_ns" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_wakeup_granularity_ns" "$p"; fi
p="/proc/sys/kernel/sched_migration_cost_ns"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_migration_cost_ns" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_migration_cost_ns" "$p"; fi
p="/proc/sys/kernel/sched_nr_migrate"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_nr_migrate" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_nr_migrate" "$p"; fi
p="/proc/sys/kernel/sched_autogroup_enabled"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_autogroup_enabled" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_autogroup_enabled" "$p"; fi
p="/proc/sys/kernel/sched_child_runs_first"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_child_runs_first" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_child_runs_first" "$p"; fi
p="/proc/sys/kernel/sched_walt_rotate_big_tasks"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_walt_rotate_big_tasks" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_walt_rotate_big_tasks" "$p"; fi
p="/proc/sys/kernel/sched_little_cluster_coloc_fmin_khz"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_little_cluster_coloc_fmin_khz" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_little_cluster_coloc_fmin_khz" "$p"; fi
p="/proc/sys/kernel/sched_upmigrate"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_upmigrate" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_upmigrate" "$p"; fi
p="/proc/sys/kernel/sched_downmigrate"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_downmigrate" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_downmigrate" "$p"; fi
p="/proc/sys/kernel/sched_group_upmigrate"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_group_upmigrate" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_group_upmigrate" "$p"; fi
p="/proc/sys/kernel/sched_group_downmigrate"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_group_downmigrate" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_group_downmigrate" "$p"; fi
p="/proc/sys/kernel/sched_min_task_util_for_boost"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_min_task_util_for_boost" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_min_task_util_for_boost" "$p"; fi
p="/proc/sys/kernel/sched_min_task_util_for_colocation"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_min_task_util_for_colocation" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_min_task_util_for_colocation" "$p"; fi
p="/proc/sys/kernel/sched_sync_hint_enable"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_sync_hint_enable" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_sync_hint_enable" "$p"; fi
p="/proc/sys/kernel/sched_many_wakeup_threshold"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_many_wakeup_threshold" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_many_wakeup_threshold" "$p"; fi
p="/proc/sys/kernel/sched_time_avg_ms"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_time_avg_ms" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_time_avg_ms" "$p"; fi
p="/proc/sys/kernel/timer_migration"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.timer_migration" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.timer_migration" "$p"; fi
p="/proc/sys/kernel/sched_rr_timeslice_ms"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "kernel.sched_rr_timeslice_ms" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "kernel.sched_rr_timeslice_ms" "$p"; fi
p="/proc/sys/vm/vfs_cache_pressure"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "vm.vfs_cache_pressure" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "vm.vfs_cache_pressure" "$p"; fi
p="/proc/sys/vm/dirty_ratio"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "vm.dirty_ratio" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "vm.dirty_ratio" "$p"; fi
p="/proc/sys/vm/dirty_background_ratio"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "vm.dirty_background_ratio" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "vm.dirty_background_ratio" "$p"; fi
p="/proc/sys/vm/dirty_expire_centisecs"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "vm.dirty_expire_centisecs" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "vm.dirty_expire_centisecs" "$p"; fi
p="/proc/sys/vm/dirty_writeback_centisecs"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "vm.dirty_writeback_centisecs" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "vm.dirty_writeback_centisecs" "$p"; fi
p="/proc/sys/vm/stat_interval"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "vm.stat_interval" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "vm.stat_interval" "$p"; fi
p="/proc/sys/vm/watermark_scale_factor"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "vm.watermark_scale_factor" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "vm.watermark_scale_factor" "$p"; fi
p="/proc/sys/vm/extra_free_kbytes"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "vm.extra_free_kbytes" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "vm.extra_free_kbytes" "$p"; fi
p="/proc/sys/net/ipv4/tcp_congestion_control"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "net.tcp_congestion_control" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "net.tcp_congestion_control" "$p"; fi
p="/proc/sys/net/ipv4/tcp_rmem"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "net.tcp_rmem" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "net.tcp_rmem" "$p"; fi
p="/proc/sys/net/ipv4/tcp_wmem"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "net.tcp_wmem" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "net.tcp_wmem" "$p"; fi
p="/proc/sys/net/ipv4/tcp_slow_start_after_idle"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "net.tcp_slow_start_after_idle" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "net.tcp_slow_start_after_idle" "$p"; fi
p="/sys/module/cpu_boost/parameters/input_boost_freq"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "cpu_boost.input_boost_freq" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "cpu_boost.input_boost_freq" "$p"; fi
p="/proc/sys/net/ipv4/tcp_mtu_probing"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "net.tcp_mtu_probing" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "net.tcp_mtu_probing" "$p"; fi
p="/proc/sys/net/ipv4/tcp_fin_timeout"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "net.tcp_fin_timeout" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "net.tcp_fin_timeout" "$p"; fi
p="/proc/sys/net/ipv4/tcp_fastopen"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "net.tcp_fastopen" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "net.tcp_fastopen" "$p"; fi
p="/dev/stune/top-app/schedtune.boost"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "stune.top-app.boost" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "stune.top-app.boost" "$p"; fi
p="/dev/stune/top-app/schedtune.prefer_idle"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "stune.top-app.prefer_idle" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "stune.top-app.prefer_idle" "$p"; fi
p="/dev/stune/foreground/schedtune.boost"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "stune.foreground.boost" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "stune.foreground.boost" "$p"; fi
p="/dev/stune/foreground/schedtune.prefer_idle"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "stune.foreground.prefer_idle" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "stune.foreground.prefer_idle" "$p"; fi
p="/dev/cpuset/background/cpus"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "cpuset.background.cpus" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "cpuset.background.cpus" "$p"; fi
p="/dev/cpuset/system-background/cpus"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "cpuset.system-background.cpus" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "cpuset.system-background.cpus" "$p"; fi
p="/dev/cpuset/foreground/cpus"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "cpuset.foreground.cpus" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "cpuset.foreground.cpus" "$p"; fi
p="/dev/cpuset/top-app/cpus"
if [ -e "$p" ]; then printf "%s|%s|yes|%s|%s|%s\n" "cpuset.top-app.cpus" "$p" "$(stat -c %A "$p" 2>/dev/null)" "$(stat -c %U:%G "$p" 2>/dev/null)" "$(cat "$p" 2>/dev/null | tr "\n" " ")"; else printf "%s|%s|no|-|-|-\n" "cpuset.top-app.cpus" "$p"; fi
