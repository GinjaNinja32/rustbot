use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fmt;
use std::fmt::Display;
use std::iter;

// enums
use self::EvaluatedValue::*;

// space eater
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

pub fn parse(input: &str) -> Result<Expression, String> {
    fullexpr(&format!("{}\n", input))
        .map(|(_, c)| c)
        .map_err(|e| format!("{:?}", e))
}

pub fn eval(expr: Expression) -> Result<String, String> {
    let (s, v) = expr.eval()?;

    Ok(format!("{}: {}", v.to_string(), s))
}

trait Evaluable {
    fn eval(&self) -> Result<(String, EvaluatedValue), String>;
}
enum EvaluatedValue {
    Integer(i64),
    IntSlice(Vec<i64>),
    Bool(bool),
    BoolSlice(Vec<bool>),
}
impl Display for EvaluatedValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(),fmt::Error> {
        match self {
            Integer(i) => write!(f, "{}", i),
            IntSlice(s) => {
                let strs: Vec<String> = s.iter().map(|v| format!("{}", v)).collect();
                write!(f, "[{}]", strs.join(", "))
            }
            Bool(b) => write!(f, "{}", b),
            BoolSlice(s) => {
                let strs: Vec<String> = s.iter().map(|v| format!("{}", v)).collect();
                write!(f, "[{}]", strs.join(", "))
            }
        }
    }
}
impl EvaluatedValue {
    fn as_i64(&self) -> Result<i64, String> {
        match self {
            Integer(i) =>Ok(*i),
            IntSlice(s) => Ok(s.iter().fold(0, |x, y| x + y)),
            Bool(true) => Ok(1),
            Bool(false) => Ok(0),
            BoolSlice(s) => Ok(s.iter().filter(|&v| *v).count() as i64),
        }
    }
    fn as_int_slice(&self) -> Result<Vec<i64>, String> {
        match self {
            Integer(i) => Err(format!("cannot convert {} to slice", i)),
            IntSlice(s) => Ok(s.to_vec()),
            Bool(b) => Err(format!("cannot convert {} to slice", b)),
            BoolSlice(s) => Ok(s.iter().map(|&v| if v { 1 } else { 0 }).collect()),
        }
    }
    fn to_string(&self) -> String {
        match self {
            Integer(i) => format!("{}", i),
            IntSlice(s) => {
                let v: Vec<String> = s.iter().map(|e| format!("{}", e)).collect();
                format!("[{}]", v.join(", "))
            }
            Bool(b) => format!("{}", b),
            BoolSlice(s) => {
                let v: Vec<String> = s.iter().map(|e| format!("{}", e)).collect();
                format!("[{}]", v.join(", "))
            }
        }
    }
}

named!(fullexpr<&str, Expression>,
    terminated!(expression, tag!("\n"))
);

named!(expression<&str, Expression>,
    map!(repeat, |v| Expression{expr: v})
);
#[derive(Debug)]
pub struct Expression {
    pub expr: Repeat, // ...
}
impl Evaluable for Expression {
    fn eval(&self) -> Result<(String, EvaluatedValue), String> {
        self.expr.eval()
    }
}

named!(repeat<&str, Repeat>, sp!(alt!(
    do_parse!(
        n: number >>
        tag!("#") >>
        c: comparison >>
        (Repeat{ repeat: Some(n), term: c })
    ) |
    map!(comparison, |v| Repeat{ repeat: None, term: v })
)));
#[derive(Debug)]
pub struct Repeat {
    pub repeat: Option<i64>, // ( integer "#" )?
    pub term: Comparison,    // ...
}
impl Evaluable for Repeat {
    fn eval(&self) -> Result<(String, EvaluatedValue), String> {
        match self.repeat {
            None => self.term.eval(),
            Some(n) if n > 10 => Err("no".to_string()),
            Some(n) => {
                let mut strings: Vec<String> = vec![];
                let mut values: Vec<EvaluatedValue> = vec![];
                for _ in 0..n {
                    let (s, v) = self.term.eval()?;
                    strings.push(s);
                    values.push(v);
                }
                let result: Result<Vec<i64>, String> = values.iter().map(|v| v.as_i64()).collect();
                Ok((strings.join(", "), IntSlice(result?)))
            }
        }
    }
}

