//! verify unit tests (split from verify.rs).

use super::*;

#[test]
fn freq_mismatches_are_transient_classified() {
    assert!(mismatch_is_transient(catalog::Kind::Freq));
    assert!(mismatch_is_transient(catalog::Kind::FreqMin));
    assert!(mismatch_is_transient(catalog::Kind::FreqMax));
    assert!(!mismatch_is_transient(catalog::Kind::Int));
    assert!(!mismatch_is_transient(catalog::Kind::Mask));
}
