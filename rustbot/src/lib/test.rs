use super::duration;

#[test]
fn test_parse_duration() {
    let cases = &[
        ("0s", 0),
        ("1s", 1),
        ("60s", 60),
        ("1m", 60),
        ("60m", 3600),
        ("1h", 3600),
        ("24h", 86400),
        ("1d", 86400),
        ("30d", 2_592_000),
        ("365d", 31_536_000),
    ];

    for case in cases {
        assert_eq!(duration::parse_duration(case.0).unwrap().as_secs(), case.1);
    }

    #[rustfmt::skip]
    let error_cases = &[
        ("1s1m", "unexpected input at 1m"),
        ("1x", "unexpected input at 1x"),
    ];

    for case in error_cases {
        assert_eq!(duration::parse_duration(case.0).unwrap_err().to_string(), case.1);
    }
}
