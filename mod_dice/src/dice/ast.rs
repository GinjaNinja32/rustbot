use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{self, Display};

use rustbot::prelude::{span_join, Color, Format, Span};
use rustbot::{span, spans};

use super::limits::Limiter;
use super::value::Value;
use super::Evaluable;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{anychar, multispace0},
    combinator::{eof, map_res, opt},
    multi::{many0, separated_list0, separated_list1},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
};
use nom::{error::ParseError, IResult, Parser};

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
/// trailing whitespace, returning the output of `inner`.
fn ws<'a, F: 'a, O, E: ParseError<&'a str>>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

pub fn command(i: &str) -> IResult<&str, Command> {
    let (i, res) = alt((
        tuple((
            separated_list1(ws(tag(";")), tuple((terminated(anychar, tag(":")), expression))),
            preceded(ws(tag(";")), many0(output_segment)),
        ))
        .map(|(bindings, output)| Command::Complex { bindings, output }),
        expression.map(Command::Simple),
    ))(i)?;
    let (i, _) = eof(i)?;

    Ok((i, res))
}
pub enum Command {
    Simple(Expression),
    Complex {
        bindings: Vec<(char, Expression)>,
        output: Vec<OutputSegment>,
    },
}
impl Command {
    pub fn eval(&self, limit: &mut Limiter) -> Result<Vec<Span>, String> {
        match self {
            Self::Simple(expr) => {
                let (s, v) = expr.eval(limit, &BTreeMap::new())?;

                Ok(spans!(v.to_string(), ": ", s))
            }
            Self::Complex { bindings, output } => {
                let mut vals = BTreeMap::new();

                for (ch, expr) in bindings {
                    let (_, v) = expr.eval(limit, &vals)?;
                    vals.insert(*ch, v);
                }

                let mut spans = vec![];
                let mut last_plural = false;
                for seg in output {
                    match seg {
                        OutputSegment::Text(s) => spans.push(span! {s}),
                        OutputSegment::Value(ch) => match vals.get(ch) {
                            Some(v) => {
                                if let Value::Int(1) = v {
                                    last_plural = false
                                } else {
                                    last_plural = true
                                }
                                spans.push(span! {format!("{}", v)});
                            }
                            None => {
                                return Err(format!("binding ${ch} not defined"));
                            }
                        },
                        OutputSegment::Plural(sg, pl) => {
                            if last_plural {
                                spans.push(span! {pl});
                            } else {
                                spans.push(span! {sg});
                            }
                        }
                    }
                }

                Ok(spans)
            }
        }
    }
}

fn output_segment(i: &str) -> IResult<&str, OutputSegment> {
    alt((
        preceded(
            tag("%"),
            alt((
                tag("s").map(|_| OutputSegment::Plural(String::new(), "s".into())),
                delimited(
                    tag("["),
                    separated_pair(
                        take_while(|c| c != '|').map(String::from),
                        tag("|"),
                        take_while(|c| c != ']').map(String::from),
                    ),
                    tag("]"),
                )
                .map(|(sg, pl)| OutputSegment::Plural(sg, pl)),
                delimited(tag("["), take_while(|c| c != ']').map(String::from), tag("]"))
                    .map(|pl| OutputSegment::Plural(String::new(), pl)),
            )),
        ),
        preceded(tag("$"), anychar).map(OutputSegment::Value),
        take_while1(|c| c != '%' && c != '$')
            .map(String::from)
            .map(OutputSegment::Text),
    ))(i)
}
pub enum OutputSegment {
    Text(String),
    Value(char),
    Plural(String, String),
}

fn expression(i: &str) -> IResult<&str, Expression> {
    let (i, repeat) = ws(repeat)(i)?;

    Ok((i, Expression { repeat }))
}
#[derive(Debug)]
pub struct Expression {
    pub repeat: Repeat, // ...
}
impl Evaluable for Expression {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        self.repeat.eval(limit, values)
    }
}

