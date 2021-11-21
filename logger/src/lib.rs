use std::io::Write;
use std::cell::RefCell;

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Level {
    Debug, Info, Warn, Error, Off
}

#[macro_export]
macro_rules! dbg {
    ($logger:expr,$($arg:tt)*) => ({
        let s = std::format!("{}", std::format_args!($($arg)*));
        $logger.write(Level::Debug, s);
    })
}

#[macro_export]
macro_rules! warn {
    ($logger:expr,$($arg:tt)*) => ({
        let s = std::format!("{}", std::format_args!($($arg)*));
        $logger.write(Level::Warn, s);
    })
}

#[macro_export]
macro_rules! info {
    ($logger:expr,$($arg:tt)*) => ({
        let s = std::format!("{}", std::format_args!($($arg)*));
        $logger.write(Level::Info, s);
    })
}

#[macro_export]
macro_rules! err {
    ($logger:expr,$($arg:tt)*) => ({
        let s = std::format!("{}", std::format_args!($($arg)*));
        $logger.write(Level::Error, s);
    })
}

pub struct Logger {
    name: String,
    level: Level,
    out: RefCell<Box<dyn Write>>,
}

impl Logger {
    pub fn new(name: &str, level: Level) -> Logger {
        Logger {
            name: String::from(name),
            level,
            out: RefCell::new(Box::new(std::io::stdout()))
        }
    }

    pub fn new_with_out(name: &str, level: Level, out: Box<dyn Write>) -> Logger {
        Logger {
            name: String::from(name),
            level,
            out: RefCell::new(out),
        }
    }

    pub fn write(&self, level: Level, line: String) {
        if level >= self.level {
            let s = std::format!("[{}] {}\n", self.name, line);
            self.out.borrow_mut().write_all(s.as_bytes()).expect("Could not write a log line");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct Tester {
        logger: Logger
    }

    impl Tester {
        fn new(logger: Logger) -> Tester {
            Tester { logger }
        }

        fn run(&self) {
            dbg!(self.logger, "A debugging message");
            info!(self.logger, "An info message");
            warn!(self.logger, "A warning");
            err!(self.logger, "An error");
        }
    }

    /// Run with --nocapture to acknowledge the results
    #[test]
    fn it_prints_to_stdout() {
        let t = Tester::new(Logger::new("TST", Level::Warn));
        t.run();
    }
}