named!(comparison<&str, Comparison>, sp!(do_parse!(
    l: addsub >>
    r: opt!(tuple!(compareOp, addsub)) >>
    (Comparison{left: l, right: r})
)));
#[derive(Debug)]
pub struct Comparison {
    pub left: AddSub,                       // ...
    pub right: Option<(CompareOp, AddSub)>, // ( operator ... )?
}
impl Evaluable for Comparison {
    fn eval(&self) -> Result<(String, EvaluatedValue), String> {
        let l = self.left.eval()?;
        match &self.right {
            None => Ok(l),
            Some((op, term)) => {
                let r = term.eval()?;
                let (os, v) = op.apply(l.1, r.1)?;
                match os {
                    None => Ok((format!("{}{}{}", l.0, op, r.0), v)),
                    Some(s) => Ok((format!("{}{}{}={}", l.0, op, r.0, s), v))
                }
            }
        }
    }
}

named!(addsub<&str, AddSub>, sp!(do_parse!(
    l: muldiv >>
    r: many0!(tuple!(addsubOp, muldiv)) >>
    (AddSub{left: l, right: r})
)));
#[derive(Debug)]
pub struct AddSub {
    pub left: MulDiv,                   // ...
    pub right: Vec<(AddSubOp, MulDiv)>, // ( operator ... )*
}
impl Evaluable for AddSub {
    fn eval(&self) -> Result<(String, EvaluatedValue), String> {
        let (s, mut l) = self.left.eval()?;
        let mut ss = s.to_string();
        for elem in &self.right {
            let (rs, r) = elem.1.eval()?;

            ss = format!("{}{}{}", ss, elem.0, rs);
            l = elem.0.apply(l, r)?;
        }
        Ok((ss, l))
    }
}

named!(muldiv<&str, MulDiv>, sp!(do_parse!(
    l: sum >>
    r: many0!(tuple!(muldivOp, sum)) >>
    (MulDiv{left: l, right: r})
)));
#[derive(Debug)]
pub struct MulDiv {
    pub left: Sum,                   // ...
    pub right: Vec<(MulDivOp, Sum)>, // ( operator ... )*
}
impl Evaluable for MulDiv {
    fn eval(&self) -> Result<(String, EvaluatedValue), String> {
        let (s, mut l) = self.left.eval()?;
        let mut ss = s.to_string();
        for elem in &self.right {
            let (rs, r) = elem.1.eval()?;

            ss = format!("{}{}{}", ss, elem.0, rs);
            l = elem.0.apply(l, r)?;
        }
        Ok((ss, l))
    }
}

named!(sum<&str, Sum>, sp!(do_parse!(
    s: alt!(value!(true, tag!("s")) | value!(false)) >>
    t: dicemod >>
    (Sum{is_sum: s, term: t})
)));
#[derive(Debug)]
pub struct Sum {
    pub is_sum: bool,  // ( "s" )?
    pub term: DiceMod, // ...
}
impl Evaluable for Sum {
    fn eval(&self) -> Result<(String, EvaluatedValue), String> {
        let (s, v) = self.term.eval()?;
        if self.is_sum {
            Ok((format!("s{}", s), Integer(v.as_i64()?)))
        } else {
            Ok((s, v))
        }
    }
}

