//! The logosaurus crate provides a logging implementation that works with the [`log`] crate. The
//! crate and the logger are modeled after the Go standard library's log package.
//!
//! The primary type is [`Logger`], which represents a logging object.
//!
//! Use [`init`] to globally initialize a logger with the `log` crate.
//!
//! Every log message is output on a separate line: if the message being printed does not end in a
//! newline, the logger will add one.
//!
//! Sample default log output:
//! ```txt
//! WARN  2020/10/02 21:27:03 hello, world
//! ```
//!
//! # Examples
//!
//! ## Using the default logger
//!
//! ```
//! use log::{debug};
//! use logosaurus::{Logger};
//!
//! fn main() {
//!   logosaurus::init(Logger::default()).unwrap();
//!   debug!("hello, world"); // DEBUG 2020/10/02 21:27:03 hello, world
//! }
//! ```
//!
//! ## Using a custom logger
//!
//! ```
//! use log::{self, debug};
//! use logosaurus::{Logger, L_STD, L_SHORT_FILE, L_MICROSECONDS};
//! use std::io;
//!
//! fn main() {
//!   let logger = Logger::builder()
//!                   .set_level(log::LevelFilter::Debug)
//!                   .set_out(Box::new(io::stderr()))
//!                   .set_flags(L_STD | L_SHORT_FILE | L_MICROSECONDS)
//!                   .set_prefix("myprogram: ")
//!                   .build();
//!
//!   logosaurus::init(logger).unwrap();
//!   debug!("hello, world"); // myprogram: DEBUG 2020/10/02 21:27:03.123123 main.rs:12: hello, world
//! }
//! ```
//!
//! [`log`]: https://crates.io/crates/log
//! [`Logger`]: struct.Logger.html
//! [`init`]: fn.init.html
use chrono::{self, Timelike};
use log;
use std::fmt;
use std::io::{self, Write};
use std::path;
use std::sync::Arc;
use std::sync::Mutex;

/// Formatting flags for the header in log output.
/// See the `L_*` constants.
///
/// With the exception of the `L_MSG_PREFIX` flag, there is no control over the order that header
/// text appears, or the format they present (described in the `L*` constants).
///
/// For example, the `L_DATE | L_TIME` flags produce:
/// ```txt
/// 2009/01/23 17:05:23 message
/// ```
/// while `L_DATE | L_TIME | L_MICROSECONDS | L_SHORT_FILE | L_LEVEL` produce:
/// ```txt
/// INFO  2009/01/23 17:05:23.123123 main.rs:3: message
/// ```
pub type Flag = u8;

/// No header.
pub const L_NONE: Flag = 0;
/// Date in local time zone: 2009/01/23.
pub const L_DATE: Flag = 1;
/// Time in local time zone: 17:05:23.
pub const L_TIME: Flag = 2;
/// Microsecond resolution: 17:05:23.023123; assumes `L_TIME`.
pub const L_MICROSECONDS: Flag = 4;
/// Module, file name, and line number: `foo src/file.rs:3`.
pub const L_LONG_FILE: Flag = 8;
/// Final file name element and line number: `file.rs:3`.
pub const L_SHORT_FILE: Flag = 16;
/// If `L_DATE` or `L_TIME` is set, use UTC rather than the local time.
pub const L_UTC: Flag = 32;
/// Move the "prefix" from the beginning of the header to the end of the header, just before the
/// message.
pub const L_MSG_PREFIX: Flag = 64;
/// Log level printed in capitalized form: INFO, TRACE, etc. Padded to width 5.
pub const L_LEVEL: Flag = 128;
/// Initial values for the default logger constructed with `Logger::default()`.
pub const L_STD: Flag = L_DATE | L_TIME | L_LEVEL;

// TODO: https://doc.rust-lang.org/beta/unstable-book/language-features/trait-alias.html
// Rewrite as trait alias when stable.
// trait W = Write + Send

