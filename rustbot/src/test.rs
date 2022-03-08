use crate::bot;
use rustbot::utils;

#[test]
fn test_truncate_module_path() {
    let test_path = "some::module::path";

    let expected = &[
        "s~..", // 0
        "s~..",
        "s~..", // 2
        "s~..",
        "s~..", // 4
        "so~..",
        "some..", // 6
        "some..",
        "some..p~", // 8
        "some..pa~",
        "some..path", // 10
        "some..path",
        "some..path", // 12
        "some..path",
        "some::m~::path", // 14
        "some::mo~::path",
        "some::mod~::path", // 16
        "some::modu~::path",
        "some::module::path", // 18
        "some::module::path",
        "some::module::path", // 20
    ];

    for i in 0..test_path.len() {
        assert!(i < 4 || expected[i].len() <= i); // sanity check
        assert_eq!(bot::truncate_module_path(test_path, i), expected[i]);
    }
}

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
        ("30d", 2592000),
        ("365d", 31536000),
    ];

    for case in cases {
        assert_eq!(utils::parse_duration(case.0).unwrap().as_secs(), case.1);
    }

    #[rustfmt::skip]
    let error_cases = &[
        ("1s1m", "unexpected input at 1m"),
        ("1x", "unexpected input at 1x"),
    ];

    for case in error_cases {
        assert_eq!(utils::parse_duration(case.0).unwrap_err().to_string(), case.1);
    }
}
