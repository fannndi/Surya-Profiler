#!/system/bin/sh
# rnd_keepawake.sh on|off
# Keep the panel on while the USB is physically present but charging is
# bypassed (`input_suspend=1` reports Discharging; STAY_ON still keys off the
# plug). Restores the user's setting at the end of a session.
case "$1" in
  on)  svc power stayon true ;;
  off) svc power stayon false ;;
  *) echo "usage: $0 on|off"; exit 2 ;;
esac
settings get global stay_on_while_plugged_in
