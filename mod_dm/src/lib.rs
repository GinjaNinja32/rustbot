extern crate shared;

use shared::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::Hasher;
use std::io::prelude::*;
use std::process::Command as ProcessCommand;

#[no_mangle]
pub fn get_meta() -> Meta {
    let mut meta = Meta::new();
    meta.cmd("dm", Command::new(|ctx, args| dm(ctx, args, false, false)));
    meta.cmd("dml", Command::new(|ctx, args| dm(ctx, args, false, true)));
    meta.cmd(
        "dms",
        Command::new(|ctx, args| dm(ctx, args, true, false)).req_perms(Perms::Admin),
    );
    meta.cmd(
        "dmsl",
        Command::new(|ctx, args| dm(ctx, args, true, true)).req_perms(Perms::Admin),
    );
    meta
}

fn dm(ctx: &Context, args: &str, secure: bool, multiline: bool) -> Result<()> {
    // Security check
    if !secure {
        if args.contains("##") || args.contains("include") {
            ctx.say("You attempted to use either ## or include; both are blocked for security reasons.")?;
            if ctx.perms()?.contains(Perms::Admin) {
                ctx.say("Use !dms or !dmsl to bypass this warning")?;
            }
            return Ok(());
        }
    }

    let args = args.clone().trim().trim_matches('`');

    let code: String = if args.contains("\n") {
        if args.contains("\nMAIN\n") || args.contains("\nproc/main()\n") || args.contains("\n/proc/main()\n") {
            format!(
                r#"
#include "util.dm"
/world/loop_checks = 0
/world/New()
    main()
    del(src)
{}
"#,
                args
            )
        } else {
            format!(
                r#"
#include "util.dm"
/world/loop_checks = 0
/world/New()
    main()
    del(src)
/proc/main()
    {}
"#,
                args.replace("\n", "\n    ")
            )
        }
    } else {
        let (pre, main) = {
            let parts: Vec<&str> = args.splitn(2, ";;;").collect();
            if parts.len() == 1 {
                ("", parts[0])
            } else {
                (parts[0], parts[1])
            }
        };

        let pre_lines: Vec<&str> = pre.split(";;").collect();
        let main_lines: Vec<&str> = main.split(";;").collect();
        let setup = &main_lines[0..main_lines.len() - 1];

        let mut value = main_lines[main_lines.len() - 1].to_string();
        if value != "" {
            value = format!(r#"world.log << "[{}]""#, value);
        }

        format!(
            r#"
#include "util.dm"
/world/loop_checks = 0
{}
/world/New()
    main()
    del(src)
/proc/main()
    {}
    {}
"#,
            pre_lines.join("\n"),
            setup.join("\n    "),
            value
        )
    };

    let mut hasher = DefaultHasher::new();
    hasher.write(code.as_bytes());
    let name = format!("{:x}", hasher.finish());

    let mut file = File::create(format!("dm/{}.dme", name))?;
    file.write_all(code.as_bytes())?;

    let result = ProcessCommand::new("scripts/dm_compile_run.sh")
        .arg(&name)
        .env("multiline", format!("{}", multiline))
        .env("secure", format!("{}", secure))
        .output()?;

    let stdout = String::from_utf8(result.stdout)?;
    if stdout == "" {
        ctx.say("<no output>")
    } else {
        ctx.reply(Message::Code(stdout))
    }
}
