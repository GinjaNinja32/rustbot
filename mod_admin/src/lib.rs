extern crate shared;

use shared::types;
use std::collections::BTreeMap;
use std::process::Command;
use std::str;

#[no_mangle]
pub fn get_meta() -> types::Meta {
    let mut commands: BTreeMap<String, types::Command> = BTreeMap::new();
    commands.insert("drop".to_string(), drop);
    commands.insert("load".to_string(), load);
    commands.insert("reload".to_string(), reload);
    commands.insert("recompile".to_string(), recompile);
    types::Meta { commands }
}

fn exec(ctx: &mut types::Context, args: &str, what: fn(&mut types::Context, &str)) {
    for m in args.split(' ') {
        what(ctx, m);
    }
    ctx.reply("done");
}

fn drop(ctx: &mut types::Context, args: &str) {
    exec(ctx, args, |ctx, m| ctx.bot().drop_module(m));
}

fn load(ctx: &mut types::Context, args: &str) {
    exec(ctx, args, |ctx, m| ctx.bot().load_module(m));
}

fn reload(ctx: &mut types::Context, args: &str) {
    exec(ctx, args, |ctx, m| {
        ctx.bot().drop_module(m);
        ctx.bot().load_module(m);
    });
}

fn recompile(ctx: &mut types::Context, args: &str) {
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    if !cfg!(debug_assertions) {
        cmd.arg("--release");
    }

    match cmd.output() {
        Ok(result) => {
            if result.status.success() {
                reload(ctx, args);
            } else {
                ctx.reply("compile failed:");
                for line in str::from_utf8(&result.stderr).unwrap().split('\n') {
                    if line.starts_with("   Compiling") {
                        continue;
                    }
                    if line == "" {
                        break;
                    }
                    ctx.reply(line);
                }
            }
        }
        Err(e) => ctx.reply(&format!("failed to run build: {}", e)),
    }
}
