use rust_embed::RustEmbed;
use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../target/doc"]
pub struct DocAssets;

/// Pre-indexed documentation structure
pub static DOCS_INDEX: Lazy<HashMap<String, Vec<String>>> = Lazy::new(|| {
    let mut index = HashMap::new();
    
    for file_path in DocAssets::iter() {
        let file_name = file_path.as_ref();
        
        if file_name.ends_with(".md") {
            let parts: Vec<&str> = file_name.split('/').collect();
            let crate_name = if parts.len() > 1 {
                parts[0].to_string()
            } else {
                file_name.trim_end_matches(".md").to_string()
            };
            
            index.entry(crate_name).or_insert_with(Vec::new).push(file_name.to_string());
        }
    }
    
    index
});

/// Get markdown content for a crate
pub fn get_crate_docs(crate_name: &str) -> Option<String> {
    let file_key = format!("{}.md", crate_name);
    
    if let Some(content) = DocAssets::get(&file_key) {
        std::str::from_utf8(&content.data).ok().map(String::from)
    } else {
        None
    }
}

/// Get list of all documented crates
pub fn list_crates() -> Vec<String> {
    DOCS_INDEX.keys().cloned().collect()
}

/// Check if docs are available
pub fn docs_available() -> bool {
    !DOCS_INDEX.is_empty()
}
