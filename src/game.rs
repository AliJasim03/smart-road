// src/game.rs - Updated to use simple block renderer
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::collections::VecDeque;

use crate::smart_algorithm::SmartIntersectionManager;
use crate::intersection::Intersection;
use crate::simple_renderer::SimpleRenderer;
use crate::statistics::Statistics;
use crate::vehicle::{Direction, Route, Vehicle, VehicleState};

const GRID_SIZE: i32 = 32;

pub struct Game<'a> {
    canvas: Canvas<Window>,
    intersection: Intersection,
    vehicles: VecDeque<Vehicle>,
    smart_manager: SmartIntersectionManager,
    statistics: Statistics,
    renderer: SimpleRenderer<'a>,
    spawn_cooldown: f32,
    current_cooldown: f32,
    continuous_spawn: bool,
    spawn_timer: f32,
    // Debug features
    debug_mode: bool,
    show_grid: bool,
}

impl<'a> Game<'a> {
    pub fn new(canvas: Canvas<Window>, renderer: SimpleRenderer<'a>) -> Result<Self, String> {
        let intersection = Intersection::new();
        let smart_manager = SmartIntersectionManager::new(crate::WINDOW_WIDTH, crate::WINDOW_HEIGHT);
        let statistics = Statistics::new();

        Ok(Game {
            canvas,
            intersection,
            vehicles: VecDeque::new(),
            smart_manager,
            statistics,
            renderer,
            spawn_cooldown: 1.5, // 1.5 seconds between spawns
            current_cooldown: 0.0,
            continuous_spawn: false,
            spawn_timer: 0.0,
            debug_mode: false,
            show_grid: true, // Start with grid visible
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
                    if self.spawn_vehicle(Direction::North) {
                        println!("Spawned vehicle moving North");
                    } else {
                        println!("Could not spawn vehicle moving North");
                    }
                }
                Keycode::Down => {
                    if self.spawn_vehicle(Direction::South) {
                        println!("Spawned vehicle moving South");
                    } else {
                        println!("Could not spawn vehicle moving South");
                    }
                }
                Keycode::Left => {
                    // Note: Left arrow spawns vehicles moving from East to West
                    if self.spawn_vehicle(Direction::East) {
                        println!("Spawned vehicle moving East (West-bound)");
                    } else {
                        println!("Could not spawn vehicle moving East");
                    }
                }
                Keycode::Right => {
                    // Note: Right arrow spawns vehicles moving from West to East
                    if self.spawn_vehicle(Direction::West) {
                        println!("Spawned vehicle moving West (East-bound)");
                    } else {
                        println!("Could not spawn vehicle moving West");
                    }
                }
                Keycode::R => {
                    self.continuous_spawn = !self.continuous_spawn;
                    println!("Continuous spawn: {}", if self.continuous_spawn { "ON" } else { "OFF" });
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
                    let (processed, avg_wait, reservations, pending) = self.smart_manager.get_statistics();
                    println!("=== Statistics ===");
                    println!("Vehicles processed: {}", processed);
                    println!("Average wait time: {:.2}s", avg_wait);
                    println!("Active reservations: {}", reservations);
                    println!("Pending requests: {}", pending);
                    println!("Current vehicles: {}", self.vehicles.len());

                    // Vehicle state breakdown
                    let approaching = self.vehicles.iter().filter(|v| v.state == VehicleState::Approaching).count();
                    let entering = self.vehicles.iter().filter(|v| v.state == VehicleState::Entering).count();
                    let turning = self.vehicles.iter().filter(|v| v.state == VehicleState::Turning).count();
                    let exiting = self.vehicles.iter().filter(|v| v.state == VehicleState::Exiting).count();

                    println!("Vehicle states - Approaching: {}, Entering: {}, Turning: {}, Exiting: {}",
                             approaching, entering, turning, exiting);
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        // Update cooldowns
        if self.current_cooldown > 0.0 {
            self.current_cooldown -= delta_time;
        }

        // Handle continuous spawning
        if self.continuous_spawn {
            self.spawn_timer += delta_time;
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
                    println!("Auto-spawned vehicle moving {:?}", direction);
                }
                self.spawn_timer = 0.0;
            }
        }

        // Update the smart intersection manager
        self.smart_manager.update(&mut self.vehicles, delta_time);

        // Update individual vehicles
        for vehicle in &mut self.vehicles {
            vehicle.update((delta_time * 1000.0) as u32, &self.intersection);
        }

        // Update statistics
        self.statistics.update(&self.vehicles);

        // Remove completed vehicles and record statistics
        let mut vehicles_to_remove = Vec::new();

        // First, identify vehicles to remove and collect their indices
        for (index, vehicle) in self.vehicles.iter().enumerate() {
            if vehicle.state == VehicleState::Completed || vehicle.has_left_intersection(&self.intersection) {
                vehicles_to_remove.push(index);
            }
        }

        // Remove vehicles from back to front to maintain indices
        for &index in vehicles_to_remove.iter().rev() {
            if let Some(vehicle) = self.vehicles.remove(index) {
                let vehicle_id = vehicle.id; // Get ID before moving the vehicle
                self.statistics.record_vehicle_exit(vehicle);
                if self.debug_mode {
                    println!("Vehicle {} completed intersection", vehicle_id);
                }
            }
        }
    }

