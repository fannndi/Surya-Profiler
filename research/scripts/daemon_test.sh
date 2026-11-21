#!/system/bin/sh
# Heartbeat daemon survival test (R&D only).
LOG=/data/local/tmp/daemon_test.log
echo "start $(date +%s) uid=$(id -u) ctx=$(cat /proc/self/attr/current 2>/dev/null)" >> "$LOG"
i=0
while [ "$i" -lt 60 ]; do
  echo "beat $(date +%s) $i" >> "$LOG"
  sleep 2
  i=$((i+1))
done
echo "end $(date +%s)" >> "$LOG"
