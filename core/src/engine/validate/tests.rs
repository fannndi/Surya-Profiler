//! validate unit tests (split from validate.rs).

use super::*;

#[test]
fn mask_validated_against_cpu_count() {
    assert!(validate_mask("0-7", 8).is_ok());
    assert!(validate_mask("0-8", 8).is_err());
    assert!(validate_mask("0,2,4-7", 8).is_ok());
    assert!(validate_mask("7-0", 8).is_err());
    assert!(validate_mask("a", 8).is_err());
}
