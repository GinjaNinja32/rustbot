use rustbot::prelude::*;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

macro_rules! build_message {
    ($resp:ident, $fmt:literal, $($e:ident),*) => {
        {
            format!($fmt,
            $(
                $resp.get(stringify!($name)).unwrap_or(&Cow::Borrowed("?"))
            ),*
            )
        }
    }
}

#[derive(Debug)]
enum TopicResponse {
    Null,
    Float(f32),
    Text(String),
}

fn topic<A>(addr: A, msg: &[u8]) -> Result<TopicResponse>
where
    A: ToSocketAddrs,
{
    let mut stream = TcpStream::connect(addr)?;

    let len = (msg.len() + 6) as u16;
    stream.write_all(&[0, 131, (len >> 8) as u8, len as u8, 0, 0, 0, 0, 0])?;
    stream.write_all(msg)?;
    stream.write_all(&[0])?;

    let mut header = [0u8; 4];
    stream.read_exact(&mut header)?;

    let id = u16::from_be_bytes(header[0..2].try_into().unwrap());
    let len = u16::from_be_bytes(header[2..4].try_into().unwrap());

    if id != 131 {
        bail!("got Topic response packet with id = {}, expected 131", id);
    }
    if len == 0 {
        bail!("got Topic response packet with zero length, expected at least one byte");
    }

    let mut data = vec![0; len as usize];

    stream.read_exact(&mut data)?;

    let typ = data[0];
    let data = &data[1..];

    match typ {
        0x00 => {
            if !data.is_empty() {
                bail!(
                    "got Topic response packet with type=Null, but nonzero data size {}",
                    data.len()
                );
            }
            Ok(TopicResponse::Null)
        }
        0x06 => {
            if data[data.len() - 1] != 0 {
                bail!(
                    "got Topic response packet with type=Text and last byte {}, expected null terminator",
                    data[data.len() - 1]
                );
            }
            let text = std::str::from_utf8(&data[..data.len() - 1])?;
            // special case for throttled because otherwise it's very verbose to handle
            if text == "Throttled" {
                bail_user!("Hit rate limit");
            }
            if text.starts_with("Throttled (") && text.ends_with(')') {
                let throttle_reason = &text["Throttled (".len()..text.len() - 1];
                bail_user!("Hit rate limit ({})", throttle_reason);
            }
            Ok(TopicResponse::Text(text.to_string()))
        }
        0x2a => {
            if data.len() != 4 {
                bail!(
                    "got Topic response packet with type=Float but incorrect data size {}, expected 4",
                    data.len()
                );
            }
            Ok(TopicResponse::Float(f32::from_be_bytes(data.try_into().unwrap())))
        }
        _ => {
            bail!("got Topic response packet with unknown type byte {}", typ);
        }
    }
}

fn render_fields(data: &BTreeMap<String, Cow<'_, str>>, fields: &[(&str, &str)]) -> String {
    let mut v = vec![];

    for (name, key) in fields {
        if let Some(val) = data.get(*key) {
            v.push(format!("{}: {}", name, val));
        }
    }

    v.join(", ")
}

fn resolve_server(ctx: &dyn Context, args: &str) -> Result<String> {
    if let Some(s) = args.strip_prefix("byond://") {
        return Ok(s.parse()?);
    }

    if args.is_empty() {
        let addr = ctx.bot().sql().lock().query(
            "SELECT addr FROM ss13_servers JOIN ss13_server_channels USING (id) WHERE channel = '$default' OR channel = $1 ORDER BY channel ASC NULLS LAST LIMIT 1",
            &[&"asdf"],
        )?;

        if addr.is_empty() {
            bail_user!("no server name passed and no default configured");
        }

        return Ok(addr.get(0).unwrap().get::<_, String>(0));
    }

    let addr = ctx.bot().sql().lock().query(
        "SELECT addr FROM ss13_servers JOIN ss13_server_names USING (id) WHERE name = $1",
        &[&args],
    )?;
    if addr.is_empty() {
        bail_user!("unknown server name {:?}", args);
    }

    Ok(addr.get(0).unwrap().get::<_, String>(0))
}

