use rustbot::prelude::*;
use std::borrow::Cow;

fn paste(text: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let mut result = client.post("http://ix.io").form(&[("f:1", text)]).send()?;

    let url = result.text()?;

    Ok(format!("[full message: {}]", url.trim()))
}

fn paste_max_lines(input: String, max_lines: usize) -> Result<(Vec<String>, Option<String>)> {
    let lines: Vec<String> = input.split('\n').map(std::string::ToString::to_string).collect();
    if lines.len() > max_lines {
        let v = lines[0..max_lines - 1].to_vec();
        Ok((v, Some(paste(&input)?)))
    } else {
        Ok((lines, None))
    }
}

fn render_irc(spans: Vec<Span>) -> String {
    let mut col = Color::None;
    let mut fmt = Format::None;
    let mut st = "".to_string();

    for sp in &spans {
        if sp.color == col && sp.format == fmt {
            st.push_str(&sp.text);
            continue;
        }

        if sp.color == Color::None && sp.format == Format::None {
            col = sp.color;
            fmt = sp.format;
            st.push('\x0F');
            st.push_str(&sp.text);
            continue;
        }

        if sp.color != col {
            match sp.color {
                Color::None => {
                    if sp.format == fmt && sp.text.starts_with(|c| '0' <= c && c <= '9') {
                        st.push_str("\x03\x02\x02")
                    } else {
                        st.push('\x03')
                    }
                }
                _ => {
                    let code = match sp.color {
                        Color::None => unreachable!(),
                        Color::Red => "\x0304",
                        Color::Yellow => "\x0308",
                        Color::Green => "\x0309",
                    };
                    st.push_str(code);
                    if sp.format == fmt && sp.text.starts_with(',') {
                        st.push_str("\x02\x02");
                    }
                }
            }
            col = sp.color;
        }

        if sp.format != fmt {
            let toggle = sp.format ^ fmt;
            if toggle.contains(Format::Bold) {
                st.push('\x02');
            }
            if toggle.contains(Format::Italic) {
                st.push('\x1D');
            }
            if toggle.contains(Format::Underline) {
                st.push('\x1F');
            }

            fmt = sp.format;
        }

        st.push_str(&sp.text);
    }

    st
}

pub fn format_irc(m: Message) -> Result<Vec<String>> {
    let msg = match m {
        Message::Simple(s) | Message::Code(s) => s,
        Message::Spans(s) => render_irc(s),
    };

    match paste_max_lines(msg, 3)? {
        (vec, None) => Ok(vec),
        (mut vec, Some(link)) => {
            vec.push(link);
            Ok(vec)
        }
    }
}

fn render_dis<'a>(s: &'a Span) -> Cow<'a, str> {
    if s.format == Format::None {
        return s.text.clone();
    }
    let mut formats = "".to_string();
    if s.format.contains(Format::Italic) {
        formats += "*";
    }
    if s.format.contains(Format::Bold) {
        formats += "**";
    }
    if s.format.contains(Format::Underline) {
        formats += "__";
    }

    return Cow::Owned(format!(
        "\u{FEFF}{}{}{}\u{FEFF}",
        formats,
        s.text,
        formats.chars().rev().collect::<String>()
    ));
}

pub fn format_discord(m: Message) -> Result<String> {
    let (msg, code) = match m {
        Message::Simple(s) => (s, false),
        Message::Code(s) => (s, true),
        Message::Spans(s) => (s.iter().map(render_dis).collect::<Vec<Cow<str>>>().join(""), false),
    };

    if code && !msg.contains('\n') {
        Ok(format!("`{}`", msg))
    } else {
        let (mut res, url) = paste_max_lines(msg, 11)?;
        if let Some(u) = url {
            res.push(u);
        }
        if code {
            Ok(format!("```{}```", res.join("\n")))
        } else {
            Ok(res.join("\n"))
        }
    }
}
