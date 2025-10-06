mod constants;
mod direction;
mod core;
mod geometry;
mod intersection;
mod rendering;
mod simulation;

use constants::*;
use direction::*;
use rendering::{render_stats_modal, RoadRenderer};
use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::keyboard::Keycode;
use simulation::VehicleManager;
use std::time::Instant;

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

        if random_generation
            && Instant::now().duration_since(last_random_spawn) >= VEHICLE_SPAWN_INTERVAL
        {
            let direction = Direction::new(None);
            vehicle_manager.try_spawn_vehicle(direction);
            last_random_spawn = Instant::now();
        }

        RoadRenderer::render_background(&mut canvas);
        RoadRenderer::render_road_surface(&mut canvas);
        RoadRenderer::render_lane_markers(&mut canvas);

        if !show_stats {
            vehicle_manager.update_vehicles();
        }

        for vehicle in vehicle_manager.get_vehicles() {
            canvas
                .copy_ex(
                    &car_textures[vehicle.texture_index],
                    None,
                    Some(vehicle.rect),
                    vehicle.rotation,
                    None,
                    false,
                    false,
                )
                .map_err(|e| e.to_string())?;
        }

        if show_stats {
            render_stats_modal(&mut canvas, vehicle_manager.get_statistics(), &font)?;
        }

        canvas.present();
        ::std::thread::sleep(FRAME_DURATION);
    }

    Ok(())
}
