use regex::Regex;
use rustbot::prelude::*;

const IRC_COLOR: char = 0x03 as char;
const IRC_RESET: char = 0x0f as char;
const IRC_BOLD: char = 0x02 as char;
const IRC_UNDERLINE: char = 0x1f as char;
const IRC_ITALIC: char = 0x1d as char;

lazy_static! {
    static ref COLOR_REGEX: Regex = Regex::new("^([0-9]{1,2})(,([0-9]{1,2}))?").unwrap();
}

pub fn irc_parse(s: &str) -> Vec<Span> {
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut spans = vec![];
    let mut current = vec![];

    let mut format = Format::None;
    let mut fg = Color::None;
    let mut bg = Color::None;

    while c.len() > i {
        match c[i] {
            IRC_COLOR | IRC_RESET | IRC_BOLD | IRC_UNDERLINE | IRC_ITALIC => {
                if !current.is_empty() {
                    spans.push(Span {
                        text: current.iter().collect::<String>().into(),
                        format,
                        color: fg,
                        bg,
                    });
                    current.clear();
                }

                match c[i] {
                    IRC_COLOR => match COLOR_REGEX.captures(&c[i + 1..].iter().copied().collect::<String>()) {
                        None => {
                            fg = Color::None;
                            bg = Color::None;
                        }
                        Some(m) => {
                            i += m.get(0).unwrap().as_str().len();
                            fg = str::parse::<u8>(m.get(1).unwrap().as_str()).unwrap().into();
                            bg = m
                                .get(3)
                                .map(|v| str::parse::<u8>(v.as_str()).unwrap().into())
                                .unwrap_or(Color::None)
                        }
                    },
                    IRC_RESET => {
                        format = Format::None;
                        fg = Color::None;
                        bg = Color::None;
                    }
                    IRC_BOLD => format ^= Format::Bold,
                    IRC_UNDERLINE => format ^= Format::Underline,
                    IRC_ITALIC => format ^= Format::Italic,
                    _ => unreachable!(),
                }
            }
            t => current.push(t),
        }
        i += 1;
    }

    if !current.is_empty() {
        spans.push(Span {
            text: current.iter().collect::<String>().into(),
            format,
            color: fg,
            bg,
        });
        current.clear();
    }

    spans
}
