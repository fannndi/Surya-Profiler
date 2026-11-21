#!/usr/bin/env bash
# Keep the bundled profile pack and engine binary in lockstep with core/.
set -euo pipefail
cd "$(dirname "$0")/.."

cmp core/profiles.json app/src/main/assets/profiles.json

ver=$(sed -n 's/^version = "\(.*\)"/\1/p' core/Cargo.toml | head -1)
if [ -z "$ver" ]; then
  echo "FAIL: cannot read version from core/Cargo.toml"
  exit 1
fi
if ! grep -aq "surya-prof $ver" app/src/main/assets/surya-prof; then
  echo "FAIL: bundled binary does not report version $ver"
  exit 1
fi
if [ -f core/target/aarch64-linux-android/release/surya-prof ]; then
  cmp core/target/aarch64-linux-android/release/surya-prof \
    app/src/main/assets/surya-prof \
    || { echo "FAIL: bundled binary differs from the cross-build"; exit 1; }
else
  echo "WARNING: no cross-build present; cannot prove the bundled binary matches source"
fi
