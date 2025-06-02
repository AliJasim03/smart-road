// src/main.rs - FIXED VERSION
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

mod vehicle;
mod intersection;
mod statistics;

use vehicle::{Vehicle, Direction, Route, VehicleState};
use intersection::Intersection;
use statistics::Statistics;

pub const WINDOW_WIDTH: u32 = 1024;
pub const WINDOW_HEIGHT: u32 = 768;
const FPS: u32 = 60;

fn main() -> Result<(), String> {
    println!("=== Smart Road Intersection Simulation ===");

    // Initialize SDL2
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    // Create window
    let window = video_subsystem
        .window("Smart Road Simulation", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;

    // Initialize game state
    let mut game = GameState::new()?;
    let mut event_pump = sdl_context.event_pump()?;
    let mut running = true;
    let mut last_frame = Instant::now();

    println!("\n=== CONTROLS ===");
    println!("↑ Arrow Up:    Spawn vehicle from South (moving North)");
    println!("↓ Arrow Down:  Spawn vehicle from North (moving South)");
    println!("← Arrow Left:  Spawn vehicle from East (moving West)");
    println!("→ Arrow Right: Spawn vehicle from West (moving East)");
    println!("R:             Toggle continuous spawning");
    println!("Esc:           Exit and show statistics");
    println!("\nSimulation started!\n");

    // Main game loop
    while running {
        let now = Instant::now();
        let delta_time = now.duration_since(last_frame).as_secs_f32();
        last_frame = now;

        // Handle events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => running = false,
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => running = false,
                _ => {
                    game.handle_event(&event);
                }
            }
        }

        // Update
        game.update(delta_time);

        // Render
        game.render(&mut canvas)?;

        // Cap frame rate
        let frame_time = now.elapsed();
        if frame_time < Duration::from_millis(1000 / FPS as u64) {
            std::thread::sleep(Duration::from_millis(1000 / FPS as u64) - frame_time);
        }
    }

    // Show final statistics
    game.show_statistics();
    Ok(())
}

struct GameState {
    vehicles: VecDeque<Vehicle>,
    intersection: Intersection,
    statistics: Statistics,
    spawn_cooldown: f32,
    current_cooldown: f32,
    continuous_spawn: bool,
    spawn_timer: f32,
    next_vehicle_id: u32,
}

impl GameState {
    fn new() -> Result<Self, String> {
        Ok(GameState {
            vehicles: VecDeque::new(),
            intersection: Intersection::new(),
            statistics: Statistics::new(),
            spawn_cooldown: 1.5, // 1.5 seconds between spawns
            current_cooldown: 0.0,
            continuous_spawn: false,
            spawn_timer: 0.0,
            next_vehicle_id: 0,
        })
    }

