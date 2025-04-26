use crate::intersection::{intersection_center, Intersection, LANE_WIDTH};
use sdl2::rect::Point;
use std::sync::atomic::{AtomicU32, Ordering};

// Global atomic counter for vehicle IDs
static NEXT_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    North, // Moving from south to north
    South, // Moving from north to south
    East,  // Moving from west to east
    West,  // Moving from east to west
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
    pub position: Point, // Current position
    pub direction: Direction,
    pub route: Route,
    pub state: VehicleState,
    pub velocity_level: VelocityLevel,
    pub current_velocity: f64, // Current velocity in pixels per second
    pub target_velocity: f64,  // Target velocity in pixels per second
    pub width: u32,
    pub height: u32,
    pub angle: f64, // Current rotation angle in degrees
    pub time_in_intersection: u32, // Time spent within the intersection in milliseconds
    pub start_time: std::time::Instant, // When the vehicle was created
    pub entry_time: Option<std::time::Instant>, // When the vehicle entered the intersection
}

impl Vehicle {
    pub const SLOW_VELOCITY: f64 = 50.0; // pixels per second
    pub const MEDIUM_VELOCITY: f64 = 100.0;
    pub const FAST_VELOCITY: f64 = 150.0;
    pub const SAFE_DISTANCE: f64 = 50.0; // pixels

    // Vehicle dimensions
    pub const WIDTH: u32 = 30;
    pub const HEIGHT: u32 = 60;

    pub fn new(direction: Direction, route: Route) -> Self {
        // Get a unique ID using the atomic counter
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);

        // Set initial position based on direction
        let position = match direction {
            Direction::North => Point::new(
                intersection_center().0 - (LANE_WIDTH as i32 / 2),
                0,
            ),
            Direction::South => Point::new(
                intersection_center().0 + (LANE_WIDTH as i32 / 2),
                crate::WINDOW_HEIGHT as i32,
            ),
            Direction::East => Point::new(
                crate::WINDOW_WIDTH as i32,
                intersection_center().1 - (LANE_WIDTH as i32 / 2),
            ),
            Direction::West => Point::new(
                0,
                intersection_center().1 + (LANE_WIDTH as i32 / 2),
            ),
        };

        // Set initial angle based on direction
        let angle = match direction {
            Direction::North => 0.0,   // Facing up
            Direction::South => 180.0, // Facing down
            Direction::East => 270.0,  // Facing left
            Direction::West => 90.0,   // Facing right
        };

        // Choose a random velocity level
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let velocity_level = match rng.gen_range(0..3) {
            0 => VelocityLevel::Slow,
            1 => VelocityLevel::Medium,
            _ => VelocityLevel::Fast,
        };

        let initial_velocity = match velocity_level {
            VelocityLevel::Slow => Self::SLOW_VELOCITY,
            VelocityLevel::Medium => Self::MEDIUM_VELOCITY,
            VelocityLevel::Fast => Self::FAST_VELOCITY,
        };