named!(dicemod<&str, DiceMod>, sp!(do_parse!(
    r: diceroll >>
    o: opt!(tuple!(dicemodOp, value)) >>
    (DiceMod{roll: r, op: o})
)));
#[derive(Debug)]
pub struct DiceMod {
    pub roll: DiceRoll,             // ...
    pub op: Option<(ModOp, Value)>, // ( operator ... )?
}
impl Evaluable for DiceMod {
    fn eval(&self) -> Result<(String, EvaluatedValue), String> {
        match &self.op {
            None => return self.roll.eval(),
            Some((op, r)) => {
                match self.roll {
                    DiceRoll::NoRoll(_) => {
                        let l = self.roll.eval()?;
                        let (rs, rv) = r.eval()?;
                        let (_, v) = op.apply(l.1, rv)?;
                        Ok((format!("{}{}{}", l.0, op, rs), v))
                    }
                    DiceRoll::Roll{..} => {
                        let (s, l) = self.roll._eval()?;
                        let (rs, rv) = r.eval()?;
                        let (vs, v) = op.apply(l, rv)?;
                        Ok((format!("{}{}{}:{}", s, op, rs, vs), v))
                    }
                }
            }
        }
    }
}

named!(explode<&str, Explode>, sp!(alt!(
    do_parse!(
        tag!("!") >>
        n: number >>
        (Explode::Target(n))
    ) |
    value!(Explode::Default, tag!("!"))
)));
#[derive(Debug)]
pub enum Explode {
    Default,
    Target(i64),
}
impl Explode {
    fn to_string(&self) -> String {
        match self {
            Explode::Default => return "!".to_string(),
            Explode::Target(t) => return format!("!{}", t),
        }
    }
}

named!(diceroll<&str, DiceRoll>, sp!(alt!(
    do_parse!(
        c: opt!(value) >>
        tag!("d") >>
        s: opt!(value) >>
        e: opt!(explode) >>
        (DiceRoll::Roll{count: c, sides: s, explode: e})
    ) |
    map!(value, |v| DiceRoll::NoRoll(v))
)));
#[derive(Debug)]
pub enum DiceRoll {
    NoRoll(Value), // ...
    Roll {
        count: Option<Value>,     // ( ... )? "d"
        sides: Option<Value>,     // ( ... )?
        explode: Option<Explode>, // ( "!" ( integer )? )?
    },
}
impl Evaluable for DiceRoll {
    fn eval(&self) -> Result<(String, EvaluatedValue), String> {
        let (s, r) = self._eval()?;
        match self {
            DiceRoll::NoRoll(_) => return Ok((s, r)),
            DiceRoll::Roll{..} => return Ok((format!("{}:{:?}", s, r.as_int_slice()?), r))
        }
    }
}

impl DiceRoll {
    fn _eval(&self) -> Result<(String, EvaluatedValue), String> {
        match self {
            DiceRoll::NoRoll(v) => v.eval(),
            DiceRoll::Roll {
                count: cv,
                sides: sv,
                explode: e,
            } => {
                let (cs, c) = match cv {
                    Some(v) => {
                        let (vs, vv) = v.eval()?;
                        let count = vv.as_i64()?;
                        if count > 1000 {
                            return Err("too many dice".to_string());
                        }
                        (vs, count)
                    }
                    None => ("".to_string(), 1),
                };
                let (ss, s) = match sv {
                    Some(v) => {
                        let (vs, vv) = v.eval()?;
                        let opts: Vec<i64> = match vv {
                            Integer(i) => {
                                if i > 1000 {
                                    return Err("dice too large".to_string());
                                }
                                (1..).take(i as usize).collect()
                            }
                            IntSlice(s) => s,
                            Bool(b) => return Err(format!("cannot roll a d{}", b)),
                            BoolSlice(b) => return Err(format!("cannot roll a d{:?}", b)),
                        };
                        (vs, opts)
                    }
                    None => ("".to_string(), vec![1, 2, 3, 4, 5, 6]),
                };

                let mut n = c as usize;
                let target = match e {
                    None => None,
                    Some(Explode::Default) => Some(*s.iter().max().unwrap()),
                    Some(Explode::Target(t)) => Some(*t),
                };

                let mut rng = thread_rng();
                let results = iter::repeat_with(|| *s.choose(&mut rng).unwrap())
                    .take_while(|&roll| {
                        if n <= 0 {
                            return false
                        }
                        match target {
                            None => { n -= 1 }
                            Some(t) => {
                                if roll < t {
                                    n -= 1
                                }
                            }
                        };
                        return true
                    })
                    .collect();

                let exp_str = match e {
                    None => "".to_string(),
                    Some(exp) => exp.to_string(),
                };
                Ok((
                    format!("{}d{}{}", cs, ss, exp_str),
                    IntSlice(results),
                ))
            }
        }
    }
}

