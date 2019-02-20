use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fmt;
use std::fmt::Display;
use std::iter;

pub fn parse(input: &str) -> Result<Expression, String> {
    fullexpr(&format!("{}\n", input))
        .map(|(_, c)| c)
        .map_err(|e| format!("{:?}", e))
}

pub fn eval(expr: Expression) -> Result<String, String> {
    let (s, v) = expr.eval();

    Ok(format!("{}: {}", v.to_string(), s))
}

trait Evaluable {
    fn eval(&self) -> (String, EvaluatedValue);
}
enum EvaluatedValue {
    Integer(i64),
    Bool(bool),
    IntSlice(Vec<i64>),
}
impl EvaluatedValue {
    fn as_i64(&self) -> i64 {
        match self {
            EvaluatedValue::Integer(i) => *i,
            EvaluatedValue::Bool(true) => 1,
            EvaluatedValue::Bool(false) => 0,
            EvaluatedValue::IntSlice(s) => s.iter().fold(0, |x, y| x + y),
        }
    }
    fn as_int_slice(&self) -> Vec<i64> {
        match self {
            EvaluatedValue::Integer(i) => vec![*i],
            EvaluatedValue::Bool(true) => vec![1 as i64],
            EvaluatedValue::Bool(false) => vec![0 as i64],
            EvaluatedValue::IntSlice(s) => s.to_vec(),
        }
    }
    fn to_string(&self) -> String {
        match self {
            EvaluatedValue::Integer(i) => format!("{}", i),
            EvaluatedValue::Bool(b) => format!("{}", b),
            EvaluatedValue::IntSlice(s) => {
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
    fn eval(&self) -> (String, EvaluatedValue) {
        self.expr.eval()
    }
}

named!(repeat<&str, Repeat>, alt!(
    do_parse!(
        n: number >>
        tag!("#") >>
        c: comparison >>
        (Repeat{ repeat: Some(n), term: c })
    ) |
    map!(comparison, |v| Repeat{ repeat: None, term: v })
));
#[derive(Debug)]
pub struct Repeat {
    pub repeat: Option<i64>, // ( integer "#" )?
    pub term: Comparison,    // ...
}
impl Evaluable for Repeat {
    fn eval(&self) -> (String, EvaluatedValue) {
        self.term.eval()
    }
}

named!(comparison<&str, Comparison>, do_parse!(
    l: addsub >>
    r: opt!(tuple!(compareOp, addsub)) >>
    (Comparison{left: l, right: r})
));
#[derive(Debug)]
pub struct Comparison {
    pub left: AddSub,                       // ...
    pub right: Option<(CompareOp, AddSub)>, // ( operator ... )?
}
impl Evaluable for Comparison {
    fn eval(&self) -> (String, EvaluatedValue) {
        let l = self.left.eval();
        match &self.right {
            None => l,
            Some((op, term)) => {
                let r = term.eval();
                let v = op.apply(l.1, r.1);

                (format!("{}{}{}", l.0, op, r.0), v)
            }
        }
    }
}

named!(addsub<&str, AddSub>, do_parse!(
    l: muldiv >>
    r: many0!(tuple!(addsubOp, muldiv)) >>
    (AddSub{left: l, right: r})
));
#[derive(Debug)]
pub struct AddSub {
    pub left: MulDiv,                   // ...
    pub right: Vec<(AddSubOp, MulDiv)>, // ( operator ... )*
}
impl Evaluable for AddSub {
    fn eval(&self) -> (String, EvaluatedValue) {
        let (s, mut l) = self.left.eval();
        let mut ss = s.to_string();
        for elem in &self.right {
            let (rs, r) = elem.1.eval();

            ss = format!("{}{}{}", ss, elem.0, rs);
            l = elem.0.apply(l, r);
        }
        (ss, l)
    }
}

named!(muldiv<&str, MulDiv>, do_parse!(
    l: sum >>
    r: many0!(tuple!(muldivOp, sum)) >>
    (MulDiv{left: l, right: r})
));
#[derive(Debug)]
pub struct MulDiv {
    pub left: Sum,                   // ...
    pub right: Vec<(MulDivOp, Sum)>, // ( operator ... )*
}
impl Evaluable for MulDiv {
    fn eval(&self) -> (String, EvaluatedValue) {
        let (s, mut l) = self.left.eval();
        let mut ss = s.to_string();
        for elem in &self.right {
            let (rs, r) = elem.1.eval();

            ss = format!("{}{}{}", ss, elem.0, rs);
            l = elem.0.apply(l, r);
        }
        (ss, l)
    }
}

named!(sum<&str, Sum>, do_parse!(
    s: alt!(value!(true, tag!("s")) | value!(false)) >>
    t: dicemod >>
    (Sum{is_sum: s, term: t})
));
#[derive(Debug)]
pub struct Sum {
    pub is_sum: bool,  // ( "s" )?
    pub term: DiceMod, // ...
}
impl Evaluable for Sum {
    fn eval(&self) -> (String, EvaluatedValue) {
        let (s, v) = self.term.eval();
        if self.is_sum {
            (format!("s{}", s), EvaluatedValue::Integer(v.as_i64()))
        } else {
            (s, v)
        }
    }
}

named!(dicemod<&str, DiceMod>, do_parse!(
    r: diceroll >>
    o: opt!(tuple!(dicemodOp, value)) >>
    (DiceMod{roll: r, op: o})
));
#[derive(Debug)]
pub struct DiceMod {
    pub roll: DiceRoll,             // ...
    pub op: Option<(ModOp, Value)>, // ( operator ... )?
}
impl Evaluable for DiceMod {
    fn eval(&self) -> (String, EvaluatedValue) {
        let l = self.roll.eval();
        match &self.op {
            None => l,
            Some((op, r)) => {
                let (rs, rv) = r.eval();
                let v = op.apply(l.1, rv);

                (format!("{}{}{}", l.0, op, rs), v)
            }
        }
    }
}

named!(diceroll<&str, DiceRoll>, alt!(
    do_parse!(
        c: opt!(value) >>
        tag!("d") >>
        s: opt!(value) >>
        e: opt!(tuple!(tag!("!"), number)) >>
        (DiceRoll::Roll{count: c, sides: s, explode: e.map(|v| v.1)})
    ) |
    map!(value, |v| DiceRoll::NoRoll(v))
));
#[derive(Debug)]
pub enum DiceRoll {
    NoRoll(Value), // ...
    Roll {
        count: Option<Value>, // ( ... )? "d"
        sides: Option<Value>, // ( ... )?
        explode: Option<i64>, // ( "!" integer )?
    },
}
impl Evaluable for DiceRoll {
    fn eval(&self) -> (String, EvaluatedValue) {
        match self {
            DiceRoll::NoRoll(v) => v.eval(),
            DiceRoll::Roll {
                count: cv,
                sides: sv,
                explode: e,
            } => {
                let (cs, c) = match cv {
                    Some(v) => {
                        let (vs, vv) = v.eval();
                        (vs, vv.as_i64())
                    }
                    None => ("".to_string(), 1),
                };
                let (ss, s) = match sv {
                    Some(v) => {
                        let (vs, vv) = v.eval();
                        let opts: Vec<i64> = match vv {
                            EvaluatedValue::Integer(i) => (1..).take(i as usize).collect(),
                            EvaluatedValue::IntSlice(s) => s,
                            EvaluatedValue::Bool(_) => panic!("foo"),
                        };
                        (vs, opts)
                    }
                    None => ("".to_string(), vec![1, 2, 3, 4, 5, 6]),
                };

                let mut rng = thread_rng();
                let results = iter::repeat_with(|| *s.choose(&mut rng).unwrap())
                    .take(c as usize)
                    .collect();

                (
                    format!("{}d{}:{:?}", cs, ss, results),
                    EvaluatedValue::IntSlice(results),
                )
            }
        }
    }
}

named!(value<&str, Value>, alt!(
    map!(number, |v| Value::Integer(v)) |
    map!(delimited!(tag!("("), expression, tag!(")")), |v| Value::Sub(Box::new(v))) |
    map!(delimited!(tag!("["), separated_list!(tag!(","), number), tag!("]")), |v| Value::Slice(v)) |
    value!(Value::Fate, tag!("F")) |
    value!(Value::Hundred, tag!("%"))
));
#[derive(Debug)]
pub enum Value {
    Integer(i64),         // ...
    Sub(Box<Expression>), // "(" ... ")"
    Slice(Vec<i64>),      // "[" ... "]"
    Fate,                 // "F"
    Hundred,              // "%"
}
impl Evaluable for Value {
    fn eval(&self) -> (String, EvaluatedValue) {
        match self {
            Value::Integer(i) => (format!("{}", i), EvaluatedValue::Integer(*i)),
            Value::Sub(expr) => {
                let (es, ev) = expr.eval();
                (format!("({})", es), ev)
            }
            Value::Slice(s) => (format!("{:?}", s), EvaluatedValue::IntSlice(s.clone())),
            Value::Fate => ("F".to_string(), EvaluatedValue::IntSlice(vec![-1, 0, 1])),
            Value::Hundred => ("%".to_string(), EvaluatedValue::Integer(100)),
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
    fn apply(&self, left: EvaluatedValue, right: EvaluatedValue) -> EvaluatedValue {
        let l = left.as_i64();
        let r = right.as_i64();
        let result = match self {
            AddSubOp::Add => l + r,
            AddSubOp::Sub => l - r,
        };
        EvaluatedValue::Integer(result)
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
    fn apply(&self, left: EvaluatedValue, right: EvaluatedValue) -> EvaluatedValue {
        let l = left.as_i64();
        let r = right.as_i64();
        let result = match self {
            MulDivOp::Mul => l * r,
            MulDivOp::Div => l / r,
        };
        EvaluatedValue::Integer(result)
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

named!(compareOp<&str, CompareOp>, alt!(
    value!(CompareOp::Less, tag!("<")) |
    value!(CompareOp::LessEq, alt!(tag!("<=") | tag!("=<"))) |
    value!(CompareOp::Greater, tag!(">")) |
    value!(CompareOp::GreaterEq, alt!(tag!(">=") | tag!("=>"))) |
    value!(CompareOp::Equal, alt!(tag!("==") | tag!("="))) |
    value!(CompareOp::Unequal, alt!(tag!("!=") | tag!("<>")))
));
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
    fn apply(&self, left: EvaluatedValue, right: EvaluatedValue) -> EvaluatedValue {
        let l = left.as_i64();
        let r = right.as_i64();
        let result = match self {
            CompareOp::Less => l < r,
            CompareOp::LessEq => l <= r,
            CompareOp::Greater => l > r,
            CompareOp::GreaterEq => l >= r,
            CompareOp::Equal => l == r,
            CompareOp::Unequal => l != r,
        };
        EvaluatedValue::Bool(result)
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
impl ModOp {
    fn apply(&self, left: EvaluatedValue, right: EvaluatedValue) -> EvaluatedValue {
        let l = left.as_int_slice();
        let r = right.as_i64() as usize;
        let result = match self {
            ModOp::DropLowest => &l[r..],
            ModOp::DropHighest => &l[..l.len() - r],
            ModOp::KeepLowest => &l[..r],
            ModOp::KeepHighest => &l[l.len() - r..],
        };
        EvaluatedValue::IntSlice(result.to_vec())
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

// Low-level helpers
fn is_digit(c: char) -> bool {
    //nom::is_digit(c as u8)
    '0' <= c && c <= '9'
}
