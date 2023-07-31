use std::fmt::{self, Display};

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    IntSlice(Vec<i64>),
    Bool(bool),
    BoolSlice(Vec<bool>),
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::Int(i) => write!(f, "{i}"),
            Self::IntSlice(s) => {
                if s.len() <= 10 {
                    let strs: Vec<String> = s.iter().map(|v| format!("{v}")).collect();
                    write!(f, "[{}]", strs.join(", "))
                } else {
                    write!(f, "[{} ints, total {}]", s.len(), s.iter().sum::<i64>())
                }
            }
            Self::Bool(b) => write!(f, "{b}"),
            Self::BoolSlice(s) => {
                if s.len() <= 10 {
                    let strs: Vec<String> = s.iter().map(|v| format!("{v}")).collect();
                    write!(f, "[{}]", strs.join(", "))
                } else {
                    write!(f, "[{} bools, {} true]", s.len(), s.iter().filter(|v| **v).count())
                }
            }
        }
    }
}

impl std::iter::FromIterator<Value> for Value {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Value>,
    {
        let mut iter = iter.into_iter().peekable();

        match iter.peek() {
            None => Value::IntSlice(vec![]),
            Some(Value::Bool(_)) => Value::BoolSlice(iter.map(|v| v.to_bool()).collect()),
            _ => Value::IntSlice(iter.map(|v| v.to_int()).collect()),
        }
    }
}

impl Value {
    fn to_bool(&self) -> bool {
        !matches!(self, Self::Int(0) | Self::Bool(false))
    }
    pub fn to_int(&self) -> i64 {
        match self {
            Self::Int(i) => *i,
            Self::IntSlice(s) => s.iter().sum(),
            Self::Bool(true) => 1,
            Self::Bool(false) => 0,
            Self::BoolSlice(s) => s.iter().filter(|&v| *v).count() as i64,
        }
    }
    pub fn to_int_slice(&self) -> Result<Vec<i64>, String> {
        match self {
            Self::Int(i) => Err(format!("cannot convert {i} to slice")),
            Self::IntSlice(s) => Ok(s.clone()),
            Self::Bool(b) => Err(format!("cannot convert {b} to slice")),
            Self::BoolSlice(s) => Ok(s.iter().map(|&v| i64::from(v)).collect()),
        }
    }
}