fn repeat(i: &str) -> IResult<&str, Repeat> {
    alt((
        tuple((number, ws(tag("#")), comparison)).map(|(r, _, term)| Repeat { repeat: Some(r), term }),
        comparison.map(|term| Repeat { repeat: None, term }),
    ))(i)
}
#[derive(Debug)]
pub struct Repeat {
    pub repeat: Option<i64>, // ( integer "#" )?
    pub term: Comparison,    // ...
}
impl Evaluable for Repeat {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        match self.repeat {
            None => self.term.eval(limit, values),
            Some(n) => {
                let (strs, vals) = (0..n)
                    .map(|_| {
                        let (s, v) = self.term.eval(limit, values)?;
                        Ok((s, v.to_int()?))
                    })
                    .collect::<Result<Vec<(Vec<Span>, _)>, String>>()?
                    .drain(..)
                    .unzip();

                Ok((span_join(strs, ", "), Value::IntSlice(vals)))
            }
        }
    }
}

fn comparison(i: &str) -> IResult<&str, Comparison> {
    let (i, left) = addsub(i)?;
    let (i, right) = opt(tuple((ws(compare_op), addsub)))(i)?;

    Ok((i, Comparison { left, right }))
}
#[derive(Debug)]
pub struct Comparison {
    pub left: AddSub,                       // ...
    pub right: Option<(CompareOp, AddSub)>, // ( operator ... )?
}
impl Evaluable for Comparison {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        let l = self.left.eval(limit, values)?;
        match &self.right {
            None => Ok(l),
            Some((op, term)) => {
                let r = term.eval(limit, values)?;
                let v = op.apply(l.1, r.1)?;

                Ok((spans!(l.0, format!("{}", op), r.0), v))
            }
        }
    }
}

fn addsub(i: &str) -> IResult<&str, AddSub> {
    let (i, left) = muldiv(i)?;
    let (i, right) = many0(tuple((ws(addsub_op), muldiv)))(i)?;

    Ok((i, AddSub { left, right }))
}
#[derive(Debug)]
pub struct AddSub {
    pub left: MulDiv,                   // ...
    pub right: Vec<(AddSubOp, MulDiv)>, // ( operator ... )*
}
impl Evaluable for AddSub {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        let (s, mut l) = self.left.eval(limit, values)?;
        let mut ss = s;
        for elem in &self.right {
            let (mut rs, r) = elem.1.eval(limit, values)?;

            ss.push(format!("{}", elem.0).into());
            ss.append(&mut rs);
            l = elem.0.apply(l, r)?;
        }
        Ok((ss, l))
    }
}

fn muldiv(i: &str) -> IResult<&str, MulDiv> {
    let (i, left) = sum(i)?;
    let (i, right) = many0(tuple((ws(muldiv_op), sum)))(i)?;

    Ok((i, MulDiv { left, right }))
}
#[derive(Debug)]
pub struct MulDiv {
    pub left: Sum,                   // ...
    pub right: Vec<(MulDivOp, Sum)>, // ( operator ... )*
}
impl Evaluable for MulDiv {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        let (s, mut l) = self.left.eval(limit, values)?;
        let mut ss = s;
        for elem in &self.right {
            let (mut rs, r) = elem.1.eval(limit, values)?;

            ss.push(format!("{}", elem.0).into());
            ss.append(&mut rs);
            l = elem.0.apply(l, r)?;
        }
        Ok((ss, l))
    }
}

fn sum(i: &str) -> IResult<&str, Sum> {
    let (i, is_sum) = opt(tag("s")).map(|o| o.is_some()).parse(i)?;
    let (i, _) = multispace0(i)?;
    let (i, term) = dicemod(i)?;

    Ok((i, Sum { is_sum, term }))
}
#[derive(Debug)]
pub struct Sum {
    pub is_sum: bool,  // ( "s" )?
    pub term: DiceMod, // ...
}
impl Evaluable for Sum {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        let (s, v) = self.term.eval(limit, values)?;
        if self.is_sum {
            Ok((spans!("s", s), Value::Int(v.to_int()?)))
        } else {
            Ok((s, v))
        }
    }
}

