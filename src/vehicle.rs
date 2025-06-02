// src/vehicle.rs - FIXED AND SIMPLIFIED VERSION
use crate::intersection::Intersection;
use sdl2::rect::Point;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North, // Moving from south to north (up)
    South, // Moving from north to south (down)
    East,  // Moving from west to east (right)
    West,  // Moving from east to west (left)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Route {
    Left,     // Will turn left at intersection
    Straight, // Will go straight through intersection
    Right,    // Will turn right at intersection
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VehicleState {
    Approaching, // Moving towards the intersection
    Entering,    // Just entered the intersection area
    Turning,     // Currently turning within the intersection
    Exiting,     // Leaving the intersection
    Completed,   // Has left the intersection
}

#[derive(Debug, Clone, Copy)]
pub enum VelocityLevel {
    Slow,
    Medium,
    Fast,
}

pub struct Vehicle {
    pub id: u32,
    pub position: Point,
    position_f: (f64, f64), // For smooth movement calculations
    pub direction: Direction,
    pub lane: usize,
    pub route: Route,
    pub state: VehicleState,
    pub velocity_level: VelocityLevel,
    pub current_velocity: f64,
    pub target_velocity: f64,
    pub width: u32,
    pub height: u32,
    pub start_time: std::time::Instant,
    turning_progress: f64, // 0.0 to 1.0 for turning animation
}

impl Vehicle {
    // Velocity constants
    pub const SLOW_VELOCITY: f64 = 30.0;   // pixels per second
    pub const MEDIUM_VELOCITY: f64 = 60.0; // pixels per second
    pub const FAST_VELOCITY: f64 = 90.0;   // pixels per second
    pub const SAFE_DISTANCE: f64 = 60.0;   // pixels

    // Vehicle dimensions
    pub const WIDTH: u32 = 24;
    pub const HEIGHT: u32 = 24;

