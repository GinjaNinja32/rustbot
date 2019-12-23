use rustbot::prelude::Message::*;
use rustbot::prelude::*;

fn paste_max_lines(input: String, max_lines: usize) -> Result<(Vec<String>, Option<String>)> {
    let lines: Vec<String> = input.split('\n').map(|l| l.to_string()).collect();
    if lines.len() > max_lines {
        let client = reqwest::Client::new();
        let mut result = client.post("http://ix.io").form(&[("f:1", input)]).send()?;

        let url = result.text()?;

        Ok((
            lines[0..max_lines - 1].to_vec(),
            Some(format!("[full message: {}]", url.trim())),
        ))
    } else {
        Ok((lines, None))
    }
}

pub fn format_irc(m: Message) -> Result<Vec<String>> {
    match m {
        Simple(s) | Code(s) => match paste_max_lines(s, 3)? {
            (lines, None) => Ok(lines),
            (mut lines, Some(extra)) => {
                lines.push(extra);
                Ok(lines)
            }
        },
    }
}

pub fn format_discord(m: Message) -> Result<String> {
    match m {
        Simple(s) => match paste_max_lines(s, 11)? {
            (lines, None) => Ok(lines.join("\n")),
            (lines, Some(extra)) => Ok(format!("{}\n{}", lines.join("\n"), extra)),
        },
        Code(s) => {
            if !s.contains('\n') {
                Ok(format!("`{}`", s))
            } else {
                match paste_max_lines(s, 11)? {
                    (lines, None) => Ok(format!("```\n{}\n```", lines.join("\n"))),
                    (lines, Some(extra)) => Ok(format!("```\n{}\n```{}", lines.join("\n"), extra)),
                }
            }
        }
    }
}