fn dicemod(i: &str) -> IResult<&str, DiceMod> {
    let (i, roll) = diceroll(i)?;
    let (i, op) = opt(tuple((ws(dicemod_op), value)))(i)?;

    Ok((i, DiceMod { roll, op }))
}
#[derive(Debug)]
pub struct DiceMod {
    pub roll: DiceRoll,                // ...
    pub op: Option<(ModOp, AstValue)>, // ( operator ... )?
}
impl Evaluable for DiceMod {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        match &self.op {
            None => self.roll.eval(limit, values),
            Some((op, r)) => match self.roll {
                DiceRoll::NoRoll(_) => {
                    let l = self.roll.eval(limit, values)?;
                    let (rs, rv) = r.eval(limit, values)?;
                    let (_, v) = op.apply(l.1, rv)?;
                    Ok((spans!(l.0, format!("{}", op), rs), v))
                }
                DiceRoll::Roll { .. } => {
                    let (s, l) = self.roll._eval(limit, values)?;
                    let (rs, rv) = r.eval(limit, values)?;
                    let (vs, v) = op.apply(l, rv)?;
                    Ok((spans!(s, format!("{}", op), rs, ":", vs), v))
                }
            },
        }
    }
}

fn explode(i: &str) -> IResult<&str, Explode> {
    let (i, n) = preceded(tag("!"), opt(preceded(multispace0, number)))(i)?;

    let res = if let Some(n) = n {
        Explode::Target(n)
    } else {
        Explode::Default
    };

    Ok((i, res))
}
#[derive(Debug)]
pub enum Explode {
    Default,
    Target(i64),
}
impl Display for Explode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            Explode::Default => write!(f, "!"),
            Explode::Target(t) => write!(f, "!{t}"),
        }
    }
}

fn diceroll(i: &str) -> IResult<&str, DiceRoll> {
    alt((
        tuple((
            terminated(opt(value), ws(tag("d"))),
            opt(value),
            preceded(multispace0, opt(explode)),
        ))
        .map(|(count, sides, explode)| DiceRoll::Roll { count, sides, explode }),
        value.map(DiceRoll::NoRoll),
    ))(i)
}
#[derive(Debug)]
pub enum DiceRoll {
    NoRoll(AstValue), // ...
    Roll {
        count: Option<AstValue>,  // ( ... )? "d"
        sides: Option<AstValue>,  // ( ... )?
        explode: Option<Explode>, // ( "!" ( integer )? )?
    },
}
impl Evaluable for DiceRoll {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        let (s, r) = self._eval(limit, values)?;
        match self {
            DiceRoll::NoRoll(_) => Ok((s, r)),
            DiceRoll::Roll { .. } => Ok((spans!(s, ":", r.to_string()), r)),
        }
    }
}

