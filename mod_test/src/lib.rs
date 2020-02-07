extern crate rustbot;

use rustbot::prelude::*;
use rustbot::spans;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd(
        "test",
        Command::new(|ctx, args| {
            ctx.say(&format!("beep boop {}", ctx.perms()?))?;
            ctx.say(&format!("you passed: {}", args))?;
            ctx.reply(Message::Spans(spans!(
                "simple ",
                Span::Formatted(Format::Bold, "bold".into()),
                " ",
                Span::Formatted(Format::Italic, "italic".into()),
                " ",
                Span::Formatted(Format::Underline, "underline".into()),
                " ",
                Span::Colored(Color::Red, "red".into()),
                " ",
                Span::Colored(Color::Yellow, "yellow".into()),
                " ",
                Span::Colored(Color::Green, "green".into()),
                " ",
                Span::Text(Color::Red, Format::Bold | Format::Italic, "bold_italic_red".into()),
            )))
        }),
    );
}
