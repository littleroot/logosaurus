//! The logosaurus crate provides a logging implementation that works with the [`log`] crate. The
//! crate and the logger are modeled after the Go standard library's log package.
//!
//! The primary type is [`Logger`], which represents a logging object. Use
//! [`init`] to globally initialize a logger with the `log` crate.
//!
//! Every log message is output on a separate line: if the message being printed does not end in a
//! newline, the logger will add one.
//!
//! The default logger writes logs to stderr using this format:
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
//!   let logger = Logger::builder(io::stdout())
//!                   .set_level(log::LevelFilter::Debug)
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
/// let mut builder = Logger::builder(io::stdout());
/// let logger = builder
///                 .set_level(log::LevelFilter::Debug)
///                 .set_flags(L_STD | L_SHORT_FILE)
///                 .set_prefix("myprogram: ")
///                 .build();
/// ```
///
/// [`Logger`]: struct.Logger.html
/// [`Logger::default()`]: struct.Logger.html#impl-Default
pub struct LoggerBuilder<W: Write + Send> {
    level: log::LevelFilter,
    out: Option<W>,
    flag: Flag,
    prefix: String,
}

impl<W: Write + Send> LoggerBuilder<W> {
    /// Set the allowed log level.
    pub fn set_level(mut self, level: log::LevelFilter) -> LoggerBuilder<W> {
        self.level = level;
        self
    }

    /// Set the formatting flags.
    pub fn set_flags(mut self, flag: Flag) -> LoggerBuilder<W> {
        self.flag = flag;
        self
    }

    /// Set the prefix.
    pub fn set_prefix(mut self, prefix: &str) -> LoggerBuilder<W> {
        self.prefix = String::from(prefix);
        self
    }

    /// Construct a `Logger` from this `LoggerBuilder`. Consumes the
    /// `LoggerBuilder`.
    pub fn build(mut self) -> Logger<W> {
        Logger {
            level: self.level,
            out: Mutex::new(self.out.take().unwrap()),
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
pub struct Logger<W: Write + Send> {
    level: log::LevelFilter,
    out: Mutex<W>,
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
//
/// See [`LoggerBuilder`] to create a custom logger.
///
/// [`log`]: https://crates.io/crates/log
/// [`LoggerBuilder`]: struct.LoggerBuilder.html
pub fn init<W: Write + Send + 'static>(l: Logger<W>) -> Result<(), log::SetLoggerError> {
    log::set_max_level(l.level);
    log::set_boxed_logger(Box::new(l))
}

impl<W: Write + Send> Logger<W> {
    /// Returns a `LoggerBuilder` that can be used to build a `Logger`.
    pub fn builder(w: W) -> LoggerBuilder<W> {
        LoggerBuilder {
            level: log::LevelFilter::Trace,
            out: Some(w),
            flag: L_STD,
            prefix: String::from(""),
        }
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

        let mut out = self.out.lock().unwrap();
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
        header_with_flags(target, file, line, level, now, self.flag, &self.prefix)
    }

    fn enabled(&self, incoming_level: log::Level) -> bool {
        incoming_level <= self.level
    }
}

fn format_datetime<Tz: chrono::TimeZone>(buf: &mut String, flag: Flag, now: chrono::DateTime<Tz>)
where
    Tz::Offset: fmt::Display,
{
    if flag & L_DATE != 0 {
        buf.push_str(&format!("{} ", now.format("%Y/%m/%d")));
    }
    if flag & (L_TIME | L_MICROSECONDS) != 0 {
        buf.push_str(&format!("{}", now.format("%H:%M:%S")));
        if flag & L_MICROSECONDS != 0 {
            let micro = now.nanosecond() / 1000;
            buf.push_str(&format!(".{:0>wid$}", micro, wid = 6));
        }
        buf.push_str(&format!(" "));
    }
}

fn header_with_flags<Tz: chrono::TimeZone>(
    target: &str,
    file: &str,
    line: u32,
    level: log::Level,
    now: chrono::DateTime<Tz>,
    flag: Flag,
    prefix: &str,
) -> String
where
    Tz::Offset: fmt::Display,
{
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
            format_datetime(&mut buf, flag, now);
        } else {
            format_datetime(&mut buf, flag, now);
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

impl Default for Logger<io::Stderr> {
    /// Returns a default `Logger`.
    ///
    /// A default `Logger` has
    ///   * level:  `log::LevelFilter::Trace`,
    ///   * out:    stderr,
    ///   * flags:  `L_STD`, and
    ///   * prefix: `""` (empty string)
    ///
    fn default() -> Logger<io::Stderr> {
        Logger::builder(io::stderr()).build()
    }
}

impl<W: Write + Send> log::Log for Logger<W> {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.enabled(metadata.level())
    }

    fn log(&self, record: &log::Record) {
        self.write_record(record);
    }

    fn flush(&self) {
        let _ = self.out.lock().unwrap().flush();
    }
}

#[doc(hidden)]
pub mod test_util;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::prelude::*;

    #[test]
    fn test_header() {
        let time = FixedOffset::east(3600 * 5 + 1800)
            .ymd(2020, 10, 3)
            .and_hms_micro(1, 2, 3, 9876);

        let flags = L_STD | L_MICROSECONDS | L_SHORT_FILE;
        let expect = "TRACE 2020/10/03 01:02:03.009876 file.rs:9: ";
        let got = header_with_flags(
            "foo",
            "src/dir/file.rs",
            9,
            log::Level::Trace,
            time,
            flags,
            "",
        );
        assert_eq!(expect, got);

        let flags = L_DATE | L_TIME | L_UTC | L_LONG_FILE;
        let expect = "2020/10/02 19:32:03 foo src/dir/file.rs:9: ";
        let got = header_with_flags(
            "foo",
            "src/dir/file.rs",
            9,
            log::Level::Info,
            time,
            flags,
            "",
        );
        assert_eq!(expect, got);

        let flags = L_TIME | L_LEVEL;
        let prefix = "myprog: ";
        let expect = "myprog: INFO  01:02:03 ";
        let got = header_with_flags(
            "foo",
            "src/dir/file.rs",
            9,
            log::Level::Info,
            time,
            flags,
            prefix,
        );
        assert_eq!(expect, got);

        let flags = L_MSG_PREFIX | L_TIME | L_LEVEL;
        let expect = "INFO  01:02:03 myprog: ";
        let got = header_with_flags(
            "foo",
            "src/dir/file.rs",
            9,
            log::Level::Info,
            time,
            flags,
            prefix,
        );
        assert_eq!(expect, got);
    }
}