enum DiceOptions {
    Vector(Vec<i64>),
    Range(i64, i64),
}
impl DiceOptions {
    fn roll(&self, rng: &mut rand::rngs::ThreadRng) -> i64 {
        match self {
            Self::Vector(v) => *v.choose(rng).unwrap(),
            Self::Range(lo, hi) => rng.gen_range(lo, hi + 1),
        }
    }
    fn is_empty(&self) -> bool {
        match self {
            Self::Vector(v) => v.is_empty(),
            Self::Range(lo, hi) => hi < lo,
        }
    }
    fn get_max_value(&self) -> i64 {
        match self {
            Self::Vector(v) => *v.iter().max().unwrap(),
            Self::Range(_lo, hi) => *hi,
        }
    }
    fn get_min_value(&self) -> i64 {
        match self {
            Self::Vector(v) => *v.iter().min().unwrap(),
            Self::Range(lo, _hi) => *lo,
        }
    }
    fn get_options(&self) -> u64 {
        match self {
            Self::Vector(v) => v.len() as u64,
            Self::Range(lo, hi) => (hi - lo + 1) as u64,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.is_empty() {
            return Err("tried to roll a die with no options".to_string());
        }
        if let Self::Range(_, i64::MAX) = self {
            return Err("tried to roll a d(2^63 - 1)".to_string());
        }

        Ok(())
    }
}

impl DiceRoll {
    fn _eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        match self {
            DiceRoll::NoRoll(v) => v.eval(limit, values),
            DiceRoll::Roll {
                count: cv,
                sides: sv,
                explode: ex,
            } => {
                let (cs, c) = match cv {
                    Some(v) => {
                        let (vs, vv) = v.eval(limit, values)?;
                        let count = vv.to_int()?;
                        (vs, count)
                    }
                    None => (vec![], 1),
                };

                if c < 0 {
                    return Err(format!("tried to roll {c} dice"));
                }

                let (ss, s) = match sv {
                    Some(v) => {
                        let (vs, vv) = v.eval(limit, values)?;
                        let opts: DiceOptions = match vv {
                            Value::Int(i) if i >= 1 => DiceOptions::Range(1, i),
                            Value::Int(0) => return Err("cannot roll a d0".to_string()),
                            Value::Int(i) => return Err(format!("cannot roll a d({i})")),
                            Value::IntSlice(s) => DiceOptions::Vector(s),
                            Value::Bool(b) => return Err(format!("cannot roll a d{b}")),
                            Value::BoolSlice(_) => return Err("cannot roll a d[list of bool]".to_string()),
                        };
                        (vs, opts)
                    }
                    None => (vec![], DiceOptions::Range(1, 6)),
                };

                s.validate()?;

                let mut n = c as usize;
                let target = match ex {
                    None => None,
                    Some(Explode::Default) => Some(s.get_max_value()),
                    Some(Explode::Target(t)) => Some(*t),
                };

                if let Some(target) = target {
                    let min_roll = s.get_min_value();
                    if min_roll >= target {
                        return Err("tried to roll an always-exploding die".to_string());
                    }
                }

                let n_options = s.get_options();
                limit.use_entropy(n as u64, n_options)?;

                let mut rng = thread_rng();
                let mut entropy_err = None;
                let results = std::iter::repeat_with(|| s.roll(&mut rng))
                    .take_while(|&roll| {
                        if n == 0 {
                            return false;
                        }
                        match target {
                            None => n -= 1,
                            Some(target) => {
                                if roll < target {
                                    n -= 1
                                } else {
                                    let e = limit.use_entropy(1, n_options);
                                    if e.is_err() {
                                        entropy_err = Some(e);
                                        return false;
                                    }
                                }
                            }
                        };
                        true
                    })
                    .collect();

                if let Some(e) = entropy_err {
                    e?;
                }

                let exp_str: Cow<str> = match ex {
                    None => "".into(),
                    Some(exp) => format!("{exp}").into(),
                };
                Ok((spans!(cs, "d", ss, exp_str), Value::IntSlice(results)))
            }
        }
    }
}

fn value(i: &str) -> IResult<&str, AstValue> {
    alt((
        preceded(tuple((tag("-"), multispace0)), number).map(|v| AstValue::Int(-v)),
        number.map(AstValue::Int),
        delimited(tag("("), ws(expression), tag(")")).map(|v| AstValue::Sub(Box::new(v))),
        delimited(
            terminated(tag("["), multispace0),
            separated_list0(ws(tag(",")), expression),
            preceded(multispace0, tag("]")),
        )
        .map(AstValue::Slice),
        tag("F").map(|_| AstValue::Fate),
        tag("%").map(|_| AstValue::Hundred),
        preceded(tag("$"), anychar).map(AstValue::Binding),
    ))(i)
}
#[derive(Debug)]
pub enum AstValue {
    Int(i64),               // ...
    Sub(Box<Expression>),   // "(" ... ")"
    Slice(Vec<Expression>), // "[" ... "]"
    Fate,                   // "F"
    Hundred,                // "%"
    Binding(char),          // "$" ...
}
impl Evaluable for AstValue {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        match self {
            AstValue::Int(i) => Ok((spans!(format!("{}", i)), Value::Int(*i))),
            AstValue::Sub(expr) => {
                let (es, ev) = expr.eval(limit, values)?;
                Ok((spans!("(", es, ")"), ev))
            }
            AstValue::Slice(s) => {
                let (strs, vals) = s
                    .iter()
                    .map(|e| {
                        let (s, v) = e.eval(limit, values)?;
                        Ok((s, v.to_int()?))
                    })
                    .collect::<Result<Vec<(Vec<Span>, _)>, String>>()?
                    .drain(..)
                    .unzip();

                Ok((spans!("[", span_join(strs, ", "), "]"), Value::IntSlice(vals)))
            }
            AstValue::Fate => Ok((spans!("F"), Value::IntSlice(vec![-1, 0, 1]))),
            AstValue::Hundred => Ok((spans!("%"), Value::Int(100))),
            AstValue::Binding(ch) => match values.get(ch) {
                Some(v) => Ok((spans!("$", format!("{}", ch)), v.clone())),
                None => Err(format!("binding ${ch} not defined")),
            },
        }
    }
}

