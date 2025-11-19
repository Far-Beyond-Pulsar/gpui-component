// Build script to embed setup scripts at compile time

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Read setup scripts
    let setup_ps1 = fs::read_to_string("script/setup-dev-environment.ps1")
        .expect("Failed to read setup-dev-environment.ps1");
    let setup_sh = fs::read_to_string("script/setup-dev-environment.sh")
        .expect("Failed to read setup-dev-environment.sh");
    
    // Write embedded scripts
    fs::write(out_dir.join("setup-dev-environment.ps1"), setup_ps1)
        .expect("Failed to write embedded PowerShell script");
    fs::write(out_dir.join("setup-dev-environment.sh"), setup_sh)
        .expect("Failed to write embedded bash script");
    
    // Tell Cargo to rerun if the scripts change
    println!("cargo:rerun-if-changed=script/setup-dev-environment.ps1");
    println!("cargo:rerun-if-changed=script/setup-dev-environment.sh");
}
