extern crate rustbot;

use rustbot::prelude::*;
use std::process::Command as ProcessCommand;
use std::str;

#[no_mangle]
pub fn get_meta() -> Meta {
    let mut meta = Meta::new();
    meta.cmd("drop", Command::new(drop).req_perms(Perms::Modules));
    meta.cmd("load", Command::new(load).req_perms(Perms::Modules));
    meta.cmd("reload", Command::new(reload).req_perms(Perms::Modules));
    meta.cmd("recompile", Command::new(recompile).req_perms(Perms::Modules));
    meta.cmd(
        "enable",
        Command::new(move |ctx, args| set_enabled(ctx, args, true)).req_perms(Perms::Modules),
    );
    meta.cmd(
        "disable",
        Command::new(move |ctx, args| set_enabled(ctx, args, false)).req_perms(Perms::Modules),
    );
    meta
}

fn exec(ctx: &Context, args: &str, what: fn(&Context, &str) -> Result<()>) -> Result<()> {
    for m in args.split(' ') {
        if m == "core" {
            ctx.say("skipping core")?;
            continue;
        }
        match what(ctx, m) {
            Ok(()) => Ok(()),
            Err(e) => ctx.say(&format!("{} failed: {}", m, e)),
        }?;
    }
    ctx.say("done")
}

fn drop(ctx: &Context, args: &str) -> Result<()> {
    exec(ctx, args, |ctx, m| ctx.bot.drop_module(m))
}

fn load(ctx: &Context, args: &str) -> Result<()> {
    exec(ctx, args, |ctx, m| ctx.bot.load_module(m))
}

fn reload(ctx: &Context, args: &str) -> Result<()> {
    exec(ctx, args, |ctx, m| {
        ctx.bot.drop_module(m)?;
        ctx.bot.load_module(m)
    })
}

fn recompile(ctx: &Context, args: &str) -> Result<()> {
    let mut cmd = ProcessCommand::new("cargo");
    cmd.arg("build");
    if !cfg!(debug_assertions) {
        cmd.arg("--release");
    }

    match cmd.output() {
        Ok(result) => {
            if result.status.success() {
                reload(ctx, args)
            } else {
                // ctx.say("compile failed:")?;
                let mut lines: Vec<&str> = vec![];
                for line in str::from_utf8(&result.stderr).unwrap().split('\n') {
                    if line.starts_with("   Compiling") {
                        continue;
                    }
                    if line == "" {
                        break;
                    }
                    lines.push(line);
                }
                ctx.reply(Message::Code(lines.join("\n")))
            }
        }
        Err(e) => ctx.say(&format!("failed to run build: {}", e)),
    }
}

fn set_enabled(ctx: &Context, args: &str, target: bool) -> Result<()> {
    let a = args.split(' ').collect::<Vec<&str>>();
    if a.len() < 2 {
        return Err(Error::new("Usage: (enable/disable) config_id module [module [...]]"));
    }

    let config_id = a[0];

    for m in &a[1..] {
        let db = ctx.bot.sql().lock();
        if target {
            db.execute(
                "INSERT INTO enabled_modules (config_id, name) VALUES ($1, $2)",
                &[&config_id, &m],
            )?;
        } else {
            db.execute(
                "DELETE FROM enabled_modules WHERE config_id = $1 AND name = $2",
                &[&config_id, &m],
            )?;
        }
    }

    ctx.reply(Message::Simple("Done".to_string()))
}