fn addsub_op(i: &str) -> IResult<&str, AddSubOp> {
    alt((tag("+").map(|_| AddSubOp::Add), tag("-").map(|_| AddSubOp::Sub)))(i)
}
#[derive(Debug)]
pub enum AddSubOp {
    Add, // +
    Sub, // -
}
impl AddSubOp {
    fn apply(&self, left: Value, right: Value) -> Result<Value, String> {
        if let (Value::IntSlice(l), Value::IntSlice(r)) = (&left, &right) {
            let mut l = l.clone();
            l.extend_from_slice(r);
            return Ok(Value::IntSlice(l));
        }
        let l = left.to_int()?;
        let r = right.to_int()?;
        let result = match self {
            AddSubOp::Add => l.wrapping_add(r),
            AddSubOp::Sub => l.wrapping_sub(r),
        };
        Ok(Value::Int(result))
    }
}
impl Display for AddSubOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AddSubOp::Add => write!(f, "+"),
            AddSubOp::Sub => write!(f, "-"),
        }
    }
}

fn muldiv_op(i: &str) -> IResult<&str, MulDivOp> {
    alt((tag("*").map(|_| MulDivOp::Mul), tag("/").map(|_| MulDivOp::Div)))(i)
}
#[derive(Debug)]
pub enum MulDivOp {
    Mul, // *
    Div, // /
}
impl MulDivOp {
    fn apply(&self, left: Value, right: Value) -> Result<Value, String> {
        let l = left.to_int()?;
        let r = right.to_int()?;
        let result = match self {
            MulDivOp::Mul => l.wrapping_mul(r),
            MulDivOp::Div => l.wrapping_div(r),
        };
        Ok(Value::Int(result))
    }
}
impl Display for MulDivOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MulDivOp::Mul => write!(f, "*"),
            MulDivOp::Div => write!(f, "/"),
        }
    }
}

fn compare_op(i: &str) -> IResult<&str, CompareOp> {
    let (i, each_left) = opt(tag("e")).map(|o| o.is_some()).parse(i)?;
    let (i, op) = compare_base_op(i)?;
    let (i, each_right) = opt(tag("e")).map(|o| o.is_some()).parse(i)?;

    Ok((
        i,
        CompareOp {
            each_left,
            op,
            each_right,
        },
    ))
}
#[derive(Debug)]
pub struct CompareOp {
    each_left: bool,
    op: CompareBaseOp,
    each_right: bool,
}
impl CompareOp {
    fn apply(&self, left: Value, right: Value) -> Result<Value, String> {
        self._apply(&left, &right)
            .map_err(|e| format!("cannot compare {left} {self} {right}: {e}"))
    }
    fn _apply(&self, left: &Value, right: &Value) -> Result<Value, String> {
        let result = match (self.each_left, self.each_right) {
            (false, false) => Value::Bool(self.op.compare(left.to_int()?, right.to_int()?)),
            (false, true) => {
                let l = left.to_int()?;
                Value::BoolSlice(right.to_int_slice()?.iter().map(|r| self.op.compare(l, *r)).collect())
            }
            (true, false) => {
                let r = right.to_int()?;
                Value::BoolSlice(left.to_int_slice()?.iter().map(|l| self.op.compare(*l, r)).collect())
            }
            (true, true) => {
                let lv = left.to_int_slice()?;
                let rv = right.to_int_slice()?;
                if lv.len() != rv.len() {
                    return Err("mismatched lengths".to_string());
                }
                Value::BoolSlice((0..lv.len()).map(|i| self.op.compare(lv[i], rv[i])).collect())
            }
        };

        Ok(result)
    }
}
impl Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.each_left {
            write!(f, "e")?;
        }
        write!(f, "{}", self.op)?;
        if self.each_right {
            write!(f, "e")?;
        }
        Ok(())
    }
}

