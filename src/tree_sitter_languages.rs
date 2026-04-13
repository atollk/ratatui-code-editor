use anyhow::anyhow;
use rust_embed::RustEmbed;
use tree_sitter::Language;

#[cfg(feature = "tree-sitter-languages")]
pub fn get_language_by_name(lang: &str) -> Option<Language> {
    match lang {
        "rust" => Some(tree_sitter_rust::LANGUAGE.into()),
        "javascript" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "typescript" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "python" => Some(tree_sitter_python::LANGUAGE.into()),
        "go" => Some(tree_sitter_go::LANGUAGE.into()),
        "java" => Some(tree_sitter_java::LANGUAGE.into()),
        "c_sharp" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        "c" => Some(tree_sitter_c::LANGUAGE.into()),
        "cpp" => Some(tree_sitter_cpp::LANGUAGE.into()),
        "html" => Some(tree_sitter_html::LANGUAGE.into()),
        "css" => Some(tree_sitter_css::LANGUAGE.into()),
        "yaml" => Some(tree_sitter_yaml::LANGUAGE.into()),
        "json" => Some(tree_sitter_json::LANGUAGE.into()),
        "toml" => Some(tree_sitter_toml_ng::LANGUAGE.into()),
        "shell" => Some(tree_sitter_bash::LANGUAGE.into()),
        "markdown" => Some(tree_sitter_md::LANGUAGE.into()),
        "markdown-inline" => Some(tree_sitter_md::INLINE_LANGUAGE.into()),
        _ => None,
    }
}

#[cfg(feature = "tree-sitter-languages")]
#[derive(RustEmbed)]
#[folder = ""]
#[include = "langs/*/*"]
pub struct LangAssets;

#[cfg(feature = "tree-sitter-languages")]
pub fn get_highlights(lang: &str) -> anyhow::Result<String> {
    let p = format!("langs/{}/highlights.scm", lang);
    let highlights_bytes =
        LangAssets::get(&p).ok_or_else(|| anyhow!("No highlights found for {}", lang))?;
    let highlights_bytes = highlights_bytes.data.as_ref();
    let highlights = std::str::from_utf8(highlights_bytes)?;
    Ok(highlights.to_string())
}