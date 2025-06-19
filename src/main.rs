// src/main.rs
use sdl2::event::Event;
use sdl2::image::{self, InitFlag, LoadTexture};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

// +++ ADDED: Import the necessary crate for the statistics window +++
use tinyfiledialogs;

mod algorithm;
mod intersection;
mod statistics;
mod vehicle;

use algorithm::SmartIntersection;
use intersection::Intersection;
use statistics::Statistics;
use vehicle::{Direction, Route, Vehicle, VehicleColor, VehicleState};

pub const WINDOW_WIDTH: u32 = 1024;
pub const WINDOW_HEIGHT: u32 = 768;
const FPS: u32 = 60;
const PHYSICS_TIMESTEP: f64 = 1.0 / 60.0;

const LANE_WIDTH: f32 = 30.0;
const TOTAL_ROAD_WIDTH: f32 = 180.0;
const HALF_ROAD_WIDTH: f32 = 90.0;
const TILE_SIZE: u32 = 128;

fn main() -> Result<(), String> {
    println!("=== Smart Road - All Assets Integrated ===");
    println!("✅ Rendering car sprites, tiled roads, and background.");

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let _image_context = image::init(InitFlag::PNG)?;

    let window = video_subsystem
        .window("Smart Road - All Assets", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();
    let mut game = GameState::new(&texture_creator)?;
    let mut event_pump = sdl_context.event_pump()?;
    let mut running = true;
    let mut last_frame = Instant::now();
    let mut physics_accumulator = 0.0;

    print_controls();
    print_lane_mathematics();

    while running {
        let now = Instant::now();
        let frame_delta = now.duration_since(last_frame).as_secs_f64();
        last_frame = now;
        physics_accumulator += frame_delta;

        for event in event_pump.poll_iter() {
            if let Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } = event
            {
                running = false;
            }
            if let Event::Quit { .. } = event {
                running = false;
            }
            game.handle_event(&event);
        }

        while physics_accumulator >= PHYSICS_TIMESTEP {
            game.update_physics(PHYSICS_TIMESTEP);
            physics_accumulator -= PHYSICS_TIMESTEP;
        }

        game.render(&mut canvas)?;

        let frame_time = now.elapsed();
        if frame_time < Duration::from_millis(1000 / FPS as u64) {
            std::thread::sleep(Duration::from_millis(1000 / FPS as u64) - frame_time);
        }
    }

    game.show_final_statistics();
    Ok(())
}

fn print_controls() {
    println!("=== CONTROLS ===");
    println!("↑ Arrow Up:    Spawn vehicle from South (moving North)");
    println!("↓ Arrow Down:  Spawn vehicle from North (moving South)");
    println!("← Arrow Left:  Spawn vehicle from East (moving West)");
    println!("→ Arrow Right: Spawn vehicle from West (moving East)");
    println!("R:             Toggle continuous random spawning");
    println!("D:             Toggle debug visualization");
    println!("Space:         Show current statistics");
    println!("Esc:           Exit and show final statistics");
}

fn print_lane_mathematics() {
    let center_x = WINDOW_WIDTH as f32 / 2.0;
    let center_y = WINDOW_HEIGHT as f32 / 2.0;
    println!("\n=== PERFECT LANE MATHEMATICS ===");
    println!("Screen center: ({}, {})", center_x, center_y);
    println!("Lane width: {} pixels each", LANE_WIDTH);
    println!("Total road width: {} pixels", TOTAL_ROAD_WIDTH);
    println!("\nNorth-bound lanes (right side of vertical road):");
    for lane in 0..3 {
        let x = get_lane_center_x(Direction::North, lane);
        println!(
            "  Lane {}: {} at x={} ({})",
            lane,
            get_lane_color_name(lane),
            x,
            get_route_name(lane)
        );
    }
    println!("\nSouth-bound lanes (left side of vertical road):");
    for lane in 0..3 {
        let x = get_lane_center_x(Direction::South, lane);
        println!(
            "  Lane {}: {} at x={} ({})",
            lane,
            get_lane_color_name(lane),
            x,
            get_route_name(lane)
        );
    }
    println!("=====================================\n");
}

fn get_lane_center_x(direction: Direction, lane: usize) -> f32 {
    let center_x = WINDOW_WIDTH as f32 / 2.0;
    match direction {
        Direction::North => center_x + HALF_ROAD_WIDTH - LANE_WIDTH * (lane as f32 + 0.5),
        Direction::South => center_x - HALF_ROAD_WIDTH + LANE_WIDTH * (lane as f32 + 0.5),
        _ => center_x,
    }
}

