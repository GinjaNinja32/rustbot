use format;
use rustbot::prelude::*;

#[test]
fn test_irc_parse() {
    // empty
    assert_eq!(format::irc_parse(""), vec![]);

    // basic text
    assert_eq!(
        format::irc_parse("foo"),
        vec![Span {
            text: "foo".into(),
            format: Format::None,
            color: Color::None,
            bg: Color::None
        }]
    );

    // colored text
    assert_eq!(
        format::irc_parse("\x032,1foo"),
        vec![Span {
            text: "foo".into(),
            format: Format::None,
            color: Color::Blue,
            bg: Color::Black
        }]
    );
    assert_eq!(
        format::irc_parse("\x0302,01foo"),
        vec![Span {
            text: "foo".into(),
            format: Format::None,
            color: Color::Blue,
            bg: Color::Black
        }]
    );
    assert_eq!(
        format::irc_parse("\x0302,01foo\x03bar"),
        vec![
            Span {
                text: "foo".into(),
                format: Format::None,
                color: Color::Blue,
                bg: Color::Black
            },
            Span {
                text: "bar".into(),
                format: Format::None,
                color: Color::None,
                bg: Color::None
            }
        ]
    );
    assert_eq!(
        format::irc_parse("\x0302,01foo\x03,bar"),
        vec![
            Span {
                text: "foo".into(),
                format: Format::None,
                color: Color::Blue,
                bg: Color::Black
            },
            Span {
                text: ",bar".into(),
                format: Format::None,
                color: Color::None,
                bg: Color::None
            }
        ]
    );
    assert_eq!(
        format::irc_parse("\x0302,01foo\x0301bar"),
        vec![
            Span {
                text: "foo".into(),
                format: Format::None,
                color: Color::Blue,
                bg: Color::Black
            },
            Span {
                text: "bar".into(),
                format: Format::None,
                color: Color::Black,
                bg: Color::None
            }
        ]
    );
    assert_eq!(
        format::irc_parse("\x0302,01foo\x03,02bar"),
        vec![
            Span {
                text: "foo".into(),
                format: Format::None,
                color: Color::Blue,
                bg: Color::Black
            },
            Span {
                text: ",02bar".into(),
                format: Format::None,
                color: Color::None,
                bg: Color::None
            }
        ]
    );

    // bold text
    assert_eq!(
        format::irc_parse("\x02foo\x02bar\x02baz"),
        vec![
            Span {
                text: "foo".into(),
                format: Format::Bold,
                color: Color::None,
                bg: Color::None
            },
            Span {
                text: "bar".into(),
                format: Format::None,
                color: Color::None,
                bg: Color::None
            },
            Span {
                text: "baz".into(),
                format: Format::Bold,
                color: Color::None,
                bg: Color::None
            }
        ]
    );

    // italic text
    assert_eq!(
        format::irc_parse("\x1dfoo\x1dbar\x1dbaz"),
        vec![
            Span {
                text: "foo".into(),
                format: Format::Italic,
                color: Color::None,
                bg: Color::None
            },
            Span {
                text: "bar".into(),
                format: Format::None,
                color: Color::None,
                bg: Color::None
            },
            Span {
                text: "baz".into(),
                format: Format::Italic,
                color: Color::None,
                bg: Color::None
            }
        ]
    );

    // underlined text
    assert_eq!(
        format::irc_parse("\x1ffoo\x1fbar\x1fbaz"),
        vec![
            Span {
                text: "foo".into(),
                format: Format::Underline,
                color: Color::None,
                bg: Color::None
            },
            Span {
                text: "bar".into(),
                format: Format::None,
                color: Color::None,
                bg: Color::None
            },
            Span {
                text: "baz".into(),
                format: Format::Underline,
                color: Color::None,
                bg: Color::None
            }
        ]
    );

    // multiple formats, reset
    assert_eq!(
        format::irc_parse("\x02\x1d\x1ffoo\x034,14bar\x0fbaz"),
        vec![
            Span {
                text: "foo".into(),
                format: Format::Bold | Format::Underline | Format::Italic,
                color: Color::None,
                bg: Color::None
            },
            Span {
                text: "bar".into(),
                format: Format::Bold | Format::Underline | Format::Italic,
                color: Color::BrightRed,
                bg: Color::BrightBlack,
            },
            Span {
                text: "baz".into(),
                format: Format::None,
                color: Color::None,
                bg: Color::None
            }
        ]
    );

    // UTF-8
    assert_eq!(
        format::irc_parse("ΨΩΔ"),
        vec![Span {
            text: "ΨΩΔ".into(),
            format: Format::None,
            color: Color::None,
            bg: Color::None
        }]
    );
}
