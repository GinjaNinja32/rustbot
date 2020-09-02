use rustbot::prelude::*;
use std::borrow::Cow;
use std::io::{Read, Write};
use std::process::{Command, Stdio};

fn paste(text: &str) -> Result<String> {
    let mut cmd = Command::new("./external/paste")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    {
        let stdin = cmd.stdin.take();
        write!(stdin.unwrap(), "{}", text)?;
    }

    cmd.wait().expect("failed to wait for paste");

    let url = {
        let stdout = cmd.stdout.take();
        let mut url = "".to_string();
        stdout.unwrap().read_to_string(&mut url)?;
        url
    };

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
        match sp {
            Span::Text {
                ref text,
                format,
                color,
                ..
            } => {
                let format = *format;
                let color = *color;

                if color == col && format == fmt {
                    st.push_str(&text);
                    continue;
                }

                if color == Color::None && format == Format::None {
                    col = color;
                    fmt = format;
                    st.push('\x0F');
                    st.push_str(&text);
                    continue;
                }

                if color != col {
                    match color {
                        Color::None => {
                            if format == fmt && text.starts_with(|c| '0' <= c && c <= '9') {
                                st.push_str("\x03\x02\x02")
                            } else {
                                st.push('\x03')
                            }
                        }
                        _ => {
                            let code = format!("\03{:02}", color as u8);
                            st.push_str(&code);
                            if format == fmt && text.starts_with(',') {
                                st.push_str("\x02\x02");
                            }
                        }
                    }
                    col = color;
                }

                if format != fmt {
                    let toggle = format ^ fmt;
                    if toggle.contains(Format::Bold) {
                        st.push('\x02');
                    }
                    if toggle.contains(Format::Italic) {
                        st.push('\x1D');
                    }
                    if toggle.contains(Format::Underline) {
                        st.push('\x1F');
                    }

                    fmt = format;
                }

                st.push_str(&text);
            }
            Span::DiscordEmoji(name, _) => {
                st.push(':');
                st.push_str(name);
                st.push(':');
            }
        }
    }

    st
}

pub fn format_irc(m: Message) -> Result<Vec<String>> {
    let msg = match m {
        Message::Simple(s) | Message::Code(s) => s,
        Message::Spans(s) => render_irc(s),
        Message::Prefixed(p, s) => {
            let p = render_irc(p);
            let s = render_irc(s);

            let (vec, link) = paste_max_lines(s, 3)?;

            let mut vec = vec.iter().map(|line| p.clone() + line).collect::<Vec<_>>();
            if let Some(link) = link {
                vec.push(link);
            }
            return Ok(vec);
        }
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
    match s {
        Span::Text { text, format, .. } => {
            if *format == Format::None {
                return text.clone();
            }
            let mut formats = "".to_string();
            if format.contains(Format::Italic) {
                formats += "*";
            }
            if format.contains(Format::Bold) {
                formats += "**";
            }
            if format.contains(Format::Underline) {
                formats += "__";
            }

            Cow::Owned(format!(
                "\u{FEFF}{}{}{}\u{FEFF}",
                formats,
                text,
                formats.chars().rev().collect::<String>()
            ))
        }
        Span::DiscordEmoji(name, id) => Cow::Owned(format!("<:{}:{}>", name, id)),
    }
}

fn render_dis_spans(s: Vec<Span>) -> String {
    s.iter().map(render_dis).collect::<Vec<Cow<str>>>().join("")
}

pub fn format_discord(m: Message) -> Result<String> {
    let (msg, code) = match m {
        Message::Simple(s) => (s, false),
        Message::Code(s) => (s, true),
        Message::Spans(s) => (render_dis_spans(s), false),
        Message::Prefixed(p, s) => {
            let p = render_dis_spans(p);
            let s = render_dis_spans(s);

            let (res, url) = paste_max_lines(s, 11)?;
            let mut res = res.iter().map(|line| p.clone() + line).collect::<Vec<_>>();
            if let Some(u) = url {
                res.push(u)
            }
            return Ok(res.join("\n"));
        }
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