/// Builder for [`Logger`].
///
/// Use `Logger:builder()` to obtain a `LoggerBuilder`.
///
/// Unmodified or unset values in the builder will default to the values used by
/// [`Logger::default()`].
///
/// # Example
///
/// ```
/// use log;
/// use logosaurus::{Logger, L_STD, L_SHORT_FILE};
/// use std::io;
///
/// let mut builder = Logger::builder();
/// let logger = builder.set_level(log::LevelFilter::Debug)
///                 .set_out(Box::new(io::stderr()))
///                 .set_flags(L_STD | L_SHORT_FILE)
///                 .set_prefix("myprogram: ")
///                 .build();
/// ```
///
/// [`Logger`]: struct.Logger.html
/// [`Logger::default()`]: struct.Logger.html#impl-Default
pub struct LoggerBuilder {
    level: log::LevelFilter,
    out: Arc<Mutex<Box<dyn Write + Send>>>,
    flag: Flag,
    prefix: String,
}

impl LoggerBuilder {
    /// Set the allowed log level.
    pub fn set_level<'a>(&'a mut self, level: log::LevelFilter) -> &'a mut LoggerBuilder {
        self.level = level;
        self
    }

    /// Set the destination where output should be written.
    pub fn set_out<'a>(&'a mut self, out: Box<dyn Write + Send>) -> &'a mut LoggerBuilder {
        self.out = Arc::new(Mutex::new(out));
        self
    }

    /// Set the formatting flags.
    pub fn set_flags<'a>(&'a mut self, flag: Flag) -> &'a mut LoggerBuilder {
        self.flag = flag;
        self
    }

    /// Set the prefix.
    pub fn set_prefix<'a>(&'a mut self, prefix: &str) -> &'a mut LoggerBuilder {
        self.prefix = String::from(prefix);
        self
    }

    /// Construct a `Logger` from this `LoggerBuilder`.
    pub fn build(&self) -> Logger {
        Logger {
            mu: Mutex::new(()),
            level: self.level,
            out: Arc::clone(&self.out),
            flag: self.flag,
            prefix: self.prefix.clone(),
        }
    }
}

/// Represents a logging object that writes to a specified output. It can be used simultaneously
/// from multiple threads; it guarantees to serialize writes.
///
/// Use [`LoggerBuilder`] to construct a `Logger`, or use `Logger::default()`.
///
/// [`LoggerBuilder`]: struct.LoggerBuilder.html
pub struct Logger {
    mu: Mutex<()>, // guards below fields
    level: log::LevelFilter,
    out: Arc<Mutex<Box<dyn Write + Send>>>,
    flag: Flag,
    prefix: String,
}

/// Initialize the logger to use with the [`log`] crate.
///
/// ```
/// use log::{debug};
///
/// fn main() {
///   logosaurus::init(logosaurus::Logger::default()).unwrap();
///   debug!("hello, world");
/// }
/// ```
///
/// See [`LoggerBuilder`] or [`Logger`] to initialize a custom logger.
///
/// [`log`]: https://crates.io/crates/log
/// [`LoggerBuilder`]: struct.LoggerBuilder.html
/// [`Logger`]: struct.Logger.html
pub fn init(l: Logger) -> Result<(), log::SetLoggerError> {
    log::set_max_level(l.level);
    log::set_boxed_logger(Box::new(l))
}

impl Logger {
    /// Returns a `LoggerBuilder` that can be used to build a `Logger`.
    pub fn builder() -> LoggerBuilder {
        LoggerBuilder {
            level: log::LevelFilter::Trace,
            out: Arc::new(Mutex::new(Box::new(io::stderr()))),
            flag: L_STD,
            prefix: String::from(""),
        }
    }

    /// Returns the current level.
    pub fn level(&self) -> log::LevelFilter {
        let _lock = self.mu.lock();
        self.level
    }

    /// Set the level.
    pub fn set_level(&mut self, level: log::LevelFilter) {
        let _lock = self.mu.lock();
        self.level = level;
    }

    /// Returns the destination where output will be written.
    pub fn out(&self) -> Arc<Mutex<Box<dyn Write + Send>>> {
        let _lock = self.mu.lock();
        Arc::clone(&self.out)
    }

    /// Set the destination to write output.
    pub fn set_out(&mut self, out: Box<dyn Write + Send>) {
        let _lock = self.mu.lock();
        self.out = Arc::new(Mutex::new(out));
    }

    /// Returns the current formatting flags.
    pub fn flags(&self) -> Flag {
        let _lock = self.mu.lock();
        self.flag
    }

    /// Set the formatting flags.
    pub fn set_flags(&mut self, flag: Flag) {
        let _lock = self.mu.lock();
        self.flag = flag;
    }

    /// Returns the current ouput prefix.
    pub fn prefix(&self) -> &str {
        let _lock = self.mu.lock();
        &self.prefix
    }

