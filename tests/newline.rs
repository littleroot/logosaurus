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

    let logger = Logger::builder()
        .set_out(Box::new(w))
        .set_flags(L_NONE)
        .build();
    init(logger).unwrap();

    warn!("message0");
    warn!("message1\n\n");
    warn!("message2\n");

    let expect = r"message0
message1

message2
";
    let got = arc.lock().unwrap();
    let got = str::from_utf8(got.as_slice()).unwrap();
    assert_eq!(expect, got);
}