        Vehicle {
            id,
            position,
            direction,
            route,
            state: VehicleState::Approaching,
            velocity_level,
            current_velocity: initial_velocity,
            target_velocity: initial_velocity,
            width: Self::WIDTH,
            height: Self::HEIGHT,
            angle,
            time_in_intersection: 0,
            start_time: std::time::Instant::now(),
            entry_time: None,
        }
    }

    // Update the vehicle's position and state
    pub fn update(&mut self, delta_time: u32, intersection: &Intersection) {
        let dt = delta_time as f64 / 1000.0; // Convert to seconds

        // Adjust velocity towards target_velocity
        if (self.current_velocity - self.target_velocity).abs() > 1.0 {
            let direction = if self.current_velocity < self.target_velocity { 1.0 } else { -1.0 };
            self.current_velocity += direction * 50.0 * dt; // Acceleration/deceleration rate

            // Ensure we don't overshoot
            if direction > 0.0 && self.current_velocity > self.target_velocity {
                self.current_velocity = self.target_velocity;
            } else if direction < 0.0 && self.current_velocity < self.target_velocity {
                self.current_velocity = self.target_velocity;
            }
        } else {
            self.current_velocity = self.target_velocity;
        }

        // Move vehicle based on current direction and velocity
        let distance = self.current_velocity * dt;
        match self.direction {
            Direction::North => {
                self.position = Point::new(self.position.x, self.position.y - distance as i32);
            }
            Direction::South => {
                self.position = Point::new(self.position.x, self.position.y + distance as i32);
            }
            Direction::East => {
                self.position = Point::new(self.position.x - distance as i32, self.position.y);
            }
            Direction::West => {
                self.position = Point::new(self.position.x + distance as i32, self.position.y);
            }
        }

        // Update state based on position relative to intersection
        self.update_state(intersection);

        // Update time in intersection
        if self.state == VehicleState::Entering || self.state == VehicleState::Turning || self.state == VehicleState::Exiting {
            self.time_in_intersection += delta_time;

            // Record entry time if we just entered
            if self.state == VehicleState::Entering && self.entry_time.is_none() {
                self.entry_time = Some(std::time::Instant::now());
            }
        }
    }

    // Update the vehicle's state based on its position
    fn update_state(&mut self, intersection: &Intersection) {
        match self.state {
            VehicleState::Approaching => {
                if self.has_entered_intersection(intersection) {
                    self.state = VehicleState::Entering;
                }
            }
            VehicleState::Entering => {
                if self.is_in_turning_area() {
                    self.state = VehicleState::Turning;
                    // Start turning if not going straight
                    if self.route != Route::Straight {
                        self.start_turning();
                    }
                }
            }
            VehicleState::Turning => {
                if self.has_completed_turn() {
                    self.state = VehicleState::Exiting;
                    self.complete_turning();
                } else {
                    self.continue_turning();
                }
            }
            VehicleState::Exiting => {
                if self.has_left_intersection(intersection) {
                    self.state = VehicleState::Completed;
                }
            }
            VehicleState::Completed => {}
        }
    }

    // Check if vehicle has entered the intersection area
    pub fn has_entered_intersection(&self, intersection: &Intersection) -> bool {
        match self.direction {
            Direction::North => self.position.y <= intersection.north_entry,
            Direction::South => self.position.y >= intersection.south_entry,
            Direction::East => self.position.x <= intersection.east_entry,
            Direction::West => self.position.x >= intersection.west_entry,
        }
    }

    // Check if vehicle has left the intersection area
    pub fn has_left_intersection(&self, intersection: &Intersection) -> bool {
        match self.get_exit_direction() {
            Direction::North => self.position.y <= intersection.north_exit,
            Direction::South => self.position.y >= intersection.south_exit,
            Direction::East => self.position.x >= intersection.east_exit,
            Direction::West => self.position.x <= intersection.west_exit,
        }
    }

    // Get the direction the vehicle will exit from
    fn get_exit_direction(&self) -> Direction {
        match (self.direction, self.route) {
            (Direction::North, Route::Left) => Direction::West,
            (Direction::North, Route::Straight) => Direction::South,
            (Direction::North, Route::Right) => Direction::East,
            (Direction::South, Route::Left) => Direction::East,
            (Direction::South, Route::Straight) => Direction::North,
            (Direction::South, Route::Right) => Direction::West,
            (Direction::East, Route::Left) => Direction::North,
            (Direction::East, Route::Straight) => Direction::West,
            (Direction::East, Route::Right) => Direction::South,
            (Direction::West, Route::Left) => Direction::South,
            (Direction::West, Route::Straight) => Direction::East,
            (Direction::West, Route::Right) => Direction::North,
        }
    }

    // Check if vehicle is close to the center of the intersection
    fn is_in_turning_area(&self) -> bool {
        let center = intersection_center();
        let center_x = center.0;
        let center_y = center.1;
        let distance_squared =
            (self.position.x - center_x).pow(2) +
                (self.position.y - center_y).pow(2);

        // Check if within a certain radius of the center
        distance_squared < (LANE_WIDTH as i32 * 2).pow(2)
    }

    // Initialize turning
    fn start_turning(&mut self) {
        // Initialize turning logic based on direction and route
        // This will be expanded with actual turning logic
    }

    // Continue turning process
    fn continue_turning(&mut self) {
        // This will be expanded with actual turning logic

        // For now, just update angle based on route and direction
        match (self.direction, self.route) {
            // Left turn: 90 degrees counterclockwise
            (Direction::North, Route::Left) => {
                self.angle = (self.angle - 1.0) % 360.0;
                if self.angle < 0.0 { self.angle += 360.0; }
            }
            (Direction::South, Route::Left) => {
                self.angle = (self.angle - 1.0) % 360.0;
                if self.angle < 0.0 { self.angle += 360.0; }
            }
            (Direction::East, Route::Left) => {
                self.angle = (self.angle - 1.0) % 360.0;
                if self.angle < 0.0 { self.angle += 360.0; }
            }
            (Direction::West, Route::Left) => {
                self.angle = (self.angle - 1.0) % 360.0;
                if self.angle < 0.0 { self.angle += 360.0; }
            }

            // Right turn: 90 degrees clockwise
            (Direction::North, Route::Right) => {
                self.angle = (self.angle + 1.0) % 360.0;
            }
            (Direction::South, Route::Right) => {
                self.angle = (self.angle + 1.0) % 360.0;
            }
            (Direction::East, Route::Right) => {
                self.angle = (self.angle + 1.0) % 360.0;
            }
            (Direction::West, Route::Right) => {
                self.angle = (self.angle + 1.0) % 360.0;
            }

            // Straight: no change in angle
            _ => {}
        }
    }

    // Check if turning is completed
    fn has_completed_turn(&self) -> bool {
        // For simple implementation, consider turn completed when
        // angle matches the expected final angle for the route
        match (self.direction, self.route) {
            // Left turns should end at these angles
            (Direction::North, Route::Left) => (self.angle - 270.0).abs() < 5.0,
            (Direction::South, Route::Left) => (self.angle - 90.0).abs() < 5.0,
            (Direction::East, Route::Left) => (self.angle - 0.0).abs() < 5.0 || (self.angle - 360.0).abs() < 5.0,
            (Direction::West, Route::Left) => (self.angle - 180.0).abs() < 5.0,

            // Right turns should end at these angles
            (Direction::North, Route::Right) => (self.angle - 90.0).abs() < 5.0,
            (Direction::South, Route::Right) => (self.angle - 270.0).abs() < 5.0,
            (Direction::East, Route::Right) => (self.angle - 180.0).abs() < 5.0,
            (Direction::West, Route::Right) => (self.angle - 0.0).abs() < 5.0 || (self.angle - 360.0).abs() < 5.0,

            // Straight routes don't need to complete a turn
            (_, Route::Straight) => true,
        }
    }

    // Finalize the turning process
    fn complete_turning(&mut self) {
        // Set the final direction based on the turn
        self.direction = self.get_exit_direction();

        // Set the final angle based on the new direction
        self.angle = match self.direction {
            Direction::North => 0.0,
            Direction::South => 180.0,
            Direction::East => 90.0,
            Direction::West => 270.0,
        };
    }

    // Get the distance from the spawn point
    pub fn distance_from_spawn(&self) -> f64 {
        match self.direction {
            Direction::North => self.position.y as f64,
            Direction::South => (crate::WINDOW_HEIGHT as i32 - self.position.y) as f64,
            Direction::East => (crate::WINDOW_WIDTH as i32 - self.position.x) as f64,
            Direction::West => self.position.x as f64,
        }
    }

    // Set the target velocity of the vehicle
    pub fn set_target_velocity(&mut self, level: VelocityLevel) {
        self.velocity_level = level;
        self.target_velocity = match level {
            VelocityLevel::Slow => Self::SLOW_VELOCITY,
            VelocityLevel::Medium => Self::MEDIUM_VELOCITY,
            VelocityLevel::Fast => Self::FAST_VELOCITY,
        };
    }

    // Check if this vehicle could collide with another vehicle
    pub fn could_collide_with(&self, other: &Vehicle, intersection: &Intersection) -> bool {
        // Only check vehicles that are approaching or in the intersection
        if self.state == VehicleState::Completed || other.state == VehicleState::Completed {
            return false;
        }

        // Get the paths for both vehicles
        let path1 = intersection.get_path(&self.direction, &self.route);
        let path2 = intersection.get_path(&other.direction, &other.route);

        // Check if paths could collide
        intersection.paths_could_collide(&path1, &path2)
    }

    // Calculate time to intersection for vehicles approaching the intersection
    pub fn time_to_intersection(&self, intersection: &Intersection) -> f64 {
        if self.state != VehicleState::Approaching {
            return 0.0;
        }

        // Calculate distance to intersection entry point
        let distance = match self.direction {
            Direction::North => (self.position.y - intersection.north_entry).abs() as f64,
            Direction::South => (self.position.y - intersection.south_entry).abs() as f64,
            Direction::East => (self.position.x - intersection.east_entry).abs() as f64,
            Direction::West => (self.position.x - intersection.west_entry).abs() as f64,
        };

        // Calculate time based on current velocity
        if self.current_velocity > 0.0 {
            distance / self.current_velocity
        } else {
            f64::MAX // Avoid division by zero
        }
    }
}