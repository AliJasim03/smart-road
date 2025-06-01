use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::{Duration, Instant};

mod algorithm;
mod game;
mod intersection;
mod vehicle;
mod renderer;
mod statistics;
mod input;

pub const WINDOW_WIDTH: u32 = 1024;
pub const WINDOW_HEIGHT: u32 = 768;
const FPS: u32 = 60;
const FRAME_DELAY: u32 = 1000 / FPS;

fn main() -> Result<(), String> {
    println!("=== Smart Road Intersection Simulation ===");
    println!("Initializing SDL2...");

    // Initialize SDL2
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    // Initialize SDL2_image
    let _image_context = sdl2::image::init(sdl2::image::InitFlag::PNG | sdl2::image::InitFlag::JPG)?;

    println!("Creating window...");

    // Create window
    let window = video_subsystem
        .window("Smart Road Simulation - Autonomous Intersection", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    println!("Creating renderer...");

    // Create canvas
    let canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;

    // Get texture creator
    let texture_creator = canvas.texture_creator();

    // Create renderer with proper asset loading
    let renderer = match renderer::Renderer::new(&texture_creator) {
        Ok(r) => {
            println!("Renderer created successfully");
            r
        },
        Err(e) => {
            println!("Warning: Could not create renderer: {}", e);
            return Err(format!("Failed to create renderer: {}", e));
        }
    };

    // Initialize game
    println!("Initializing game...");
    let mut game = game::Game::new(canvas, renderer)?;

    // Initialize event pump
    let mut event_pump = sdl_context.event_pump()?;

    // Game loop variables
    let mut running = true;
    let mut last_frame = Instant::now();
    let mut frame_count = 0;
    let mut fps_timer = Instant::now();

    println!("\n=== CONTROLS ===");
    println!("↑ Arrow Up:    Spawn vehicle from South (moving North)");
    println!("↓ Arrow Down:  Spawn vehicle from North (moving South)");
    println!("← Arrow Left:  Spawn vehicle from East (moving West)");
    println!("→ Arrow Right: Spawn vehicle from West (moving East)");
    println!("R:             Toggle continuous random spawning");
    println!("D:             Toggle debug mode");
    println!("G:             Toggle grid overlay");
    println!("Space:         Show current statistics");
    println!("Esc:           Exit and show final statistics");
    println!("\n=== FEATURES ===");
    println!("• Smart intersection management without traffic lights");
    println!("• Collision-free autonomous vehicle navigation");
    println!("• Realistic vehicle physics and movement");
    println!("• Multiple vehicle velocities and behaviors");
    println!("• Real-time statistics and performance monitoring");
    println!("• 6-lane intersection with turning capabilities");
    println!("\nSimulation started! Try spawning some vehicles...\n");

    // Main game loop
    while running {
        let now = Instant::now();
        let delta_time = now.duration_since(last_frame).as_secs_f32();
        last_frame = now;

        // Cap delta time to prevent large jumps (minimum 30 FPS, maximum 120 FPS)
        let capped_delta = delta_time.min(1.0 / 30.0).max(1.0 / 120.0);

        // Handle events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    println!("\nShutting down simulation...");
                    running = false;
                }
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    println!("\nExiting simulation...");
                    running = false;
                }
                _ => {
                    game.handle_event(&event);
                }
            }
        }

        // Update game state
        game.update(capped_delta);

        // Render
        if let Err(e) = game.render() {
            eprintln!("Rendering error: {}", e);
        }

        // FPS monitoring (every 5 seconds)
        frame_count += 1;
        if fps_timer.elapsed().as_secs() >= 5 {
            let fps = frame_count as f32 / fps_timer.elapsed().as_secs_f32();
            if fps < 50.0 {
                println!("Warning: Low FPS detected: {:.1}", fps);
            } else {
                println!("FPS: {:.1}", fps);
            }
            fps_timer = Instant::now();
            frame_count = 0;
        }

        // Cap frame rate
        let frame_time = now.elapsed();
        if frame_time < Duration::from_millis(FRAME_DELAY as u64) {
            std::thread::sleep(Duration::from_millis(FRAME_DELAY as u64) - frame_time);
        }
    }

    println!("\nSimulation ended. Generating final statistics...");

    // Show final statistics
    if let Err(e) = game.show_statistics() {
        eprintln!("Error showing statistics: {}", e);
    }

    println!("Thank you for using Smart Road Simulation!");
    Ok(())
}