    /// Set the output prefix.
    pub fn set_prefix(&mut self, prefix: &str) {
        let _lock = self.mu.lock();
        self.prefix = String::from(prefix);
    }

    /// Writes the given string `s` using the logger. Typically, you would not use this directly
    /// but instead use the macros provided by the `log` crate.
    pub fn write_output(
        &self,
        level: log::Level,
        target: &str,
        file: Option<&str>,
        line: Option<u32>,
        s: &str,
    ) {
        if !self.enabled(level) {
            return;
        }

        let now = chrono::offset::Local::now(); // get this early
        let file = match file {
            Some(f) => f,
            None => "???",
        };
        let line = match line {
            Some(n) => n,
            None => 0,
        };
        let h = self.header(target, file, line, level, now);
        let maybe_newline = if s.ends_with("\n") { "" } else { "\n" };

        let out = self.out();
        let mut out = out.lock().unwrap();
        let _ = write!(out, "{}{}{}", h, s, maybe_newline);
    }

    fn write_record(&self, record: &log::Record) {
        self.write_output(
            record.level(),
            record.target(),
            record.file(),
            record.line(),
            &record.args().to_string(),
        );
    }

    fn format_datetime<Tz: chrono::TimeZone>(
        &self,
        buf: &mut String,
        flag: Flag,
        now: chrono::DateTime<Tz>,
    ) where
        Tz::Offset: fmt::Display,
    {
        if flag & L_DATE != 0 {
            buf.push_str(&format!("{} ", now.format("%Y/%m/%d")));
        }
        if flag & (L_TIME | L_MICROSECONDS) != 0 {
            buf.push_str(&format!("{}", now.format("%H:%M:%S")));
            if flag & L_MICROSECONDS != 0 {
                let micro = now.nanosecond() / 1000;
                buf.push_str(&format!(".{:0<wid$}", micro, wid = 6));
            }
            buf.push_str(&format!(" "));
        }
    }

    fn header<Tz: chrono::TimeZone>(
        &self,
        target: &str,
        file: &str,
        line: u32,
        level: log::Level,
        now: chrono::DateTime<Tz>,
    ) -> String
    where
        Tz::Offset: fmt::Display,
    {
        let flag = self.flags();
        let prefix = self.prefix();
        let mut buf = String::new();

        if flag & L_MSG_PREFIX == 0 {
            buf.push_str(&format!("{}", prefix));
        }

        if flag & L_LEVEL != 0 {
            buf.push_str(&format!("{: <5} ", level));
        }

        if flag & (L_DATE | L_TIME | L_MICROSECONDS) != 0 {
            if flag & L_UTC != 0 {
                let now = now.with_timezone(&chrono::Utc);
                self.format_datetime(&mut buf, flag, now);
            } else {
                self.format_datetime(&mut buf, flag, now);
            }
        }

        if flag & (L_LONG_FILE | L_SHORT_FILE) != 0 {
            if flag & L_LONG_FILE != 0 {
                buf.push_str(&format!("{} ", target));
            }

            // TODO: reduce String::from calls
            let f = if flag & L_SHORT_FILE != 0 {
                match path::Path::new(file).file_name() {
                    Some(base) => base.to_string_lossy().into_owned(),
                    None => String::from("???"),
                }
            } else {
                String::from(file)
            };
            buf.push_str(&format!("{}", f));

            buf.push_str(&format!(":{}", line));
            buf.push_str(&format!(": "));
        }

        if flag & L_MSG_PREFIX != 0 {
            buf.push_str(&format!("{}", prefix));
        }

        buf
    }

    fn enabled(&self, incoming_level: log::Level) -> bool {
        incoming_level <= self.level()
    }
}

impl Default for Logger {
    /// Returns a default `Logger`.
    ///
    /// A default `Logger` has
    ///   * level:  `log::LevelFilter::Trace`,
    ///   * out:    stderr,
    ///   * flags:  `L_STD`, and
    ///   * prefix: `""` (empty string)
    ///
    fn default() -> Logger {
        let b = Logger::builder();
        b.build()
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.enabled(metadata.level())
    }

    fn log(&self, record: &log::Record) {
        self.write_record(record);
    }

    fn flush(&self) {
        let _ = self.out().lock().unwrap().flush();
    }
}

pub mod test_util;

#[cfg(test)]
mod tests {}