named!(value<&str, Value>, sp!(alt!(
    map!(number, |v| Value::Integer(v)) |
    map!(delimited!(tag!("("), expression, tag!(")")), |v| Value::Sub(Box::new(v))) |
    map!(delimited!(tag!("["), separated_list!(tag!(","), expression), tag!("]")), |v| Value::Slice(v)) |
    value!(Value::Fate, tag!("F")) |
    value!(Value::Hundred, tag!("%"))
)));
#[derive(Debug)]
pub enum Value {
    Integer(i64),           // ...
    Sub(Box<Expression>),   // "(" ... ")"
    Slice(Vec<Expression>), // "[" ... "]"
    Fate,                   // "F"
    Hundred,                // "%"
}
impl Evaluable for Value {
    fn eval(&self) -> Result<(String, EvaluatedValue), String> {
        match self {
            Value::Integer(i) => Ok((format!("{}", i), Integer(*i))),
            Value::Sub(expr) => {
                let (es, ev) = expr.eval()?;
                Ok((format!("({})", es), ev))
            }
            Value::Slice(s) => {
                let r: Result<Vec<(String, EvaluatedValue)>, _> = s.iter().map(|v| v.eval()).collect();
                match r {
                    Err(e) => Err(e),
                    Ok(v) => {
                        let strs: Vec<String> = v.iter().map(|&(ref s, _)| s.clone()).collect();
                        let vals: Result<Vec<i64>, String> = v.iter().map(|&(_, ref v)| v.as_i64()).collect();
                        Ok((format!("[{}]", strs.join(", ")), IntSlice(vals?)))
                    }
                }
            }
            Value::Fate => Ok(("F".to_string(), IntSlice(vec![-1, 0, 1]))),
            Value::Hundred => Ok(("%".to_string(), Integer(100))),
        }
    }
}

