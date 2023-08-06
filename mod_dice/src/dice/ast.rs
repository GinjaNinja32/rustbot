use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{self, Display};

use rustbot::prelude::{span_join, Color, Format, FormatColor, Span};
use rustbot::{span, spans};

use super::limits::Limiter;
use super::value::Value;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{anychar, multispace0},
    combinator::{eof, map_res, opt, value},
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

pub trait Parse: Sized {
    fn parse(i: &str) -> IResult<&str, Self>;
}
pub trait Evaluable: Parse {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String>;
}
pub trait Operator: Parse {
    fn apply(&self, left: &Value, right: &Value) -> Result<Value, String>;
}

pub enum Command {
    Simple(Expression),
    Complex {
        bindings: Vec<(char, Expression)>,
        output: Vec<OutputSegment>,
    },
}
impl Parse for Command {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, res) = alt((
            tuple((
                separated_list1(ws(tag(";")), tuple((terminated(anychar, tag(":")), Expression::parse))),
                preceded(ws(tag(";")), many0(output_segment)),
            ))
            .map(|(bindings, output)| Self::Complex { bindings, output }),
            Expression::parse.map(Self::Simple),
        ))(i)?;
        let (i, _) = eof(i)?;

        Ok((i, res))
    }
}
impl Command {
    pub fn new(input: &str) -> Result<Self, String> {
        Self::parse(input).map(|(_, c)| c).map_err(|e| format!("{e}"))
    }
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
                        OutputSegment::Select(ch, strs) => match vals.get(ch) {
                            None => {
                                return Err(format!("binding ${ch} not defined"));
                            }
                            Some(v) => {
                                let idx = v.to_int();
                                if idx < 0 {
                                    return Err(format!(
                                        "index {idx} out of range for format with {} options",
                                        strs.len()
                                    ));
                                }

                                match strs.get(idx as usize) {
                                    None => {
                                        return Err(format!(
                                            "index {idx} out of range for format with {} options",
                                            strs.len()
                                        ))
                                    }
                                    Some(s) => spans.push(span! {s}),
                                }
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
                        take_while(|c| c != '|' && c != ']').map(String::from),
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
        tuple((
            preceded(tuple((tag("%"), tag("$"))), anychar),
            delimited(
                tag("["),
                separated_list1(tag("|"), take_while(|c| c != '|' && c != ']').map(String::from)),
                tag("]"),
            ),
        ))
        .map(|(ch, lst)| OutputSegment::Select(ch, lst)),
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
    Select(char, Vec<String>),
}

#[derive(Debug)]
pub struct Expression {
    pub repeat: Repeat, // ...
}
impl Parse for Expression {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, repeat) = ws(Repeat::parse)(i)?;

        Ok((i, Expression { repeat }))
    }
}
impl Evaluable for Expression {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        self.repeat.eval(limit, values)
    }
}

#[derive(Debug)]
pub struct Repeat {
    pub repeat: Option<i64>, // ( integer "#" )?
    pub term: Comparison,    // ...
}
impl Parse for Repeat {
    fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            tuple((number, ws(tag("#")), Comparison::parse)).map(|(r, _, term)| Self { repeat: Some(r), term }),
            Comparison::parse.map(|term| Self { repeat: None, term }),
        ))(i)
    }
}
impl Evaluable for Repeat {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        match self.repeat {
            None => self.term.eval(limit, values),
            Some(n) => {
                let (strs, vals) = (0..n)
                    .map(|_| {
                        let (s, v) = self.term.eval(limit, values)?;
                        Ok((s, v.to_int()))
                    })
                    .collect::<Result<Vec<(Vec<Span>, _)>, String>>()?
                    .drain(..)
                    .unzip();

                Ok((span_join(strs, ", "), Value::IntSlice(vals)))
            }
        }
    }
}

