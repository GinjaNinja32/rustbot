use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::borrow::Cow;
use std::convert::Infallible;
use std::net::SocketAddr;

use rustbot::prelude::*;
use rustbot::{span, spans};

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd(
        "test",
        Command::new(|ctx, args| {
            ctx.say(&format!("beep boop {}", ctx.perms()?))?;
            ctx.say(&format!("you passed: {args}"))?;
            ctx.reply(Message::Spans(spans!(
                "simple ",
                span!(Format::Bold; "bold"),
                " ",
                span!(Format::Italic; "italic"),
                " ",
                span!(Format::Underline; "underline"),
                " ",
                span!(Color::Red; "red"),
                " ",
                span!(Color::Yellow; "yellow"),
                " ",
                span!(Color::Green; "green"),
                " ",
                span!(Color::Red + Format::Bold + Format::Italic; "bold_italic_red"),
            )))
        }),
    );

    meta.cmd("test2", Command::new(test2));

    thread!(meta, async {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

        let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(hello_world)) });

        let server = Server::bind(&addr).serve(make_svc);

        server.await
    })
}

fn test2(ctx: &dyn Context, args: &str) -> Result<()> {
    parse_args! {args,
        a: u64,
        b: Atom,
        c: Cow<str>,
    }

    ctx.reply(Message::Simple(format!("You passed {:?}", (a, b, c))))
}

async fn hello_world(_req: Request<Body>) -> std::result::Result<Response<Body>, Infallible> {
    info!("hello world server called");
    Ok(Response::new("Hello, World".into()))
}