fn compare_base_op(i: &str) -> IResult<&str, CompareBaseOp> {
    alt((
        alt((tag("<="), tag("=<"))).map(|_| CompareBaseOp::LessEq),
        tag("<").map(|_| CompareBaseOp::Less),
        alt((tag(">="), tag("=>"))).map(|_| CompareBaseOp::GreaterEq),
        tag(">").map(|_| CompareBaseOp::Greater),
        alt((tag("=="), tag("="))).map(|_| CompareBaseOp::Equal),
        alt((tag("!="), tag("<>"))).map(|_| CompareBaseOp::Unequal),
    ))(i)
}
#[derive(Debug)]
pub enum CompareBaseOp {
    Less,      // <
    LessEq,    // <=, =<
    Greater,   // >
    GreaterEq, // >=, =>
    Equal,     // ==, =
    Unequal,   // !=, <>
}
impl CompareBaseOp {
    fn compare(&self, l: i64, r: i64) -> bool {
        match self {
            Self::Less => l < r,
            Self::LessEq => l <= r,
            Self::Greater => l > r,
            Self::GreaterEq => l >= r,
            Self::Equal => l == r,
            Self::Unequal => l != r,
        }
    }
}
impl Display for CompareBaseOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Less => write!(f, "<"),
            Self::LessEq => write!(f, "<="),
            Self::Greater => write!(f, ">"),
            Self::GreaterEq => write!(f, ">="),
            Self::Equal => write!(f, "=="),
            Self::Unequal => write!(f, "!="),
        }
    }
}

fn dicemod_op(i: &str) -> IResult<&str, ModOp> {
    alt((
        tag("l").map(|_| ModOp::DropLowest),
        tag("h").map(|_| ModOp::DropHighest),
        tag("L").map(|_| ModOp::KeepLowest),
        tag("H").map(|_| ModOp::KeepHighest),
    ))(i)
}
#[derive(Debug)]
pub enum ModOp {
    DropLowest,  // l
    DropHighest, // h
    KeepLowest,  // L
    KeepHighest, // H
}

fn format_arrays(ac: Color, aa: &[i64], bc: Color, ba: &[i64]) -> Vec<Span<'static>> {
    let vec = Iterator::chain(
        aa.iter().map(|v| span!(ac + Format::Bold; "{}", v)),
        ba.iter().map(|v| span!(bc + Format::Bold; "{}", v)),
    )
    .collect::<Vec<_>>();
    spans!("[", span_join(vec, ", "), "]")
}

impl ModOp {
    fn apply(&self, left: Value, right: Value) -> Result<(Vec<Span>, Value), String> {
        let mut l = left.to_int_slice()?;
        l.sort_unstable();
        let r = right.to_int()? as usize;
        if r > l.len() {
            return Err(format!(
                "cannot evaluate a keep/drop {} operation on {} dice",
                r,
                l.len()
            ));
        }
        let (s, result) = match self {
            ModOp::DropLowest => (format_arrays(Color::Red, &l[..r], Color::Yellow, &l[r..]), &l[r..]),
            ModOp::DropHighest => {
                let i = l.len() - r;
                (format_arrays(Color::Yellow, &l[..i], Color::Red, &l[i..]), &l[..i])
            }
            ModOp::KeepLowest => (format_arrays(Color::Yellow, &l[..r], Color::Red, &l[r..]), &l[..r]),
            ModOp::KeepHighest => {
                let i = l.len() - r;
                (format_arrays(Color::Red, &l[..i], Color::Yellow, &l[i..]), &l[i..])
            }
        };
        Ok((s, Value::IntSlice(result.to_vec())))
    }
}
impl Display for ModOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ModOp::DropLowest => write!(f, "l"),
            ModOp::DropHighest => write!(f, "h"),
            ModOp::KeepLowest => write!(f, "L"),
            ModOp::KeepHighest => write!(f, "H"),
        }
    }
}

fn number(i: &str) -> IResult<&str, i64> {
    map_res(take_while(|c: char| c.is_ascii_digit()), str::parse)(i)
}
