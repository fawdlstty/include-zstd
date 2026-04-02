use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::quote;
use std::fs;
use std::path::{Path, PathBuf};
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

    let source_dir = if let Some(source_file) = source_file {
        source_dir_from_source_file(source_file, source_path)?
    } else {
        source_dir_from_invocation(source_path)?
    };

    Ok(source_dir.join(target_path))
}

fn source_dir_from_source_file(source_file: &str, source_path: &str) -> Result<PathBuf, String> {
    let source_file_path = Path::new(source_file);
    let source_dir = source_file_path.parent().ok_or_else(|| {
        format!(
            "failed to resolve include path '{}': invocation source file '{}' has no parent directory",
            source_path,
            source_file_path.display()
        )
    })?;

    if source_file_path.is_absolute() {
        Ok(source_dir.to_path_buf())
    } else {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .map_err(|err| format!("failed to read CARGO_MANIFEST_DIR: {err}"))?;
        Ok(manifest_dir.join(source_dir))
    }
}

fn source_dir_from_invocation(source_path: &str) -> Result<PathBuf, String> {
    let source_file_path = invocation_source_file();
    if let Some(source_dir) = source_file_path.parent() {
        return Ok(source_dir.to_path_buf());
    }

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        return Ok(PathBuf::from(manifest_dir));
    }

    std::env::current_dir().map_err(|err| {
        format!(
            "failed to resolve include path '{}': no invocation source path and no usable base directory: {err}",
            source_path
        )
    })
}

fn invocation_source_file() -> PathBuf {
    // local_file provides the canonical on-disk path when available.
    proc_macro::Span::call_site()
        .local_file()
        .unwrap_or_else(|| PathBuf::from(proc_macro::Span::call_site().file()))
}
