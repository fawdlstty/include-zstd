# include-zstd

![version](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Ffawdlstty%2Finclude-zstd%2Fmain%2F/include-zstd/Cargo.toml&query=package.version&label=version)
![status](https://img.shields.io/github/actions/workflow/status/fawdlstty/include-zstd/rust.yml)

[English](README.md) | 简体中文

`include-zstd` 是一个 Rust 宏库，用于在**编译期压缩**文本或二进制数据，并在运行时按需解压，返回 `&'static str` 或 `&'static [u8]`。

## 用法说明

### 1. 添加依赖

执行命令：

```shell
cargo add include-zstd
```

### 2. 常用宏

- `include_zstd::str!("...")`：内联字符串，返回 `&'static str`
- `include_zstd::bytes!(b"...")`：内联字节串，返回 `&'static [u8]`
- `include_zstd::file_str!("path")`：读取文件并按 UTF-8 返回 `&'static str`
- `include_zstd::file_bytes!("path")`：读取文件并返回 `&'static [u8]`

### 4. 示例

```rust
fn main() {
    let _msg: &'static str = include_zstd::str!("hello include-zstd");
    let _raw: &'static [u8] = include_zstd::bytes!(b"\x00\x01\x02\x03");
    let _text: &'static str = include_zstd::file_str!("data/sample.txt");
    let _bytes: &'static [u8] = include_zstd::file_bytes!("data/sample.bin");
}
```
