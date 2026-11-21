#!/system/bin/sh
# rnd_stim.sh MODE [DUR]
# Scriptable stimulus for A/B runs (research R&D). All read-only wrt tuning.
MODE=$1
DUR=${2:-600}
WAKE() { input keyevent 224; sleep 1; wm dismiss-keyguard 2>/dev/null; sleep 1; }
case "$MODE" in
  unlock) WAKE ;;
  on) input keyevent 224 ;;
  home) input keyevent 3 ;;
  off) input keyevent 26 ;;
  idle)
    WAKE
    input keyevent 3
    ;;
  scroll)
    WAKE
    am start -a android.intent.action.VIEW -d "file:///sdcard/surya_rnd/scroll.html" -n com.android.chrome/com.google.android.apps.chrome.Main
    # page self-scrolls via JS (input injection is blocked by MIUI unless
    # "USB debugging (Security settings)" is on); keep the process alive.
    sleep "$DUR"
    ;;
  video)
    WAKE
    # MX Player via MediaStore content URI (no storage prompt, no resolver).
    am start -n com.mxtech.videoplayer.ad/.ActivityScreen -d "content://media/external/video/media/1000002255" -t "video/*"
    sleep "$DUR"
    ;;
  game)
    WAKE
    monkey -p com.stove.epic7.google -c android.intent.category.LAUNCHER 1
    ;;
  stop)
    am force-stop com.android.chrome
    am force-stop com.mxtech.videoplayer.ad
    am force-stop com.stove.epic7.google
    input keyevent 3
    ;;
esac
