use log::*;
use logosaurus::test_util::SyncWriter;
use logosaurus::*;
use std::str;
use std::sync::{Arc, Mutex};

#[test]
fn test_level_filter() {
    let v = Mutex::new(Vec::new());
    let arc = Arc::new(v);
    let w = SyncWriter::new(Arc::clone(&arc));

    let logger = Logger::builder(w)
        .set_level(log::LevelFilter::Warn)
        .set_flags(L_LEVEL)
        .build();
    init(logger).unwrap();

    trace!("suppressed trace message");
    debug!("suppressed debug message");
    info!("suppressed info message");
    warn!("warn message");
    error!("error message");

    let expect = r"WARN  warn message
ERROR error message
";
    let got = arc.lock().unwrap();
    let got = str::from_utf8(got.as_slice()).unwrap();
    assert_eq!(expect, got);
}
