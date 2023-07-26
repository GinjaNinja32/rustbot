use std::time::Duration;

use crate::prelude::*;

use nom::{bytes::complete::*, combinator::*, sequence::*};
use nom::{error::Error as NomError, Finish, IResult};

fn _parse_duration(i: &str) -> IResult<&str, Duration> {
    let (i, d) = opt(terminated(unumber, tag("d")))(i)?;
    let (i, h) = opt(terminated(unumber, tag("h")))(i)?;
    let (i, m) = opt(terminated(unumber, tag("m")))(i)?;
    let (i, s) = opt(terminated(unumber, tag("s")))(i)?;
    let (i, _) = eof(i)?;

    let d = d.unwrap_or(0);
    let h = h.unwrap_or(0);
    let m = m.unwrap_or(0);
    let s = s.unwrap_or(0);

    Ok((i, Duration::from_secs(s + 60 * (m + 60 * (h + 24 * d)))))
}

fn unumber(i: &str) -> IResult<&str, u64> {
    map_res(take_while(|c: char| c.is_ascii_digit()), |s: &str| s.parse::<u64>())(i)
}

pub fn parse_duration(s: &str) -> Result<Duration> {
    _parse_duration(s)
        .finish()
        .map(|(_, d)| d)
        .map_err(|NomError { input, .. }| UserError::new(format!("unexpected input at {}", input)).into())
}
