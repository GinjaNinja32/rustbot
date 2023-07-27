use std::collections::BTreeMap;

use rustbot::prelude::Span;
use rustbot::spans;

mod ast;
mod value;
pub mod limits {
    pub struct Limiter {
        entropy: u64,
    }

    impl Limiter {
        pub fn new(entropy: u64) -> Self {
            Self { entropy }
        }

        pub fn use_entropy(&mut self, count: u64, options: u64) -> Result<(), String> {
            let entropy = match options
                .checked_next_power_of_two()
                .map(|v| v.trailing_zeros())
                .map(|v| count.checked_mul(v as u64))
                .flatten()
            {
                Some(v) => v,
                None => return Err("overflow calculating entropy".to_string()),
            };

            if self.entropy < entropy {
                Err("roll too complex".to_string())
            } else {
                self.entropy -= entropy;
                Ok(())
            }
        }
    }
}

pub trait Evaluable {
    fn eval(
        &self,
        limit: &mut limits::Limiter,
        values: &BTreeMap<char, value::Value>,
    ) -> Result<(Vec<Span>, value::Value), String>;
}

pub fn parse(input: &str) -> Result<ast::Command, String> {
    ast::command(input).map(|(_, c)| c).map_err(|e| format!("{:?}", e))
}

pub fn eval(cmd: &ast::Command, mut limit: limits::Limiter) -> Result<Vec<Span>, String> {
    cmd.eval(&mut limit)
}
