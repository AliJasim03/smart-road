// src/main.rs - FIXED: Realistic turning and corrected spawning
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Rect, Point};
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::collections::{VecDeque, HashSet};
use std::time::{Duration, Instant};

mod vehicle;
mod intersection;
mod statistics;
mod algorithm;

use vehicle::{Vehicle, Direction, Route, VehicleState, VelocityLevel, VehicleColor, Vec2};
use intersection::Intersection;
use statistics::Statistics;
use algorithm::SmartIntersection;

pub const WINDOW_WIDTH: u32 = 1024;
pub const WINDOW_HEIGHT: u32 = 768;
const FPS: u32 = 60;
const PHYSICS_TIMESTEP: f64 = 1.0 / 60.0;

// PERFECT 6-LANE MATHEMATICS
const LANE_WIDTH: f32 = 30.0;
const TOTAL_ROAD_WIDTH: f32 = 180.0;
const HALF_ROAD_WIDTH: f32 = 90.0;

fn main() -> Result<(), String> {
    println!("=== Smart Road - FIXED REALISTIC TURNS ===");
    println!("✅ Realistic turning at designated points.");
    println!("✅ Corrected vehicle spawn logic (queuing enabled).");
    println!("✅ Perfect lane positioning.");
    println!("✅ Improved collision avoidance.\n");

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("Smart Road - FIXED SYSTEM", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;

    let mut game = GameState::new()?;
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
            match event {
                Event::Quit { .. } => running = false,
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => running = false,
                _ => {
                    game.handle_event(&event);
                }
            }
        }

        // Fixed timestep physics
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
        let color = get_lane_color_name(lane);
        println!("  Lane {}: {} at x={} ({})", lane, color, x, get_route_name(lane));
    }

    println!("\nSouth-bound lanes (left side of vertical road):");
    for lane in 0..3 {
        let x = get_lane_center_x(Direction::South, lane);
        let color = get_lane_color_name(lane);
        println!("  Lane {}: {} at x={} ({})", lane, color, x, get_route_name(lane));
    }
    println!("=====================================\n");
}

// PERFECT LANE POSITIONING FUNCTIONS
fn get_lane_center_x(direction: Direction, lane: usize) -> f32 {
    let center_x = WINDOW_WIDTH as f32 / 2.0;
    match direction {
        Direction::North => center_x + HALF_ROAD_WIDTH - LANE_WIDTH * (lane as f32 + 0.5), // Lanes from right to left
        Direction::South => center_x - HALF_ROAD_WIDTH + LANE_WIDTH * (lane as f32 + 0.5), // Lanes from right to left
        _ => center_x,
    }
}

fn get_lane_center_y(direction: Direction, lane: usize) -> f32 {
    let center_y = WINDOW_HEIGHT as f32 / 2.0;
    match direction {
        Direction::East => center_y + HALF_ROAD_WIDTH - LANE_WIDTH * (lane as f32 + 0.5), // Lanes from bottom to top
        Direction::West => center_y - HALF_ROAD_WIDTH + LANE_WIDTH * (lane as f32 + 0.5), // Lanes from bottom to top
        _ => center_y,
    }
}

fn get_destination_for_route(incoming: Direction, route: Route) -> Direction {
    match (incoming, route) {
        // From SOUTH moving NORTH
        (Direction::North, Route::Left) => Direction::West,
        (Direction::North, Route::Straight) => Direction::North,
        (Direction::North, Route::Right) => Direction::East,

        // From NORTH moving SOUTH
        (Direction::South, Route::Left) => Direction::East,
        (Direction::South, Route::Straight) => Direction::South,
        (Direction::South, Route::Right) => Direction::West,

        // From WEST moving EAST
        (Direction::East, Route::Left) => Direction::North,
        (Direction::East, Route::Straight) => Direction::East,
        (Direction::East, Route::Right) => Direction::South,

        // From EAST moving WEST
        (Direction::West, Route::Left) => Direction::South,
        (Direction::West, Route::Straight) => Direction::West,
        (Direction::West, Route::Right) => Direction::North,
    }
}

// Maps lane index to its designated route
fn get_route_for_lane(lane: usize) -> Route {
    match lane {
        0 => Route::Right,
        1 => Route::Straight,
        2 => Route::Left,
        _ => Route::Straight,
    }
}

// Maps the route to a distinct color
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


