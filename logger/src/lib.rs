use std::io::{self, Write};

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => ({
        let mut out = io::stdout();
        let s = std::format!("{}", std::format_args!($($arg)*));
        out.write_all(s.as_bytes());
    })
}

struct Logger<'a> {
    name: &'a str
}

impl Logger<'_> {
    fn new(name: &str) -> Logger {
        Logger {
            name
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_works() {
        log!("Hi {:08x}", 123);
    }
}