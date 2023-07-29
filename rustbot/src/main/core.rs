use log::Level;
use rustbot::prelude::*;
use std::collections::BTreeMap;
use std::process::Command as ProcessCommand;
use std::str;
use std::time::Instant;

use crate::context::Context;
use rustbot::types::Context as TypesContext; // trait

pub type CoreCommand = dyn Fn(&Context, &str) -> Result<()> + Send + Sync;

pub fn get_commands() -> BTreeMap<String, (Perms, Box<CoreCommand>)> {
    let mut cmds: BTreeMap<_, (_, Box<CoreCommand>)> = BTreeMap::new();

    cmds.insert("drop".to_string(), (Perms::Modules, Box::new(drop)));
    cmds.insert("load".to_string(), (Perms::Modules, Box::new(load)));
    cmds.insert("reload".to_string(), (Perms::Modules, Box::new(reload)));
    cmds.insert("recompile".to_string(), (Perms::Modules, Box::new(recompile)));
    cmds.insert("log".to_string(), (Perms::Modules, Box::new(log)));
    cmds.insert("suppress".to_string(), (Perms::Modules, Box::new(suppress)));
    cmds.insert(
        "enable".to_string(),
        (Perms::Modules, Box::new(move |ctx, args| set_enabled(ctx, args, true))),
    );
    cmds.insert(
        "disable".to_string(),
        (Perms::Modules, Box::new(move |ctx, args| set_enabled(ctx, args, false))),
    );

    cmds
}

fn exec(ctx: &Context, args: &str, what: fn(&Context, &str) -> Result<()>) -> Result<()> {
    for m in args.split(' ') {
        if m == "core" {
            ctx.say("skipping core")?;
            continue;
        }
        match what(ctx, m) {
            Ok(()) => Ok(()),
            Err(e) => ctx.say(&format!("{m} failed: {e}")),
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
                    if line.is_empty() {
                        break;
                    }
                    lines.push(line);
                }
                ctx.reply(Message::Code(lines.join("\n")))
            }
        }
        Err(e) => ctx.say(&format!("failed to run build: {e}")),
    }
}

fn parse_log_level(s: &str) -> Result<Option<Level>> {
    Ok(Some(match s {
        "err" | "error" => Level::Error,
        "warn" => Level::Warn,
        "info" => Level::Info,
        "debug" => Level::Debug,
        "trace" => Level::Trace,
        "none" => return Ok(None),
        _ => bail_user!("invalid log level specification"),
    }))
}

fn log(ctx: &Context, args: &str) -> Result<()> {
    let a = args.split(' ').collect::<Vec<_>>();
    match a.as_slice() {
        [module, level] => {
            let level = parse_log_level(level)?;
            ctx.bot.set_module_log_level(module, level)?;
        }
        [level] => match parse_log_level(level)? {
            Some(level) => ctx.bot.set_log_level(level)?,
            None => bail_user!("invalid log level specification"),
        },
        _ => bail_user!("unknown argument format; try 'log LEVEL' or 'log MODULE LEVEL'"),
    }
    ctx.reply(Message::Simple("Done".to_string()))
}

fn suppress(ctx: &Context, args: &str) -> Result<()> {
    let a = args.split(' ').collect::<Vec<&str>>();
    if a.len() != 2 {
        bail_user!("Usage: suppress <module> <seconds>");
    }

    let module = a[0].to_string();
    let duration = parse_duration(a[1])?;
    let ts = Instant::now() + duration;

    ctx.bot.suppress_errors.write().insert(module, ts);

    ctx.reply(Message::Simple("Done.".to_string()))
}

fn set_enabled(ctx: &Context, args: &str, target: bool) -> Result<()> {
    let a = args.split(' ').collect::<Vec<&str>>();
    if a.len() < 2 {
        bail_user!("Usage: (enable/disable) config_id module [module [...]]");
    }

    let config_id = a[0];

    for m in &a[1..] {
        let mut db = ctx.bot().sql().lock();
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
