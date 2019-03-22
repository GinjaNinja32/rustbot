extern crate shared;

use shared::prelude::*;
use std::process::Command;
use std::str;
use std::sync::Arc;

#[no_mangle]
pub fn get_meta() -> Meta {
    let mut meta = Meta::new();
    meta.commandrc("drop", Arc::new(wrap(drop)));
    meta.commandrc("load", Arc::new(wrap(load)));
    meta.commandrc("reload", Arc::new(wrap(reload)));
    meta.commandrc("recompile", Arc::new(wrap(recompile)));
    meta
}

fn wrap(f: impl Fn(&mut Context, &str) -> Result<()>) -> impl Fn(&mut Context, &str) -> Result<()> {
    move |ctx: &mut Context, args| {
        if ctx.has_perm(Perms::Admin)? {
            f(ctx, args)
        } else {
            ctx.reply("permission denied")
        }
    }
}

fn exec(ctx: &mut Context, args: &str, what: fn(&mut Context, &str) -> Result<()>) -> Result<()> {
    for m in args.split(' ') {
        if m == "core" {
            ctx.reply("skipping core")?;
            continue;
        }
        match what(ctx, m) {
            Ok(()) => Ok(()),
            Err(e) => ctx.reply(&format!("{} failed: {}", m, e)),
        }?;
    }
    ctx.reply("done")
}

fn drop(ctx: &mut Context, args: &str) -> Result<()> {
    exec(ctx, args, |ctx, m| ctx.bot.drop_module(m))
}

fn load(ctx: &mut Context, args: &str) -> Result<()> {
    exec(ctx, args, |ctx, m| ctx.bot.load_module(m))
}

fn reload(ctx: &mut Context, args: &str) -> Result<()> {
    exec(ctx, args, |ctx, m| {
        ctx.bot.drop_module(m)?;
        ctx.bot.load_module(m)
    })
}

fn recompile(ctx: &mut Context, args: &str) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    if !cfg!(debug_assertions) {
        cmd.arg("--release");
    }

    match cmd.output() {
        Ok(result) => {
            if result.status.success() {
                reload(ctx, args)
            } else {
                ctx.reply("compile failed:")?;
                for line in str::from_utf8(&result.stderr).unwrap().split('\n') {
                    if line.starts_with("   Compiling") {
                        continue;
                    }
                    if line == "" {
                        break;
                    }
                    ctx.reply(line)?;
                }
                Ok(())
            }
        }
        Err(e) => ctx.reply(&format!("failed to run build: {}", e)),
    }
}