#[derive(Debug)]
pub struct Comparison {
    pub left: AddSub,                       // ...
    pub right: Option<(CompareOp, AddSub)>, // ( operator ... )?
}
impl Parse for Comparison {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, left) = AddSub::parse(i)?;
        let (i, right) = opt(tuple((ws(CompareOp::parse), AddSub::parse)))(i)?;

        Ok((i, Self { left, right }))
    }
}
impl Evaluable for Comparison {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        let l = self.left.eval(limit, values)?;
        match &self.right {
            None => Ok(l),
            Some((op, term)) => {
                let r = term.eval(limit, values)?;
                let v = op.apply(&l.1, &r.1)?;

                Ok((spans!(l.0, format!("{}", op), r.0), v))
            }
        }
    }
}

#[derive(Debug)]
pub struct BinaryOpClass<Sub: Evaluable, Op: Operator + Display> {
    pub left: Sub,
    pub right: Vec<(Op, Sub)>,
}
impl<Sub: Evaluable, Op: Operator + Display + 'static> Parse for BinaryOpClass<Sub, Op> {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, left) = Sub::parse(i)?;
        let (i, right) = many0(tuple((ws(Op::parse), Sub::parse)))(i)?;

        Ok((i, Self { left, right }))
    }
}
impl<Sub: Evaluable, Op: Operator + Display + 'static> Evaluable for BinaryOpClass<Sub, Op> {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        let (s, mut l) = self.left.eval(limit, values)?;
        let mut ss = s;
        for elem in &self.right {
            let (mut rs, r) = elem.1.eval(limit, values)?;

            ss.push(format!("{}", elem.0).into());
            ss.append(&mut rs);
            l = elem.0.apply(&l, &r)?;
        }
        Ok((ss, l))
    }
}

pub type AddSub = BinaryOpClass<MulDiv, AddSubOp>;

pub type MulDiv = BinaryOpClass<Sum, MulDivOp>;

#[derive(Debug)]
pub struct Sum {
    pub is_sum: bool,  // ( "s" )?
    pub term: DiceMod, // ...
}
impl Parse for Sum {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, is_sum) = opt(tag("s")).map(|o| o.is_some()).parse(i)?;
        let (i, _) = multispace0(i)?;
        let (i, term) = DiceMod::parse(i)?;

        Ok((i, Self { is_sum, term }))
    }
}
impl Evaluable for Sum {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        let (s, v) = self.term.eval(limit, values)?;
        if self.is_sum {
            Ok((spans!("s", s), Value::Int(v.to_int())))
        } else {
            Ok((s, v))
        }
    }
}

#[derive(Debug)]
pub struct DiceMod {
    pub roll: DiceRoll,                // ...
    pub op: Option<(ModOp, AstValue)>, // ( operator ... )?
}
impl Parse for DiceMod {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, roll) = DiceRoll::parse(i)?;
        let (i, op) = opt(tuple((ws(ModOp::parse), AstValue::parse)))(i)?;

        Ok((i, Self { roll, op }))
    }
}
impl Evaluable for DiceMod {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        match &self.op {
            None => self.roll.eval(limit, values),
            Some((op, r)) => match self.roll {
                DiceRoll::NoRoll(_) | DiceRoll::Index { .. } => {
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

#[derive(Debug)]
pub enum Explode {
    Default,
    Target(i64),
}
impl Parse for Explode {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, n) = preceded(tag("!"), opt(preceded(multispace0, number)))(i)?;

        let res = if let Some(n) = n {
            Self::Target(n)
        } else {
            Self::Default
        };

        Ok((i, res))
    }
}
impl Display for Explode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            Explode::Default => write!(f, "!"),
            Explode::Target(t) => write!(f, "!{t}"),
        }
    }
}

