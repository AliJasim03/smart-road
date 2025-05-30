// src/main.rs - Updated to use smart intersection algorithm
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::{Duration, Instant};

mod game;
mod intersection;
mod vehicle;
mod simple_renderer; // Simple block-based renderer
mod statistics;
mod smart_algorithm;
// New smart algorithm module

pub const WINDOW_WIDTH: u32 = 1024;
pub const WINDOW_HEIGHT: u32 = 768;
const FPS: u32 = 60;
const FRAME_DELAY: u32 = 1000 / FPS;

fn main() -> Result<(), String> {
    // Initialize SDL2
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    // Create window
    let window = video_subsystem
        .window("Smart Road Simulation - Intelligent Intersection", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    // Create renderer
    let canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;

    // Get texture creator from canvas
    let texture_creator = canvas.texture_creator();

    // Create simple block-based renderer with texture creator
    let renderer = simple_renderer::SimpleRenderer::new(&texture_creator)?;

    // Initialize game with smart algorithm
    let mut game = game::Game::new(canvas, renderer)?;

    // Initialize event pump
    let mut event_pump = sdl_context.event_pump()?;

    // Game loop
    let mut running = true;
    let mut frame_time;
    let mut last_frame = Instant::now();

    println!("Smart Road Simulation with Intelligent Intersection Management");
    println!("===============================================================");
    println!("Controls:");
    println!("- Arrow Up: Generate vehicles from south (moving north)");
    println!("- Arrow Down: Generate vehicles from north (moving south)");
    println!("- Arrow Left: Generate vehicles from east (moving west)");
    println!("- Arrow Right: Generate vehicles from west (moving east)");
    println!("- R: Toggle continuous random vehicle generation");
    println!("- D: Toggle debug mode (shows vehicle states)");
    println!("- G: Toggle grid overlay (shows 32x32 calculation grid)");
    println!("- Space: Show current statistics");
    println!("- Esc: Exit and show final statistics");
    println!();
    println!("Features:");
    println!("- Reservation-based intersection management");
    println!("- Priority system for traffic flow");
    println!("- Collision-free intersection traversal");
    println!("- Realistic vehicle physics and movement");
    println!("- 32x32 pixel grid-based calculations");
    println!();

    while running {
        // Calculate frame time
        let now = Instant::now();
        frame_time = now.duration_since(last_frame).as_millis() as f32 / 1000.0; // Convert to seconds
        last_frame = now;

        // Ensure reasonable frame time bounds
        frame_time = frame_time.min(1.0 / 30.0); // Cap at 30 FPS minimum
        if frame_time < 0.001 {
            frame_time = 1.0 / 60.0; // Default to 60 FPS
        }

        // Handle events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    running = false;
                }
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    running = false;
                }
                _ => {
                    game.handle_event(&event);
                }
            }
        }

        // Update game state
        game.update(frame_time);

        // Render
        if let Err(e) = game.render() {
            eprintln!("Rendering error: {}", e);
        }

        // Cap the frame rate
        let elapsed = now.elapsed().as_millis() as u32;
        if elapsed < FRAME_DELAY {
            std::thread::sleep(Duration::from_millis((FRAME_DELAY - elapsed) as u64));
        }
    }

    println!("\nSimulation ended. Showing final statistics...");

    // Show statistics when game ends
    if let Err(e) = game.show_statistics() {
        eprintln!("Error showing statistics: {}", e);
    }

    Ok(())
}