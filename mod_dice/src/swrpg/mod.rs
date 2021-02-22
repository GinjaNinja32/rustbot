use rand::seq::SliceRandom;
use rustbot::prelude::{span_join, Format, Span};
use rustbot::{span, spans};

mod emoji;

named!(space<&str,&str>, eat_separator!(" \t"));
macro_rules! sp (
  ($i:expr, $($args:tt)*) => (
    {
      match sep!($i, space, $($args)*) {
        Err(e) => Err(e),
        Ok((i1,o))    => {
          match space(i1) {
            Err(e) => Err(e),
            Ok((i2,_))    => Ok((i2, o))
          }
        }
      }
    }
  )
);

fn format_dice<'a>(
    n: i8,
    positive_emoji: Span<'static>,
    positive: &'a str,
    positive_pl: &'a str,
    negative_emoji: Span<'static>,
    negative: &'a str,
    negative_pl: &'a str,
) -> Vec<Vec<Span<'a>>> {
    if n < 0 {
        format_single(-n, negative_emoji, negative, negative_pl)
    } else {
        format_single(n, positive_emoji, positive, positive_pl)
    }
}

fn format_single<'a>(n: i8, emoji: Span<'static>, string: &'a str, pl: &'a str) -> Vec<Vec<Span<'a>>> {
    match n {
        0 => vec![],
        1 => vec![spans! {span!(Format::Bold; "1"), " ", emoji, " ", string}],
        _ => vec![spans! {span!(Format::Bold; "{}", n), " ", emoji, " ", string, pl}],
    }
}

pub fn parse_and_eval(input: &str) -> Result<Vec<Span>, String> {
    let expr = line(&format!("{}\n", input))
        .map(|(_, c)| c)
        .map_err(|e| format!("{:?}", e))?;

    let results: Vec<_> = expr
        .0
        .iter()
        .map(|(n, die)| (0..*n).map(move |_| die.options().choose(&mut rand::thread_rng()).unwrap().clone()))
        .flatten()
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
            result_spans.append(&mut format_dice(
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
                    span_join(format_dice(
                        total.success_fail,
                        emoji::RS,
                        "Success",
                        "es",
                        emoji::RF,
                        "Failure",
                        "s",
                    ), ""),
                    " (net ",
                    span_join(format_dice(
                        net_success_fail,
                        emoji::RS,
                        "Success",
                        "es",
                        emoji::RF,
                        "Failure",
                        "s",
                    ), ""),
                    ")",
                });
            } else {
                // Either unchanged, or one side is zero
                if total.success_fail + net_success_fail < 0 {
                    result_spans.push(spans! {
                        span!{Format::Bold; "{}", -total.success_fail}, " (net ",
                        span!{Format::Bold; "{}", -net_success_fail}, ") ",
                        emoji::RF, "Failures",
                    });
                } else {
                    result_spans.push(spans! {
                        span!{Format::Bold; "{}", total.success_fail}, " (net ",
                        span!{Format::Bold; "{}", net_success_fail}, ") ",
                        emoji::RS, "Successes",
                    });
                }
            }
        }
        result_spans.append(&mut format_dice(
            total.advantage_threat,
            emoji::RA,
            "Advantage",
            "s",
            emoji::RT,
            "Threat",
            "s",
        ));
        result_spans.append(&mut format_single(total.triumph, emoji::RTR, "Triumph", "s"));
        result_spans.append(&mut format_single(total.despair, emoji::RD, "Despair", "s"));
        result_spans.append(&mut format_single(total.light, emoji::RLF, "Light Side", ""));
        result_spans.append(&mut format_single(total.dark, emoji::RDF, "Dark Side", ""));

        result_spans
    };

    Ok(spans! {
       span_join(dice_spans, ""),
       ": ",
       span_join(result_spans, ", "),
    })
}

named!(line<&str, Dice>, do_parse!(
    dice: terminated!(many1!(complete!(tuple!(number, die))), tag!("\n")) >>
    ( Dice(dice) )
));

struct Dice(Vec<(u8, Die)>);

named!(die<&str, Die>, sp!(alt!(
    value!(Die::Boost,       alt!(tag!("B") | tag!("b"))) | // boost, blue
    value!(Die::Setback,     alt!(tag!("S") | tag!("s"))) | // setback, black
    value!(Die::Ability,     alt!(tag!("A") | tag!("g"))) | // ability, green
    value!(Die::Difficulty,  alt!(tag!("D") | tag!("p"))) | // difficulty, purple
    value!(Die::Proficiency, alt!(tag!("P") | tag!("y"))) | // proficiency, yellow
    value!(Die::Challenge,   alt!(tag!("C") | tag!("r"))) | // challenge, red
    value!(Die::Force,       alt!(tag!("F") | tag!("w")))   // force, white
)));

#[derive(Copy, Clone)]
enum Die {
    Boost,
    Setback,
    Ability,
    Difficulty,
    Proficiency,
    Challenge,
    Force,
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

named!(number<&str, u8>,
    map_res!(take_while!(is_digit), |s: &str| s.parse::<u8>())
);

fn is_digit(c: char) -> bool {
    '0' <= c && c <= '9'
}
