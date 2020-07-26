use rustbot::prelude::*;
use rustbot::{span, spans};

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd(
        "test",
        Command::new(|ctx, args| {
            ctx.say(&format!("beep boop {}", ctx.perms()?))?;
            ctx.say(&format!("you passed: {}", args))?;
            ctx.reply(Message::Spans(spans!(
                "simple ",
                span!(Format::Bold; "bold"),
                " ",
                span!(Format::Italic; "italic"),
                " ",
                span!(Format::Underline; "underline"),
                " ",
                span!(Color::Red; "red"),
                " ",
                span!(Color::Yellow; "yellow"),
                " ",
                span!(Color::Green; "green"),
                " ",
                span!(Color::Red + Format::Bold + Format::Italic; "bold_italic_red"),
            )))
        }),
    );
}
