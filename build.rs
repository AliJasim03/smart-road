use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Get the output directory
    let out_dir = env::var("OUT_DIR").unwrap();

    // Create assets directory in the output directory if it doesn't exist
    let assets_dir = Path::new(&out_dir).join("assets");
    if !assets_dir.exists() {
        fs::create_dir_all(&assets_dir).unwrap();
    }

    // Create necessary subdirectories
    let subdirs = ["fonts", "vehicles", "road"];
    for subdir in subdirs.iter() {
        let dir_path = assets_dir.join(subdir);
        if !dir_path.exists() {
            fs::create_dir_all(&dir_path).unwrap();
        }
    }

    // Get SDL2 paths from homebrew
    let sdl2_path = get_homebrew_path("sdl2");
    let sdl2_image_path = get_homebrew_path("sdl2_image");
    let sdl2_ttf_path = get_homebrew_path("sdl2_ttf");

    // Link to SDL2 libraries explicitly
    println!("cargo:rustc-link-search={}/lib", sdl2_path);
    println!("cargo:rustc-link-search={}/lib", sdl2_image_path);
    println!("cargo:rustc-link-search={}/lib", sdl2_ttf_path);

    println!("cargo:rustc-link-lib=SDL2");
    println!("cargo:rustc-link-lib=SDL2_image");
    println!("cargo:rustc-link-lib=SDL2_ttf");

    // On macOS, also consider adding framework links if needed
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=CoreAudio");
        println!("cargo:rustc-link-lib=framework=AudioToolbox");
        println!("cargo:rustc-link-lib=framework=Metal");
    }

    // Print cargo instructions
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/");
}

fn get_homebrew_path(package: &str) -> String {
    let output = Command::new("brew")
        .args(&["--prefix", package])
        .output()
        .expect(&format!("Failed to execute brew --prefix {}", package));

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}