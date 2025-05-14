use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::{Duration, Instant};

mod game;
mod intersection;
mod vehicle;
mod renderer;
mod input;
mod algorithm;
mod statistics;

pub const WINDOW_WIDTH: u32 = 1024; // Increased to better show 6 lanes
pub const WINDOW_HEIGHT: u32 = 768; // Increased to better show 6 lanes
const FPS: u32 = 60;
const FRAME_DELAY: u32 = 1000 / FPS;

fn main() -> Result<(), String> {
    // Initialize SDL2
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    // Create window
    let window = video_subsystem
        .window("Smart Road Simulation (6-Lane Intersection)", WINDOW_WIDTH, WINDOW_HEIGHT)
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

    // Create renderer with texture creator
    let renderer = renderer::Renderer::new(&texture_creator)?;

    // Initialize game
    let mut game = game::Game::new(canvas, renderer)?;

    // Initialize event pump
    let mut event_pump = sdl_context.event_pump()?;

    // Game loop
    let mut running = true;
    let mut frame_time;
    let mut last_frame = Instant::now();

    println!("Smart Road Simulation started");
    println!("Controls:");
    println!("- Arrow Up: Generate vehicles from south");
    println!("- Arrow Down: Generate vehicles from north");
    println!("- Arrow Left: Generate vehicles from east");
    println!("- Arrow Right: Generate vehicles from west");
    println!("- R: Toggle continuous random vehicle generation");
    println!("- Esc: Exit and show statistics");

    while running {
        // Calculate frame time
        let now = Instant::now();
        frame_time = now.duration_since(last_frame).as_millis() as u32;
        last_frame = now;

        // Make sure frame_time is never zero or too small
        if frame_time < 10 {
            frame_time = 10; // Ensure a more reasonable minimum time delta
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
        game.render()?;

        // Cap the frame rate
        if frame_time < FRAME_DELAY {
            std::thread::sleep(Duration::from_millis((FRAME_DELAY - frame_time) as u64));
        }
    }

    // Show statistics when game ends
    game.show_statistics()?;

    Ok(())
}