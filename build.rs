/// Build script to generate rustdoc JSON and convert to Markdown
use std::process::Command;
use std::path::Path;
use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=crates/");
    println!("cargo:rerun-if-changed=ui-crates/");
    
    // Generate rustdocs as JSON then convert to Markdown
    generate_documentation();
}

fn generate_documentation() {
    // Only generate docs in release builds
    if std::env::var("PROFILE").unwrap_or_default() != "release" {
        println!("cargo:warning=Skipping doc generation in debug mode");
        return;
    }
    
    println!("cargo:warning=Generating workspace documentation as JSON...");
    
    // Step 1: Generate JSON docs
    let json_output = Command::new("cargo")
        .env("RUSTC_BOOTSTRAP", "1")
        .env("RUSTDOCFLAGS", "-Z unstable-options --output-format json")
        .args([
            "doc",
            "--workspace",
            "--no-deps",
            "--target-dir",
            "target"
        ])
        .output();
    
    match json_output {
        Ok(result) if result.status.success() => {
            println!("cargo:warning=JSON documentation generated successfully");
            
            // Step 2: Convert JSON to Markdown using rustdoc-md
            convert_json_to_markdown();
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            println!("cargo:warning=Failed to generate JSON docs: {}", stderr);
        }
        Err(e) => {
            println!("cargo:warning=Failed to run cargo doc: {}", e);
        }
    }
}

fn convert_json_to_markdown() {
    let doc_dir = Path::new("target/doc");
    
    if !doc_dir.exists() {
        println!("cargo:warning=Doc directory doesn't exist");
        return;
    }
    
    // Find all .json files in target/doc
    if let Ok(entries) = fs::read_dir(doc_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let output_path = doc_dir.join(format!("{}.md", file_stem));
                    
                    println!("cargo:warning=Converting {} to Markdown...", file_stem);
                    
                    let convert_result = Command::new("rustdoc-md")
                        .args([
                            "--path", path.to_str().unwrap(),
                            "--output", output_path.to_str().unwrap()
                        ])
                        .output();
                    
                    match convert_result {
                        Ok(result) if result.status.success() => {
                            println!("cargo:warning=Converted {}.json to Markdown", file_stem);
                        }
                        Ok(result) => {
                            let stderr = String::from_utf8_lossy(&result.stderr);
                            println!("cargo:warning=Failed to convert {}: {}", file_stem, stderr);
                        }
                        Err(e) => {
                            println!("cargo:warning=rustdoc-md not found: {}. Install with: cargo install rustdoc-md", e);
                        }
                    }
                }
            }
        }
    }
}
