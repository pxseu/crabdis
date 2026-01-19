const ITER_LENGTH: usize = 512;

/// Constant time password equality check
#[must_use]
pub fn constant_time_str(expected: &str, provided: &str) -> bool {
    let expected = expected.as_bytes();
    let provided = provided.as_bytes();

    let mut diff = expected.len() ^ provided.len();

    for i in 0..ITER_LENGTH {
        let expected_byte = expected.get(i).copied().unwrap_or(0);
        let provided_byte = provided.get(i).copied().unwrap_or(0);
        diff |= (expected_byte ^ provided_byte) as usize;
    }

    diff == 0
}

#[cfg(test)]
mod test {
    use std::hint::black_box;

    #[test]
    fn test_constant_time_password_timing() {
        use std::time::Instant;

        let password = "this_is_a_very_long_password_for_timing_test";
        let wrong_password = "xhis_is_a_very_long_password_for_timing_test";
        let very_wrong = "completely_different_password_of_different_length";

        let iterations = 10000;

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = super::constant_time_str(black_box(password), black_box(password));
        }
        let match_time = start.elapsed();

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = super::constant_time_str(black_box(password), black_box(wrong_password));
        }
        let one_char_diff_time = start.elapsed();

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = super::constant_time_str(black_box(password), black_box(very_wrong));
        }
        let completely_different_time = start.elapsed();

        let max_diff = match_time
            .max(one_char_diff_time)
            .max(completely_different_time);
        let min_diff = match_time
            .min(one_char_diff_time)
            .min(completely_different_time);

        assert!(
            max_diff.as_nanos() < min_diff.as_nanos() * 2,
            "Timing variation too large: max={max_diff:?}, min={min_diff:?}"
        );
    }

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