fn get_lane_center_y(direction: Direction, lane: usize) -> f32 {
    let center_y = WINDOW_HEIGHT as f32 / 2.0;
    match direction {
        Direction::East => center_y + HALF_ROAD_WIDTH - LANE_WIDTH * (lane as f32 + 0.5),
        Direction::West => center_y - HALF_ROAD_WIDTH + LANE_WIDTH * (lane as f32 + 0.5),
        _ => center_y,
    }
}

fn get_destination_for_route(incoming: Direction, route: Route) -> Direction {
    match (incoming, route) {
        (Direction::North, Route::Left) => Direction::West,
        (Direction::North, Route::Straight) => Direction::North,
        (Direction::North, Route::Right) => Direction::East,
        (Direction::South, Route::Left) => Direction::East,
        (Direction::South, Route::Straight) => Direction::South,
        (Direction::South, Route::Right) => Direction::West,
        (Direction::East, Route::Left) => Direction::North,
        (Direction::East, Route::Straight) => Direction::East,
        (Direction::East, Route::Right) => Direction::South,
        (Direction::West, Route::Left) => Direction::South,
        (Direction::West, Route::Straight) => Direction::West,
        (Direction::West, Route::Right) => Direction::North,
    }
}

fn get_route_for_lane(lane: usize) -> Route {
    match lane {
        0 => Route::Right,
        1 => Route::Straight,
        2 => Route::Left,
        _ => Route::Straight,
    }
}

fn get_color_for_route(route: Route) -> VehicleColor {
    match route {
        Route::Left => VehicleColor::Red,
        Route::Straight => VehicleColor::Blue,
        Route::Right => VehicleColor::Green,
    }
}

fn get_lane_color_name(lane: usize) -> &'static str {
    match get_route_for_lane(lane) {
        Route::Left => "RED (Left)",
        Route::Straight => "BLUE (Straight)",
        Route::Right => "GREEN (Right)",
    }
}

fn get_route_name(lane: usize) -> &'static str {
    match get_route_for_lane(lane) {
        Route::Left => "LEFT",
        Route::Straight => "STRAIGHT",
        Route::Right => "RIGHT",
    }
}

struct GameState<'a> {
    vehicles: VecDeque<Vehicle>,
    intersection: Intersection,
    statistics: Statistics,
    algorithm: SmartIntersection,
    spawn_cooldown: f32,
    continuous_spawn: bool,
    spawn_timer: f32,
    next_vehicle_id: u32,
    debug_mode: bool,
    asphalt_texture: Texture<'a>,
    background_texture: Texture<'a>,
    car_red_texture: Texture<'a>,
    car_green_texture: Texture<'a>,
    car_blue_texture: Texture<'a>,
}

impl<'a> GameState<'a> {
    pub fn new(texture_creator: &'a TextureCreator<WindowContext>) -> Result<Self, String> {
        let asphalt_texture = texture_creator.load_texture("assets/asphalt_tile.png")?;
        let background_texture = texture_creator.load_texture("assets/grass_tile.png")?;
        let car_red_texture = texture_creator.load_texture("assets/car_red.png")?;
        let car_green_texture = texture_creator.load_texture("assets/car_green.png")?;
        let car_blue_texture = texture_creator.load_texture("assets/car_blue.png")?;

        Ok(GameState {
            vehicles: VecDeque::new(),
            intersection: Intersection::new(),
            statistics: Statistics::new(),
            algorithm: SmartIntersection::new(),
            spawn_cooldown: 0.0,
            continuous_spawn: false,
            spawn_timer: 0.0,
            next_vehicle_id: 0,
            debug_mode: false,
            asphalt_texture,
            background_texture,
            car_red_texture,
            car_green_texture,
            car_blue_texture,
        })
    }

