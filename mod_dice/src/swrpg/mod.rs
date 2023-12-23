use rand::seq::SliceRandom;
use rustbot::prelude::{span_join, Format, Span};
use rustbot::{span, spans};

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    combinator::{eof, map_res, opt},
    multi::{many0, many1},
    sequence::{preceded, tuple},
};
use nom::{IResult, Parser};

mod emoji;

fn format_dice<'a>(
    n: i8,
    positive_emoji: Span<'static>,
    positive: &'a str,
    positive_pl: &'a str,
    negative_emoji: Span<'static>,
    negative: &'a str,
    negative_pl: &'a str,
) -> Vec<Span<'a>> {
    if n < 0 {
        format_single(-n, negative_emoji, negative, negative_pl)
    } else {
        format_single(n, positive_emoji, positive, positive_pl)
    }
}

fn format_single<'a>(n: i8, emoji: Span<'static>, string: &'a str, pl: &'a str) -> Vec<Span<'a>> {
    match n {
        0 => vec![],
        1 => spans! {span!(Format::Bold; "1"), " ", emoji, " ", string},
        _ => spans! {span!(Format::Bold; "{}", n), " ", emoji, " ", string, pl},
    }
}

pub fn parse_and_eval(input: &str) -> Result<Vec<Span>, String> {
    let expr = line(input).map(|(_, c)| c).map_err(|e| format!("{e:?}"))?;

    let results: Vec<_> = expr
        .0
        .iter()
        .flat_map(|(n, die)| (0..*n).map(move |_| die.options().choose(&mut rand::thread_rng()).unwrap().clone()))
        .collect();

    let mut total = DR_ZERO;
    let mut dice_spans = vec![];

    for res in results {
        total = total + res.1;
        dice_spans.push(res.0);
    }

    let result_spans = if total == DR_ZERO {
        vec![spans! {"Neutral"}]
    } else {
        let mut result_spans = vec![];

        if total.triumph + total.despair == 0 {
            result_spans.push(format_dice(
                total.success_fail,
                emoji::RS,
                "Success",
                "es",
                emoji::RF,
                "Failure",
                "s",
            ));
        } else {
            let net_success_fail = total.success_fail + total.triumph - total.despair;
            let signum_delta = net_success_fail.signum() - total.success_fail.signum();
            if signum_delta.abs() == 2 {
                // Failure to success or vice versa
                result_spans.push(spans! {
                    format_dice(
                        total.success_fail,
                        emoji::RS,
                        "Success",
                        "es",
                        emoji::RF,
                        "Failure",
                        "s",
                    ),
                    " (net ",
                    format_dice(
                        net_success_fail,
                        emoji::RS,
                        "Success",
                        "es",
                        emoji::RF,
                        "Failure",
                        "s",
                    ),
                    ")",
                });
            } else {
                // Either unchanged, or one side is zero
                if total.success_fail + net_success_fail < 0 {
                    result_spans.push(spans! {
                        span!{Format::Bold; "{}", -total.success_fail}, " (net ",
                        span!{Format::Bold; "{}", -net_success_fail}, ") ",
                        emoji::RF, " Failures",
                    });
                } else {
                    result_spans.push(spans! {
                        span!{Format::Bold; "{}", total.success_fail}, " (net ",
                        span!{Format::Bold; "{}", net_success_fail}, ") ",
                        emoji::RS, " Successes",
                    });
                }
            }
        }
        result_spans.push(format_dice(
            total.advantage_threat,
            emoji::RA,
            "Advantage",
            "s",
            emoji::RT,
            "Threat",
            "s",
        ));
        result_spans.push(format_single(total.triumph, emoji::RTR, "Triumph", "s"));
        result_spans.push(format_single(total.despair, emoji::RD, "Despair", "s"));
        result_spans.push(format_single(total.light, emoji::RLF, "Light Side", ""));
        result_spans.push(format_single(total.dark, emoji::RDF, "Dark Side", ""));

        result_spans
    };

    Ok(spans! {
       span_join(dice_spans, ""),
       ": ",
       span_join(result_spans.into_iter().filter(|v| !v.is_empty()).collect(), ", "),
    })
}

