use rustbot::prelude::*;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

#[derive(Debug)]
pub(crate) enum TopicResponse {
    Null,
    Float(f32),
    Text(String),
}

fn connect<A>(addr: A) -> Result<TcpStream>
where
    A: ToSocketAddrs,
{
    let addrs = addr.to_socket_addrs()?;

    for try_addr in addrs {
        match TcpStream::connect_timeout(&try_addr, Duration::from_secs(5)) {
            Ok(s) => return Ok(s),
            Err(e) => {
                warn!("failed to connect to {}: {}", try_addr, e);
            }
        }
    }

    bail!("all addresses failed")
}

pub(crate) fn topic<A>(addr: A, msg: &[u8]) -> Result<TopicResponse>
where
    A: ToSocketAddrs,
{
    let mut stream = connect(addr)?;

    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;

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

pub(crate) fn render_fields(data: &BTreeMap<String, Cow<'_, str>>, fields: &[(&str, &str)]) -> String {
    let mut v = vec![];

    for (name, key) in fields {
        if let Some(val) = data.get(*key) {
            v.push(format!("{name}: {val}"));
        }
    }

    v.join(", ")
}

pub(crate) struct ResolvedServer<'a> {
    pub(crate) prefix: Cow<'a, str>,
    pub(crate) id: Option<Cow<'a, str>>,
    pub(crate) address: Cow<'a, str>,
    pub(crate) git_data: Option<(String, String)>, // (repo_url, branch)
}

pub(crate) fn resolve_server<'a>(ctx: &dyn Context, args: &'a str) -> Result<ResolvedServer<'a>> {
    if let Some(s) = args.strip_prefix("byond://") {
        return Ok(ResolvedServer {
            prefix: format!("(byond://{s}) ").into(),
            id: None,
            address: s.into(),
            git_data: None,
        });
    }

    let addr = if args.is_empty() {
        let channel = format!("{}:{}", ctx.config_id(), ctx.source().channel_string());

        let addr = ctx.bot().sql().lock().query(
            "SELECT id, addr, repo_url, branch FROM ss13_servers JOIN ss13_server_channels USING (id) LEFT JOIN ss13_repositories USING (id) WHERE $1 LIKE channel ORDER BY channel DESC LIMIT 1",
            &[&channel],
        )?;
        if addr.is_empty() {
            bail_user!("no server name passed and no default configured");
        }
        addr
    } else {
        let addr = ctx.bot().sql().lock().query(
            "SELECT id, addr, repo_url, branch FROM ss13_servers JOIN ss13_server_names USING (id) LEFT JOIN ss13_repositories USING (id) WHERE name = $1",
            &[&args],
        )?;
        if addr.is_empty() {
            bail_user!("unknown server name {:?}", args);
        }
        addr
    };

    let row = addr.get(0).unwrap();

    let id: String = row.get(0);
    let address: String = row.get(1);

    let repo_url: Option<String> = row.get(2);
    let branch: Option<String> = row.get(3);

    Ok(ResolvedServer {
        prefix: if args.is_empty() {
            "".into()
        } else {
            format!("({id}) ").into()
        },
        id: Some(id.into()),
        address: address.into(),
        git_data: repo_url.map(|ru| (ru, branch.unwrap())),
    })
}

pub(crate) fn get_topic_map<A>(addr: A, msg: &[u8]) -> Result<BTreeMap<String, Cow<'static, str>>>
where
    A: ToSocketAddrs,
{
    match topic(addr, msg)? {
        TopicResponse::Text(t) => Ok(parse_urlencoded(&t)),
        other => bail!("got Topic response with unexpected value {:?}", other),
    }
}

pub(crate) fn parse_urlencoded(t: &str) -> BTreeMap<String, Cow<'static, str>> {
    form_urlencoded::parse(t.as_bytes())
        .map(|(k, v)| (k.to_string(), v.to_string().into()))
        .collect::<BTreeMap<String, Cow<'static, str>>>()
}
