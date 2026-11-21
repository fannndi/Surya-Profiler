#!/usr/bin/env bash
# One-command host check: Rust + Kotlin + architecture guardrails.
set -euo pipefail
cd "$(dirname "$0")/.."

if [ -f "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; fi
CARGO=${CARGO:-cargo}
export JAVA_HOME=${JAVA_HOME:-$HOME/.local/opt/jdk-17}
export ANDROID_HOME=${ANDROID_HOME:-$HOME/Android/Sdk}
export PATH="$JAVA_HOME/bin:$PATH"

echo "== rust fmt =="
(cd core && "$CARGO" fmt -- --check)

echo "== rust clippy =="
(cd core && "$CARGO" clippy --all-targets -- -D warnings)

echo "== rust tests =="
(cd core && "$CARGO" test)

echo "== asset lockstep =="
./scripts/check-assets.sh

echo "== architecture =="
./scripts/arch-check.sh

echo "== kotlin unit tests + APK =="
./gradlew :app:testDebugUnitTest :app:assembleDebug

echo "== ok =="
