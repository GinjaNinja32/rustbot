use rustbot::prelude::*;
use std::process::Command;

pub fn bash(ctx: &dyn Context, args: &str) -> Result<()> {
    do_bash(ctx, args, true)
}

pub fn bashl(ctx: &dyn Context, args: &str) -> Result<()> {
    do_bash(ctx, args, false)
}

fn format_output(raw: &[u8], oneline: bool) -> Result<String> {
    let mut out = std::str::from_utf8(raw)?.trim_end().to_string().replace("\r", "");
    if oneline {
        out = out.replace("\n", "\x0314;\x03 ");
    }

    Ok(out)
}

fn do_bash(ctx: &dyn Context, args: &str, oneline: bool) -> Result<()> {
    if cfg!(target_os = "windows") {
        bail_user!("unsupported");
    }

    let result = Command::new("bash").arg("-c").arg(args).output()?;

    let out = format_output(&result.stdout, oneline)?;
    if !out.is_empty() {
        ctx.reply(Message::Code(out))?;
    }

    let err = format_output(&result.stderr, oneline)?;
    if !err.is_empty() {
        ctx.reply(Message::Code(format!("stderr: {}", err)))?;
    }

    Ok(())
}
