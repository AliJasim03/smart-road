use std::env;
use std::fs;
use std::path::Path;
use std::io::Write;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/");

    // Handle SDL2 library linking based on platform
    handle_sdl2_linking();

    println!("Build script completed successfully!");
}

fn handle_sdl2_linking() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => handle_macos_linking(),
        "linux" => handle_linux_linking(),
        "windows" => handle_windows_linking(),
        _ => {
            println!("cargo:warning=Unsupported target OS: {}", target_os);
            println!("cargo:warning=You may need to manually configure SDL2 libraries");
        }
    }
}

fn handle_macos_linking() {
    println!("Configuring SDL2 for macOS...");

    // Try to find SDL2 via Homebrew
    if is_command_available("brew") {
        if let Ok(sdl2_path) = get_homebrew_path("sdl2") {
            println!("cargo:rustc-link-search={}/lib", sdl2_path);
            println!("cargo:rustc-link-lib=SDL2");
            println!("Found SDL2 via Homebrew at: {}", sdl2_path);
        } else {
            println!("cargo:warning=SDL2 not found via Homebrew");
            println!("cargo:warning=Install with: brew install sdl2");
        }

        if let Ok(sdl2_image_path) = get_homebrew_path("sdl2_image") {
            println!("cargo:rustc-link-search={}/lib", sdl2_image_path);
            println!("cargo:rustc-link-lib=SDL2_image");
        } else {
            println!("cargo:warning=SDL2_image not found");
            println!("cargo:warning=Install with: brew install sdl2_image");
        }

        if let Ok(sdl2_ttf_path) = get_homebrew_path("sdl2_ttf") {
            println!("cargo:rustc-link-search={}/lib", sdl2_ttf_path);
            println!("cargo:rustc-link-lib=SDL2_ttf");
        } else {
            println!("cargo:warning=SDL2_ttf not found");
            println!("cargo:warning=Install with: brew install sdl2_ttf");
        }
    } else {
        println!("cargo:warning=Homebrew not found");
        println!("cargo:warning=Install Homebrew or manually configure SDL2");
    }

    // Link required macOS frameworks
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
    println!("cargo:rustc-link-lib=framework=CoreGraphics");
    println!("cargo:rustc-link-lib=framework=CoreAudio");
    println!("cargo:rustc-link-lib=framework=AudioToolbox");
    println!("cargo:rustc-link-lib=framework=Metal");
    println!("cargo:rustc-link-lib=framework=QuartzCore");
}

fn handle_linux_linking() {
    println!("Configuring SDL2 for Linux...");

    // Try pkg-config first
    if is_command_available("pkg-config") {
        if check_pkg_config("sdl2") {
            println!("Found SDL2 via pkg-config");
        } else {
            println!("cargo:warning=SDL2 not found via pkg-config");
            print_linux_install_instructions();
        }

        if check_pkg_config("SDL2_image") {
            println!("Found SDL2_image via pkg-config");
        } else {
            println!("cargo:warning=SDL2_image not found");
        }

        if check_pkg_config("SDL2_ttf") {
            println!("Found SDL2_ttf via pkg-config");
        } else {
            println!("cargo:warning=SDL2_ttf not found");
        }
    } else {
        println!("cargo:warning=pkg-config not available");
        print_linux_install_instructions();
    }
}

fn handle_windows_linking() {
    println!("Configuring SDL2 for Windows...");

    // Check for vcpkg
    if env::var("VCPKG_ROOT").is_ok() {
        println!("VCPKG detected - using vcpkg SDL2");
        // vcpkg should handle the linking automatically
    } else {
        println!("cargo:warning=VCPKG not found");
        println!("cargo:warning=For Windows, we recommend using vcpkg to install SDL2");
        println!("cargo:warning=Alternatively, download SDL2 development libraries");
        println!("cargo:warning=and set appropriate environment variables");
    }
}

fn is_command_available(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .output()
        .is_ok()
}

fn get_homebrew_path(package: &str) -> Result<String, String> {
    let output = Command::new("brew")
        .args(&["--prefix", package])
        .output()
        .map_err(|e| format!("Failed to execute brew: {}", e))?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(path)
    } else {
        Err(format!("Package {} not found in homebrew", package))
    }
}

fn check_pkg_config(library: &str) -> bool {
    Command::new("pkg-config")
        .args(&["--exists", library])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn print_linux_install_instructions() {
    println!("cargo:warning=To install SDL2 on Linux:");
    println!("cargo:warning=Ubuntu/Debian: sudo apt install libsdl2-dev libsdl2-image-dev libsdl2-ttf-dev");
    println!("cargo:warning=Fedora: sudo dnf install SDL2-devel SDL2_image-devel SDL2_ttf-devel");
    println!("cargo:warning=Arch: sudo pacman -S sdl2 sdl2_image sdl2_ttf");
}

// Additional helper function to check if we're in a CI environment
fn is_ci_environment() -> bool {
    env::var("CI").is_ok() ||
        env::var("CONTINUOUS_INTEGRATION").is_ok() ||
        env::var("GITHUB_ACTIONS").is_ok() ||
        env::var("TRAVIS").is_ok()
}

// Function to set up cross-compilation hints
fn setup_cross_compilation() {
    if let Ok(target) = env::var("TARGET") {
        println!("Cross-compiling for target: {}", target);

        // Add target-specific configuration if needed
        match target.as_str() {
            "x86_64-pc-windows-gnu" => {
                println!("cargo:warning=Cross-compiling for Windows from non-Windows");
                println!("cargo:warning=Ensure mingw-w64 and SDL2 Windows libraries are available");
            }
            "aarch64-apple-darwin" => {
                println!("cargo:warning=Cross-compiling for Apple Silicon");
                println!("cargo:warning=Ensure SDL2 ARM64 libraries are available");
            }
            _ => {}
        }
    }
}