fn line(i: &str) -> IResult<&str, Dice> {
    let (i, dice): (_, Vec<_>) = many1(tuple((number, die)))(i)?;
    let (i, extra): (_, Option<Vec<_>>) = opt(preceded(tag("+"), many0(tuple((number, extra_die)))))(i)?;
    let (i, _) = eof(i)?;

    let dice = {
        let mut v = vec![];
        v.extend_from_slice(&dice);
        if let Some(e) = extra {
            v.extend_from_slice(&e);
        }
        v
    };

    Ok((i, Dice(dice)))
}
struct Dice(Vec<(u8, Die)>);

fn die(i: &str) -> IResult<&str, Die> {
    alt((
        alt((tag("B"), tag("b"))).map(|_| Die::Boost),       // boost, blue
        alt((tag("S"), tag("s"))).map(|_| Die::Setback),     // setback, black
        alt((tag("A"), tag("g"))).map(|_| Die::Ability),     // ability, green
        alt((tag("D"), tag("p"))).map(|_| Die::Difficulty),  // difficulty, purple
        alt((tag("P"), tag("y"))).map(|_| Die::Proficiency), // proficiency, yellow
        alt((tag("C"), tag("r"))).map(|_| Die::Challenge),   // challenge, red
        alt((tag("F"), tag("w"))).map(|_| Die::Force),       // force, white
    ))(i)
}

fn extra_die(i: &str) -> IResult<&str, Die> {
    alt((
        alt((tag("S"), tag("s"))).map(|_| Die::AddSuccess),
        alt((tag("F"), tag("f"))).map(|_| Die::AddFailure),
        alt((tag("A"), tag("a"))).map(|_| Die::AddAdvantage),
        alt((tag("TR"), tag("tr"))).map(|_| Die::AddTriumph), // must precede threat
        alt((tag("T"), tag("t"))).map(|_| Die::AddThreat),
        alt((tag("D"), tag("d"))).map(|_| Die::AddDespair),
    ))(i)
}

#[derive(Copy, Clone)]
enum Die {
    Boost,
    Setback,
    Ability,
    Difficulty,
    Proficiency,
    Challenge,
    Force,

    AddSuccess,
    AddFailure,
    AddAdvantage,
    AddThreat,
    AddTriumph,
    AddDespair,
}

