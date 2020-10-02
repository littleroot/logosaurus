use chrono::{self, Timelike};
use log;
use std::fmt;
use std::io;
use std::path;
use std::sync::Mutex;

#[derive(Clone, Copy)]
pub enum Output {
    Stdout,
    Stderr,
}

pub type Flag = u8;

pub const L_NONE: Flag = 0;
pub const L_DATE: Flag = 1;
pub const L_TIME: Flag = 2;
pub const L_MICROSECONDS: Flag = 4;
pub const L_LONG_FILE: Flag = 8;
pub const L_SHORT_FILE: Flag = 16;
pub const L_UTC: Flag = 32;
pub const L_MSG_PREFIX: Flag = 64;
pub const L_LEVEL: Flag = 128;
pub const L_STD: Flag = L_DATE | L_TIME | L_LEVEL;

pub struct LoggerBuilder {
    level: log::LevelFilter,
    output: Output,
    flag: Flag,
    prefix: String,
}

impl LoggerBuilder {
    pub fn set_level<'a>(&'a mut self, level: log::LevelFilter) -> &'a mut LoggerBuilder {
        self.level = level;
        self
    }

    pub fn set_output<'a>(&'a mut self, output: Output) -> &'a mut LoggerBuilder {
        self.output = output;
        self
    }

    pub fn set_flags<'a>(&'a mut self, flag: Flag) -> &'a mut LoggerBuilder {
        self.flag = flag;
        self
    }

    pub fn set_prefix<'a>(&'a mut self, prefix: &str) -> &'a mut LoggerBuilder {
        self.prefix = String::from(prefix);
        self
    }

    pub fn build(&self) -> Logger {
        Logger {
            mu: Mutex::new(()),
            level: self.level,
            output: self.output,
            flag: self.flag,
            prefix: self.prefix.clone(),
        }
    }
}

pub struct Logger {
    mu: Mutex<()>,
    level: log::LevelFilter,
    output: Output,
    flag: Flag,
    prefix: String,
}

pub fn init(l: Logger) -> Result<(), log::SetLoggerError> {
    log::set_max_level(l.level);
    log::set_boxed_logger(Box::new(l))
}

impl Logger {
    pub fn builder() -> LoggerBuilder {
        LoggerBuilder {
            level: log::LevelFilter::Trace,
            output: Output::Stderr,
            flag: L_STD,
            prefix: String::from(""),
        }
    }

    pub fn level(&self) -> log::LevelFilter {
        let _lock = self.mu.lock();
        self.level
    }

    pub fn set_level(&mut self, level: log::LevelFilter) {
        let _lock = self.mu.lock();
        self.level = level;
    }

    pub fn output(&self) -> Output {
        let _lock = self.mu.lock();
        self.output
    }

    pub fn set_output(&mut self, output: Output) {
        let _lock = self.mu.lock();
        self.output = output;
    }

    pub fn flags(&self) -> Flag {
        let _lock = self.mu.lock();
        self.flag
    }

    pub fn set_flags(&mut self, flag: Flag) {
        let _lock = self.mu.lock();
        self.flag = flag;
    }

    fn out(&self) -> Box<dyn io::Write> {
        match self.output() {
            Output::Stdout => Box::new(io::stdout()),
            Output::Stderr => Box::new(io::stderr()),
        }
    }

    fn write_output(&self, record: &log::Record) {
        let now = chrono::offset::Local::now(); // get this early
        let args = record.args().to_string();
        let maybe_newline = if args.ends_with("\n") { "" } else { "\n" };

        self.write_header(&mut self.out(), record, now);

        let mut out = self.out();
        let _lock = self.mu.lock().unwrap(); // lock for write
        let _ = write!(out, "{}{}", args, maybe_newline);
    }

    fn write_datetime<Tz: chrono::TimeZone, W: io::Write>(
        &self,
        w: &mut W,
        flag: Flag,
        now: chrono::DateTime<Tz>,
    ) where
        Tz::Offset: fmt::Display,
    {
        if flag & L_DATE != 0 {
            let _ = write!(w, "{} ", now.format("%Y/%m/%d"));
        }
        if flag & (L_TIME | L_MICROSECONDS) != 0 {
            let _ = write!(w, "{}", now.format("%H:%M:%S"));
            if flag & L_MICROSECONDS != 0 {
                let micro = now.nanosecond() / 1000;
                let _ = write!(w, ".{:0wid$}", micro, wid = 6);
            }
            let _ = write!(w, " ");
        }
    }

    fn write_header<Tz: chrono::TimeZone, W: io::Write>(
        &self,
        w: &mut W,
        record: &log::Record,
        now: chrono::DateTime<Tz>,
    ) where
        Tz::Offset: fmt::Display,
    {
        let flag = self.flags();
        let _lock = self.mu.lock().unwrap(); // lock for writes

        // TODO: flag msg prefix

        if flag & L_LEVEL != 0 {
            let _ = write!(w, "{} ", record.level());
        }

        if flag & (L_DATE | L_TIME | L_MICROSECONDS) != 0 {
            if flag & L_UTC != 0 {
                let now = now.with_timezone(&chrono::Utc);
                self.write_datetime(w, flag, now);
            } else {
                self.write_datetime(w, flag, now);
            }
        }

        if flag & (L_LONG_FILE | L_SHORT_FILE) != 0 {
            if flag & L_LONG_FILE != 0 {
                let _ = write!(w, "{} ", record.target());
            }
            if let Some(f) = record.file() {
                if flag & L_SHORT_FILE != 0 {
                    // only use basename of path
                    let p = path::Path::new(f);
                    if let Some(base) = p.file_name() {
                        if let Some(s) = base.to_str() {
                            let _ = write!(w, "{}", s);
                        }
                    }
                } else {
                    // write whole path
                    let _ = write!(w, "{}", f);
                }
                if let Some(n) = record.line() {
                    let _ = write!(w, ":{}", n);
                }
                let _ = write!(w, ": ");
            }
        }
    }
}

impl Default for Logger {
    fn default() -> Logger {
        Logger::builder().build()
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level()
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
