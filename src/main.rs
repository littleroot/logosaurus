use log::info;
use package_log::{self, L_LONG_FILE, L_MICROSECONDS, L_SHORT_FILE, L_STD, L_UTC};

fn main() {
    let mut b = package_log::Logger::builder();
    let logger = b
        .set_level(log::LevelFilter::Debug)
        .set_flags(L_STD | L_SHORT_FILE)
        .set_prefix("foo: ")
        .build();

    package_log::init(logger).unwrap();

    info!("hello, world!");
}
