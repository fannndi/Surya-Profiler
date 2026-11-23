#!/system/bin/sh
P=/sys/block/sda/queue/scheduler
echo "before=[$(cat $P)]"
echo cfq > "$P"
echo "rc=$?"
echo "after=[$(cat $P)]"
ls -la /sys/block/sda/queue/iosched 2>&1 | head
