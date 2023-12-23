use rustbot::prelude::*;

use chrono::{DateTime, Datelike, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use chrono_tz::Tz;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd("time", Command::new(time));
}

fn usage(ctx: &dyn Context) -> Result<()> {
    ctx.reply(Message::Simple(
        "invalid argument count; expected timezone, or time plus one or two timezones".into(),
    ))
}
fn time(ctx: &dyn Context, args: &str) -> Result<()> {
    // !time <timezone> : find current time in TZ
    // !time <time> <timezone> : output discord timestamps for given time
    // !time <time> <src> <dst> : convert given timestamp between given TZs

    if args == "" {
        return usage(ctx);
    }

    let args = args.split(" ").collect::<Vec<_>>();

    let dst = parse_tz(args[args.len() - 1])?;

    if args.len() == 1 {
        return current_time(ctx, dst);
    }

    if let Ok(src) = parse_tz(args[args.len() - 2]) {
        return convert_timezone(ctx, parse_time(&args[..args.len() - 2], src)?, dst);
    }

    return timestamps_for(ctx, parse_time(&args[..args.len() - 1], dst)?);
}

fn parse_tz(tz: &str) -> Result<Tz> {
    Ok(Tz::from_str_insensitive(tz).map_err(|e| UserError::new("bad tz"))?)
}

fn current_time(ctx: &dyn Context, tz: Tz) -> Result<()> {
    let now = Utc::now();

    let now_tz = now.with_timezone(&tz);

    ctx.reply(Message::Simple(format!(
        "It is currently {}",
        now_tz.format("%Y-%m-%d %H:%M:%S %Z")
    )))
}

fn timestamps_for(ctx: &dyn Context, time: DateTime<Tz>) -> Result<()> {
    let utc = time.naive_utc();

    ctx.reply(Message::Simple(format!(
        "Provided timestamp: {}\nUTC: {}\nDiscord timestamps:\n{}",
        time.format("%Y-%m-%d %H:%M:%S %Z"),
        utc.format("%Y-%m-%d %H:%M:%S"),
        ["t", "T", "d", "D", "f", "F", "R"]
            .map(|c| format!("`<t:{}:{}>` <t:{}:{}>", utc.timestamp(), c, utc.timestamp(), c))
            .join("\n"),
    )))
}

fn convert_timezone(ctx: &dyn Context, time: DateTime<Tz>, dst: Tz) -> Result<()> {
    ctx.reply(Message::Simple(format!(
        "{} is {}",
        time.format("%Y-%m-%d %H:%M:%S %Z"),
        time.with_timezone(&dst).format("%Y-%m-%d %H:%M:%S %Z"),
    )))
}

fn parse_time(time: &[&str], tz: Tz) -> Result<DateTime<Tz>> {
    let mut ctx = TimeParseCtx { found: None };

    ctx.parse(
        time,
        tz,
        PartialDateTime {
            year: None,
            month: None,
            day: None,
            time: None,
        },
    )?;

    if let Some(ts) = ctx.found {
        match ts.and_local_timezone(tz) {
            LocalResult::None => Err(UserError::new("timestamp did not occur in provided timezone").into()),
            LocalResult::Ambiguous(_, _) => Err(UserError::new("timestamp is ambiguous in provided timezone").into()),

            LocalResult::Single(t) => Ok(t),
        }
    } else {
        Err(UserError::new("no valid timestamp parsed").into())
    }
}

struct TimeParseCtx {
    found: Option<NaiveDateTime>,
}

#[derive(Copy, Clone, Default, PartialEq)]
struct PartialDateTime {
    year: Option<i32>,
    month: Option<u32>,
    day: Option<u32>,

    time: Option<NaiveTime>,
}

impl TimeParseCtx {
    fn parse(&mut self, parts: &[&str], tz: Tz, partial: PartialDateTime) -> Result<()> {
        if parts.len() == 0 {
            if let (Some(year), Some(month), Some(day), Some(time)) =
                (partial.year, partial.month, partial.day, partial.time)
            {
                if self.found.is_some() {
                    return Err(UserError::new("multiple valid parse results").into());
                }

                let date = NaiveDate::from_ymd_opt(year, month, day)
                    .ok_or_else(|| UserError::new("invalid y/m/d specified"))?;

                self.found = Some(date.and_time(time));
                return Ok(());
            }

            if partial.year.is_none() || partial.month.is_none() || partial.day.is_none() {
                return Err(UserError::new("incomplete date specified").into());
            }
            if partial.time.is_none() {
                return Err(UserError::new("no time specified").into());
            }
        }

        for fmt in &[
            "%Y-%m-%dT%H:%M:%S%.f",
            "%Y-%m-%dT%H:%M:%S",
            "%Y-%m-%dT%H:%M\\:%S%.f",
            "%Y-%m-%dT%H:%M\\:%S",
        ] {
            if let Ok(datetime) = NaiveDateTime::parse_from_str(parts[0], fmt) {
                if parts.len() != 1 || partial != PartialDateTime::default() {
                    return Err(UserError::new("overspecified date").into());
                }

                self.found = Some(datetime);
                return Ok(());
            }
        }

        if let Some(date) = match parts[0].to_lowercase().as_str() {
            "today" => Some(Utc::now().with_timezone(&tz).date()),
            "tomorrow" => Utc::now().with_timezone(&tz).date().succ_opt(),
            "yesterday" => Utc::now().with_timezone(&tz).date().pred_opt(),

            _ => None,
        } {
            if partial.year.is_some() || partial.month.is_some() || partial.day.is_some() {
                return Err(UserError::new("multiple dates specified").into());
            }

            return self.parse(
                &parts[1..],
                tz,
                PartialDateTime {
                    year: Some(date.year()),
                    month: Some(date.month()),
                    day: Some(date.day()),

                    ..partial
                },
            );
        }

        for fmt in &["%H:%M", "%H:%M:%S", "%H:%M:%S%.f"] {
            if let Ok(time) = NaiveTime::parse_from_str(parts[0], fmt) {
                if partial.time.is_some() {
                    return Err(UserError::new("multiple times specified").into());
                }

                let next = PartialDateTime {
                    time: Some(time),
                    ..partial
                };
                return self.parse(&parts[1..], tz, next);
            }
        }

        let sub = parts[0].split(['-', '/']).collect::<Vec<_>>();

        self.permute_parts(&sub, &parts[1..], tz, partial)
    }

