#!/system/bin/sh
# rnd_bus.sh off|medium|high
# Direct R&D override for the uncataloged bus-DCVS floors. Always restores
# with `off`. Not part of the product catalog (evidence gathering only).
case "$1" in
  off)    DDR=762;  LLCC=4577; LAT=762;  LLCCLAT=4577; LLCCDDR=762;  GPUBW=0 ;;
  medium) DDR=1720; LLCC=7110; LAT=1144; LLCCLAT=7110; LLCCDDR=1144; GPUBW=1720 ;;
  high)   DDR=2929; LLCC=9155; LAT=1720; LLCCLAT=9155; LLCCDDR=1720; GPUBW=3441 ;;
  *) echo "usage: $0 off|medium|high"; exit 2 ;;
esac

echo "$DDR"     > /sys/class/devfreq/soc:qcom,cpu-llcc-ddr-bw/min_freq
echo "$LLCC"    > /sys/class/devfreq/soc:qcom,cpu-cpu-llcc-bw/min_freq
echo "$LAT"     > /sys/class/devfreq/soc:qcom,cpu0-cpu-ddr-latfloor/min_freq
echo "$LAT"     > /sys/class/devfreq/soc:qcom,cpu6-cpu-ddr-latfloor/min_freq
echo "$LLCCLAT" > /sys/class/devfreq/soc:qcom,cpu0-cpu-llcc-lat/min_freq
echo "$LLCCLAT" > /sys/class/devfreq/soc:qcom,cpu6-cpu-llcc-lat/min_freq
echo "$LLCCDDR" > /sys/class/devfreq/soc:qcom,cpu0-llcc-ddr-lat/min_freq
echo "$LLCCDDR" > /sys/class/devfreq/soc:qcom,cpu6-llcc-ddr-lat/min_freq
echo "$GPUBW"   > /sys/class/devfreq/soc:qcom,gpubw/min_freq

for f in \
  /sys/class/devfreq/soc:qcom,cpu-llcc-ddr-bw/min_freq \
  /sys/class/devfreq/soc:qcom,cpu-cpu-llcc-bw/min_freq \
  /sys/class/devfreq/soc:qcom,cpu0-cpu-ddr-latfloor/min_freq \
  /sys/class/devfreq/soc:qcom,cpu6-cpu-ddr-latfloor/min_freq \
  /sys/class/devfreq/soc:qcom,cpu0-cpu-llcc-lat/min_freq \
  /sys/class/devfreq/soc:qcom,cpu6-cpu-llcc-lat/min_freq \
  /sys/class/devfreq/soc:qcom,cpu0-llcc-ddr-lat/min_freq \
  /sys/class/devfreq/soc:qcom,cpu6-llcc-ddr-lat/min_freq \
  /sys/class/devfreq/soc:qcom,gpubw/min_freq; do
  printf '%s=%s\n' "$f" "$(cat "$f" 2>/dev/null)"
done
