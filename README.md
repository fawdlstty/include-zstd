# include-zstd

![version](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Ffawdlstty%2Finclude-zstd%2Fmain%2F/include-zstd/Cargo.toml&query=package.version&label=version)
![status](https://img.shields.io/github/actions/workflow/status/fawdlstty/include-zstd/rust.yml)

English | [简体中文](README.zh.md)

`include-zstd` is a Rust macro library that **compresses** text or binary data at compile time and decompresses it on demand at runtime, returning either `&'static str` or `&'static [u8]`.

## Usage

### 1. Add dependency

Run:

```shell
cargo add include-zstd
```

### 2. Common macros

- `include_zstd::str!("...")`: inline string, returns `&'static str`
- `include_zstd::bytes!(b"...")`: inline byte string, returns `&'static [u8]`
- `include_zstd::file_str!("path")`: reads a file and returns `&'static str` as UTF-8
- `include_zstd::file_bytes!("path")`: reads a file and returns `&'static [u8]`

> Path resolution for `include_zstd::file_str!` / `include_zstd::file_bytes!` is the same as `include_str!` / `include_bytes!`: relative paths are resolved from the directory of the source file where the macro is invoked.

### 4. Example

```rust
fn main() {
    let msg: &'static str = include_zstd::str!("hello include-zstd");
    let raw: &'static [u8] = include_zstd::bytes!(b"\x00\x01\x02\x03");

    let text: &'static str = include_zstd::file_str!("data/sample.txt");
    let bytes: &'static [u8] = include_zstd::file_bytes!("data/sample.bin");

    println!("msg={msg}, text_len={}, bytes_len={}, raw_len={}", text.len(), bytes.len(), raw.len());
}
```
