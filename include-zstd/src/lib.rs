extern crate self as include_zstd;

pub use include_zstd_derive::{bytes, file_bytes, file_str, r#str};

#[doc(hidden)]
pub mod __private {
    pub fn decode_utf8(bytes: &'static [u8]) -> &'static str {
        std::str::from_utf8(bytes).unwrap_or_else(|err| {
            panic!("include_zstd::str!/file_str! decoded data is not UTF-8: {err}")
        })
    }

    pub fn decompress_bytes(compressed: &[u8]) -> Box<[u8]> {
        zstd::stream::decode_all(compressed)
            .unwrap_or_else(|err| panic!("include_zstd decode failed: {err}"))
            .into_boxed_slice()
    }
}

#[cfg(test)]
mod tests {
    fn include_str_fixture() -> &'static str {
        crate::str!("hello include-zstd")
    }

    fn include_bytes_fixture() -> &'static [u8] {
        crate::bytes!(b"\x00\x01\x02\x03")
    }

    fn include_file_str_fixture() -> &'static str {
        crate::file_str!("../Cargo.toml")
    }

    fn include_file_fixture() -> &'static [u8] {
        crate::file_bytes!("../Cargo.toml")
    }

    #[test]
    fn str_matches_original_text() {
        let expected: &'static str = "hello include-zstd";
        let actual: &'static str = include_str_fixture();

        assert_eq!(actual, expected);
    }

    #[test]
    fn binary_matches_original_bytes() {
        let expected: &'static [u8] = b"\x00\x01\x02\x03";
        let actual: &'static [u8] = include_bytes_fixture();

        assert_eq!(actual, expected);
    }

    #[test]
    fn file_str_matches_include_str() {
        let expected: &'static str = include_str!("../Cargo.toml");
        let actual: &'static str = include_file_str_fixture();

        assert_eq!(actual, expected);
    }

    #[test]
    fn file_matches_include_bytes() {
        let expected: &'static [u8] = include_bytes!("../Cargo.toml");
        let actual: &'static [u8] = include_file_fixture();

        assert_eq!(actual, expected);
    }

    #[test]
    fn macros_use_once_lock_for_each_callsite() {
        let first = include_str_fixture().as_ptr();
        let second = include_str_fixture().as_ptr();
        assert_eq!(first, second);

        let first = include_bytes_fixture().as_ptr();
        let second = include_bytes_fixture().as_ptr();
        assert_eq!(first, second);

        let first = include_file_str_fixture().as_ptr();
        let second = include_file_str_fixture().as_ptr();
        assert_eq!(first, second);

        let first = include_file_fixture().as_ptr();
        let second = include_file_fixture().as_ptr();
        assert_eq!(first, second);
    }
}
