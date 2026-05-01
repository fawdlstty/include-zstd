extern crate self as include_zstd;

pub use include_zstd_derive::{bytes, file_bytes, file_str, include_zstd, r#str};
use std::sync::OnceLock;
use std::time::SystemTime;

/// 文件元数据信息（从原始文件提取）
pub struct ZstdMetadata {
    pub len: u64,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub is_file: bool,
    pub is_dir: bool,
}

/// 编译期压缩的 zstd 资源
pub struct ZstdAsset {
    metadata: ZstdMetadata,
    compressed: &'static [u8],
    cache: OnceLock<Box<[u8]>>,
}

impl ZstdAsset {
    /// 返回原始文件的元数据信息
    pub fn metadata(&self) -> &ZstdMetadata {
        &self.metadata
    }

    /// 返回 zstd 解压后的文件内容（惰性解压，首次调用时解压并缓存）
    pub fn bytes(&self) -> &[u8] {
        self.cache
            .get_or_init(|| __private::decompress_bytes(self.compressed))
            .as_ref()
    }
}

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

    pub fn create_zstd_asset(
        metadata: crate::ZstdMetadata,
        compressed: &'static [u8],
    ) -> crate::ZstdAsset {
        crate::ZstdAsset {
            metadata,
            compressed,
            cache: std::sync::OnceLock::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ZstdAsset;

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

    fn include_zstd_fixture() -> ZstdAsset {
        // 使用相对于 src/lib.rs 的路径，与宏的解析逻辑一致
        crate::include_zstd!("../Cargo.toml")
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

    #[test]
    fn zstd_asset_bytes_matches_original() {
        let asset = include_zstd_fixture();
        let expected = include_bytes!("../Cargo.toml");
        assert_eq!(asset.bytes(), expected);
    }

    #[test]
    fn zstd_asset_metadata_len_matches() {
        let asset = include_zstd_fixture();
        // 宏在编译期相对于 src/lib.rs 解析路径，所以 ../Cargo.toml 指向 include-zstd/Cargo.toml
        let expected_len =
            std::fs::metadata(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
                .unwrap()
                .len();
        assert_eq!(asset.metadata().len, expected_len);
    }

    #[test]
    fn zstd_asset_bytes_cached() {
        let asset = include_zstd_fixture();
        let first = asset.bytes().as_ptr();
        let second = asset.bytes().as_ptr();
        assert_eq!(first, second);
    }

    #[test]
    fn zstd_asset_metadata_is_file() {
        let asset = include_zstd_fixture();
        assert!(asset.metadata().is_file);
        assert!(!asset.metadata().is_dir);
    }
}
