use std::env;
use std::fs;
use std::path::Path;
use std::io::Write;
use std::process::Command;

fn main() {
    // Get the project directory
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Create assets directory in the project directory
    let assets_dir = Path::new(&manifest_dir).join("assets");
    if !assets_dir.exists() {
        fs::create_dir_all(&assets_dir).unwrap();
    }

    // Create necessary subdirectories and sample assets
    create_asset_directories(&assets_dir);
    create_sample_assets(&assets_dir);

    // Let Cargo know to rerun if any of these directories change
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/");

    // Handle SDL2 library linking
    link_sdl2_libraries();
}

fn link_sdl2_libraries() {
    // Check if we're on macOS
    #[cfg(target_os = "macos")]
    {
        // Try to find SDL2 libraries via homebrew
        if let Ok(true) = is_homebrew_available() {
            println!("Homebrew detected, trying to locate SDL2 libraries...");

            // Get SDL2 paths using homebrew
            if let Ok(sdl2_path) = get_homebrew_path("sdl2") {
                println!("cargo:rustc-link-search={}/lib", sdl2_path);
                println!("cargo:rustc-link-lib=SDL2");
            } else {
                println!("SDL2 not found via homebrew, you may need to install it with 'brew install sdl2'");
            }

            // Try to get SDL2_image path
            if let Ok(sdl2_image_path) = get_homebrew_path("sdl2_image") {
                println!("cargo:rustc-link-search={}/lib", sdl2_image_path);
                println!("cargo:rustc-link-lib=SDL2_image");
            } else {
                println!("SDL2_image not found via homebrew, you may need to install it with 'brew install sdl2_image'");
            }

            // Try to get SDL2_ttf path
            if let Ok(sdl2_ttf_path) = get_homebrew_path("sdl2_ttf") {
                println!("cargo:rustc-link-search={}/lib", sdl2_ttf_path);
                println!("cargo:rustc-link-lib=SDL2_ttf");
            } else {
                println!("SDL2_ttf not found via homebrew, you may need to install it with 'brew install sdl2_ttf'");
            }

            // Link system frameworks
            println!("cargo:rustc-link-lib=framework=CoreFoundation");
            println!("cargo:rustc-link-lib=framework=CoreGraphics");
            println!("cargo:rustc-link-lib=framework=CoreAudio");
            println!("cargo:rustc-link-lib=framework=AudioToolbox");
            println!("cargo:rustc-link-lib=framework=Metal");
        } else {
            println!("Homebrew not found. Please install SDL2 libraries manually and set appropriate environment variables.");
        }
    }

    // For Linux systems
    #[cfg(target_os = "linux")]
    {
        println!("On Linux, you may need to install SDL2 libraries with your package manager.");
        println!("For example: sudo apt-get install libsdl2-dev libsdl2-image-dev libsdl2-ttf-dev");
    }

    // For Windows systems
    #[cfg(target_os = "windows")]
    {
        println!("On Windows, make sure SDL2 libraries are in your PATH or use appropriate environment variables.");
    }
}

fn is_homebrew_available() -> Result<bool, String> {
    match Command::new("brew").arg("--version").output() {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

fn get_homebrew_path(package: &str) -> Result<String, String> {
    match Command::new("brew").args(&["--prefix", package]).output() {
        Ok(output) => {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                Ok(path)
            } else {
                Err(format!("Package {} not found in homebrew", package))
            }
        }
        Err(e) => Err(format!("Failed to execute brew command: {}", e)),
    }
}

fn create_asset_directories(assets_dir: &Path) {
    let subdirs = ["fonts", "vehicles", "road"];
    for subdir in subdirs.iter() {
        let dir_path = assets_dir.join(subdir);
        if !dir_path.exists() {
            fs::create_dir_all(&dir_path).unwrap();
        }
    }
}

fn create_sample_assets(assets_dir: &Path) {
    // Create placeholder files for the assets
    // These aren't valid image files, but the renderer has fallbacks

    // Vehicles directory
    let vehicles_dir = assets_dir.join("vehicles");
    let car_file = vehicles_dir.join("cars.png");
    if !car_file.exists() {
        println!("Creating placeholder cars.png...");
        let mut file = fs::File::create(&car_file).unwrap();
        file.write_all(b"PLACEHOLDER IMAGE").unwrap();
    }

    // Road directory
    let road_dir = assets_dir.join("road");
    let road_file = road_dir.join("road.png");
    if !road_file.exists() {
        println!("Creating placeholder road.png...");
        let mut file = fs::File::create(&road_file).unwrap();
        file.write_all(b"PLACEHOLDER IMAGE").unwrap();
    }

    let acera_file = road_dir.join("acera.png");
    if !acera_file.exists() {
        println!("Creating placeholder acera.png...");
        let mut file = fs::File::create(&acera_file).unwrap();
        file.write_all(b"PLACEHOLDER IMAGE").unwrap();
    }

    // Fonts directory (for statistics display)
    let fonts_dir = assets_dir.join("fonts");
    let font_file = fonts_dir.join("font.ttf");
    if !font_file.exists() {
        println!("Creating placeholder font.ttf...");
        let mut file = fs::File::create(&font_file).unwrap();
        file.write_all(b"PLACEHOLDER FONT").unwrap();
    }
}