#[derive(Debug)]
pub enum DiceRoll {
    NoRoll(AstValue), // ...
    Index {
        val: AstValue,   // ...
        each: bool,      // @ e?
        index: AstValue, // ...
    },
    Roll {
        count: Option<AstValue>,  // ( ... )? "d"
        sides: Option<AstValue>,  // ( ... )?
        explode: Option<Explode>, // ( "!" ( integer )? )?
    },
}
impl Parse for DiceRoll {
    fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            tuple((
                terminated(opt(AstValue::parse), ws(tag("d"))),
                opt(AstValue::parse),
                preceded(multispace0, opt(Explode::parse)),
            ))
            .map(|(count, sides, explode)| Self::Roll { count, sides, explode }),
            tuple((
                terminated(AstValue::parse, multispace0),
                preceded(tag("@"), opt(tag("e"))),
                preceded(multispace0, AstValue::parse),
            ))
            .map(|(val, each, index)| Self::Index {
                val,
                each: each.is_some(),
                index,
            }),
            AstValue::parse.map(Self::NoRoll),
        ))(i)
    }
}
impl Evaluable for DiceRoll {
    fn eval(&self, limit: &mut Limiter, values: &BTreeMap<char, Value>) -> Result<(Vec<Span>, Value), String> {
        let (s, r) = self._eval(limit, values)?;
        match self {
            DiceRoll::NoRoll(_) | DiceRoll::Index { .. } => Ok((s, r)),
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
            DiceRoll::Index { val, each, index } => {
                let (sv, v) = val.eval(limit, values)?;
                let (si, i) = index.eval(limit, values)?;

                if *each {
                    let val = i
                        .to_int_slice()?
                        .into_iter()
                        .map(|idx| v.index_slice(idx))
                        .collect::<Result<Value, _>>()?;
                    Ok((spans!(sv, "@e", si), val))
                } else {
                    v.index_slice(i.to_int()).map(|val| (spans!(sv, "@", si), val))
                }
            }
            DiceRoll::Roll {
                count: cv,
                sides: sv,
                explode: ex,
            } => {
                let (cs, c) = match cv {
                    Some(v) => {
                        let (vs, vv) = v.eval(limit, values)?;
                        let count = vv.to_int();
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

#[derive(Debug)]
pub enum AstValue {
    Int(i64),               // ...
    Sub(Box<Expression>),   // "(" ... ")"
    Slice(Vec<Expression>), // "[" ... "]"
    Fate,                   // "F"
    Hundred,                // "%"
    Binding(char),          // "$" ...
}
impl Parse for AstValue {
    fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            preceded(tuple((tag("-"), multispace0)), number).map(|v| Self::Int(-v)),
            number.map(Self::Int),
            delimited(tag("("), ws(Expression::parse), tag(")")).map(|v| Self::Sub(Box::new(v))),
            delimited(
                terminated(tag("["), multispace0),
                separated_list0(ws(tag(",")), Expression::parse),
                preceded(multispace0, tag("]")),
            )
            .map(Self::Slice),
            tag("F").map(|_| Self::Fate),
            tag("%").map(|_| Self::Hundred),
            preceded(tag("$"), anychar).map(Self::Binding),
        ))(i)
    }
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
                        Ok((s, v.to_int()))
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

macro_rules! operator_group {
    ($name:ident($l:ident, $r:ident): $( $opname:ident, $opeval:expr, $optext:literal $(, $opalt:literal )* ;)+) => {
        #[derive(Debug, Copy, Clone)]
        pub enum $name {
            $( $opname ),+
        }
        impl Parse for $name {
            fn parse(i: &str) -> IResult<&str, Self> {
                alt((
                    $(
                        value(Self::$opname, alt((
                            tag($optext),
                            $(tag($opalt)),*
                        )))
                    ),+
                ))(i)
            }
        }
        impl Operator for $name {
            fn apply(&self, left: &Value, right: &Value) -> Result<Value, String> {
                let $l = left.to_int();
                let $r = right.to_int();
                let result = match self {
                    $(
                        Self::$opname => $opeval,
                    )+
                };
                Ok(result)
            }
        }
        impl Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match self {
                    $(
                        Self::$opname => write!(f, $optext),
                    )+
                }
            }
        }
    };
}

operator_group! {
    AddSubBaseOp(l, r):
        Add, Value::Int(l.wrapping_add(r)), "+";
        Sub, Value::Int(l.wrapping_sub(r)), "-";
}
type AddSubOp = MaybeElementwise<AddSubBaseOp>;

operator_group! {
    MulDivBaseOp(l, r):
        Mul, Value::Int(l.wrapping_mul(r)), "*";
        Div, Value::Int(l.wrapping_div(r)), "/";
}
type MulDivOp = MaybeElementwise<MulDivBaseOp>;

#[derive(Debug)]
pub struct MaybeElementwise<Op: Operator> {
    each_left: bool,
    op: Op,
    each_right: bool,
}
impl<Op: Operator> Parse for MaybeElementwise<Op> {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, each_left) = opt(tag("e")).map(|o| o.is_some()).parse(i)?;
        let (i, op) = Op::parse(i)?;
        let (i, each_right) = opt(tag("e")).map(|o| o.is_some()).parse(i)?;

        Ok((
            i,
            Self {
                each_left,
                op,
                each_right,
            },
        ))
    }
}
impl<Op: Operator + Display> Operator for MaybeElementwise<Op> {
    fn apply(&self, left: &Value, right: &Value) -> Result<Value, String> {
        (|| match (self.each_left, self.each_right) {
            (false, false) => self.op.apply(left, right),
            (false, true) => right
                .to_int_slice()?
                .into_iter()
                .map(|r| self.op.apply(left, &Value::Int(r)))
                .collect(),
            (true, false) => left
                .to_int_slice()?
                .into_iter()
                .map(|l| self.op.apply(&Value::Int(l), right))
                .collect(),
            (true, true) => {
                let lv = left.to_int_slice()?;
                let rv = right.to_int_slice()?;
                if lv.len() != rv.len() {
                    return Err("mismatched lengths".to_string());
                }
                lv.into_iter()
                    .zip(rv)
                    .map(|(l, r)| self.op.apply(&Value::Int(l), &Value::Int(r)))
                    .collect()
            }
        })()
        .map_err(|e| format!("cannot compare {left} {self} {right}: {e}"))
    }
}
impl<Op: Operator + Display> Display for MaybeElementwise<Op> {
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

operator_group! {
    CompareBaseOp(l, r):
        Unequal, Value::Bool(l != r), "!=", "<>"; // must precede Less
        LessEq, Value::Bool(l <= r), "<=", "=<"; // must precede Less and Equal
        Less, Value::Bool(l < r), "<";
        GreaterEq, Value::Bool(l >= r), ">=", "=>"; // must precede Greater and Equal
        Greater, Value::Bool(l > r), ">";
        Equal, Value::Bool(l == r), "==", "=";
}
pub type CompareOp = MaybeElementwise<CompareBaseOp>;

#[derive(Debug)]
pub enum ModOp {
    DropLowest,  // l
    DropHighest, // h
    KeepLowest,  // L
    KeepHighest, // H
}
impl Parse for ModOp {
    fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            tag("l").map(|_| Self::DropLowest),
            tag("h").map(|_| Self::DropHighest),
            tag("L").map(|_| Self::KeepLowest),
            tag("H").map(|_| Self::KeepHighest),
        ))(i)
    }
}