    pub fn handle_event(&mut self, event: &Event) {
        if let Event::KeyDown {
            keycode: Some(keycode),
            repeat: false,
            ..
        } = event
        {
            match keycode {
                Keycode::Up => self.try_spawn_vehicle(Direction::North),
                Keycode::Down => self.try_spawn_vehicle(Direction::South),
                Keycode::Left => self.try_spawn_vehicle(Direction::West),
                Keycode::Right => self.try_spawn_vehicle(Direction::East),
                Keycode::R => {
                    self.continuous_spawn = !self.continuous_spawn;
                    println!(
                        "🤖 Continuous spawn: {}",
                        if self.continuous_spawn { "ON" } else { "OFF" }
                    );
                }
                Keycode::D => {
                    self.debug_mode = !self.debug_mode;
                    println!(
                        "🔍 Debug mode: {}",
                        if self.debug_mode { "ON" } else { "OFF" }
                    );
                }
                Keycode::Space => self.print_current_statistics(),
                _ => {}
            }
        }
    }

    pub fn update_physics(&mut self, dt: f64) {
        if self.spawn_cooldown > 0.0 {
            self.spawn_cooldown -= dt as f32;
        }
        if self.continuous_spawn {
            self.spawn_timer += dt as f32;
            if self.spawn_timer >= 2.0 {
                let direction = match rand::random::<u8>() % 4 {
                    0 => Direction::North,
                    1 => Direction::South,
                    2 => Direction::East,
                    _ => Direction::West,
                };
                self.try_spawn_vehicle(direction);
                self.spawn_timer = 0.0;
            }
        }
        self.algorithm.process_vehicles(
            &mut self.vehicles,
            &self.intersection,
            (dt * 1000.0) as u32,
        );
        for vehicle in &mut self.vehicles {
            vehicle.update_physics(dt, &self.intersection);
        }
        self.cleanup_completed_vehicles();
        self.statistics
            .update(&self.vehicles, self.algorithm.close_calls);
    }

    pub fn render(&mut self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        canvas.clear();
        self.draw_tiled_background(canvas)?;
        self.draw_road_surface(canvas)?;
        self.draw_lane_markings(canvas)?;
        self.draw_intersection(canvas)?;
        for vehicle in &self.vehicles {
            self.draw_vehicle(canvas, vehicle)?;
        }
        if self.debug_mode {
            self.draw_debug_overlays(canvas)?;
        }
        canvas.present();
        Ok(())
    }

    fn draw_tiled_background(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let mut y = 0;
        while y < WINDOW_HEIGHT {
            let mut x = 0;
            while x < WINDOW_WIDTH {
                let dest_rect = Rect::new(x as i32, y as i32, TILE_SIZE, TILE_SIZE);
                canvas.copy(&self.background_texture, None, Some(dest_rect))?;
                x += TILE_SIZE;
            }
            y += TILE_SIZE;
        }
        Ok(())
    }

    fn draw_road_surface(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let center_x = WINDOW_WIDTH as f32 / 2.0;
        let center_y = WINDOW_HEIGHT as f32 / 2.0;
        let vertical_road = Rect::new(
            (center_x - HALF_ROAD_WIDTH) as i32,
            0,
            TOTAL_ROAD_WIDTH as u32,
            WINDOW_HEIGHT,
        );
        let horizontal_road = Rect::new(
            0,
            (center_y - HALF_ROAD_WIDTH) as i32,
            WINDOW_WIDTH,
            TOTAL_ROAD_WIDTH as u32,
        );
        canvas.copy(&self.asphalt_texture, None, vertical_road)?;
        canvas.copy(&self.asphalt_texture, None, horizontal_road)?;
        Ok(())
    }

