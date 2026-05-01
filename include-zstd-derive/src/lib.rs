use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::quote;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use syn::parse::{Parse, ParseStream};
use syn::{LitByteStr, LitStr, Token, parse_macro_input};

struct FileMacroInput {
    source_file: Option<LitStr>,
    target_path: LitStr,
}

impl Parse for FileMacroInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let first: LitStr = input.parse()?;
        if input.is_empty() {
            return Ok(Self {
                source_file: None,
                target_path: first,
            });
        }

        let _comma: Token![,] = input.parse()?;
        let second: LitStr = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("expected one string literal path or 'source_file, path'"));
        }

        Ok(Self {
            source_file: Some(first),
            target_path: second,
        })
    }
}

#[proc_macro]
pub fn r#str(input: TokenStream) -> TokenStream {
    let value = parse_macro_input!(input as LitStr);
    let data = value.value().into_bytes();
    expand_from_data(data, true)
}

#[proc_macro]
pub fn bytes(input: TokenStream) -> TokenStream {
    let value = parse_macro_input!(input as LitByteStr);
    let data = value.value();
    expand_from_data(data, false)
}

#[proc_macro]
pub fn file_str(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as FileMacroInput);
    let source_file = input.source_file.as_ref().map(LitStr::value);
    let source_path = input.target_path.value();

    let absolute_path = match resolve_path(source_file.as_deref(), &source_path) {
        Ok(path) => path,
        Err(err) => {
            return syn::Error::new(input.target_path.span(), err)
                .to_compile_error()
                .into();
        }
    };

    let data = match fs::read(&absolute_path) {
        Ok(data) => data,
        Err(err) => {
            return syn::Error::new(
                input.target_path.span(),
                format!("failed to read '{}': {err}", absolute_path.display()),
            )
            .to_compile_error()
            .into();
        }
    };

    expand_from_data(data, true)
}

#[proc_macro]
pub fn file_bytes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as FileMacroInput);
    let source_file = input.source_file.as_ref().map(LitStr::value);
    let source_path = input.target_path.value();

    let absolute_path = match resolve_path(source_file.as_deref(), &source_path) {
        Ok(path) => path,
        Err(err) => {
            return syn::Error::new(input.target_path.span(), err)
                .to_compile_error()
                .into();
        }
    };

    let data = match fs::read(&absolute_path) {
        Ok(data) => data,
        Err(err) => {
            return syn::Error::new(
                input.target_path.span(),
                format!("failed to read '{}': {err}", absolute_path.display()),
            )
            .to_compile_error()
            .into();
        }
    };

    expand_from_data(data, false)
}

