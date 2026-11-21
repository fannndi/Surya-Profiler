#!/usr/bin/env bash
# Enforce the dependency direction, protocol purity and file-size budget.
set -euo pipefail
cd "$(dirname "$0")/.."
fail=0

echo "-- engine must not depend on daemon --"
if grep -RIn 'crate::daemon' core/src/engine; then
  echo "FAIL: engine imports daemon"
  fail=1
fi

echo "-- lower layers must not depend on UI --"
if grep -RIn 'com\.suryaprofiler\.ui' \
  app/src/main/java/com/suryaprofiler/core \
  app/src/main/java/com/suryaprofiler/config \
  app/src/main/java/com/suryaprofiler/dynamic \
  app/src/main/java/com/suryaprofiler/protocol; then
  echo "FAIL: non-UI package imports UI"
  fail=1
fi

echo "-- protocol/config must stay Android-free --"
if grep -RIn '^import android\.' \
  app/src/main/java/com/suryaprofiler/protocol \
  app/src/main/java/com/suryaprofiler/config; then
  echo "FAIL: protocol/config imports android.*"
  fail=1
fi

echo "-- production file size (target <= 300; data/tests exempt) --"
oversized=$(
  {
    find core/src -name '*.rs' \
      ! -path '*/tests.rs' ! -path '*/catalog/entries/*' \
      -exec wc -l {} + | awk '$1 > 300 && $2 != "total" {print "  rust:", $1, $2}'
    find app/src/main/java -name '*.kt' -exec wc -l {} + \
      | awk '$1 > 300 && $2 != "total" {print "  kotlin:", $1, $2}'
  } | sort
)
if [ -n "$oversized" ]; then
  echo "$oversized"
  echo "FAIL: production file exceeds 300 lines"
  fail=1
fi

echo "-- protocol version parity --"
rust_ver=$(sed -n 's/^pub const VERSION: u32 = \([0-9]*\);/\1/p' core/src/daemon/proto.rs)
kt_ver=$(sed -n 's/.*PROTOCOL_VERSION = \([0-9][0-9]*\).*/\1/p' \
  app/src/main/java/com/suryaprofiler/protocol/DaemonCommands.kt | head -1)
if [ -z "$rust_ver" ] || [ "$rust_ver" != "$kt_ver" ]; then
  echo "FAIL: protocol version mismatch rust='$rust_ver' kotlin='$kt_ver'"
  fail=1
fi

echo "-- stale path references --"
if grep -RIn 'engine/doctor\.rs\|bridge/thermal\.rs\|serve\.rs\|contract/ServiceControl' \
  core/src AGENTS.md README.md docs; then
  echo "FAIL: stale file references"
  fail=1
fi

exit "$fail"
