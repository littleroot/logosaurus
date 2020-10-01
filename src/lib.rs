use chrono;
use log;
use std::io;

#[derive(Clone, Copy)]
pub enum Output {
    Stdout,
    Stderr,
}

pub type Flag = u8;

pub const F_DATE: Flag = 0b0001;
pub const F_TIME: Flag = 0b0010;
pub const F_SHORTFILE: Flag = 0b0100;
pub const F_STD: Flag = F_DATE | F_TIME;

pub struct LoggerBuilder {
    level: log::LevelFilter,
    out: Output,
    flag: Flag,
    prefix: String,
}

impl LoggerBuilder {
    pub fn new() -> LoggerBuilder {
        LoggerBuilder {
            level: log::LevelFilter::Trace,
            out: Output::Stderr,
            flag: F_STD,
            prefix: String::from(""),
        }
    }

    pub fn set_level<'a>(&'a mut self, level: log::LevelFilter) -> &'a mut LoggerBuilder {
        self.level = level;
        self
    }

    pub fn set_output<'a>(&'a mut self, output: Output) -> &'a mut LoggerBuilder {
        self.out = output;
        self
    }

    pub fn set_flag<'a>(&'a mut self, flag: Flag) -> &'a mut LoggerBuilder {
        self.flag = flag;
        self
    }

    pub fn set_prefix<'a>(&'a mut self, prefix: &str) -> &'a mut LoggerBuilder {
        self.prefix = String::from(prefix);
        self
    }

    pub fn build(&self) -> Logger {
        Logger {
            level: self.level,
            out: self.out,
            flag: self.flag,
            prefix: self.prefix.clone(),
        }
    }
}

pub struct Logger {
    level: log::LevelFilter,
    out: Output,
    flag: Flag,
    prefix: String,
}

pub fn init(l: Logger) -> Result<(), log::SetLoggerError> {
    log::set_max_level(l.level);
    log::set_boxed_logger(Box::new(l))
}

impl Logger {
    fn out(&self) -> Box<dyn io::Write> {
        match self.out {
            Output::Stdout => Box::new(io::stdout()),
            Output::Stderr => Box::new(io::stderr()),
        }
    }

    fn write_output(&self, record: &log::Record) {
        let s = record.args().to_string();
        let newline = if s.ends_with("\n") { "" } else { "\n" };

        let _ = write!(self.out(), "{}{}{}", self.header(record), s, newline);
    }

    fn header(&self, record: &log::Record) -> String {
        let now = chrono::offset::Local::now(); // get this early
        let mut buf = String::new();

        if self.flag & (F_DATE | F_TIME) != 0 {
            if self.flag & F_DATE != 0 {
                buf.push_str(&now.format("%Y/%m/%d").to_string());
                buf.push_str(" ");
            }
            if self.flag & F_TIME != 0 {
                buf.push_str(&now.format("%H:%M:%S").to_string());
                buf.push_str(" ");
            }
        }

        buf
    }
}

impl Default for Logger {
    fn default() -> Logger {
        LoggerBuilder::new().build()
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        self.write_output(record);
    }

    fn flush(&self) {
        let _ = self.out().flush();
    }
}