#[proc_macro]
pub fn include_zstd(input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(input as LitStr);
    let source_path = path.value();

    // 对于 include_zstd! 宏，直接使用 invocation_source_file_abs 获取源文件路径
    // 确保在 examples/ 目录中也能正确解析相对路径
    let source_file_abs = invocation_source_file_abs();
    let source_dir = source_file_abs.parent().unwrap_or(&source_file_abs);

    let absolute_path = if Path::new(&source_path).is_absolute() {
        PathBuf::from(&source_path)
    } else {
        source_dir.join(&source_path)
    };

    // 尝试读取文件元数据，如果失败则尝试在其他常见位置查找
    let (metadata, absolute_path) = match fs::metadata(&absolute_path) {
        Ok(m) => (m, absolute_path),
        Err(_) => {
            // Fallback: try to find the file in common locations
            match find_file_in_candidates(&source_path, source_dir) {
                Some(found_path) => match fs::metadata(&found_path) {
                    Ok(m) => (m, found_path),
                    Err(err) => {
                        return syn::Error::new(
                            path.span(),
                            format!("failed to read metadata '{}': {err}", found_path.display()),
                        )
                        .to_compile_error()
                        .into();
                    }
                },
                None => {
                    return syn::Error::new(
                        path.span(),
                        format!(
                            "failed to read metadata '{}': file not found",
                            absolute_path.display()
                        ),
                    )
                    .to_compile_error()
                    .into();
                }
            }
        }
    };

    let data = match fs::read(&absolute_path) {
        Ok(d) => d,
        Err(err) => {
            return syn::Error::new(
                path.span(),
                format!("failed to read file '{}': {err}", absolute_path.display()),
            )
            .to_compile_error()
            .into();
        }
    };

    let compressed = match zstd::stream::encode_all(data.as_slice(), 0) {
        Ok(c) => c,
        Err(err) => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("failed to compress data: {err}"),
            )
            .to_compile_error()
            .into();
        }
    };

    let len = metadata.len();
    let is_file = metadata.is_file();
    let is_dir = metadata.is_dir();

    let modified = timestamp_to_code(&metadata.modified());
    let accessed = timestamp_to_code(&metadata.accessed());
    let created = timestamp_to_code(&metadata.created());

    let include_zstd_crate = match crate_name("include-zstd") {
        Ok(FoundCrate::Itself) => quote!(::include_zstd),
        Ok(FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote!(::#ident)
        }
        Err(_) => quote!(::include_zstd),
    };

    let expanded = quote! {
        {
            static __INCLUDE_ZSTD_COMPRESSED: &[u8] = &[#(#compressed),*];

            #include_zstd_crate::__private::create_zstd_asset(
                #include_zstd_crate::ZstdMetadata {
                    len: #len,
                    modified: #modified,
                    accessed: #accessed,
                    created: #created,
                    is_file: #is_file,
                    is_dir: #is_dir,
                },
                __INCLUDE_ZSTD_COMPRESSED,
            )
        }
    };

    expanded.into()
}