fn get_topic_map<A>(addr: A, msg: &[u8]) -> Result<BTreeMap<String, Cow<'static, str>>>
where
    A: ToSocketAddrs,
{
    match topic(addr, msg)? {
        TopicResponse::Text(t) => Ok(parse_urlencoded(&t)),
        other => bail!("got Topic response with unexpected value {:?}", other),
    }
}

fn parse_urlencoded(t: &str) -> BTreeMap<String, Cow<'static, str>> {
    form_urlencoded::parse(t.as_bytes())
        .map(|(k, v)| (k.to_string(), v.to_string().into()))
        .collect::<BTreeMap<String, Cow<'static, str>>>()
}

pub(crate) fn status(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;
    let resp = get_topic_map(server, b"status=2")?;

    ctx.reply(Message::Simple(render_fields(
        &resp,
        &[
            ("Players", "players"),
            ("Active Players", "active_players"),
            ("Mode", "mode"),
            ("Station Time", "stationtime"),
            ("Round Duration", "roundduration"),
            ("Map", "map"),
        ],
    )))
}

pub(crate) fn address(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;

    ctx.reply(Message::Simple(format!("byond://{}", server)))
}

pub(crate) fn revision(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;
    let resp = get_topic_map(server, b"revision")?;

    ctx.reply(Message::Simple(build_message!(
        resp,
        "Revision: {} on {} at {}. Game ID: {}. DM: {}.{}; DD: {}.{}",
        revision,
        branch,
        date,
        gameid,
        dm_version,
        dm_build,
        dd_version,
        dd_build
    )))
}

pub(crate) fn mode(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;
    let resp = get_topic_map(server, b"status=2")?;

    ctx.reply(Message::Simple(build_message!(resp, "Mode: {}", mode)))
}

pub(crate) fn admins(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;
    let resp = get_topic_map(server, b"status=2")?;

    let admins = parse_urlencoded(
        resp.get("adminlist")
            .ok_or_else(|| Error::msg("got status=2 Topic response without adminlist key"))?,
    );

    if admins.is_empty() {
        ctx.reply(Message::Simple("No admins online.".to_string()))
    } else {
        ctx.reply(Message::List {
            prefix: format!("Admins ({}): ", admins.len()).into(),
            sep: "; ".into(),
            items: admins
                .iter()
                .map(|(name, rank)| format!("{} is {} {}", name, a(rank), rank).into())
                .collect::<Vec<_>>(),
        })
    }
}

fn a(s: &str) -> &'static str {
    if s.starts_with(|c| "aeiouAEIOU".contains(c)) {
        "an"
    } else {
        "a"
    }
}

pub(crate) fn players(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;
    let resp = get_topic_map(server, b"status=2")?;

    let players = parse_urlencoded(
        resp.get("playerlist")
            .ok_or_else(|| Error::msg("got status=2 Topic response without playerlist key"))?,
    );

    if players.is_empty() {
        ctx.reply(Message::Simple("No players online.".to_string()))
    } else {
        ctx.reply(Message::List {
            prefix: format!("Players ({}): ", players.len()).into(),
            sep: ", ".into(),
            items: players.keys().map(Into::into).collect(),
        })
    }
}

pub(crate) fn manifest(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;
    let resp = get_topic_map(server, b"manifest")?;

    let resp = resp
        .iter()
        .map(|(k, v)| (k, parse_urlencoded(v)))
        .collect::<BTreeMap<_, _>>();

    if resp.is_empty() {
        ctx.reply(Message::Simple("Manifest is empty.".to_string()))
    } else {
        let mut lines = vec![];
        for (dept, list) in resp {
            lines.push(format!(
                "{}: {}",
                dept,
                list.iter()
                    .map(|(name, job)| format!("{}: {}", name, job))
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        ctx.reply(Message::Simple(lines.join("\n")))
    }
}
