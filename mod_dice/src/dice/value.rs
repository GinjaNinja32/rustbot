use std::fmt::{self, Display};

pub enum Value {
    Int(i64),
    IntSlice(Vec<i64>),
    Bool(bool),
    BoolSlice(Vec<bool>),
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::Int(i) => write!(f, "{}", i),
            Self::IntSlice(s) => {
                if s.len() <= 10 {
                    let strs: Vec<String> = s.iter().map(|v| format!("{}", v)).collect();
                    write!(f, "[{}]", strs.join(", "))
                } else {
                    write!(f, "[{} ints, total {}]", s.len(), s.iter().sum::<i64>())
                }
            }
            Self::Bool(b) => write!(f, "{}", b),
            Self::BoolSlice(s) => {
                if s.len() <= 10 {
                    let strs: Vec<String> = s.iter().map(|v| format!("{}", v)).collect();
                    write!(f, "[{}]", strs.join(", "))
                } else {
                    write!(f, "[{} bools, {} true]", s.len(), s.iter().filter(|v| **v).count())
                }
            }
        }
    }
}

impl Value {
    pub fn to_int(&self) -> Result<i64, String> {
        match self {
            Self::Int(i) => Ok(*i),
            Self::IntSlice(s) => Ok(s.iter().sum()),
            Self::Bool(true) => Ok(1),
            Self::Bool(false) => Ok(0),
            Self::BoolSlice(s) => Ok(s.iter().filter(|&v| *v).count() as i64),
        }
    }
    pub fn to_int_slice(&self) -> Result<Vec<i64>, String> {
        match self {
            Self::Int(i) => Err(format!("cannot convert {} to slice", i)),
            Self::IntSlice(s) => Ok(s.to_vec()),
            Self::Bool(b) => Err(format!("cannot convert {} to slice", b)),
            Self::BoolSlice(s) => Ok(s.iter().map(|&v| if v { 1 } else { 0 }).collect()),
        }
    }
}
