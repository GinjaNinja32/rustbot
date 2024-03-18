use crate::args::{Arg, Atom};
use std::borrow::Cow;

#[test]
fn test_arg_parse_full() {
    macro_rules! parse_ok {
        ($( $input:literal => $ty:tt => $res:tt; )*) => {
            $(
                assert_eq!(<$ty as Arg>::parse_full($input).unwrap(), $res);
            )*
        };
    }

    parse_ok!(
        "1" => u64 => 1;
        "1" => (u64,) => (1,);

        "1 2 3" => (u64, u16, u8) => (1, 2, 3);
        "2 true" => (i32, bool) => (2, true);
        "1 foo 3" => (u32, String, u32) => (1, "foo".to_string(), 3);
        "1 \"foo bar\" 3" => (u32, String, u32) => (1, "foo bar".to_string(), 3);

        "1 foo 3" => (u32, Atom, u32) => (1, Atom("foo".to_string()), 3);

        "1 foo 3" => (u32, Option<u32>, Atom, u32) => (1, None, Atom("foo".to_string()), 3);
        "1 2 foo 3" => (u32, Option<u32>, Atom, u32) => (1, Some(2), Atom("foo".to_string()), 3);
    );

    macro_rules! parse_err {
        ($( $input:literal => $ty:tt => $err:literal; )*) => {
            $(
                assert_eq!(<$ty as Arg>::parse_full($input).unwrap_err().to_string(), $err);
            )*
        }
    }

    parse_err!(
        "2 tru" => (i32, bool) => "parsing (i32, bool): failed to parse \"tru\" as bool: provided string was not `true` or `false`";
        "foo" => u32 => "parsing u32: failed to parse \"foo\" as u32: invalid digit found in string";
        "1 foo bar 3" => (u32, String, u32) => "parsing (u32, string, u32): failed to parse \"bar\" as u32: invalid digit found in string";
        "1 2 3" => (u32, u32) => "parsing (u32, u32): extra arguments at end: \"3\"";
        "1 \"foo bar\" 3" => (u32, Atom, u32) => "parsing (u32, atom, u32): failed to parse \"bar\\\"\" as u32: invalid digit found in string";
        "foo bar 2" => (Option<u32>, Atom, u32) => "parsing (optional u32, atom, u32): failed to parse \"bar\" as u32: invalid digit found in string";
    );
}
