use shared::prelude::*;
use std::process::Command;

pub fn bash(ctx: &Context, args: &str) -> Result<()> {
    do_bash(ctx, args, true)
}

pub fn bashl(ctx: &Context, args: &str) -> Result<()> {
    do_bash(ctx, args, false)
}

fn do_bash(ctx: &Context, args: &str, oneline: bool) -> Result<()> {
    if cfg!(target_os = "windows") {
        return Err(Error::new("unsupported"));
    }

    let result = Command::new("bash").arg("-c").arg(args).output()?;

    let mut text = std::str::from_utf8(&result.stdout)?.trim_end().to_string();
    if oneline {
        text = text.replace("\r", "").replace("\n", "\x0314; \x03\x02\x02");
    }

    ctx.say(&text)
}
