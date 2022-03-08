use std::time::Duration;

use crate::prelude::*;

use nom::{Context as NomCtx, Err as NomErr};

named!(_parse_duration<&str, Duration>,
    do_parse!(
        d: alt!(do_parse!(d: unumber >> tag!("d") >> (d)) | value!(0)) >>
        h: alt!(do_parse!(h: unumber >> tag!("h") >> (h)) | value!(0)) >>
        m: alt!(do_parse!(m: unumber >> tag!("m") >> (m)) | value!(0)) >>
        s: alt!(do_parse!(s: unumber >> tag!("s") >> (s)) | value!(0)) >>
        tag!("\n") >>
        (Duration::from_secs(s + 60 * (m + 60 * (h + 24 * d))))
    )
);

named!(unumber<&str, u64>,
    map_res!(take_while!(|c: char| c.is_ascii_digit()), |s: &str| s.parse::<u64>())
);

pub fn parse_duration(s: &str) -> Result<Duration> {
    _parse_duration(&format!("{}\n", s))
        .map(|(_, d)| d) //
        .map_err(|e| match e {
            NomErr::Incomplete(_) => UserError::new("not enough input").into(),
            NomErr::Error(NomCtx::Code(i, _)) => {
                UserError::new(format!("unexpected input at {}", &i[..i.len() - 1])).into()
            }
            NomErr::Failure(f) => anyhow!("unknown Failure {:?}", f),
        })
}
