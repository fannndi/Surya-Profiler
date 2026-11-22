#!/system/bin/sh
# R&D write/readback probe for uncataloged SoC nodes.
# Usage: su -c 'sh rnd_probe_write.sh apply|read|restore'
LLCC=/sys/class/devfreq/soc:qcom,cpu-cpu-llcc-bw/min_freq
DDRLAT=/sys/class/devfreq/soc:qcom,cpu0-cpu-ddr-latfloor/min_freq
LLCCLAT=/sys/class/devfreq/soc:qcom,cpu0-cpu-llcc-lat/min_freq
GPUBW=/sys/class/devfreq/soc:qcom,gpubw/min_freq
L3MAX=/sys/class/devfreq/soc:qcom,cpu0-cpu-l3-lat/max_freq
SHARES_TOP=/dev/cpuctl/top-app/cpu.shares
SHARES_BG=/dev/cpuctl/background/cpu.shares
CSTATE=/proc/sys/kernel/sched_cstate_aware
POPP=/sys/class/kgsl/kgsl-3d0/popp

read_all() {
  for f in "$LLCC" "$DDRLAT" "$LLCCLAT" "$GPUBW" "$L3MAX" "$SHARES_TOP" "$SHARES_BG" "$CSTATE" "$POPP"; do
    printf '%s=%s\n' "$f" "$(cat "$f" 2>/dev/null)"
  done
}

case "$1" in
  apply)
    echo 7110 > "$LLCC"
    echo 1144 > "$DDRLAT"
    echo 7110 > "$LLCCLAT"
    echo 1720 > "$GPUBW"
    echo 1017600000 > "$L3MAX"
    echo 2048 > "$SHARES_TOP"
    echo 256 > "$SHARES_BG"
    echo 0 > "$CSTATE"
    echo 1 > "$POPP"
    ;;
  read) ;;
  restore)
    echo 4577 > "$LLCC"
    echo 762 > "$DDRLAT"
    echo 4577 > "$LLCCLAT"
    echo 0 > "$GPUBW"
    echo 1459200000 > "$L3MAX"
    echo 1024 > "$SHARES_TOP"
    echo 1024 > "$SHARES_BG"
    echo 1 > "$CSTATE"
    echo 0 > "$POPP"
    ;;
  *) echo "usage: $0 apply|read|restore"; exit 2 ;;
esac
read_all