struct GameState {
    vehicles: VecDeque<Vehicle>,
    intersection: Intersection,
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
            vehicles: VecDeque::new(),
            intersection: Intersection::new(),
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
            Event::KeyDown { keycode: Some(keycode), repeat: false, .. } => {
                match keycode {
                    Keycode::Up => self.try_spawn_vehicle(Direction::North),
                    Keycode::Down => self.try_spawn_vehicle(Direction::South),
                    Keycode::Left => self.try_spawn_vehicle(Direction::West),
                    Keycode::Right => self.try_spawn_vehicle(Direction::East),
                    Keycode::R => {
                        self.continuous_spawn = !self.continuous_spawn;
                        println!("🤖 Continuous spawn: {}", if self.continuous_spawn { "ON" } else { "OFF" });
                    }
                    Keycode::D => {
                        self.debug_mode = !self.debug_mode;
                        println!("🔍 Debug mode: {}", if self.debug_mode { "ON" } else { "OFF" });
                    }
                    Keycode::Space => self.print_current_statistics(),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn update_physics(&mut self, dt: f64) {
        if self.spawn_cooldown > 0.0 {
            self.spawn_cooldown -= dt as f32;
        }

        if self.continuous_spawn {
            self.spawn_timer += dt as f32;
            if self.spawn_timer >= 2.0 {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let direction = match rng.gen_range(0..4) {
                    0 => Direction::North,
                    1 => Direction::South,
                    2 => Direction::East,
                    _ => Direction::West,
                };
                self.try_spawn_vehicle(direction);
                self.spawn_timer = 0.0;
            }
        }

        self.algorithm.process_vehicles(&mut self.vehicles, &self.intersection, (dt * 1000.0) as u32);

        for vehicle in &mut self.vehicles {
            vehicle.update_physics(dt, &self.intersection);
        }

        self.cleanup_completed_vehicles();
        self.statistics.update(&self.vehicles, self.algorithm.close_calls);
    }

    fn try_spawn_vehicle(&mut self, direction: Direction) {
        if self.spawn_cooldown > 0.0 {
            return;
        }

        // --- FIXED SPAWN LOGIC ---
        // Check if a vehicle is already too close to the spawn point for this direction.
        let min_spawn_distance = 50.0; // Distance threshold to prevent overlap
        let is_spawn_blocked = self.vehicles.iter().any(|v| {
            if v.direction == direction {
                let distance_traveled = match v.direction {
                    Direction::North => (WINDOW_HEIGHT as f32 + 100.0) - v.position.y,
                    Direction::South => v.position.y + 100.0,
                    Direction::East => v.position.x + 100.0,
                    Direction::West => (WINDOW_WIDTH as f32 + 100.0) - v.position.x,
                };
                distance_traveled < min_spawn_distance
            } else { false }
        });

        if is_spawn_blocked {
            // println!("❌ Spawn blocked: a vehicle is too close to the spawn point.");
            return;
        }
        // --- END FIXED SPAWN LOGIC ---

        use rand::Rng;
        let lane = rand::thread_rng().gen_range(0..3);
        let route = get_route_for_lane(lane);
        let destination = get_destination_for_route(direction, route);
        let color = get_color_for_route(route);

        let vehicle = Vehicle::new(self.next_vehicle_id, direction, destination, lane, route, color);
        println!("🚗 Spawned Vehicle {}: {:?} Lane {} ({}) → {:?} (Route: {:?})",
                 vehicle.id, direction, lane, get_route_name(lane), destination, route);

        self.vehicles.push_back(vehicle);
        self.next_vehicle_id += 1;
        self.spawn_cooldown = 0.2; // Short cooldown
        self.statistics.record_vehicle_spawn(direction, route);
    }

    fn cleanup_completed_vehicles(&mut self) {
        let initial_count = self.vehicles.len();
        self.vehicles.retain(|v| {
            let completed = v.state == VehicleState::Completed;
            if completed {
                self.statistics.record_vehicle_completion(v.time_in_intersection);
                println!("✅ Vehicle {} completed journey", v.id);
            }
            !completed
        });

        let active_ids: Vec<u32> = self.vehicles.iter().map(|v| v.id).collect();
        self.statistics.cleanup_completed_vehicle_data(&active_ids);
    }

    fn print_current_statistics(&self) {
        println!("\n=== CURRENT STATISTICS ===");
        println!("🚗 Active vehicles: {}", self.vehicles.len());
        println!("✅ Vehicles completed: {}", self.statistics.vehicles_completed);
        println!("⚠️  Close calls: {}", self.algorithm.close_calls);
    }

    fn render(&mut self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(40, 120, 40));
        canvas.clear();
        self.draw_roads(canvas)?;
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

    fn draw_roads(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let center_x = WINDOW_WIDTH as f32 / 2.0;
        let center_y = WINDOW_HEIGHT as f32 / 2.0;
        canvas.set_draw_color(Color::RGB(60, 60, 60));
        canvas.fill_rect(Rect::new((center_x - HALF_ROAD_WIDTH) as i32, 0, TOTAL_ROAD_WIDTH as u32, WINDOW_HEIGHT))?;
        canvas.fill_rect(Rect::new(0, (center_y - HALF_ROAD_WIDTH) as i32, WINDOW_WIDTH, TOTAL_ROAD_WIDTH as u32))?;
        self.draw_lane_markings(canvas, center_x, center_y)?;
        self.draw_lane_indicators(canvas)?;
        Ok(())
    }

    fn draw_lane_markings(&self, canvas: &mut Canvas<Window>, center_x: f32, center_y: f32) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        let half_size = HALF_ROAD_WIDTH;
        // Vertical dashed lines
        for i in 1..3 {
            let x = (center_x - half_size + (i as f32 * LANE_WIDTH)) as i32;
            self.draw_dashed_line(canvas, (x, 0), (x, (center_y - half_size) as i32))?;
            self.draw_dashed_line(canvas, (x, (center_y + half_size) as i32), (x, WINDOW_HEIGHT as i32))?;
        }
        for i in 1..3 {
            let x = (center_x + half_size - (i as f32 * LANE_WIDTH)) as i32;
            self.draw_dashed_line(canvas, (x, 0), (x, (center_y - half_size) as i32))?;
            self.draw_dashed_line(canvas, (x, (center_y + half_size) as i32), (x, WINDOW_HEIGHT as i32))?;
        }
        // Horizontal dashed lines
        for i in 1..3 {
            let y = (center_y - half_size + (i as f32 * LANE_WIDTH)) as i32;
            self.draw_dashed_line(canvas, (0, y), ((center_x - half_size) as i32, y))?;
            self.draw_dashed_line(canvas, ((center_x + half_size) as i32, y), (WINDOW_WIDTH as i32, y))?;
        }
        for i in 1..3 {
            let y = (center_y + half_size - (i as f32 * LANE_WIDTH)) as i32;
            self.draw_dashed_line(canvas, (0, y), ((center_x - half_size) as i32, y))?;
            self.draw_dashed_line(canvas, ((center_x + half_size) as i32, y), (WINDOW_WIDTH as i32, y))?;
        }

        // Center solid dividers
        canvas.set_draw_color(Color::RGB(255, 255, 0));
        canvas.draw_line((center_x as i32, 0), (center_x as i32, (center_y - half_size) as i32))?;
        canvas.draw_line((center_x as i32, (center_y + half_size) as i32), (center_x as i32, WINDOW_HEIGHT as i32))?;
        canvas.draw_line((0, center_y as i32), ((center_x - half_size) as i32, center_y as i32))?;
        canvas.draw_line(((center_x + half_size) as i32, center_y as i32), (WINDOW_WIDTH as i32, center_y as i32))?;
        Ok(())
    }

    fn draw_dashed_line(&self, canvas: &mut Canvas<Window>, from: (i32, i32), to: (i32, i32)) -> Result<(), String> {
        let (x1, y1) = from;
        let (x2, y2) = to;
        let dx = (x2 - x1) as f32;
        let dy = (y2 - y1) as f32;
        let distance = (dx * dx + dy * dy).sqrt();
        let num_dashes = (distance / 20.0).round() as i32;

        for i in 0..num_dashes {
            if i % 2 == 0 {
                let start_x = x1 as f32 + (dx / num_dashes as f32) * i as f32;
                let start_y = y1 as f32 + (dy / num_dashes as f32) * i as f32;
                let end_x = x1 as f32 + (dx / num_dashes as f32) * (i + 1) as f32;
                let end_y = y1 as f32 + (dy / num_dashes as f32) * (i + 1) as f32;
                canvas.draw_line(
                    (start_x.round() as i32, start_y.round() as i32),
                    (end_x.round() as i32, end_y.round() as i32)
                )?;
            }
        }
        Ok(())
    }

    fn draw_lane_indicators(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let center_x = WINDOW_WIDTH as f32 / 2.0;
        let center_y = WINDOW_HEIGHT as f32 / 2.0;
        let offset = 120.0;
        for lane in 0..3 {
            let route = get_route_for_lane(lane);
            let color = match get_color_for_route(route) {
                VehicleColor::Red => Color::RGB(255, 80, 80),
                VehicleColor::Blue => Color::RGB(80, 80, 255),
                VehicleColor::Green => Color::RGB(80, 255, 80),
                VehicleColor::Yellow => Color::RGB(255,255,80)
            };
            canvas.set_draw_color(color);

            let indicator_size = 10u32;
            // Southbound lane indicators
            let x_south = get_lane_center_x(Direction::South, lane);
            canvas.fill_rect(Rect::new((x_south - indicator_size as f32 / 2.0) as i32, (center_y - offset) as i32, indicator_size, indicator_size))?;

            // Northbound lane indicators
            let x_north = get_lane_center_x(Direction::North, lane);
            canvas.fill_rect(Rect::new((x_north - indicator_size as f32 / 2.0) as i32, (center_y + offset) as i32, indicator_size, indicator_size))?;

            // Eastbound lane indicators
            let y_east = get_lane_center_y(Direction::East, lane);
            canvas.fill_rect(Rect::new((center_x - offset) as i32, (y_east - indicator_size as f32 / 2.0) as i32, indicator_size, indicator_size))?;

            // Westbound lane indicators
            let y_west = get_lane_center_y(Direction::West, lane);
            canvas.fill_rect(Rect::new((center_x + offset) as i32, (y_west - indicator_size as f32 / 2.0) as i32, indicator_size, indicator_size))?;
        }
        Ok(())
    }

    fn draw_intersection(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let center_x = WINDOW_WIDTH as f32 / 2.0;
        let center_y = WINDOW_HEIGHT as f32 / 2.0;
        canvas.set_draw_color(Color::RGB(45, 45, 45));
        canvas.fill_rect(Rect::new( (center_x - HALF_ROAD_WIDTH) as i32, (center_y - HALF_ROAD_WIDTH) as i32, TOTAL_ROAD_WIDTH as u32, TOTAL_ROAD_WIDTH as u32 ))?;
        canvas.set_draw_color(Color::RGB(255, 255, 0));
        canvas.draw_rect(Rect::new( (center_x - HALF_ROAD_WIDTH) as i32, (center_y - HALF_ROAD_WIDTH) as i32, TOTAL_ROAD_WIDTH as u32, TOTAL_ROAD_WIDTH as u32 ))?;
        Ok(())
    }

    fn draw_vehicle(&self, canvas: &mut Canvas<Window>, vehicle: &Vehicle) -> Result<(), String> {
        let color = match vehicle.color {
            VehicleColor::Red => Color::RGB(255, 80, 80),
            VehicleColor::Blue => Color::RGB(80, 80, 255),
            VehicleColor::Green => Color::RGB(80, 255, 80),
            VehicleColor::Yellow => Color::RGB(255, 255, 80),
        };

        canvas.set_draw_color(color);
        let dest_rect = Rect::new( (vehicle.position.x - vehicle.width / 2.0) as i32, (vehicle.position.y - vehicle.height / 2.0) as i32, vehicle.width as u32, vehicle.height as u32 );
        canvas.fill_rect(dest_rect)?;
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.draw_rect(dest_rect)?;

        // Simple direction indicator
        let center = dest_rect.center();
        let (end_x, end_y) = match vehicle.get_current_movement_direction() {
            Direction::North => (center.x(), center.y() - 8),
            Direction::South => (center.x(), center.y() + 8),
            Direction::East => (center.x() + 8, center.y()),
            Direction::West => (center.x() - 8, center.y()),
        };
        canvas.set_draw_color(Color::RGB(255,255,255));
        canvas.draw_line(center, (end_x, end_y))?;
        Ok(())
    }

    fn draw_debug_overlays(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        for vehicle in &self.vehicles {
            // Draw turn point
            canvas.set_draw_color(Color::RGB(255, 0, 255));
            let (tx, ty) = (vehicle.turn_point.x as i32, vehicle.turn_point.y as i32);
            canvas.draw_rect(Rect::new(tx - 3, ty - 3, 6, 6))?;

            // Draw target lane
            canvas.set_draw_color(Color::RGB(0, 255, 255));
            let (lx, ly) = (vehicle.target_lane_pos.x as i32, vehicle.target_lane_pos.y as i32);
            canvas.draw_rect(Rect::new(lx - 3, ly - 3, 6, 6))?;
        }
        Ok(())
    }

    fn show_final_statistics(&self) {
        let _ = self.statistics.display();
    }
}