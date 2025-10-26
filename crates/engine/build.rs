use std::path::PathBuf;

fn main() {
    #[cfg(target_os = "windows")]
    {
        // Get the path relative to the workspace root
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
        let icon_path = workspace_root.join("assets").join("images").join("logo_sqrkl.ico");
        
        println!("cargo:rerun-if-changed={}", icon_path.display());
        
        let mut res = winresource::WindowsResource::new();
        res.set_icon(icon_path.to_str().unwrap());
        res.compile().expect("Failed to compile Windows resources");
    }
}
