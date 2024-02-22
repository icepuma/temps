# temps

![https://crates.io/crates/fbtoggl](https://img.shields.io/crates/v/temps)
![https://github.com/icepuma/temps/actions/workflows/ci.yml](https://github.com/icepuma/temps/actions/workflows/ci.yml/badge.svg)

`temps` or `[tã]` is a library for working with time and dates in Rust.

## Usage

Add `temps` to your `Cargo.toml`.

```toml
temps = "0.1"
```

## hh:mm:ss

I migrated the functionality of [hhmmss](https://github.com/TianyiShi2001/hhmmss) into `temps` as it is not actively maintained anymore.

```rust
let duration = std::time::Duration::new(10, 0); // also works for "chrono::Duration" and "time::Duration"

duration.hhmmss() // yields "00:00:10"
duration.hhmmssxxx() // yields "00:00:10.000"
```
