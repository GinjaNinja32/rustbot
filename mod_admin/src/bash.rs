use rustbot::prelude::*;
use std::process::Command;

pub fn bash(ctx: &dyn Context, args: &str) -> Result<()> {
    do_bash(ctx, args, true)
}

pub fn bashl(ctx: &dyn Context, args: &str) -> Result<()> {
    do_bash(ctx, args, false)
}

fn do_bash(ctx: &dyn Context, args: &str, oneline: bool) -> Result<()> {
    if cfg!(target_os = "windows") {
        return Err("unsupported".into());
    }

    let result = Command::new("bash").arg("-c").arg(args).output()?;

    let mut out = std::str::from_utf8(&result.stdout)?.trim_end().to_string();
    if oneline {
        out = out.replace("\r", "").replace("\n", "\x0314; \x03\x02\x02");
    }

    if !out.is_empty() {
        ctx.say(&out)?;
    }

    let mut err = std::str::from_utf8(&result.stderr)?.trim_end().to_string();
    if oneline {
        err = err.replace("\r", "").replace("\n", "\x0314; \x03\x02\x02");
    }

    if !err.is_empty() {
        ctx.say(&format!("stderr: {}", err))?;
    }

    Ok(())
}
