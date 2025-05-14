use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::collections::VecDeque;

use crate::algorithm::SmartIntersection;
use crate::input::InputHandler;
use crate::intersection::{Intersection, lane_route, LaneRoute};
use crate::renderer::Renderer;
use crate::statistics::Statistics;
use crate::vehicle::{Direction, Route, Vehicle};

pub struct Game<'a> {
    canvas: Canvas<Window>,
    intersection: Intersection,
    vehicles: VecDeque<Vehicle>,
    smart_algorithm: SmartIntersection,
    statistics: Statistics,
    input_handler: InputHandler,
    renderer: Renderer<'a>,
    continuous_spawn: bool,
    spawn_cooldown: u32,
    current_cooldown: u32,
    // Track lane congestion
    lane_cooldowns: [[u32; 6]; 4], // [direction][lane]
}

impl<'a> Game<'a> {
    pub fn new(canvas: Canvas<Window>, renderer: Renderer<'a>) -> Result<Self, String> {
        let intersection = Intersection::new();
        let smart_algorithm = SmartIntersection::new();
        let statistics = Statistics::new();
        let input_handler = InputHandler::new();

        Ok(Game {
            canvas,
            intersection,
            vehicles: VecDeque::new(),
            smart_algorithm,
            statistics,
            input_handler,
            renderer,
            continuous_spawn: false,
            spawn_cooldown: 1000, // 1 second between spawns to prevent spamming
            current_cooldown: 0,
            lane_cooldowns: [[0; 6]; 4], // Initialize all lane cooldowns to 0
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
                    println!("Continuous spawn toggled: {}", self.continuous_spawn);
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub fn render(&mut self) -> Result<(), String> {
        // Clear the screen
        self.canvas.set_draw_color(Color::RGB(100, 100, 100)); // Gray background
        self.canvas.clear();

        // Render the intersection
        self.renderer
            .render_intersection(&mut self.canvas, &self.intersection)?;

        // Render all vehicles
        for vehicle in &self.vehicles {
            self.renderer.render_vehicle(&mut self.canvas, vehicle)?;
        }

        // Present the frame
        self.canvas.present();

        Ok(())
    }

    pub fn update(&mut self, delta_time: u32) {
        // Update spawn cooldown
        if self.current_cooldown > 0 {
            self.current_cooldown = self.current_cooldown.saturating_sub(delta_time);
        }

        // Update lane cooldowns
        for direction in 0..4 {
            for lane in 0..6 {
                if self.lane_cooldowns[direction][lane] > 0 {
                    self.lane_cooldowns[direction][lane] =
                        self.lane_cooldowns[direction][lane].saturating_sub(delta_time);
                }
            }
        }

        // Handle continuous spawning
        if self.continuous_spawn && self.current_cooldown == 0 {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let random_direction = match rng.gen_range(0..4) {
                0 => Direction::North,
                1 => Direction::South,
                2 => Direction::East,
                _ => Direction::West,
            };
            self.spawn_vehicle(random_direction);
            self.current_cooldown = self.spawn_cooldown / 2; // Faster spawning in continuous mode
        }

        // Update all vehicles
        self.smart_algorithm
            .process_vehicles(&mut self.vehicles, &self.intersection, delta_time);

        // Update statistics
        self.statistics.update(&self.vehicles);

        // Remove vehicles that have left the intersection
        while let Some(vehicle) = self.vehicles.front() {
            if vehicle.has_left_intersection(&self.intersection) {
                self.statistics
                    .record_vehicle_exit(self.vehicles.pop_front().unwrap());
            } else {
                break;
            }
        }
    }

    pub fn show_statistics(&self) -> Result<(), String> {
        self.statistics.display()
    }

    fn spawn_vehicle(&mut self, direction: Direction) {
        if self.current_cooldown > 0 {
            return;
        }

        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Find an available lane
        let direction_index = match direction {
            Direction::North => 0,
            Direction::South => 1,
            Direction::East => 2,
            Direction::West => 3,
        };

        // Try to find an available lane
        let mut available_lanes: Vec<usize> = (0..6)
            .filter(|&lane| self.lane_cooldowns[direction_index][lane] == 0)
            .collect();

        if available_lanes.is_empty() {
            println!("No available lanes for direction {:?}", direction);
            return;
        }

        // Choose a random available lane
        let lane_idx = if available_lanes.len() > 1 {
            available_lanes[rng.gen_range(0..available_lanes.len())]
        } else {
            available_lanes[0]
        };

        // Determine valid routes for this lane
        let lane_route_options = lane_route(lane_idx);
        let route = match lane_route_options {
            LaneRoute::Left => Route::Left,
            LaneRoute::Right => Route::Right,
            LaneRoute::Straight => Route::Straight,
            LaneRoute::LeftStraight => {
                if rng.gen_bool(0.5) { Route::Left } else { Route::Straight }
            },
            LaneRoute::StraightRight => {
                if rng.gen_bool(0.5) { Route::Straight } else { Route::Right }
            },
            LaneRoute::Any => {
                match rng.gen_range(0..3) {
                    0 => Route::Left,
                    1 => Route::Straight,
                    _ => Route::Right,
                }
            },
        };

        // Check if there's enough space to spawn a new vehicle in this lane
        if self.can_spawn_vehicle_in_lane(&direction, lane_idx) {
            let new_vehicle = Vehicle::new(direction, lane_idx, route);
            println!(
                "Vehicle spawned: id={}, direction={:?}, lane={}, route={:?}",
                new_vehicle.id, new_vehicle.direction, lane_idx, new_vehicle.route
            );
            self.vehicles.push_back(new_vehicle);

            // Set cooldowns
            self.current_cooldown = self.spawn_cooldown;
            self.lane_cooldowns[direction_index][lane_idx] = self.spawn_cooldown * 3; // Longer cooldown for specific lane
        } else {
            println!("Can't spawn vehicle: not enough space in lane {}", lane_idx);
        }
    }

    fn can_spawn_vehicle_in_lane(&self, direction: &Direction, lane: usize) -> bool {
        // Check if there's enough space to spawn a new vehicle in this specific lane
        for vehicle in &self.vehicles {
            if &vehicle.direction == direction && vehicle.lane == lane &&
                vehicle.distance_from_spawn() < Vehicle::SAFE_DISTANCE * 2.0 {
                return false;
            }
        }
        true
    }
}