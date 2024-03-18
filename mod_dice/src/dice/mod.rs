mod ast;
mod value;

#[cfg(test)]
mod test;

pub use ast::Command;

pub mod limits {
    pub struct Limiter {
        entropy: u64,
    }

    impl Limiter {
        pub fn new(entropy: u64) -> Self {
            Self { entropy }
        }

        pub fn use_entropy(&mut self, count: u64, options: u64) -> Result<(), String> {
            let entropy = options
                .checked_next_power_of_two()
                .map(u64::trailing_zeros)
                .map(u64::from)
                .unwrap_or(64)
                .checked_mul(count)
                .ok_or("overflow calculating entropy")?;

            self.entropy = self.entropy.checked_sub(entropy).ok_or("roll too complex")?;

            Ok(())
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn test_limiter() {
            let mut l = Limiter::new(10);

            assert_eq!(l.use_entropy(1, 8), Ok(()));
            assert_eq!(l.use_entropy(1, 8), Ok(()));
            assert_eq!(l.use_entropy(1, 8), Ok(()));
            assert_eq!(l.use_entropy(1, 8), Err("roll too complex".into()));

            let mut l = Limiter::new(64);

            assert_eq!(l.use_entropy(1, u64::MAX), Ok(()));
            assert_eq!(l.use_entropy(1, 2), Err("roll too complex".into()));

            assert_eq!(l.use_entropy(u64::MAX, 4), Err("overflow calculating entropy".into()));
        }
    }
}
