// 示例：演示 include_zstd! 宏的使用
//
// 运行方式：
//   cargo run --example example_usage        # 运行示例
//   cargo test --example example_usage       # 运行测试

fn main() {
    // 使用 include_zstd! 宏加载文件
    // 路径相对于此示例文件所在的目录 (examples/)
    let asset = include_zstd::include_zstd!("sample.txt");

    // 获取文件元数据
    let metadata = asset.metadata();
    println!("文件大小: {} 字节", metadata.len);
    println!("是否为文件: {}", metadata.is_file);
    println!("是否为目录: {}", metadata.is_dir);

    if let Some(modified) = metadata.modified {
        println!("修改时间: {:?}", modified);
    }

    // 获取解压后的文件内容
    let bytes = asset.bytes();
    println!("解压后内容长度: {} 字节", bytes.len());
    println!(
        "内容预览:\n{}",
        String::from_utf8_lossy(&bytes[..bytes.len().min(200)])
    );

    // 验证缓存机制：多次调用 bytes() 返回相同的内存地址
    let ptr1 = asset.bytes().as_ptr();
    let ptr2 = asset.bytes().as_ptr();
    assert_eq!(ptr1, ptr2, "缓存机制应该返回相同的指针");
    println!("\n缓存验证通过：两次调用 bytes() 返回相同指针");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_include_zstd_loads_file() {
        let asset = include_zstd::include_zstd!("sample.txt");

        // 验证元数据
        assert!(asset.metadata().is_file);
        assert!(!asset.metadata().is_dir);
        assert_eq!(asset.metadata().len, 82); // sample.txt 的实际大小
    }

    #[test]
    fn test_include_zstd_bytes_correct() {
        let asset = include_zstd::include_zstd!("sample.txt");
        let bytes = asset.bytes();

        // 验证解压后的内容
        let content = String::from_utf8_lossy(bytes);
        assert!(content.contains("Hello, include-zstd!"));
        assert!(content.contains("sample file for testing"));
    }

    #[test]
    fn test_include_zstd_caching() {
        let asset = include_zstd::include_zstd!("sample.txt");

        // 验证缓存机制
        let ptr1 = asset.bytes().as_ptr();
        let ptr2 = asset.bytes().as_ptr();
        assert_eq!(ptr1, ptr2, "多次调用 bytes() 应该返回相同指针");
    }
}
