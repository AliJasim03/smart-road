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

pub const WINDOW_WIDTH: u32 = 800;
pub const WINDOW_HEIGHT: u32 = 600;
const FPS: u32 = 60;
const FRAME_DELAY: u32 = 1000 / FPS;

fn main() -> Result<(), String> {
    // Initialize SDL2
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    // Create window
    let window = video_subsystem
        .window("Smart Road Simulation", WINDOW_WIDTH, WINDOW_HEIGHT)
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

    while running {
        // Calculate frame time
        let now = Instant::now();
        frame_time = now.duration_since(last_frame).as_millis() as u32;
        last_frame = now;

        // Make sure frame_time is never zero or too small
        if frame_time < 10 {
            frame_time = 10; // Ensure a more reasonable minimum time delta
        }

        println!("Frame time: {}ms", frame_time);

        // Handle events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    running = false;
                }
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    running = false;
                }
                Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                    println!("R key pressed - toggling continuous spawn");
                    game.handle_event(&event);
                }
                Event::KeyDown { keycode: Some(Keycode::Up), .. } |
                Event::KeyDown { keycode: Some(Keycode::Down), .. } |
                Event::KeyDown { keycode: Some(Keycode::Left), .. } |
                Event::KeyDown { keycode: Some(Keycode::Right), .. } => {
                    println!("Arrow key pressed - spawning vehicle");
                    game.handle_event(&event);
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

        last_frame = Instant::now();
    }

    // Show statistics when game ends
    game.show_statistics()?;

    Ok(())
}