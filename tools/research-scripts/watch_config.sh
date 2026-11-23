#!/system/bin/sh
F=/data/user/0/com.suryaprofiler/files/config.json
i=0
while [ "$i" -lt 45 ]; do
  t=$(date +%s)
  m=$(stat -c %Y "$F" 2>/dev/null)
  b=$(grep -o '"base_profile":"[^"]*"' "$F" 2>/dev/null | head -1)
  a=$(grep -o '"active":"[^"]*"' /data/adb/suryaprofiler/state.json 2>/dev/null | head -1)
  echo "$t mtime=$m $b $a"
  sleep 1
  i=$((i+1))
done
