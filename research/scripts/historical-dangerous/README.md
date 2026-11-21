# Historical / dangerous scripts

These one-shot probes are kept only for the audit trail. **Do not run them on
a daily-use device.**

- `charge_test.sh` writes `battery_charging_enabled`, which is forbidden by
  hard rule #8. The only allowed charge node is `input_suspend`.
- `write_test3.sh`, `write_test4.sh`, `write_test7.sh` and `write_test8.sh`
  write frequency nodes and do not always restore the exact original value.
  Use the `surya-prof` engine (single writer) instead.
