use crate::bot;

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
