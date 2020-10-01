use log::info;
use package_log::{self, F_SHORTFILE, F_STD};

fn main() {
    let mut b = package_log::LoggerBuilder::new();
    let logger = b
        .set_level(log::LevelFilter::Debug)
        .set_flag(F_SHORTFILE | F_STD)
        .set_prefix("package_log")
        .build();

    package_log::init(logger).unwrap();

    info!("hello, world!");
}
