#!/usr/bin/env bash
# Cross-build the Rust daemon + Android APK.
set -euo pipefail
cd "$(dirname "$0")/.."

if [ -f "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; fi
CARGO=${CARGO:-cargo}
export ANDROID_NDK_HOME=${ANDROID_NDK_HOME:-$HOME/Android/Sdk/ndk/30.0.16248370}
export JAVA_HOME=${JAVA_HOME:-$HOME/.local/opt/jdk-17}
export ANDROID_HOME=${ANDROID_HOME:-$HOME/Android/Sdk}
export PATH="$JAVA_HOME/bin:$PATH"

(cd core && "$CARGO" ndk -t arm64-v8a build --release)
./gradlew :app:assembleDebug

md5sum core/target/aarch64-linux-android/release/surya-prof \
       app/src/main/assets/surya-prof \
       app/build/outputs/apk/debug/app-debug.apk

./scripts/check-assets.sh