    fn draw_lane_markings(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let center_x = WINDOW_WIDTH as f32 / 2.0;
        let center_y = WINDOW_HEIGHT as f32 / 2.0;
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        for i in 1..3 {
            let x_left = (center_x - HALF_ROAD_WIDTH + (i as f32 * LANE_WIDTH)) as i32;
            self.draw_dashed_line(
                canvas,
                (x_left, 0),
                (x_left, (center_y - HALF_ROAD_WIDTH) as i32),
            )?;
            self.draw_dashed_line(
                canvas,
                (x_left, (center_y + HALF_ROAD_WIDTH) as i32),
                (x_left, WINDOW_HEIGHT as i32),
            )?;
            let x_right = (center_x + HALF_ROAD_WIDTH - (i as f32 * LANE_WIDTH)) as i32;
            self.draw_dashed_line(
                canvas,
                (x_right, 0),
                (x_right, (center_y - HALF_ROAD_WIDTH) as i32),
            )?;
            self.draw_dashed_line(
                canvas,
                (x_right, (center_y + HALF_ROAD_WIDTH) as i32),
                (x_right, WINDOW_HEIGHT as i32),
            )?;
        }
        for i in 1..3 {
            let y_top = (center_y - HALF_ROAD_WIDTH + (i as f32 * LANE_WIDTH)) as i32;
            self.draw_dashed_line(
                canvas,
                (0, y_top),
                ((center_x - HALF_ROAD_WIDTH) as i32, y_top),
            )?;
            self.draw_dashed_line(
                canvas,
                ((center_x + HALF_ROAD_WIDTH) as i32, y_top),
                (WINDOW_WIDTH as i32, y_top),
            )?;
            let y_bottom = (center_y + HALF_ROAD_WIDTH - (i as f32 * LANE_WIDTH)) as i32;
            self.draw_dashed_line(
                canvas,
                (0, y_bottom),
                ((center_x - HALF_ROAD_WIDTH) as i32, y_bottom),
            )?;
            self.draw_dashed_line(
                canvas,
                ((center_x + HALF_ROAD_WIDTH) as i32, y_bottom),
                (WINDOW_WIDTH as i32, y_bottom),
            )?;
        }
        canvas.set_draw_color(Color::RGB(255, 255, 0));
        let half_size_i32 = HALF_ROAD_WIDTH as i32;
        canvas.draw_line(
            (center_x as i32, 0),
            (center_x as i32, center_y as i32 - half_size_i32),
        )?;
        canvas.draw_line(
            (center_x as i32, center_y as i32 + half_size_i32),
            (center_x as i32, WINDOW_HEIGHT as i32),
        )?;
        canvas.draw_line(
            (0, center_y as i32),
            (center_x as i32 - half_size_i32, center_y as i32),
        )?;
        canvas.draw_line(
            (center_x as i32 + half_size_i32, center_y as i32),
            (WINDOW_WIDTH as i32, center_y as i32),
        )?;
        Ok(())
    }

    fn draw_dashed_line(
        &self,
        canvas: &mut Canvas<Window>,
        from: (i32, i32),
        to: (i32, i32),
    ) -> Result<(), String> {
        let (x1, y1) = from;
        let (x2, y2) = to;
        let dx = (x2 - x1) as f32;
        let dy = (y2 - y1) as f32;
        let distance = (dx * dx + dy * dy).sqrt();
        let dash_length = 15.0;
        let gap_length = 10.0;
        let total_segment_length = dash_length + gap_length;
        if total_segment_length <= 0.0 { return Ok(()); }
        let num_segments = (distance / total_segment_length).floor();
        if distance == 0.0 { return Ok(()); }
        let dir_x = dx / distance;
        let dir_y = dy / distance;
        let mut current_pos_x = x1 as f32;
        let mut current_pos_y = y1 as f32;
        for _ in 0..num_segments as i32 {
            let end_x = current_pos_x + dir_x * dash_length;
            let end_y = current_pos_y + dir_y * dash_length;
            canvas.draw_line(
                (current_pos_x.round() as i32, current_pos_y.round() as i32),
                (end_x.round() as i32, end_y.round() as i32),
            )?;
            current_pos_x += dir_x * total_segment_length;
            current_pos_y += dir_y * total_segment_length;
        }
        Ok(())
    }

    fn draw_intersection(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let center_x = WINDOW_WIDTH as f32 / 2.0;
        let center_y = WINDOW_HEIGHT as f32 / 2.0;
        let rect = Rect::new(
            (center_x - HALF_ROAD_WIDTH) as i32,
            (center_y - HALF_ROAD_WIDTH) as i32,
            TOTAL_ROAD_WIDTH as u32,
            TOTAL_ROAD_WIDTH as u32,
        );
        canvas.set_draw_color(Color::RGB(50, 50, 50));
        canvas.fill_rect(rect)?;
        canvas.set_draw_color(Color::RGB(255, 255, 0));
        canvas.draw_rect(rect)?;
        Ok(())
    }

    fn draw_vehicle(&self, canvas: &mut Canvas<Window>, vehicle: &Vehicle) -> Result<(), String> {
        let texture = match vehicle.color {
            VehicleColor::Red => &self.car_red_texture,
            VehicleColor::Green => &self.car_green_texture,
            VehicleColor::Blue => &self.car_blue_texture,
            VehicleColor::Yellow => &self.car_blue_texture,
        };

        let angle = match vehicle.get_current_movement_direction() {
            Direction::North => 0.0,
            Direction::South => 180.0,
            Direction::East => 90.0,
            Direction::West => 270.0,
        };

        let sprite_width = vehicle.width as u32;
        let sprite_height = vehicle.height as u32;

        let dest_rect = Rect::new(
            (vehicle.position.x - (sprite_width as f32 / 2.0)) as i32,
            (vehicle.position.y - (sprite_height as f32 / 2.0)) as i32,
            sprite_width,
            sprite_height,
        );

        canvas.copy_ex(texture, None, Some(dest_rect), angle, None, false, false)?;
        Ok(())
    }