    fn permute_parts(&mut self, parts: &[&str], rest: &[&str], tz: Tz, partial: PartialDateTime) -> Result<()> {
        match parts.len() {
            3 => {
                if partial.year.is_some() || partial.day.is_some() || partial.month.is_some() {
                    return Err(UserError::new("overspecified date").into());
                }

                if let Ok(year) = try_year(parts[0]) {
                    if let Ok(month) = try_month(parts[1]) {
                        if let Ok(day) = try_day(parts[2]) {
                            self.parse(
                                rest,
                                tz,
                                PartialDateTime {
                                    year: Some(year),
                                    month: Some(month),
                                    day: Some(day),
                                    ..partial
                                },
                            )?;
                        }
                    }
                }

                let year = try_year(parts[2])?;

                if let Ok(month) = try_month(parts[0]) {
                    if let Ok(day) = try_day(parts[1]) {
                        self.parse(
                            rest,
                            tz,
                            PartialDateTime {
                                year: Some(year),
                                month: Some(month),
                                day: Some(day),
                                ..partial
                            },
                        )?;
                    }
                }
                if let Ok(month) = try_month(parts[1]) {
                    if let Ok(day) = try_day(parts[0]) {
                        self.parse(
                            rest,
                            tz,
                            PartialDateTime {
                                year: Some(year),
                                month: Some(month),
                                day: Some(day),
                                ..partial
                            },
                        )?;
                    }
                }
            }
            2 => {
                if partial.day.is_some() || partial.month.is_some() {
                    return Err(UserError::new("overspecified date").into());
                }

                if let Ok(month) = try_month(parts[0]) {
                    if let Ok(day) = try_day(parts[1]) {
                        self.parse(
                            rest,
                            tz,
                            PartialDateTime {
                                month: Some(month),
                                day: Some(day),
                                ..partial
                            },
                        )?;
                    }
                }
                if let Ok(month) = try_month(parts[1]) {
                    if let Ok(day) = try_day(parts[0]) {
                        self.parse(
                            rest,
                            tz,
                            PartialDateTime {
                                month: Some(month),
                                day: Some(day),
                                ..partial
                            },
                        )?;
                    }
                }
            }
            1 => {
                if partial.year.is_none() {
                    if let Ok(year) = try_year(parts[0]) {
                        self.parse(
                            rest,
                            tz,
                            PartialDateTime {
                                year: Some(year),
                                ..partial
                            },
                        )?;
                    }
                }
                if partial.month.is_none() {
                    if let Ok(month) = try_month(parts[0]) {
                        self.parse(
                            rest,
                            tz,
                            PartialDateTime {
                                month: Some(month),
                                ..partial
                            },
                        )?;
                    }
                }
                if partial.day.is_none() {
                    if let Ok(day) = try_day(parts[0]) {
                        self.parse(
                            rest,
                            tz,
                            PartialDateTime {
                                day: Some(day),
                                ..partial
                            },
                        )?;
                    }
                }
            }
            _ => {
                return Err(UserError::new("unrecognised date format").into());
            }
        }
        Ok(())
    }
}

fn try_year(input: &str) -> Result<i32> {
    input
        .parse()
        .map_err(|_| UserError::new(format!("could not parse {:?} as a year", input)).into())
}

fn try_month(input: &str) -> Result<u32> {
    if let Ok(n) = input.parse() {
        if 1 <= n && n <= 12 {
            return Ok(n);
        }
    }

    match input.to_lowercase().as_str() {
        "jan" | "january" => Ok(1),
        "feb" | "february" => Ok(2),
        "mar" | "march" => Ok(3),
        "apr" | "april" => Ok(4),
        "may" => Ok(5),
        "jun" | "june" => Ok(6),
        "jul" | "july" => Ok(7),
        "aug" | "august" => Ok(8),
        "sep" | "september" => Ok(9),
        "oct" | "october" => Ok(10),
        "nov" | "november" => Ok(11),
        "dec" | "december" => Ok(12),

        _ => Err(UserError::new(format!("could not parse {:?} as a month", input)).into()),
    }
}

fn try_day(input: &str) -> Result<u32> {
    if let Ok(n) = input.parse() {
        if 1 <= n && n <= 31 {
            return Ok(n);
        }
    }

    Err(UserError::new(format!("could not parse {:?} as a day", input)).into())
}
