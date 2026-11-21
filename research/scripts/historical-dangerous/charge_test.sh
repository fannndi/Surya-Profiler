#!/system/bin/sh
B=/sys/class/power_supply/battery
echo "=== before ==="
cat "$B/status" "$B/capacity" "$B/current_now" "$B/input_suspend" "$B/battery_charging_enabled" 2>&1
echo "=== input_suspend=1 ==="
echo 1 > "$B/input_suspend"
echo "rc=$?"
sleep 3
cat "$B/status" "$B/current_now" "$B/input_suspend" 2>&1
echo "=== input_suspend=0 ==="
echo 0 > "$B/input_suspend"
echo "rc=$?"
sleep 3
cat "$B/status" "$B/current_now" "$B/input_suspend" 2>&1
echo "=== battery_charging_enabled=0 ==="
echo 0 > "$B/battery_charging_enabled"
echo "rc=$?"
sleep 3
cat "$B/status" "$B/current_now" "$B/battery_charging_enabled" 2>&1
echo "=== battery_charging_enabled=1 ==="
echo 1 > "$B/battery_charging_enabled"
echo "rc=$?"
sleep 3
cat "$B/status" "$B/current_now" "$B/battery_charging_enabled" 2>&1
