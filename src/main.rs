// src/main.rs - FINAL VERSION with Assets and Stats Window
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Rect, Point};
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};
use sdl2::image::{self, LoadTexture, InitFlag};
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use tinyfiledialogs; // NEW: For the statistics pop-up window

mod vehicle;
mod intersection;
mod statistics;
mod algorithm;

use vehicle::{Vehicle, Direction, Route, VehicleState, VehicleColor, Vec2};
use intersection::Intersection;
use statistics::Statistics;
use algorithm::SmartIntersection;

pub const WINDOW_WIDTH: u32 = 1024;
pub const WINDOW_HEIGHT: u32 = 768;
const FPS: u32 = 60;
const PHYSICS_TIMESTEP: f64 = 1.0 / 60.0;

// Road geometry constants
const LANE_WIDTH: f32 = 30.0;
const TOTAL_ROAD_WIDTH: f32 = 180.0;
const HALF_ROAD_WIDTH: f32 = 90.0;

struct Textures<'a> {
    intersection: Texture<'a>,
    car_red: Texture<'a>,
    car_green: Texture<'a>,
    car_blue: Texture<'a>,
}

impl<'a> Textures<'a> {
    fn new(texture_creator: &'a TextureCreator<WindowContext>) -> Result<Self, String> {
        Ok(Textures {
            intersection: texture_creator.load_texture("assets/intersection.svg")?,
            car_red:      texture_creator.load_texture("assets/car_red.svg")?,
            car_green:    texture_creator.load_texture("assets/car_green.svg")?,
            car_blue:     texture_creator.load_texture("assets/car_blue.svg")?,
        })
    }
}

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let _image_context = image::init(InitFlag::SVG)?;

    let window = video_subsystem
        .window("Smart Road - Final Version", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build().map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().accelerated().present_vsync()
        .build().map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();
    let textures = Textures::new(&texture_creator)?;

    let mut game = GameState::new()?;
    let mut event_pump = sdl_context.event_pump()?;

    println!("Simulation started. Press 'Esc' to exit and see stats.");

    while game.running {
        let frame_start = Instant::now();
        for event in event_pump.poll_iter() {
            game.handle_event(&event);
        }

        game.update_physics(PHYSICS_TIMESTEP);
        game.render(&mut canvas, &textures)?;

        let frame_time = frame_start.elapsed();
        if frame_time < Duration::from_millis(1000 / FPS as u64) {
            std::thread::sleep(Duration::from_millis(1000 / FPS as u64) - frame_time);
        }
    }

    game.show_final_statistics();
    Ok(())
}

fn get_route_for_lane(lane: usize) -> Route { match lane { 0 => Route::Right, 1 => Route::Straight, 2 => Route::Left, _ => Route::Straight } }
fn get_color_for_route(route: Route) -> VehicleColor { match route { Route::Left => VehicleColor::Red, Route::Straight => VehicleColor::Blue, Route::Right => VehicleColor::Green } }
fn get_destination_for_route(incoming: Direction, route: Route) -> Direction {
    match (incoming, route) {
        (Direction::North, Route::Left) => Direction::West,   (Direction::North, Route::Straight) => Direction::North, (Direction::North, Route::Right) => Direction::East,
        (Direction::South, Route::Left) => Direction::East,    (Direction::South, Route::Straight) => Direction::South, (Direction::South, Route::Right) => Direction::West,
        (Direction::East,  Route::Left) => Direction::North,   (Direction::East,  Route::Straight) => Direction::East,  (Direction::East,  Route::Right) => Direction::South,
        (Direction::West,  Route::Left) => Direction::South,   (Direction::West,  Route::Straight) => Direction::West,  (Direction::West,  Route::Right) => Direction::North,
    }
}

struct GameState {
    running: bool,
    vehicles: VecDeque<Vehicle>,
    statistics: Statistics,
    algorithm: SmartIntersection,
    spawn_cooldown: f32,
    continuous_spawn: bool,
    spawn_timer: f32,
    next_vehicle_id: u32,
    debug_mode: bool,
}

impl GameState {
    fn new() -> Result<Self, String> {
        Ok(GameState {
            running: true,
            vehicles: VecDeque::new(),
            statistics: Statistics::new(),
            algorithm: SmartIntersection::new(),
            spawn_cooldown: 0.0,
            continuous_spawn: false,
            spawn_timer: 0.0,
            next_vehicle_id: 0,
            debug_mode: false,
        })
    }

