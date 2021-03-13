use crate::utils::*;
use rustbot::prelude::*;
use std::process::Command;

pub(crate) fn check_update(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;

    if server.id.is_none() {
        bail_user!("cannot check updates for a byond:// address");
    }
    let id = server.id.unwrap();

    if server.git_data.is_none() {
        bail_user!("no git data configured for '{}'", id);
    }
    let git = server.git_data.unwrap();

    let resp = get_topic_map(server.address.as_ref(), b"revision")?;
    let revision = match resp.get("revision") {
        Some(r) => r,
        None => bail_user!("server did not respond with a 'revision' key"),
    };

    let result = Command::new("./scripts/check_revision.sh")
        .arg(revision.as_ref())
        .arg(id.as_ref())
        .arg(&git.0)
        .arg(&git.1)
        .output()?;

    if !result.stdout.is_empty() {
        ctx.reply(Message::Simple(
            std::str::from_utf8(&result.stdout)?.trim_end().to_string(),
        ))
    } else {
        bail!(
            "failed to process check_update command for {}: {}",
            id,
            std::str::from_utf8(&result.stderr)?
        )
    }
}

pub(crate) fn pull_repo(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;

    if server.id.is_none() {
        bail_user!("cannot pull repo for a byond:// address");
    }
    let id = server.id.unwrap();

    if server.git_data.is_none() {
        bail_user!("no git data configured for '{}'", id);
    }
    let git = server.git_data.unwrap();

    let result = Command::new("./scripts/pull_repo.sh")
        .arg(id.as_ref())
        .arg(&git.0)
        .arg(&git.1)
        .output()?;

    if !result.stdout.is_empty() {
        ctx.reply(Message::Simple(
            std::str::from_utf8(&result.stdout)?.trim_end().to_string(),
        ))
    } else {
        bail!(
            "failed to process check_update command for {}: {}",
            id,
            std::str::from_utf8(&result.stderr)?
        )
    }
}