impl Die {
    fn options(self) -> Vec<(Span<'static>, DiceResult)> {
        match self {
            Self::Boost => vec![
                (emoji::B0, DR_ZERO),
                (emoji::B0, DR_ZERO),
                (emoji::BS, DR_SUCCESS),
                (emoji::BA, DR_ADVANTAGE),
                (emoji::BAA, DR_ADVANTAGE * 2),
                (emoji::BAS, DR_ADVANTAGE + DR_SUCCESS),
            ],
            Self::Setback => vec![(emoji::S0, DR_ZERO), (emoji::SF, DR_FAIL), (emoji::ST, DR_THREAT)],
            Self::Ability => vec![
                (emoji::A0, DR_ZERO),
                (emoji::AS, DR_SUCCESS),
                (emoji::AS, DR_SUCCESS),
                (emoji::ASS, DR_SUCCESS * 2),
                (emoji::AA, DR_ADVANTAGE),
                (emoji::AA, DR_ADVANTAGE),
                (emoji::AAA, DR_ADVANTAGE * 2),
                (emoji::AAS, DR_ADVANTAGE + DR_SUCCESS),
            ],
            Self::Difficulty => vec![
                (emoji::D0, DR_ZERO),
                (emoji::DF, DR_FAIL),
                (emoji::DFF, DR_FAIL * 2),
                (emoji::DT, DR_THREAT),
                (emoji::DT, DR_THREAT),
                (emoji::DT, DR_THREAT),
                (emoji::DTT, DR_THREAT * 2),
                (emoji::DTF, DR_THREAT + DR_FAIL),
            ],
            Self::Proficiency => vec![
                (emoji::P0, DR_ZERO),
                (emoji::PS, DR_SUCCESS),
                (emoji::PS, DR_SUCCESS),
                (emoji::PSS, DR_SUCCESS * 2),
                (emoji::PSS, DR_SUCCESS * 2),
                (emoji::PA, DR_ADVANTAGE),
                (emoji::PAA, DR_ADVANTAGE * 2),
                (emoji::PAA, DR_ADVANTAGE * 2),
                (emoji::PAS, DR_ADVANTAGE + DR_SUCCESS),
                (emoji::PAS, DR_ADVANTAGE + DR_SUCCESS),
                (emoji::PAS, DR_ADVANTAGE + DR_SUCCESS),
                (emoji::PT, DR_TRIUMPH),
            ],
            Self::Challenge => vec![
                (emoji::C0, DR_ZERO),
                (emoji::CF, DR_FAIL),
                (emoji::CF, DR_FAIL),
                (emoji::CFF, DR_FAIL * 2),
                (emoji::CFF, DR_FAIL * 2),
                (emoji::CT, DR_THREAT),
                (emoji::CT, DR_THREAT),
                (emoji::CTT, DR_THREAT * 2),
                (emoji::CTT, DR_THREAT * 2),
                (emoji::CTF, DR_THREAT + DR_FAIL),
                (emoji::CTF, DR_THREAT + DR_FAIL),
                (emoji::CD, DR_DESPAIR),
            ],
            Self::Force => vec![
                (emoji::FD, DR_DARK),
                (emoji::FD, DR_DARK),
                (emoji::FD, DR_DARK),
                (emoji::FD, DR_DARK),
                (emoji::FD, DR_DARK),
                (emoji::FD, DR_DARK),
                (emoji::FDD, DR_DARK * 2),
                (emoji::FL, DR_LIGHT),
                (emoji::FL, DR_LIGHT),
                (emoji::FLL, DR_LIGHT * 2),
                (emoji::FLL, DR_LIGHT * 2),
                (emoji::FLL, DR_LIGHT * 2),
            ],
            Self::AddSuccess => vec![(emoji::RS, DR_SUCCESS)],
            Self::AddFailure => vec![(emoji::RF, DR_FAIL)],
            Self::AddAdvantage => vec![(emoji::RA, DR_ADVANTAGE)],
            Self::AddThreat => vec![(emoji::RT, DR_THREAT)],
            Self::AddTriumph => vec![(emoji::RTR, DR_TRIUMPH)],
            Self::AddDespair => vec![(emoji::RD, DR_DESPAIR)],
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
struct DiceResult {
    success_fail: i8,
    advantage_threat: i8,
    triumph: i8,
    despair: i8,
    light: i8,
    dark: i8,
}

impl std::ops::Add<DiceResult> for DiceResult {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            success_fail: self.success_fail + rhs.success_fail,
            advantage_threat: self.advantage_threat + rhs.advantage_threat,
            triumph: self.triumph + rhs.triumph,
            despair: self.despair + rhs.despair,
            light: self.light + rhs.light,
            dark: self.dark + rhs.dark,
        }
    }
}

impl std::ops::Mul<i8> for DiceResult {
    type Output = Self;

    fn mul(self, rhs: i8) -> Self {
        Self {
            success_fail: self.success_fail * rhs,
            advantage_threat: self.advantage_threat * rhs,
            triumph: self.triumph * rhs,
            despair: self.despair * rhs,
            light: self.light * rhs,
            dark: self.dark * rhs,
        }
    }
}

const DR_ZERO: DiceResult = DiceResult {
    success_fail: 0,
    advantage_threat: 0,
    triumph: 0,
    despair: 0,
    light: 0,
    dark: 0,
};

const DR_SUCCESS: DiceResult = DiceResult {
    success_fail: 1,
    ..DR_ZERO
};

const DR_FAIL: DiceResult = DiceResult {
    success_fail: -1,
    ..DR_ZERO
};

const DR_ADVANTAGE: DiceResult = DiceResult {
    advantage_threat: 1,
    ..DR_ZERO
};

const DR_THREAT: DiceResult = DiceResult {
    advantage_threat: -1,
    ..DR_ZERO
};

const DR_TRIUMPH: DiceResult = DiceResult { triumph: 1, ..DR_ZERO };

const DR_DESPAIR: DiceResult = DiceResult { despair: 1, ..DR_ZERO };

const DR_LIGHT: DiceResult = DiceResult { light: 1, ..DR_ZERO };

const DR_DARK: DiceResult = DiceResult { dark: 1, ..DR_ZERO };

fn number(i: &str) -> IResult<&str, u8> {
    map_res(take_while(|c: char| c.is_ascii_digit()), str::parse)(i)
}