fn timestamp_to_code(
    time: &Result<std::time::SystemTime, std::io::Error>,
) -> proc_macro2::TokenStream {
    match time {
        Ok(t) => match t.duration_since(UNIX_EPOCH) {
            Ok(d) => {
                let secs = d.as_secs();
                let nanos = d.subsec_nanos();
                quote!(Some(std::time::UNIX_EPOCH + std::time::Duration::new(#secs, #nanos)))
            }
            Err(_) => quote!(None),
        },
        Err(_) => quote!(None),
    }
}

fn expand_from_data(data: Vec<u8>, decode_utf8: bool) -> TokenStream {
    let compressed = match zstd::stream::encode_all(data.as_slice(), 0) {
        Ok(compressed) => compressed,
        Err(err) => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("failed to compress data: {err}"),
            )
            .to_compile_error()
            .into();
        }
    };

    let include_zstd_crate = match crate_name("include-zstd") {
        // In a package with both lib+bin, proc-macros expanded inside the bin
        // should still target the library crate namespace.
        Ok(FoundCrate::Itself) => quote!(::include_zstd),
        Ok(FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote!(::#ident)
        }
        Err(_) => quote!(::include_zstd),
    };

    let expanded = if decode_utf8 {
        quote! {
            {
                static __INCLUDE_ZSTD_COMPRESSED: &[u8] = &[#(#compressed),*];
                static __INCLUDE_ZSTD_CACHE: ::std::sync::OnceLock<::std::boxed::Box<[u8]>> = ::std::sync::OnceLock::new();

                #include_zstd_crate::__private::decode_utf8(
                    __INCLUDE_ZSTD_CACHE
                        .get_or_init(|| #include_zstd_crate::__private::decompress_bytes(__INCLUDE_ZSTD_COMPRESSED))
                        .as_ref(),
                )
            }
        }
    } else {
        quote! {
            {
                static __INCLUDE_ZSTD_COMPRESSED: &[u8] = &[#(#compressed),*];
                static __INCLUDE_ZSTD_CACHE: ::std::sync::OnceLock<::std::boxed::Box<[u8]>> = ::std::sync::OnceLock::new();

                __INCLUDE_ZSTD_CACHE
                    .get_or_init(|| #include_zstd_crate::__private::decompress_bytes(__INCLUDE_ZSTD_COMPRESSED))
                    .as_ref()
            }
        }
    };

    expanded.into()
}

fn resolve_path(source_file: Option<&str>, source_path: &str) -> Result<PathBuf, String> {
    let target_path = Path::new(source_path);
    if target_path.is_absolute() {
        return Ok(target_path.to_path_buf());
    }

    // Match `include_str!` semantics: always resolve relative paths against the
    // parent directory of the invocation's source file, using an absolute path
    // so the result is independent of the compiler's current working directory.
    let source_file_abs = if let Some(source_file) = source_file {
        absolutize_source_file(Path::new(source_file))
    } else {
        invocation_source_file_abs()
    };

    let source_dir = source_file_abs.parent().ok_or_else(|| {
        format!(
            "failed to resolve include path '{}': invocation source file '{}' has no parent directory",
            source_path,
            source_file_abs.display()
        )
    })?;

    let absolute_path = source_dir.join(target_path);

    // If the resolved path doesn't exist, try to find it in candidate locations
    // (handles LSP analysis where path resolution may be inaccurate)
    if !absolute_path.exists() {
        if let Some(found_path) = find_file_in_candidates(source_path, source_dir) {
            return Ok(found_path);
        }
    }

    Ok(absolute_path)
}

/// Return the absolute path of the source file that contains the macro
/// invocation, mirroring how `include_str!` locates its base directory.
fn invocation_source_file_abs() -> PathBuf {
    let call_site = proc_macro::Span::call_site();

    // `local_file()` returns the canonical absolute on-disk path when the span
    // originates from a real source file; this is the same information rustc
    // uses internally to resolve `include_str!`.
    if let Some(path) = call_site.local_file() {
        // If local_file() returns a file path (not a directory), return it
        if path.extension().is_some() || path.is_file() {
            return path;
        }
        // If it returns a directory, it's likely from LSP analysis
        // Fall through to try other methods
    }

    // Fallback: `Span::file()` typically yields a path relative to the crate
    // root (e.g. "src/lib.rs" or "examples/example.rs").
    let file = call_site.file();
    let file_path = Path::new(&file);

    if file_path.is_absolute() {
        return file_path.to_path_buf();
    }

    // Use CARGO_MANIFEST_DIR (crate root) to anchor relative paths.
    // In workspace projects, this points to the specific crate's directory.
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(&manifest_dir);
        let candidate = manifest_path.join(file_path);

        // Verify the candidate path's parent directory exists
        if candidate.parent().map_or(false, |p| p.exists()) {
            return candidate;
        }
    }

    // Last resort: use current working directory
    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join(file_path);
        if candidate.parent().map_or(false, |p| p.exists()) {
            return candidate;
        }
    }

    // Final fallback: just return the relative path
    file_path.to_path_buf()
}

/// Try to find a file in common candidate locations when standard path resolution fails.
fn find_file_in_candidates(relative_path: &str, source_dir: &Path) -> Option<PathBuf> {
    let file_name = Path::new(relative_path).file_name()?;

    // Candidate locations to search:
    // 1. Current directory (where cargo is invoked)
    // 2. examples/ directory under current directory
    // 3. src/ directory under current directory
    // 4. Same directory as source file
    // 5. CARGO_MANIFEST_DIR/examples/ (for LSP analysis in workspace projects)
    let mut candidates = vec![
        PathBuf::from(file_name),
        PathBuf::from("examples").join(file_name),
        PathBuf::from("src").join(file_name),
        source_dir.join(file_name),
    ];

    // Add CARGO_MANIFEST_DIR based paths
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(&manifest_dir);
        candidates.push(manifest_path.join(file_name));
        candidates.push(manifest_path.join("examples").join(file_name));
        candidates.push(manifest_path.join("src").join(file_name));
    }

    for candidate in candidates {
        if candidate.exists() && candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

fn absolutize_source_file(source_file: &Path) -> PathBuf {
    if source_file.is_absolute() {
        return source_file.to_path_buf();
    }

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        return PathBuf::from(manifest_dir).join(source_file);
    }

    source_file.to_path_buf()
}
