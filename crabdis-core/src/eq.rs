const ITER_LENGTH: usize = 512;

/// Constant time password equality check
#[must_use]
pub fn constant_time_str(expected: &str, provided: &str) -> bool {
    let expected = expected.as_bytes();
    let provided = provided.as_bytes();

    let mut diff = expected.len() ^ provided.len();

    for i in 0..=ITER_LENGTH {
        let expected_byte = expected.get(i).map_or(0, |v| *v) as usize;
        let provided_byte = provided.get(i).map_or(0, |v| *v) as usize;
        diff |= expected_byte ^ provided_byte;
    }

    diff == 0
}

#[cfg(test)]
mod test {
    use std::hint::black_box;

    #[test]
    fn test_constant_time_str_empty_strings() {
        assert!(super::constant_time_str("", ""));
        assert!(!super::constant_time_str("", "nonempty"));
        assert!(!super::constant_time_str("nonempty", ""));
    }

    #[test]
    fn test_constant_time_str_single_char() {
        assert!(super::constant_time_str("a", "a"));
        assert!(!super::constant_time_str("a", "b"));
        assert!(!super::constant_time_str("a", ""));
        assert!(!super::constant_time_str("", "a"));
    }

    #[test]
    fn test_constant_time_str_different_lengths() {
        assert!(!super::constant_time_str("short", "longer"));
        assert!(!super::constant_time_str("longer", "short"));
        assert!(!super::constant_time_str("same", "diff"));
    }

    #[test]
    fn test_constant_time_str_unicode() {
        assert!(super::constant_time_str("café", "café"));
        assert!(!super::constant_time_str("café", "cafe"));
        assert!(!super::constant_time_str("naïve", "naive"));
        assert!(super::constant_time_str("🦀", "🦀"));
        assert!(!super::constant_time_str("🦀", "🐶"));
    }

    #[test]
    fn test_constant_time_str_case_sensitivity() {
        assert!(!super::constant_time_str("Password", "password"));
        assert!(!super::constant_time_str("PASSWORD", "password"));
        assert!(super::constant_time_str("MiXeD", "MiXeD"));
    }

    #[test]
    fn test_constant_time_str_special_characters() {
        assert!(super::constant_time_str("!@#$%^&*()", "!@#$%^&*()"));
        assert!(!super::constant_time_str("!@#$%^&*()", "!@#$%^&*("));
        assert!(super::constant_time_str("\0\x01\x02", "\0\x01\x02"));
        assert!(!super::constant_time_str("\0\x01\x02", "\0\x01\x03"));
    }

    #[test]
    fn test_constant_time_str_long_strings() {
        let long_str = "a".repeat(1000);
        let long_str_diff = "b".repeat(1000);
        let short_str = "short".to_owned();

        assert!(super::constant_time_str(
            black_box(&long_str),
            black_box(&long_str)
        ));
        assert!(!super::constant_time_str(
            black_box(&long_str),
            black_box(&long_str_diff)
        ));
        assert!(!super::constant_time_str(
            black_box(&long_str),
            black_box(&short_str)
        ));
        assert!(!super::constant_time_str(
            black_box(&short_str),
            black_box(&long_str)
        ));
    }
}
