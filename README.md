# logosaurus

[![crates.io](https://img.shields.io/crates/v/logosaurus.svg)](https://crates.io/crates/logosaurus)
[![docs.rs](https://docs.rs/logosaurus/badge.svg)](https://docs.rs/logosaurus)

Rust logging implementation modeled after the Go standard library log package.
It works with the [`log`](https://crates.io/crates/log) crate.

## Documentation

See [docs.rs](https://docs.rs/logosaurus).

## Examples

### Using the default logger

```rust
use log::{debug};
use logosaurus::{Logger};

fn main() {
  logosaurus::init(Logger::default()).unwrap();
  debug!("hello, world"); // DEBUG 2020/10/02 21:27:03 hello, world
}
```

### Using a custom logger

```rust
use log::{self, debug};
use logosaurus::{Logger, L_STD, L_SHORT_FILE, L_MICROSECONDS};
use std::io;

fn main() {
  let logger = Logger::builder()
                  .set_level(log::LevelFilter::Debug)
                  .set_out(Box::new(io::stderr()))
                  .set_flags(L_STD | L_SHORT_FILE | L_MICROSECONDS)
                  .set_prefix("myprogram: ")
                  .build();

  logosaurus::init(logger).unwrap();
  debug!("hello, world"); // myprogram: DEBUG 2020/10/02 21:27:03.123123 main.rs:12: hello, world
}
```