named!(addsubOp<&str, AddSubOp>, alt!(value!(AddSubOp::Add, tag!("+")) | value!(AddSubOp::Sub, tag!("-"))));
#[derive(Debug)]
pub enum AddSubOp {
    Add, // +
    Sub, // -
}
impl AddSubOp {
    fn apply(&self, left: EvaluatedValue, right: EvaluatedValue) -> Result<EvaluatedValue, String> {
        let l = left.as_i64()?;
        let r = right.as_i64()?;
        let result = match self {
            AddSubOp::Add => l + r,
            AddSubOp::Sub => l - r,
        };
        Ok(Integer(result))
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

named!(muldivOp<&str, MulDivOp>, alt!(value!(MulDivOp::Mul, tag!("*")) | value!(MulDivOp::Div, tag!("/"))));
#[derive(Debug)]
pub enum MulDivOp {
    Mul, // *
    Div, // /
}
impl MulDivOp {
    fn apply(&self, left: EvaluatedValue, right: EvaluatedValue) -> Result<EvaluatedValue, String> {
        let l = left.as_i64()?;
        let r = right.as_i64()?;
        let result = match self {
            MulDivOp::Mul => l * r,
            MulDivOp::Div => l / r,
        };
        Ok(Integer(result))
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

named!(compareOp<&str, CompareOp>, sp!(alt!(
    value!(CompareOp::LessEq, alt!(tag!("<=") | tag!("=<"))) |
    value!(CompareOp::Less, tag!("<")) |
    value!(CompareOp::GreaterEq, alt!(tag!(">=") | tag!("=>"))) |
    value!(CompareOp::Greater, tag!(">")) |
    value!(CompareOp::Equal, alt!(tag!("==") | tag!("="))) |
    value!(CompareOp::Unequal, alt!(tag!("!=") | tag!("<>")))
)));
#[derive(Debug)]
pub enum CompareOp {
    Less,      // <
    LessEq,    // <=, =<
    Greater,   // >
    GreaterEq, // >=, =>
    Equal,     // ==, =
    Unequal,   // !=, <>
}
impl CompareOp {
    fn compare(&self, l: i64, r: i64) -> bool {
        match self {
            CompareOp::Less => l < r,
            CompareOp::LessEq => l <= r,
            CompareOp::Greater => l > r,
            CompareOp::GreaterEq => l >= r,
            CompareOp::Equal => l == r,
            CompareOp::Unequal => l != r,
        }
    }
    fn apply(&self, left: EvaluatedValue, right: EvaluatedValue) -> Result<(Option<String>, EvaluatedValue), String> {
        let l = match left {
            Integer(v) => Ok(v),
            IntSlice(v) => IntSlice(v).as_i64(),
            v => Err(format!("cannot compare {} {} {}", v, self, right)),
        }?;
        match right {
            Integer(r) => Ok((None, Bool(self.compare(l, r)))),
            IntSlice(s) => {
                let (strings, values): (Vec<String>, Vec<bool>) = s.iter().map(|r| {
                    if self.compare(l, *r) {
                        (format!("{}{}{}", GREEN, *r, RESET), true)
                    } else {
                        (format!("{}{}{}", RED, *r, RESET), false)
                    }
                }).unzip();
                Ok((Some(format!("[{}]", strings.join(", "))), BoolSlice(values)))
            }
            v => Err(format!("cannot compare {} {} {}", l, self, v)),
        }
    }
}
impl Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompareOp::Less => write!(f, "<"),
            CompareOp::LessEq => write!(f, "<="),
            CompareOp::Greater => write!(f, ">"),
            CompareOp::GreaterEq => write!(f, ">="),
            CompareOp::Equal => write!(f, "=="),
            CompareOp::Unequal => write!(f, "!="),
        }
    }
}

named!(dicemodOp<&str, ModOp>, alt!(
    value!(ModOp::DropLowest, tag!("l")) |
    value!(ModOp::DropHighest, tag!("h")) |
    value!(ModOp::KeepLowest, tag!("L")) |
    value!(ModOp::KeepHighest, tag!("H"))
));
#[derive(Debug)]
pub enum ModOp {
    DropLowest,  // l
    DropHighest, // h
    KeepLowest,  // L
    KeepHighest, // H
}

const RED : &str = "\x0304";
const YELLOW: &str = "\x0308";
const GREEN: &str = "\x0309";
const RESET: &str = "\x03\x02\x02";

fn format_arrays(ac: &str, aa: &[i64], bc: &str, ba: &[i64]) -> String {
    let a: Vec<String> = aa.iter().map(|v| format!("{}{}{}", ac, v, RESET)).collect();
    let b: Vec<String> = ba.iter().map(|v| format!("{}{}{}", bc, v, RESET)).collect();
    return format!("[{}, {}]", a.join(", "), b.join(", "))
}

impl ModOp {
    fn apply(&self, left: EvaluatedValue, right: EvaluatedValue) -> Result<(String, EvaluatedValue), String> {
        let mut l = left.as_int_slice()?;
        l.sort();
        let r = right.as_i64()? as usize;
        let (s, result) = match self {
            ModOp::DropLowest => (format_arrays(RED, &l[..r], YELLOW, &l[r..]), &l[r..]),
            ModOp::DropHighest => {
                let i = l.len() - r;
                (format_arrays(YELLOW, &l[..i], RED, &l[i..]), &l[..l.len() - r])
            }
            ModOp::KeepLowest => (format_arrays(YELLOW, &l[..r], RED, &l[r..]), &l[..r]),
            ModOp::KeepHighest => {
                let i = l.len() - r;
                (format_arrays(RED, &l[..i], YELLOW, &l[i..]), &l[l.len() - r..])
            }
        };
        Ok((s, IntSlice(result.to_vec())))
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

named!(number<&str, i64>,
    map_res!(take_while!(is_digit), |s: &str| s.parse::<i64>())
);

fn is_digit(c: char) -> bool {
    '0' <= c && c <= '9'
}
