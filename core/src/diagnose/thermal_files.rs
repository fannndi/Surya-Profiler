//! Foreign thermal-file scan check (thermal bridge `files_scan`/`files_fix`).
//!
//! A foreign `thermal-*.conf` in `/data/vendor/thermal/config` shadows the
//! vendor config whenever `sconfig=0` loads it, so "Service OFF = stock
//! restore" quietly stops being true. The scan is informational until a
//! foreign ship is found; the pass state flips and `--fix-thermal` repairs.

use super::*;
use serde_json::json;

pub(crate) fn run(r: &mut Report, fix_requested: bool) {
    let mut scan = crate::daemon::thermal_files_scan();
    let foreign_count = scan["foreign"].as_array().map(|a| a.len()).unwrap_or(0);
    let module_count = scan["modules"].as_array().map(|a| a.len()).unwrap_or(0);
    let mut fixed = Value::Null;
    if fix_requested && (foreign_count > 0 || module_count > 0) {
        // `fix` reports what it removed; the check detail shows the state
        // AFTER the repair, so re-scan instead of trusting the fix output.
        let report = crate::daemon::thermal_files_fix();
        scan = crate::daemon::thermal_files_scan();
        fixed = json!({
            "removed": report["removed"],
            "failed": report["failed"],
            "chattr_ok": report["chattr_ok"],
            "restorecon_ok": report["restorecon_ok"],
        });
    }
    let clean = scan["clean"].as_bool().unwrap_or(false);
    let hint = if clean {
        "no foreign thermal ships found"
    } else if module_count > 0 {
        "a module re-ships thermal files on every boot; remove/disable the module"
    } else if !fixed.is_null() {
        "foreign files were removed; re-run doctor to confirm"
    } else {
        "foreign files shadow the vendor config; run `surya-prof doctor --fix-thermal` with the daemon stopped"
    };
    r.check(
        "thermal_foreign_files",
        clean,
        false,
        json!({
            "base": scan["base"],
            "clean": clean,
            "foreign": scan["foreign"],
            "modules": scan["modules"],
            "global_mode": scan["global_mode"],
            "data_path_prop": scan["data_path_prop"],
            "fixed": fixed,
            "hint": hint,
        }),
    );
}
