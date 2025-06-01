use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::collections::VecDeque;

use crate::algorithm::SmartIntersection;
use crate::intersection::Intersection;
use crate::renderer::Renderer;
use crate::statistics::Statistics;
use crate::vehicle::{Direction, Route, Vehicle, VehicleState};

pub struct Game<'a> {
    canvas: Canvas<Window>,
    intersection: Intersection,
    vehicles: VecDeque<Vehicle>,
    algorithm: SmartIntersection,
    statistics: Statistics,
    renderer: Renderer<'a>,
    spawn_cooldown: f32,
    current_cooldown: f32,
    continuous_spawn: bool,
    spawn_timer: f32,
    debug_mode: bool,
    show_grid: bool,
    next_vehicle_id: u32,
}

impl<'a> Game<'a> {
    pub fn new(canvas: Canvas<Window>, renderer: Renderer<'a>) -> Result<Self, String> {
        let intersection = Intersection::new();
        let algorithm = SmartIntersection::new();
        let statistics = Statistics::new();

        Ok(Game {
            canvas,
            intersection,
            vehicles: VecDeque::new(),
            algorithm,
            statistics,
            renderer,
            spawn_cooldown: 2.0, // 2 seconds between manual spawns
            current_cooldown: 0.0,
            continuous_spawn: false,
            spawn_timer: 0.0,
            debug_mode: false,
            show_grid: false,
            next_vehicle_id: 0,
        })
    }

    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::KeyDown {
                keycode: Some(keycode),
                repeat: false,
                ..
            } => match keycode {
                Keycode::Up => {
                    // Spawn vehicle from South going North
                    if self.spawn_vehicle(Direction::North) {
                        println!("Spawned vehicle from South (moving North)");
                    } else {
                        println!("Could not spawn vehicle - cooldown active or spawn area blocked");
                    }
                }
                Keycode::Down => {
                    // Spawn vehicle from North going South
                    if self.spawn_vehicle(Direction::South) {
                        println!("Spawned vehicle from North (moving South)");
                    } else {
                        println!("Could not spawn vehicle - cooldown active or spawn area blocked");
                    }
                }
                Keycode::Left => {
                    // Spawn vehicle from East going West
                    if self.spawn_vehicle(Direction::East) {
                        println!("Spawned vehicle from East (moving West)");
                    } else {
                        println!("Could not spawn vehicle - cooldown active or spawn area blocked");
                    }
                }
                Keycode::Right => {
                    // Spawn vehicle from West going East
                    if self.spawn_vehicle(Direction::West) {
                        println!("Spawned vehicle from West (moving East)");
                    } else {
                        println!("Could not spawn vehicle - cooldown active or spawn area blocked");
                    }
                }
                Keycode::R => {
                    self.continuous_spawn = !self.continuous_spawn;
                    println!("Continuous spawn: {}", if self.continuous_spawn { "ON" } else { "OFF" });
                    if self.continuous_spawn {
                        self.spawn_timer = 0.0; // Reset timer
                    }
                }
                Keycode::D => {
                    self.debug_mode = !self.debug_mode;
                    println!("Debug mode: {}", if self.debug_mode { "ON" } else { "OFF" });
                }
                Keycode::G => {
                    self.show_grid = !self.show_grid;
                    println!("Grid overlay: {}", if self.show_grid { "ON" } else { "OFF" });
                }
                Keycode::Space => {
                    self.print_current_statistics();
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        // Ensure delta_time is reasonable to prevent large jumps
        let safe_delta = delta_time.min(1.0/30.0).max(1.0/120.0);

        // Update cooldowns
        if self.current_cooldown > 0.0 {
            self.current_cooldown -= safe_delta;
        }

        // Handle continuous spawning
        if self.continuous_spawn {
            self.spawn_timer += safe_delta;
            if self.spawn_timer >= 3.0 { // Spawn every 3 seconds
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let direction = match rng.gen_range(0..4) {
                    0 => Direction::North,
                    1 => Direction::South,
                    2 => Direction::East,
                    _ => Direction::West,
                };

                if self.spawn_vehicle(direction) {
                    if self.debug_mode {
                        println!("Auto-spawned vehicle moving {:?}", direction);
                    }
                }
                self.spawn_timer = 0.0;
            }
        }

        // Update smart intersection algorithm
        self.algorithm.process_vehicles(
            &mut self.vehicles,
            &self.intersection,
            (safe_delta * 1000.0) as u32
        );

        // Update individual vehicles
        for vehicle in &mut self.vehicles {
            vehicle.update((safe_delta * 1000.0) as u32, &self.intersection);
        }

        // Update statistics
        self.statistics.update(&self.vehicles);

        // Remove completed vehicles
        self.remove_completed_vehicles();

        // Debug output for vehicle count
        if self.debug_mode && self.vehicles.len() > 0 {
            let approaching = self.vehicles.iter().filter(|v| v.state == VehicleState::Approaching).count();
            let in_intersection = self.vehicles.iter().filter(|v|
                matches!(v.state, VehicleState::Entering | VehicleState::Turning | VehicleState::Exiting)
            ).count();

            if approaching + in_intersection > 0 {
                println!("Vehicles: {} approaching, {} in intersection, {} total",
                         approaching, in_intersection, self.vehicles.len());
            }
        }
    }

    pub fn render(&mut self) -> Result<(), String> {
        // Clear canvas
        use sdl2::pixels::Color;
        self.canvas.set_draw_color(Color::RGB(50, 120, 50)); // Green grass
        self.canvas.clear();

        // Render intersection
        self.renderer.render_intersection(&mut self.canvas, &self.intersection)?;

        // Render vehicles (only those visible on screen)
        for vehicle in &self.vehicles {
            if vehicle.is_on_screen() {
                self.renderer.render_vehicle(&mut self.canvas, vehicle)?;
            }
        }

        // Render debug information if enabled
        if self.debug_mode {
            self.render_debug_info()?;
        }

        // Present the frame
        self.canvas.present();
        Ok(())
    }

    fn spawn_vehicle(&mut self, direction: Direction) -> bool {
        if self.current_cooldown > 0.0 {
            return false;
        }

        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Choose random route with realistic distribution
        let route = match rng.gen_range(0..10) {
            0..=2 => Route::Left,     // 30% left turns
            3..=6 => Route::Straight, // 40% straight
            _ => Route::Right,        // 30% right turns
        };

        // Choose random lane appropriate for the route
        let lane = match route {
            Route::Left => rng.gen_range(0..2),     // Lanes 0-1 for left
            Route::Straight => rng.gen_range(2..4), // Lanes 2-3 for straight
            Route::Right => rng.gen_range(4..6),    // Lanes 4-5 for right
        };

        // Check if spawn is safe
        if self.can_spawn_safely(&direction, lane) {
            let vehicle = Vehicle::new(direction, lane, route);

            if self.debug_mode {
                println!(
                    "Spawned vehicle {}: {:?} in lane {}, route {:?}, pos=({}, {})",
                    vehicle.id, vehicle.direction, vehicle.lane, vehicle.route,
                    vehicle.position.x, vehicle.position.y
                );
            }

            self.vehicles.push_back(vehicle);
            self.current_cooldown = self.spawn_cooldown;
            return true;
        }

        false
    }

    fn can_spawn_safely(&self, direction: &Direction, lane: usize) -> bool {
        // Check if there are too many vehicles in the same direction
        let same_direction_count = self.vehicles.iter()
            .filter(|v| v.direction == *direction)
            .count();

        if same_direction_count >= 8 { // Limit vehicles per direction
            return false;
        }

        // Check for vehicles too close to spawn point
        let spawn_area_radius = 150.0;

        for vehicle in &self.vehicles {
            if vehicle.direction == *direction {
                let distance = vehicle.distance_from_spawn();
                if distance < spawn_area_radius {
                    return false;
                }
            }
        }

        true
    }

    // FIXED: Remove completed vehicles without cloning
    fn remove_completed_vehicles(&mut self) {
        let initial_count = self.vehicles.len();

        // Collect indices of vehicles to remove and their data for statistics
        let mut vehicles_to_remove = Vec::new();

        for (index, vehicle) in self.vehicles.iter().enumerate() {
            if vehicle.state == VehicleState::Completed ||
                vehicle.has_left_intersection(&self.intersection) ||
                !vehicle.is_on_screen() {

                // Store the vehicle data we need for statistics
                let vehicle_stats = VehicleStatistics {
                    id: vehicle.id,
                    direction: vehicle.direction,
                    lane: vehicle.lane,
                    route: vehicle.route,
                    current_velocity: vehicle.current_velocity,
                    time_in_intersection: vehicle.time_in_intersection,
                    start_time: vehicle.start_time,
                };

                vehicles_to_remove.push((index, vehicle_stats));
            }
        }

        // Remove vehicles from back to front to maintain correct indices
        for (index, vehicle_stats) in vehicles_to_remove.into_iter().rev() {
            // Remove the vehicle
            self.vehicles.remove(index);

            // Record statistics using the extracted data
            self.statistics.record_vehicle_exit_stats(vehicle_stats);
        }

        let removed_count = initial_count - self.vehicles.len();
        if removed_count > 0 && self.debug_mode {
            println!("Removed {} completed vehicles", removed_count);
        }
    }

    fn render_debug_info(&mut self) -> Result<(), String> {
        use sdl2::pixels::Color;
        use sdl2::rect::Rect;

        // Draw vehicle count info
        let bg_color = Color::RGB(0, 0, 0);

        // Background for text
        self.canvas.set_draw_color(bg_color);
        self.canvas.fill_rect(Rect::new(10, 10, 300, 120))?;

        // Note: For actual text rendering, you'd need SDL2_ttf
        // For now, just draw colored indicators

        // Vehicle count indicators (white dots)
        self.canvas.set_draw_color(Color::RGB(255, 255, 255));
        for i in 0..self.vehicles.len().min(25) {
            self.canvas.fill_rect(Rect::new(15 + (i as i32 * 10), 20, 6, 6))?;
        }

        // Direction indicators
        let directions = [Direction::North, Direction::South, Direction::East, Direction::West];
        let colors = [
            Color::RGB(255, 0, 0),   // North - Red
            Color::RGB(0, 255, 0),   // South - Green
            Color::RGB(0, 0, 255),   // East - Blue
            Color::RGB(255, 255, 0), // West - Yellow
        ];

        for (i, (direction, color)) in directions.iter().zip(colors.iter()).enumerate() {
            let count = self.vehicles.iter().filter(|v| v.direction == *direction).count();
            self.canvas.set_draw_color(*color);

            for j in 0..count.min(15) {
                self.canvas.fill_rect(Rect::new(
                    15 + (j as i32 * 8),
                    40 + (i as i32 * 18),
                    6, 12
                ))?;
            }
        }

        // Show algorithm statistics
        self.canvas.set_draw_color(Color::RGB(255, 255, 255));
        let close_calls = self.algorithm.close_calls;
        for i in 0..close_calls.min(10) {
            self.canvas.fill_rect(Rect::new(15 + (i as i32 * 12), 110, 8, 8))?;
        }

        Ok(())
    }

    fn print_current_statistics(&self) {
        println!("\n=== Current Statistics ===");
        println!("Active vehicles: {}", self.vehicles.len());

        // Count by state
        let approaching = self.vehicles.iter().filter(|v| v.state == VehicleState::Approaching).count();
        let entering = self.vehicles.iter().filter(|v| v.state == VehicleState::Entering).count();
        let turning = self.vehicles.iter().filter(|v| v.state == VehicleState::Turning).count();
        let exiting = self.vehicles.iter().filter(|v| v.state == VehicleState::Exiting).count();

        println!("Vehicle states:");
        println!("  Approaching: {}", approaching);
        println!("  Entering: {}", entering);
        println!("  Turning: {}", turning);
        println!("  Exiting: {}", exiting);

        // Count by direction
        println!("By direction:");
        for direction in [Direction::North, Direction::South, Direction::East, Direction::West] {
            let count = self.vehicles.iter().filter(|v| v.direction == direction).count();
            println!("  {:?}: {}", direction, count);
        }

        // Count by route
        println!("By route:");
        for route in [Route::Left, Route::Straight, Route::Right] {
            let count = self.vehicles.iter().filter(|v| v.route == route).count();
            println!("  {:?}: {}", route, count);
        }

        // Algorithm statistics
        println!("Algorithm stats:");
        println!("  Close calls: {}", self.algorithm.close_calls);

        // Vehicle velocity statistics
        if !self.vehicles.is_empty() {
            let velocities: Vec<f64> = self.vehicles.iter().map(|v| v.current_velocity).collect();
            let avg_velocity = velocities.iter().sum::<f64>() / velocities.len() as f64;
            let max_velocity = velocities.iter().fold(0.0f64, |a, &b| a.max(b));
            let min_velocity = velocities.iter().fold(f64::INFINITY, |a, &b| a.min(b));

            println!("Velocity stats:");
            println!("  Average: {:.1} px/s", avg_velocity);
            println!("  Maximum: {:.1} px/s", max_velocity);
            println!("  Minimum: {:.1} px/s", min_velocity);
        }

        println!("==========================\n");
    }

    pub fn show_statistics(&self) -> Result<(), String> {
        println!("\n=== Final Statistics ===");
        self.print_current_statistics();
        self.statistics.display()
    }
}

// ADDED: Structure to hold vehicle statistics without cloning the full vehicle
#[derive(Debug, Clone)]
pub struct VehicleStatistics {
    pub id: u32,
    pub direction: Direction,
    pub lane: usize,
    pub route: Route,
    pub current_velocity: f64,
    pub time_in_intersection: u32,
    pub start_time: std::time::Instant,
}