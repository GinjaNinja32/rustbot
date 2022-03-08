use log::Level;
use rustbot::prelude::*;
use std::process::Command as ProcessCommand;
use std::str;
use std::time::{Duration, Instant};

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd("drop", Command::new(drop).req_perms(Perms::Modules));
    meta.cmd("load", Command::new(load).req_perms(Perms::Modules));
    meta.cmd("reload", Command::new(reload).req_perms(Perms::Modules));
    meta.cmd("recompile", Command::new(recompile).req_perms(Perms::Modules));
    meta.cmd("log", Command::new(log).req_perms(Perms::Modules));
    meta.cmd("suppress", Command::new(suppress).req_perms(Perms::Modules));
    meta.cmd(
        "enable",
        Command::new(move |ctx, args| set_enabled(ctx, args, true)).req_perms(Perms::Modules),
    );
    meta.cmd(
        "disable",
        Command::new(move |ctx, args| set_enabled(ctx, args, false)).req_perms(Perms::Modules),
    );
}

fn exec(ctx: &dyn Context, args: &str, what: fn(&dyn Context, &str) -> Result<()>) -> Result<()> {
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

fn drop(ctx: &dyn Context, args: &str) -> Result<()> {
    exec(ctx, args, |ctx, m| ctx.bot().drop_module(m))
}

fn load(ctx: &dyn Context, args: &str) -> Result<()> {
    exec(ctx, args, |ctx, m| ctx.bot().load_module(m))
}

fn reload(ctx: &dyn Context, args: &str) -> Result<()> {
    exec(ctx, args, |ctx, m| {
        ctx.bot().drop_module(m)?;
        ctx.bot().load_module(m)
    })
}

fn recompile(ctx: &dyn Context, args: &str) -> Result<()> {
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

fn set_enabled(ctx: &dyn Context, args: &str, target: bool) -> Result<()> {
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

fn suppress(ctx: &dyn Context, args: &str) -> Result<()> {
    let a = args.split(' ').collect::<Vec<&str>>();
    if a.len() != 2 {
        bail_user!("Usage: suppress <module> <seconds>");
    }

    let module = a[0].to_string();
    let seconds = match a[1].parse::<u64>() {
        Ok(v) => v,
        Err(e) => bail_user!("invalid duration: {}", e),
    };

    let ts = Instant::now() + Duration::from_secs(seconds);

    ctx.bot().suppress_errors(module, ts);
    ctx.reply(Message::Simple("Done.".to_string()))
}

fn log(ctx: &dyn Context, args: &str) -> Result<()> {
    let a = args.split(' ').collect::<Vec<_>>();
    match a.as_slice() {
        [module, level] => {
            let level = parse_log_level(level)?;
            ctx.bot().set_module_log_level(module, level)?;
        }
        [level] => match parse_log_level(level)? {
            Some(level) => ctx.bot().set_log_level(level)?,
            None => bail_user!("invalid log level specification"),
        },
        _ => bail_user!("unknown argument format; try 'log LEVEL' or 'log MODULE LEVEL'"),
    }
    ctx.reply(Message::Simple("Done".to_string()))
}
