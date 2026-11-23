#!/system/bin/sh
# WALT / scheduler node audit for MIUI 14 surya.
#
# Read-only by default. Run on device as root:
#   su -c 'sh /data/local/tmp/audit_walt.sh'
#
# Candidate nodes come from /vendor/bin/init.qcom.post_boot.sh (the
# msmsteppe/sdmmagpie branches). We only propose profile values after this
# audit + an A/B run; see research/SCHEDULER-MIUI14.md.

echo "=== WALT GLOBAL NODES ==="
for node in \
  sched_restrict_tasks_spread \
  sched_small_task \
  sched_window_stats_policy \
  sched_ravg_hist_size \
  sched_upmigrate \
  sched_downmigrate \
  sched_group_upmigrate \
  sched_group_downmigrate \
  sched_migration_cost_ns \
  sched_min_granularity_ns \
  sched_latency_ns \
  sched_nr_migrate \
  sched_little_cluster_coloc_fmin_khz \
  sched_min_task_util_for_boost \
  sched_min_task_util_for_colocation \
  sched_prefer_idle \
  sched_prefer_spread \
  sched_restrict_cluster_spill \
  sched_spill_nr_run \
  sched_upmigrate_min_nice \
  sched_freq_aggregate \
  sched_init_task_load \
  sched_initial_task_util \
  sched_task_unfilter_nr_windows \
  sched_walt_rotate_big_tasks \
  sched_sync_hint_enable \
  sched_many_wakeup_threshold \
  walt_low_latency_task_threshold \
  walt_rtg_cfs_boost_prio
do
  p="/proc/sys/kernel/$node"
  printf '%s: ' "$p"
  if [ -f "$p" ]; then
    ls -l "$p" | awk '{print $1, $3":"$4}' | tr '\n' ' '
    cat "$p"
  else
    echo "missing"
  fi
done

echo
echo "=== PER-CPU NODES ==="
for cpu in 0 1 2 3 4 5 6 7; do
  for node in sched_mostly_idle_load sched_mostly_idle_nr_run sched_prefer_idle sched_load_boost; do
    p="/sys/devices/system/cpu/cpu$cpu/$node"
    printf '%s: ' "$p"
    if [ -f "$p" ]; then
      cat "$p"
    else
      echo "missing"
    fi
  done
done

echo
echo "=== POST_BOOT BASELINE (surya/sm6150) ==="
POST_BOOT=""
for p in /vendor/bin/init.qcom.post_boot.sh /system/bin/init.qcom.post_boot.sh /vendor/etc/init.qcom.post_boot.sh; do
  if [ -f "$p" ]; then POST_BOOT="$p"; break; fi
done
echo "source: ${POST_BOOT:-not found}"
if [ -n "$POST_BOOT" ]; then
  grep -n -E 'sched_(restrict_tasks_spread|small_task|window_stats_policy|ravg_hist_size|mostly_idle|prefer_idle|upmigrate|downmigrate)' "$POST_BOOT" | head -80
fi

echo
echo "=== PERF HAL OVERLAP ==="
# These are the WALT nodes also written by vendor.qti.hardware.perf@2.2.
# Baseline means the profile may write, but the framework can win.
for node in sched_migration_cost_ns sched_upmigrate sched_downmigrate \
            sched_group_upmigrate sched_group_downmigrate sched_min_granularity_ns \
            sched_min_task_util_for_boost sched_min_task_util_for_colocation \
            sched_many_wakeup_threshold sched_sync_hint_enable \
            sched_little_cluster_coloc_fmin_khz; do
  printf 'perf-overlap %s\n' "$node"
done

echo
echo "=== DONE ==="