    pub fn new(direction: Direction, lane: usize, route: Route) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);

        // Calculate spawn position based on direction
        let (spawn_x, spawn_y) = Self::calculate_spawn_position(direction, lane);

        // Random initial velocity - ensure all 3 speeds are used
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let velocity_level = match rng.gen_range(0..3) {
            0 => VelocityLevel::Slow,
            1 => VelocityLevel::Medium,
            2 => VelocityLevel::Fast,
            _ => VelocityLevel::Medium, // fallback
        };

        let initial_velocity = match velocity_level {
            VelocityLevel::Slow => Self::SLOW_VELOCITY,
            VelocityLevel::Medium => Self::MEDIUM_VELOCITY,
            VelocityLevel::Fast => Self::FAST_VELOCITY,
        };

        Vehicle {
            id,
            position: Point::new(spawn_x as i32, spawn_y as i32),
            position_f: (spawn_x, spawn_y),
            direction,
            lane: lane.min(5), // Ensure lane is valid (0-5)
            route,
            state: VehicleState::Approaching,
            velocity_level,
            current_velocity: initial_velocity,
            target_velocity: initial_velocity,
            width: Self::WIDTH,
            height: Self::HEIGHT,
            start_time: std::time::Instant::now(),
            turning_progress: 0.0,
        }
    }

    fn calculate_spawn_position(direction: Direction, lane: usize) -> (f64, f64) {
        let center_x = crate::WINDOW_WIDTH as f64 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f64 / 2.0;
        let road_width = 180.0;
        let lane_width = 30.0;

        // Calculate lane offset from center of road
        let lane_offset = (lane as f64 - 2.5) * lane_width; // Center lanes around middle

        match direction {
            Direction::North => {
                // Spawn from bottom, moving up
                let x = center_x + lane_offset;
                let y = crate::WINDOW_HEIGHT as f64 + 50.0; // Start below screen
                (x, y)
            }
            Direction::South => {
                // Spawn from top, moving down
                let x = center_x - lane_offset; // Reverse for oncoming traffic
                let y = -50.0; // Start above screen
                (x, y)
            }
            Direction::East => {
                // Spawn from left, moving right
                let x = -50.0; // Start left of screen
                let y = center_y + lane_offset;
                (x, y)
            }
            Direction::West => {
                // Spawn from right, moving left
                let x = crate::WINDOW_WIDTH as f64 + 50.0; // Start right of screen
                let y = center_y - lane_offset; // Reverse for oncoming traffic
                (x, y)
            }
        }
    }

    pub fn update(&mut self, delta_time: u32, intersection: &Intersection) {
        let dt = delta_time as f64 / 1000.0; // Convert to seconds

        // Smooth velocity adjustment
        self.adjust_velocity(dt);

        // Update position based on state
        match self.state {
            VehicleState::Approaching | VehicleState::Entering => {
                self.move_straight(dt);
            }
            VehicleState::Turning => {
                self.move_turning(dt);
            }
            VehicleState::Exiting | VehicleState::Completed => {
                // Continue moving to get off screen
                self.move_straight(dt);
            }
        }

        // Update state based on position
        self.update_state(intersection);

        // Update integer position for rendering
        self.position = Point::new(self.position_f.0 as i32, self.position_f.1 as i32);
    }

    fn adjust_velocity(&mut self, dt: f64) {
        let velocity_diff = self.target_velocity - self.current_velocity;

        if velocity_diff.abs() < 1.0 {
            self.current_velocity = self.target_velocity;
        } else {
            // Smooth acceleration/deceleration
            let acceleration = if velocity_diff > 0.0 { 30.0 } else { -50.0 };
            self.current_velocity += acceleration * dt;
            self.current_velocity = self.current_velocity.max(0.0).min(Self::FAST_VELOCITY);
        }
    }

    fn move_straight(&mut self, dt: f64) {
        let distance = self.current_velocity * dt;

        match self.direction {
            Direction::North => self.position_f.1 -= distance,
            Direction::South => self.position_f.1 += distance,
            Direction::East => self.position_f.0 += distance,
            Direction::West => self.position_f.0 -= distance,
        }
    }

    fn move_turning(&mut self, dt: f64) {
        // During turning, move at half speed and follow a curved path
        let turn_speed = self.current_velocity * 0.6; // Slower during turns
        let distance = turn_speed * dt;

        // Update turning progress
        self.turning_progress += dt * 1.5; // Complete turn in ~0.67 seconds

        if self.turning_progress < 0.5 {
            // First half of turn - continue in original direction but slower
            match self.direction {
                Direction::North => self.position_f.1 -= distance,
                Direction::South => self.position_f.1 += distance,
                Direction::East => self.position_f.0 += distance,
                Direction::West => self.position_f.0 -= distance,
            }
        } else {
            // Second half of turn - move in new direction
            let new_direction = self.get_turn_direction();
            match new_direction {
                Direction::North => self.position_f.1 -= distance,
                Direction::South => self.position_f.1 += distance,
                Direction::East => self.position_f.0 += distance,
                Direction::West => self.position_f.0 -= distance,
            }
        }

        if self.turning_progress >= 1.0 {
            self.complete_turn();
        }
    }

    fn get_turn_direction(&self) -> Direction {
        match (self.direction, self.route) {
            (Direction::North, Route::Left) => Direction::West,
            (Direction::North, Route::Right) => Direction::East,
            (Direction::South, Route::Left) => Direction::East,
            (Direction::South, Route::Right) => Direction::West,
            (Direction::East, Route::Left) => Direction::North,
            (Direction::East, Route::Right) => Direction::South,
            (Direction::West, Route::Left) => Direction::South,
            (Direction::West, Route::Right) => Direction::North,
            (_, Route::Straight) => self.direction, // No direction change
        }
    }

    fn complete_turn(&mut self) {
        // Update direction after turn
        self.direction = self.get_turn_direction();
        self.state = VehicleState::Exiting;
        self.turning_progress = 0.0;
    }

    fn update_state(&mut self, _intersection: &Intersection) {
        let center_x = crate::WINDOW_WIDTH as i32 / 2;
        let center_y = crate::WINDOW_HEIGHT as i32 / 2;
        let intersection_radius = 120; // Radius of intersection area

        let distance_to_center = (
            (self.position.x - center_x).pow(2) +
                (self.position.y - center_y).pow(2)
        ) as f64;
        let distance_to_center = distance_to_center.sqrt();

        match self.state {
            VehicleState::Approaching => {
                if distance_to_center < intersection_radius as f64 {
                    self.state = VehicleState::Entering;
                }
            }
            VehicleState::Entering => {
                if distance_to_center < 60.0 {
                    if self.route != Route::Straight {
                        self.state = VehicleState::Turning;
                    } else {
                        self.state = VehicleState::Exiting;
                    }
                }
            }
            VehicleState::Turning => {
                // Handled in move_turning method
            }
            VehicleState::Exiting => {
                if distance_to_center > intersection_radius as f64 + 50.0 {
                    self.state = VehicleState::Completed;
                }
            }
            VehicleState::Completed => {
                // Continue moving until off screen
            }
        }
    }

    pub fn set_target_velocity(&mut self, level: VelocityLevel) {
        self.velocity_level = level;
        self.target_velocity = match level {
            VelocityLevel::Slow => Self::SLOW_VELOCITY,
            VelocityLevel::Medium => Self::MEDIUM_VELOCITY,
            VelocityLevel::Fast => Self::FAST_VELOCITY,
        };
    }

    pub fn is_on_screen(&self) -> bool {
        self.position.x >= -100 &&
            self.position.x <= (crate::WINDOW_WIDTH as i32 + 100) &&
            self.position.y >= -100 &&
            self.position.y <= (crate::WINDOW_HEIGHT as i32 + 100)
    }

    pub fn is_approaching_intersection(&self, intersection: &Intersection) -> bool {
        let center_x = crate::WINDOW_WIDTH as i32 / 2;
        let center_y = crate::WINDOW_HEIGHT as i32 / 2;

        // Check if vehicle is within approach distance
        let approach_distance = 150;

        match self.direction {
            Direction::North => {
                self.position.y > center_y &&
                    self.position.y < center_y + approach_distance &&
                    (self.position.x - center_x).abs() < 100
            }
            Direction::South => {
                self.position.y < center_y &&
                    self.position.y > center_y - approach_distance &&
                    (self.position.x - center_x).abs() < 100
            }
            Direction::East => {
                self.position.x < center_x &&
                    self.position.x > center_x - approach_distance &&
                    (self.position.y - center_y).abs() < 100
            }
            Direction::West => {
                self.position.x > center_x &&
                    self.position.x < center_x + approach_distance &&
                    (self.position.y - center_y).abs() < 100
            }
        }
    }

    pub fn is_in_intersection(&self, intersection: &Intersection) -> bool {
        let center_x = crate::WINDOW_WIDTH as i32 / 2;
        let center_y = crate::WINDOW_HEIGHT as i32 / 2;

        let distance = (
            (self.position.x - center_x).pow(2) +
                (self.position.y - center_y).pow(2)
        ) as f64;

        distance.sqrt() < 120.0 // Within intersection radius
    }
}