fn format_arrays(ac: FormatColor, aa: &[i64], bc: FormatColor, ba: &[i64]) -> Vec<Span<'static>> {
    let vec = Iterator::chain(
        aa.iter().map(|v| span!(ac; "{}", v)),
        ba.iter().map(|v| span!(bc; "{}", v)),
    )
    .collect::<Vec<_>>();
    spans!("[", span_join(vec, ", "), "]")
}

impl ModOp {
    fn apply(&self, left: Value, right: Value) -> Result<(Vec<Span>, Value), String> {
        let mut l = left.to_int_slice()?;
        l.sort_unstable();
        let r = right.to_int() as usize;
        if r > l.len() {
            return Err(format!(
                "cannot evaluate a keep/drop {} operation on {} dice",
                r,
                l.len()
            ));
        }
        let keep = Color::Yellow + Format::Bold;
        let drop = Color::Red + Format::Italic;
        let (s, result) = match self {
            ModOp::DropLowest => (format_arrays(drop, &l[..r], keep, &l[r..]), &l[r..]),
            ModOp::DropHighest => {
                let i = l.len() - r;
                (format_arrays(keep, &l[..i], drop, &l[i..]), &l[..i])
            }
            ModOp::KeepLowest => (format_arrays(keep, &l[..r], drop, &l[r..]), &l[..r]),
            ModOp::KeepHighest => {
                let i = l.len() - r;
                (format_arrays(drop, &l[..i], keep, &l[i..]), &l[i..])
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