    fn handle_event(&mut self, event: &Event) {
        match event {
            Event::KeyDown { keycode: Some(keycode), repeat: false, .. } => {
                match keycode {
                    Keycode::Up => {
                        self.spawn_vehicle(Direction::North);
                    }
                    Keycode::Down => {
                        self.spawn_vehicle(Direction::South);
                    }
                    Keycode::Left => {
                        self.spawn_vehicle(Direction::East);
                    }
                    Keycode::Right => {
                        self.spawn_vehicle(Direction::West);
                    }
                    Keycode::R => {
                        self.continuous_spawn = !self.continuous_spawn;
                        println!("Continuous spawn: {}", if self.continuous_spawn { "ON" } else { "OFF" });
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn update(&mut self, delta_time: f32) {
        // Update cooldowns
        if self.current_cooldown > 0.0 {
            self.current_cooldown -= delta_time;
        }

        // Handle continuous spawning
        if self.continuous_spawn {
            self.spawn_timer += delta_time;
            if self.spawn_timer >= 2.0 {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let direction = match rng.gen_range(0..4) {
                    0 => Direction::North,
                    1 => Direction::South,
                    2 => Direction::East,
                    _ => Direction::West,
                };
                self.spawn_vehicle(direction);
                self.spawn_timer = 0.0;
            }
        }

        // Update vehicles
        for vehicle in &mut self.vehicles {
            vehicle.update((delta_time * 1000.0) as u32, &self.intersection);
        }

        // Smart intersection algorithm - simple collision avoidance
        self.apply_smart_algorithm();

        // Remove completed vehicles
        self.remove_completed_vehicles();

        // Update statistics
        self.statistics.update(&self.vehicles);
    }

    fn spawn_vehicle(&mut self, direction: Direction) -> bool {
        if self.current_cooldown > 0.0 {
            println!("Spawn blocked - cooldown active ({:.1}s remaining)", self.current_cooldown);
            return false;
        }

        // Check if too many vehicles in this direction
        let same_direction_count = self.vehicles.iter()
            .filter(|v| v.direction == direction)
            .count();

        if same_direction_count >= 5 {
            println!("Too many vehicles in {:?} direction", direction);
            return false;
        }

        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Random route with realistic distribution
        let route = match rng.gen_range(0..10) {
            0..=2 => Route::Left,     // 30% left
            3..=6 => Route::Straight, // 40% straight
            _ => Route::Right,        // 30% right
        };

        // Random lane appropriate for route
        let lane = match route {
            Route::Left => rng.gen_range(0..2),     // Lanes 0-1 for left turns
            Route::Straight => rng.gen_range(2..4), // Lanes 2-3 for straight
            Route::Right => rng.gen_range(4..6),    // Lanes 4-5 for right turns
        };

        let vehicle = Vehicle::new(direction, lane, route);

        println!("Spawned vehicle {}: {:?} in lane {}, route {:?}, velocity: {:.1} px/s",
                 vehicle.id, vehicle.direction, vehicle.lane, vehicle.route, vehicle.current_velocity);

        self.vehicles.push_back(vehicle);
        self.current_cooldown = self.spawn_cooldown;
        self.next_vehicle_id += 1;
        self.statistics.add_spawned_vehicle();

        true
    }

    fn apply_smart_algorithm(&mut self) {
        // More sophisticated collision avoidance
        for i in 0..self.vehicles.len() {
            let mut should_slow = false;
            let mut should_stop = false;

            let vehicle = &self.vehicles[i];

            // Check intersection conflicts
            if vehicle.is_approaching_intersection(&self.intersection) {
                // Look for vehicles already in the intersection
                for j in 0..self.vehicles.len() {
                    if i == j { continue; }

                    let other = &self.vehicles[j];

                    // If another vehicle is in the intersection
                    if other.is_in_intersection(&self.intersection) {
                        // Check if paths could conflict
                        if self.paths_could_conflict(vehicle, other) {
                            should_slow = true;

                            // If other vehicle is very close to our path, stop completely
                            if self.vehicles_too_close(vehicle, other, 80.0) {
                                should_stop = true;
                            }
                        }
                    }

                    // Check for vehicles approaching from conflicting directions
                    if other.is_approaching_intersection(&self.intersection) &&
                        self.paths_could_conflict(vehicle, other) &&
                        self.vehicles_too_close(vehicle, other, 150.0) {
                        should_slow = true;
                    }
                }
            }

            // Check following distance in same lane
            if !should_stop {
                let following_distance = self.check_following_distance_detailed(i);
                if following_distance < 40.0 {
                    should_stop = true;
                    // Record close call
                    if following_distance < 30.0 {
                        self.statistics.add_close_call();
                    }
                } else if following_distance < 80.0 {
                    should_slow = true;
                }
            }

            // Apply speed adjustments
            if should_stop {
                self.vehicles[i].set_target_velocity(vehicle::VelocityLevel::Slow);
                // Further reduce speed for emergency stop
                self.vehicles[i].current_velocity *= 0.7;
            } else if should_slow {
                self.vehicles[i].set_target_velocity(vehicle::VelocityLevel::Slow);
            } else {
                // Normal speed based on vehicle state
                match vehicle.state {
                    VehicleState::Approaching => {
                        self.vehicles[i].set_target_velocity(vehicle::VelocityLevel::Medium);
                    }
                    VehicleState::Turning => {
                        self.vehicles[i].set_target_velocity(vehicle::VelocityLevel::Slow);
                    }
                    VehicleState::Exiting | VehicleState::Completed => {
                        self.vehicles[i].set_target_velocity(vehicle::VelocityLevel::Fast);
                    }
                    _ => {
                        self.vehicles[i].set_target_velocity(vehicle::VelocityLevel::Medium);
                    }
                }
            }
        }
    }

    fn vehicles_too_close(&self, vehicle1: &Vehicle, vehicle2: &Vehicle, threshold: f64) -> bool {
        let dx = (vehicle1.position.x - vehicle2.position.x) as f64;
        let dy = (vehicle1.position.y - vehicle2.position.y) as f64;
        let distance = (dx * dx + dy * dy).sqrt();
        distance < threshold
    }

    fn check_following_distance_detailed(&self, vehicle_index: usize) -> f64 {
        let vehicle = &self.vehicles[vehicle_index];
        let mut min_distance = f64::INFINITY;

        for (i, other) in self.vehicles.iter().enumerate() {
            if i == vehicle_index { continue; }

            // Check if in same lane and direction, and other vehicle is ahead
            if vehicle.direction == other.direction && vehicle.lane == other.lane {
                let distance = match vehicle.direction {
                    Direction::North => (other.position.y - vehicle.position.y) as f64,
                    Direction::South => (vehicle.position.y - other.position.y) as f64,
                    Direction::East => (vehicle.position.x - other.position.x) as f64,
                    Direction::West => (other.position.x - vehicle.position.x) as f64,
                };

                if distance > 0.0 && distance < min_distance {
                    min_distance = distance;
                }
            }
        }

        min_distance
    }

    fn paths_could_conflict(&self, vehicle1: &Vehicle, vehicle2: &Vehicle) -> bool {
        // Enhanced conflict detection based on direction and route
        match (vehicle1.direction, vehicle1.route, vehicle2.direction, vehicle2.route) {
            // Perpendicular crossing paths (major conflicts)
            (Direction::North, Route::Straight, Direction::East, Route::Straight) => true,
            (Direction::North, Route::Straight, Direction::West, Route::Straight) => true,
            (Direction::South, Route::Straight, Direction::East, Route::Straight) => true,
            (Direction::South, Route::Straight, Direction::West, Route::Straight) => true,
            (Direction::East, Route::Straight, Direction::North, Route::Straight) => true,
            (Direction::East, Route::Straight, Direction::South, Route::Straight) => true,
            (Direction::West, Route::Straight, Direction::North, Route::Straight) => true,
            (Direction::West, Route::Straight, Direction::South, Route::Straight) => true,

            // Left turns crossing straight paths
            (Direction::North, Route::Left, Direction::South, Route::Straight) => true,
            (Direction::South, Route::Left, Direction::North, Route::Straight) => true,
            (Direction::East, Route::Left, Direction::West, Route::Straight) => true,
            (Direction::West, Route::Left, Direction::East, Route::Straight) => true,
            (Direction::South, Route::Straight, Direction::North, Route::Left) => true,
            (Direction::North, Route::Straight, Direction::South, Route::Left) => true,
            (Direction::West, Route::Straight, Direction::East, Route::Left) => true,
            (Direction::East, Route::Straight, Direction::West, Route::Left) => true,

            // Left turns crossing each other
            (Direction::North, Route::Left, Direction::South, Route::Left) => true,
            (Direction::East, Route::Left, Direction::West, Route::Left) => true,
            (Direction::South, Route::Left, Direction::North, Route::Left) => true,
            (Direction::West, Route::Left, Direction::East, Route::Left) => true,

            // Right turns crossing straight paths
            (Direction::North, Route::Right, Direction::West, Route::Straight) => true,
            (Direction::South, Route::Right, Direction::East, Route::Straight) => true,
            (Direction::East, Route::Right, Direction::North, Route::Straight) => true,
            (Direction::West, Route::Right, Direction::South, Route::Straight) => true,
            (Direction::West, Route::Straight, Direction::North, Route::Right) => true,
            (Direction::East, Route::Straight, Direction::South, Route::Right) => true,
            (Direction::North, Route::Straight, Direction::East, Route::Right) => true,
            (Direction::South, Route::Straight, Direction::West, Route::Right) => true,

            // Left turn crossing right turn (high conflict)
            (Direction::North, Route::Left, Direction::East, Route::Right) => true,
            (Direction::East, Route::Right, Direction::North, Route::Left) => true,
            (Direction::South, Route::Left, Direction::West, Route::Right) => true,
            (Direction::West, Route::Right, Direction::South, Route::Left) => true,
            (Direction::East, Route::Left, Direction::South, Route::Right) => true,
            (Direction::South, Route::Right, Direction::East, Route::Left) => true,
            (Direction::West, Route::Left, Direction::North, Route::Right) => true,
            (Direction::North, Route::Right, Direction::West, Route::Left) => true,

            // Same direction vehicles (no conflict unless lane change)
            (d1, _, d2, _) if d1 == d2 => false,

            // Parallel opposite directions without crossing (no conflict)
            (Direction::North, Route::Right, Direction::South, Route::Right) => false,
            (Direction::North, Route::Right, Direction::South, Route::Left) => false,
            (Direction::East, Route::Right, Direction::West, Route::Right) => false,
            (Direction::East, Route::Right, Direction::West, Route::Left) => false,

            // Default: no conflict for unspecified combinations
            _ => false,
        }
    }

    fn remove_completed_vehicles(&mut self) {
        let initial_count = self.vehicles.len();

        // Remove vehicles that have left the screen completely
        self.vehicles.retain(|vehicle| {
            // Keep vehicle if it's still on screen or in the process of exiting
            let keep = match vehicle.state {
                VehicleState::Completed => {
                    // Only remove if vehicle is far off screen
                    match vehicle.direction {
                        Direction::North => vehicle.position.y > -150,
                        Direction::South => vehicle.position.y < (crate::WINDOW_HEIGHT as i32 + 150),
                        Direction::East => vehicle.position.x < (crate::WINDOW_WIDTH as i32 + 150),
                        Direction::West => vehicle.position.x > -150,
                    }
                }
                _ => true, // Keep all non-completed vehicles
            };

            if !keep {
                println!("Vehicle {} has left the simulation area", vehicle.id);
            }

            keep
        });

        let removed = initial_count - self.vehicles.len();
        if removed > 0 {
            self.statistics.add_completed_vehicles(removed);
            println!("Removed {} vehicles that completed their journey", removed);
        }
    }

    fn render(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        // Clear screen
        canvas.set_draw_color(Color::RGB(50, 120, 50)); // Green background
        canvas.clear();

        // Draw roads
        self.draw_roads(canvas)?;

        // Draw intersection
        self.draw_intersection(canvas)?;

        // Draw vehicles
        for vehicle in &self.vehicles {
            self.draw_vehicle(canvas, vehicle)?;
        }

        // Draw UI
        self.draw_ui(canvas)?;

        canvas.present();
        Ok(())
    }

    fn draw_roads(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(80, 80, 80));

        let center_x = WINDOW_WIDTH as i32 / 2;
        let center_y = WINDOW_HEIGHT as i32 / 2;
        let road_width = 180;

        // Horizontal road
        canvas.fill_rect(Rect::new(
            0,
            center_y - road_width / 2,
            WINDOW_WIDTH,
            road_width as u32,
        ))?;

        // Vertical road
        canvas.fill_rect(Rect::new(
            center_x - road_width / 2,
            0,
            road_width as u32,
            WINDOW_HEIGHT,
        ))?;

        // Lane markings
        canvas.set_draw_color(Color::RGB(255, 255, 255));

        // Horizontal lane markings
        for i in 1..6 {
            let y = center_y - road_width / 2 + (i * 30);
            canvas.draw_line((0, y), (center_x - road_width / 2, y))?;
            canvas.draw_line((center_x + road_width / 2, y), (WINDOW_WIDTH as i32, y))?;
        }

        // Vertical lane markings
        for i in 1..6 {
            let x = center_x - road_width / 2 + (i * 30);
            canvas.draw_line((x, 0), (x, center_y - road_width / 2))?;
            canvas.draw_line((x, center_y + road_width / 2), (x, WINDOW_HEIGHT as i32))?;
        }

        Ok(())
    }

    fn draw_intersection(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(60, 60, 60));

        let center_x = WINDOW_WIDTH as i32 / 2;
        let center_y = WINDOW_HEIGHT as i32 / 2;
        let size = 180;

        canvas.fill_rect(Rect::new(
            center_x - size / 2,
            center_y - size / 2,
            size as u32,
            size as u32,
        ))?;

        // Intersection border
        canvas.set_draw_color(Color::RGB(255, 255, 0));
        canvas.draw_rect(Rect::new(
            center_x - size / 2,
            center_y - size / 2,
            size as u32,
            size as u32,
        ))?;

        Ok(())
    }

    fn draw_vehicle(&self, canvas: &mut Canvas<Window>, vehicle: &Vehicle) -> Result<(), String> {
        // Vehicle color based on route
        let color = match vehicle.route {
            Route::Left => Color::RGB(255, 100, 100),   // Red
            Route::Straight => Color::RGB(100, 100, 255), // Blue
            Route::Right => Color::RGB(100, 255, 100),   // Green
        };

        canvas.set_draw_color(color);

        let size = 24;
        let rect = Rect::new(
            vehicle.position.x - size / 2,
            vehicle.position.y - size / 2,
            size as u32,
            size as u32,
        );

        canvas.fill_rect(rect)?;

        // Vehicle border
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.draw_rect(rect)?;

        // Direction arrow
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        let arrow_size = 8;
        match vehicle.direction {
            Direction::North => {
                canvas.draw_line(
                    (vehicle.position.x, vehicle.position.y - arrow_size),
                    (vehicle.position.x - 4, vehicle.position.y),
                )?;
                canvas.draw_line(
                    (vehicle.position.x, vehicle.position.y - arrow_size),
                    (vehicle.position.x + 4, vehicle.position.y),
                )?;
            }
            Direction::South => {
                canvas.draw_line(
                    (vehicle.position.x, vehicle.position.y + arrow_size),
                    (vehicle.position.x - 4, vehicle.position.y),
                )?;
                canvas.draw_line(
                    (vehicle.position.x, vehicle.position.y + arrow_size),
                    (vehicle.position.x + 4, vehicle.position.y),
                )?;
            }
            Direction::East => {
                canvas.draw_line(
                    (vehicle.position.x + arrow_size, vehicle.position.y),
                    (vehicle.position.x, vehicle.position.y - 4),
                )?;
                canvas.draw_line(
                    (vehicle.position.x + arrow_size, vehicle.position.y),
                    (vehicle.position.x, vehicle.position.y + 4),
                )?;
            }
            Direction::West => {
                canvas.draw_line(
                    (vehicle.position.x - arrow_size, vehicle.position.y),
                    (vehicle.position.x, vehicle.position.y - 4),
                )?;
                canvas.draw_line(
                    (vehicle.position.x - arrow_size, vehicle.position.y),
                    (vehicle.position.x, vehicle.position.y + 4),
                )?;
            }
        }

        Ok(())
    }

    fn draw_ui(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        // Draw vehicle count
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.fill_rect(Rect::new(10, 10, 200, 60))?;

        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.draw_rect(Rect::new(10, 10, 200, 60))?;

        // Vehicle count indicators (dots)
        for i in 0..self.vehicles.len().min(20) {
            canvas.fill_rect(Rect::new(15 + (i as i32 * 9), 15, 6, 6))?;
        }

        // Direction indicators
        let directions = [Direction::North, Direction::South, Direction::East, Direction::West];
        let colors = [
            Color::RGB(255, 100, 100), // North - Red
            Color::RGB(100, 255, 100), // South - Green
            Color::RGB(100, 100, 255), // East - Blue
            Color::RGB(255, 255, 100), // West - Yellow
        ];

        for (i, (direction, color)) in directions.iter().zip(colors.iter()).enumerate() {
            let count = self.vehicles.iter().filter(|v| v.direction == *direction).count();
            canvas.set_draw_color(*color);

            for j in 0..count.min(10) {
                canvas.fill_rect(Rect::new(
                    15 + (j as i32 * 8),
                    25 + (i as i32 * 10),
                    6, 8
                ))?;
            }
        }

        Ok(())
    }

    fn show_statistics(&self) {
        println!("\n=== FINAL STATISTICS ===");
        println!("Total vehicles spawned: {}", self.statistics.total_vehicles_spawned);
        println!("Vehicles completed: {}", self.statistics.vehicles_completed);
        println!("Active vehicles: {}", self.vehicles.len());

        if self.statistics.total_vehicles_spawned > 0 {
            let completion_rate = (self.statistics.vehicles_completed as f64 /
                self.statistics.total_vehicles_spawned as f64) * 100.0;
            println!("Completion rate: {:.1}%", completion_rate);
        }

        println!("\nVehicles by direction:");
        for direction in [Direction::North, Direction::South, Direction::East, Direction::West] {
            let count = self.vehicles.iter().filter(|v| v.direction == direction).count();
            println!("  {:?}: {}", direction, count);
        }

        println!("\nVehicles by route:");
        for route in [Route::Left, Route::Straight, Route::Right] {
            let count = self.vehicles.iter().filter(|v| v.route == route).count();
            println!("  {:?}: {}", route, count);
        }

        println!("\nVehicles by state:");
        for state in [VehicleState::Approaching, VehicleState::Entering,
            VehicleState::Turning, VehicleState::Exiting, VehicleState::Completed] {
            let count = self.vehicles.iter().filter(|v| v.state == state).count();
            println!("  {:?}: {}", state, count);
        }

        if !self.vehicles.is_empty() {
            let velocities: Vec<f64> = self.vehicles.iter().map(|v| v.current_velocity).collect();
            let avg_velocity = velocities.iter().sum::<f64>() / velocities.len() as f64;
            let max_velocity = velocities.iter().fold(0.0f64, |a, &b| a.max(b));
            let min_velocity = velocities.iter().fold(f64::INFINITY, |a, &b| a.min(b));

            println!("\nVelocity statistics:");
            println!("  Average: {:.1} px/s", avg_velocity);
            println!("  Maximum: {:.1} px/s", max_velocity);
            println!("  Minimum: {:.1} px/s", min_velocity);
        }

        println!("\nIntersection statistics:");
        println!("  Max congestion: {} vehicles", self.statistics.max_congestion);
        println!("  Close calls: {}", self.statistics.close_calls);

        // Show global velocity stats
        println!("\nOverall velocity statistics:");
        println!("  Max velocity reached: {:.1} px/s", self.statistics.max_velocity);
        if self.statistics.min_velocity != f64::MAX {
            println!("  Min velocity reached: {:.1} px/s", self.statistics.min_velocity);
        }

        println!("=========================");
    }
}