    fn draw_debug_overlays(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;
        let font = ttf_context.load_font("assets/OpenSans-Regular.ttf", 16)?;

        for vehicle in &self.vehicles {
            canvas.set_draw_color(Color::RGB(255, 0, 255));
            let (tx, ty) = (vehicle.turn_point.x as i32, vehicle.turn_point.y as i32);
            canvas.draw_rect(Rect::new(tx - 3, ty - 3, 6, 6))?;

            canvas.set_draw_color(Color::RGB(0, 255, 255));
            let (lx, ly) = (
                vehicle.target_lane_pos.x as i32,
                vehicle.target_lane_pos.y as i32,
            );
            canvas.draw_rect(Rect::new(lx - 3, ly - 3, 6, 6))?;

            let surface = font.render(&format!("{:.1}s", vehicle.time_to_intersection))
                .blended(if vehicle.has_passage_grant {Color::GREEN} else {Color::YELLOW})
                .map_err(|e| e.to_string())?;

            let texture_creator = canvas.texture_creator();
            let texture = texture_creator.create_texture_from_surface(&surface).map_err(|e| e.to_string())?;

            let sdl2::render::TextureQuery { width, height, .. } = texture.query();

            let text_pos = Rect::new(vehicle.position.x as i32 - 10, vehicle.position.y as i32 - 25, width, height);
            canvas.copy(&texture, None, text_pos)?;
        }
        Ok(())
    }

    fn try_spawn_vehicle(&mut self, direction: Direction) {
        if self.spawn_cooldown > 0.0 { return; }

        let min_spawn_distance = 150.0;
        if self.vehicles.iter().any(|v| {
            if v.direction == direction {
                let distance = match v.direction {
                    Direction::North => (WINDOW_HEIGHT as f32 - v.position.y).abs(),
                    Direction::South => v.position.y.abs(),
                    Direction::East => v.position.x.abs(),
                    Direction::West => (WINDOW_WIDTH as f32 - v.position.x).abs(),
                };
                return distance < min_spawn_distance;
            }
            false
        }) {
            return;
        }

        let lane = rand::random::<usize>() % 3;
        let route = get_route_for_lane(lane);
        let destination = get_destination_for_route(direction, route);
        let color = get_color_for_route(route);
        let vehicle = Vehicle::new(
            self.next_vehicle_id,
            direction,
            destination,
            lane,
            route,
            color,
        );
        println!(
            "🚗 Spawned Vehicle {}: {:?} Lane {} ({}) → {:?} (Route: {:?})",
            vehicle.id,
            direction,
            lane,
            get_route_name(lane),
            destination,
            route
        );
        self.vehicles.push_back(vehicle);
        self.next_vehicle_id += 1;
        self.spawn_cooldown = 0.2;
        self.statistics.record_vehicle_spawn(direction, route);
    }

    fn cleanup_completed_vehicles(&mut self) {
        self.vehicles.retain(|v| {
            let completed = v.state == VehicleState::Completed;
            if completed {
                self.statistics
                    .record_vehicle_completion(v.time_in_intersection);
                self.algorithm.clear_reservation_for_vehicle(v.id);
            }
            !completed
        });
    }

    fn print_current_statistics(&self) {
        println!("\n=== CURRENT STATISTICS ===");
        println!("🚗 Active vehicles: {}", self.vehicles.len());
        println!("✅ Vehicles completed: {}", self.statistics.vehicles_completed);
        println!("⚠️  Close calls: {}", self.algorithm.close_calls);
    }

    // --- KEY CHANGE: Use a message box for final statistics ---
    fn show_final_statistics(&self) {
        // First, print to console as a fallback.
        println!("\n=============================");
        println!("{}", self.statistics.get_display_string());
        println!("=============================");

        // Then, show the message box window.
        tinyfiledialogs::message_box_ok(
            "Final Statistics",
            &self.statistics.get_display_string(),
            tinyfiledialogs::MessageBoxIcon::Info
        );
    }
}