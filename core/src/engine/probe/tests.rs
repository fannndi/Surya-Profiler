use super::parse_cpu_count;

#[test]
fn simple_range() {
    assert_eq!(parse_cpu_count("0-7"), 8);
}

#[test]
fn comma_ranges() {
    assert_eq!(parse_cpu_count("0-3,4-7"), 8);
    assert_eq!(parse_cpu_count("0,2,4-7"), 8);
}

#[test]
fn mixed_single() {
    assert_eq!(parse_cpu_count("0-3,5"), 6);
}

#[test]
fn single_cpu() {
    assert_eq!(parse_cpu_count("0"), 1);
}

#[test]
fn empty_fallback() {
    assert_eq!(parse_cpu_count(""), 8);
}
