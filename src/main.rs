mod constants;
mod direction;
mod modal;
mod statistics;
mod trees;
mod vehicle;
mod vehicle_manager;
mod vehicle_positions;

use crate::modal::render_stats_modal;
use constants::*;
use direction::*;
use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::time::Instant;
use trees::Tree;
use vehicle_manager::VehicleManager;

pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init().expect("Failed to initialize SDL2");
    let video_subsystem = sdl_context
        .video()
        .expect("Failed to get SDL2 video subsystem");
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    let window = video_subsystem
        .window("road_intersection", WINDOW_SIZE, WINDOW_SIZE)
        .position_centered()
        .build()
        .expect("Failed to create window");

    let mut canvas = window
        .into_canvas()
        .build()
        .expect("Failed to create canvas");
    let mut event_pump = sdl_context
        .event_pump()
        .expect("Failed to get SDL2 event pump");

    // Load font
    let font = ttf_context
        .load_font("assets/font.ttf", 14)
        .map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();
    let car_textures = [
        texture_creator.load_texture("assets/cars.png")?,
        texture_creator.load_texture("assets/cars-4.png")?,
        texture_creator.load_texture("assets/green-car.png")?,
    ];

    let mut vehicle_manager = VehicleManager::new();
    let mut random_generation = false;
    let mut last_random_spawn = Instant::now();
    let mut show_stats = false;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::Escape => {
                        if show_stats {
                            break 'running;
                        } else {
                            vehicle_manager.set_end_time();
                            show_stats = true;
                            random_generation = false;
                        }
                    }
                    Keycode::Up if !show_stats => vehicle_manager.try_spawn_vehicle(Direction::Up),
                    Keycode::Down if !show_stats => {
                        vehicle_manager.try_spawn_vehicle(Direction::Down)
                    }
                    Keycode::Left if !show_stats => {
                        vehicle_manager.try_spawn_vehicle(Direction::Left)
                    }
                    Keycode::Right if !show_stats => {
                        vehicle_manager.try_spawn_vehicle(Direction::Right)
                    }
                    Keycode::R if !show_stats => random_generation = !random_generation,
                    _ => {}
                },
                _ => {}
            }
        }

        // Handle random vehicle generation when stats aren't shown
        if random_generation
            && Instant::now().duration_since(last_random_spawn) >= VEHICLE_SPAWN_INTERVAL
        {
            let direction = Direction::new(None);
            vehicle_manager.try_spawn_vehicle(direction);
            last_random_spawn = Instant::now();
        }

        // Render the scene

        // Render the scene
        // Draw background
        canvas.set_draw_color(Color::RGB(50, 205, 50));
        canvas.clear();

        // Draw road bases
        canvas.set_draw_color(Color::RGB(51, 51, 51)); // Dark gray for road surface
                                                       // Vertical road
        canvas
            .fill_rect(Rect::new(
                5 * LINE_SPACING,
                0,
                (11 - 5) * LINE_SPACING as u32,
                WINDOW_SIZE,
            ))
            .unwrap();
        // Horizontal road
        canvas
            .fill_rect(Rect::new(
                0,
                5 * LINE_SPACING - 1,
                WINDOW_SIZE,
                (11 - 5) * LINE_SPACING as u32,
            ))
            .unwrap();

        // Draw lane markers
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        // Only draw lines outside the intersection
        for i in 5..=11 {
            let x = i * LINE_SPACING;
            // Vertical lines - draw only above and below intersection
            canvas.draw_line((x, 0), (x, 5 * LINE_SPACING)).unwrap();
            canvas
                .draw_line((x, 11 * LINE_SPACING), (x, WINDOW_SIZE as i32))
                .unwrap();

            // Horizontal lines - draw only left and right of intersection
            canvas.draw_line((0, x), (5 * LINE_SPACING, x)).unwrap();
            canvas
                .draw_line((11 * LINE_SPACING, x), (WINDOW_SIZE as i32, x))
                .unwrap();
        }

        let trees = vec![
            Tree::new(LINE_SPACING, LINE_SPACING, LINE_SPACING),
            Tree::new(
                WINDOW_SIZE as i32 - 2 * LINE_SPACING,
                LINE_SPACING,
                LINE_SPACING,
            ),
            Tree::new(
                LINE_SPACING,
                WINDOW_SIZE as i32 - 2 * LINE_SPACING,
                LINE_SPACING,
            ),
            Tree::new(
                WINDOW_SIZE as i32 - 2 * LINE_SPACING,
                WINDOW_SIZE as i32 - 2 * LINE_SPACING,
                LINE_SPACING,
            ),
        ];

        // Draw all trees
        for tree in trees.iter() {
            tree.draw(&mut canvas);
        }
        // Update and draw vehicles if stats aren't shown
        if !show_stats {
            vehicle_manager.update_vehicles();
        }

        for vehicle in vehicle_manager.get_vehicles() {
            canvas
                .copy_ex(
                    &car_textures[vehicle.texture_index],
                    None,               // Source rect (None = entire texture)
                    Some(vehicle.rect), // Destination rect
                    vehicle.rotation,   // Rotation angle in degrees
                    None,               // Center of rotation (None = center of dst)
                    false,              // Flip horizontally
                    false,              // Flip vertically
                )
                .map_err(|e| e.to_string())?;
        }

        // Render stats modal if active
        if show_stats {
            render_stats_modal(&mut canvas, vehicle_manager.get_statistics(), &font)?;
        }

        canvas.present();
        ::std::thread::sleep(FRAME_DURATION);
    }

    Ok(())
}