    pub fn render(&mut self) -> Result<(), String> {
        // Use the simple renderer
        self.renderer.render(&mut self.canvas, &self.intersection, &self.vehicles, self.show_grid)?;

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

        // Choose random route
        let route = match rng.gen_range(0..3) {
            0 => Route::Left,
            1 => Route::Straight,
            _ => Route::Right,
        };

        // Choose random lane (0-5 for 6 lanes)
        let lane = rng.gen_range(0..6);

        // Check if there's space to spawn
        if self.can_spawn_vehicle_safely(&direction, lane) {
            let mut new_vehicle = Vehicle::new(direction, lane, route);

            // Set proper grid-aligned spawn position
            let spawn_pos = self.calculate_grid_aligned_spawn_position(&direction, lane);
            new_vehicle.position.x = spawn_pos.0;
            new_vehicle.position.y = spawn_pos.1;

            println!(
                "Vehicle spawned: id={}, direction={:?}, lane={}, route={:?}, pos=({}, {})",
                new_vehicle.id, new_vehicle.direction, lane, new_vehicle.route,
                new_vehicle.position.x, new_vehicle.position.y
            );

            self.vehicles.push_back(new_vehicle);
            self.current_cooldown = self.spawn_cooldown;
            return true;
        }

        false
    }

    fn calculate_grid_aligned_spawn_position(&self, direction: &Direction, lane: usize) -> (i32, i32) {
        let grid_width = (crate::WINDOW_WIDTH as i32) / GRID_SIZE;
        let grid_height = (crate::WINDOW_HEIGHT as i32) / GRID_SIZE;
        let center_x = grid_width / 2;
        let center_y = grid_height / 2;
        let road_width = 6; // 6 lanes

        match direction {
            Direction::North => {
                // Vehicles moving north spawn at bottom of screen
                let grid_x = center_x - road_width/2 + (lane as i32);
                let grid_y = grid_height - 2; // Near bottom of screen
                (grid_x * GRID_SIZE + GRID_SIZE/2, grid_y * GRID_SIZE + GRID_SIZE/2)
            }
            Direction::South => {
                // Vehicles moving south spawn at top of screen
                let grid_x = center_x + road_width/2 - 1 - (lane as i32);
                let grid_y = 1; // Near top of screen
                (grid_x * GRID_SIZE + GRID_SIZE/2, grid_y * GRID_SIZE + GRID_SIZE/2)
            }
            Direction::East => {
                // Vehicles moving east spawn at left of screen
                let grid_x = 1; // Near left of screen
                let grid_y = center_y - road_width/2 + (lane as i32);
                (grid_x * GRID_SIZE + GRID_SIZE/2, grid_y * GRID_SIZE + GRID_SIZE/2)
            }
            Direction::West => {
                // Vehicles moving west spawn at right of screen
                let grid_x = grid_width - 2; // Near right of screen
                let grid_y = center_y + road_width/2 - 1 - (lane as i32);
                (grid_x * GRID_SIZE + GRID_SIZE/2, grid_y * GRID_SIZE + GRID_SIZE/2)
            }
        }
    }

    fn can_spawn_vehicle_safely(&self, direction: &Direction, lane: usize) -> bool {
        let spawn_pos = self.calculate_grid_aligned_spawn_position(direction, lane);

        // Check if any existing vehicle is too close to spawn position
        const MIN_SPAWN_DISTANCE: f64 = 96.0; // 3 grid cells

        for vehicle in &self.vehicles {
            if vehicle.direction == *direction {
                let distance = {
                    let dx = (spawn_pos.0 - vehicle.position.x) as f64;
                    let dy = (spawn_pos.1 - vehicle.position.y) as f64;
                    (dx * dx + dy * dy).sqrt()
                };

                if distance < MIN_SPAWN_DISTANCE {
                    return false;
                }
            }
        }

        true
    }

    pub fn show_statistics(&self) -> Result<(), String> {
        self.statistics.display()
    }
}