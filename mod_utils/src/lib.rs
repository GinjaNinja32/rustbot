use rustbot::prelude::*;
use std::borrow::Cow;
use std::process::Command as StdCommand;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd("units", Command::new(units));
}

struct FromUnits(String);
impl<'a> Arg<'a> for FromUnits {
    fn parse_from<'s: 'a>(input: &'s str) -> Result<(Self, Option<&'s str>)> {
        if input.is_empty() {
            bail_user!("missing argument")
        }

        let (this, rest) = match input.split_once(" to ") {
            Some(v) => v,
            None => bail_user!("missing separator"),
        };

        Ok((FromUnits(this.to_string()), Some(rest)))
    }
    fn describe_expected() -> Cow<'static, str> {
        Cow::Borrowed("units-from 'to'")
    }
}

fn units(ctx: &dyn Context, args: &str) -> Result<()> {
    parse_args! {args,
        from: Option<FromUnits>,
        to: rustbot::args::Rest,
    }

    if cfg!(target_os = "windows") {
        bail_user!("unsupported");
    }

    let mut cmd = StdCommand::new("units");
    cmd.arg("-t").arg("--");

    if let Some(from) = from {
        cmd.arg(from.0);
    }
    cmd.arg(to.0);

    let result = cmd.output()?;

    let stdout = String::from_utf8(result.stdout)?;

    return ctx.reply(Message::Simple(stdout));
}