    fn handle_event(&mut self, event: &Event) {
        match event {
            Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => self.running = false,
            Event::KeyDown { keycode: Some(keycode), repeat: false, .. } => {
                match keycode {
                    Keycode::Up => self.try_spawn_vehicle(Direction::North),
                    Keycode::Down => self.try_spawn_vehicle(Direction::South),
                    Keycode::Left => self.try_spawn_vehicle(Direction::West),
                    Keycode::Right => self.try_spawn_vehicle(Direction::East),
                    Keycode::R => self.continuous_spawn = !self.continuous_spawn,
                    Keycode::D => self.debug_mode = !self.debug_mode,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn update_physics(&mut self, dt: f64) {
        if self.spawn_cooldown > 0.0 { self.spawn_cooldown -= dt as f32; }

        if self.continuous_spawn && self.spawn_cooldown <= 0.0 {
            self.spawn_timer += dt as f32;
            if self.spawn_timer >= 1.0 { // Faster spawning
                use rand::Rng;
                let direction = match rand::thread_rng().gen_range(0..4) {
                    0 => Direction::North, 1 => Direction::South, 2 => Direction::East, _ => Direction::West,
                };
                self.try_spawn_vehicle(direction);
                self.spawn_timer = 0.0;
            }
        }

        self.algorithm.process_vehicles(&mut self.vehicles);
        for v in &mut self.vehicles { v.update_physics(dt); }
        self.cleanup_completed_vehicles();
        self.statistics.update(&self.vehicles, self.algorithm.close_calls);
    }

    fn try_spawn_vehicle(&mut self, direction: Direction) {
        if self.spawn_cooldown > 0.0 { return; }
        let min_spawn_dist = 40.0;
        if self.vehicles.iter().any(|v| v.direction == direction && v.distance_from_spawn() < min_spawn_dist) { return; }

        use rand::Rng;
        let lane = rand::thread_rng().gen_range(0..3);
        let route = get_route_for_lane(lane);
        let dest = get_destination_for_route(direction, route);
        let color = get_color_for_route(route);

        let vehicle = Vehicle::new(self.next_vehicle_id, direction, dest, lane, route, color);
        self.vehicles.push_back(vehicle);
        self.next_vehicle_id += 1;
        self.spawn_cooldown = 0.1;
        self.statistics.record_vehicle_spawn(direction, route);
    }

    fn cleanup_completed_vehicles(&mut self) {
        self.vehicles.retain(|v| {
            if v.state == VehicleState::Completed { self.statistics.record_vehicle_completion(v.time_in_intersection); }
            v.state != VehicleState::Completed
        });
    }

    fn render(&self, canvas: &mut Canvas<Window>, textures: &Textures) -> Result<(), String> {
        canvas.clear();
        canvas.copy(&textures.intersection, None, None)?;

        for vehicle in &self.vehicles { self.draw_vehicle(canvas, vehicle, textures)?; }
        if self.debug_mode { self.draw_debug_overlays(canvas)?; }

        canvas.present();
        Ok(())
    }

    fn draw_vehicle(&self, canvas: &mut Canvas<Window>, vehicle: &Vehicle, textures: &Textures) -> Result<(), String> {
        let texture = match vehicle.color {
            VehicleColor::Red => &textures.car_red,
            VehicleColor::Green => &textures.car_green,
            VehicleColor::Blue => &textures.car_blue,
            _ => &textures.car_blue,
        };
        let (w, h) = (vehicle.width, vehicle.height);
        // Swap width and height for horizontal vehicles for correct texture mapping
        let (rect_w, rect_h) = if vehicle.get_current_movement_direction() == Direction::East || vehicle.get_current_movement_direction() == Direction::West { (h,w) } else { (w,h) };
        let dest_rect = Rect::new( (vehicle.position.x - rect_w / 2.0) as i32, (vehicle.position.y - rect_h / 2.0) as i32, rect_w as u32, rect_h as u32 );
        canvas.copy(texture, None, dest_rect)?;
        Ok(())
    }

    fn draw_debug_overlays(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        for vehicle in &self.vehicles {
            canvas.set_draw_color(Color::MAGENTA);
            let (tx, ty) = (vehicle.turn_point.x as i32, vehicle.turn_point.y as i32);
            canvas.draw_rect(Rect::new(tx - 3, ty - 3, 6, 6))?;
        }
        Ok(())
    }

    fn show_final_statistics(&self) {
        let stats_string = self.statistics.get_display_string();
        tinyfiledialogs::message_box_ok("Final Statistics", &stats_string, tinyfiledialogs::MessageBoxIcon::Info);
        println!("\n{}\n", stats_string);
    }
}