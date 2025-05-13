use crate::intersection::{intersection_center, Intersection, LANE_WIDTH};
use sdl2::rect::Point;
use std::sync::atomic::{AtomicU32, Ordering};

// Global atomic counter for vehicle IDs
static NEXT_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

// New struct for Bezier curve-based turning
struct TurningPath {
    start_point: (f64, f64),
    control_point: (f64, f64),
    end_point: (f64, f64),
    progress: f64,  // 0.0 to 1.0
}

// New struct for physics-based movement
pub struct VehiclePhysics {
    max_acceleration: f64,      // Maximum acceleration rate (pixels/second²)
    max_deceleration: f64,      // Maximum deceleration/braking rate (pixels/second²)
    current_acceleration: f64,  // Current acceleration
    mass: f64,                  // Vehicle mass affects acceleration
    drag_coefficient: f64,      // Air resistance factor
    engine_power: f64,          // Power factor - affects acceleration curve
}

pub struct Vehicle {
    pub id: u32,
    pub position: Point, // Current position
    position_f: (f64, f64), // For calculation - more precise
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
    turning_path: Option<TurningPath>, // Bezier curve path for turning
    physics: VehiclePhysics, // Physics properties for realistic movement
}

impl Vehicle {
    pub const SLOW_VELOCITY: f64 = 50.0; // pixels per second
    pub const MEDIUM_VELOCITY: f64 = 100.0;
    pub const FAST_VELOCITY: f64 = 150.0;
    pub const SAFE_DISTANCE: f64 = 50.0; // pixels

    // Vehicle dimensions
    pub const WIDTH: u32 = 40;
    pub const HEIGHT: u32 = 80;

    pub fn new(direction: Direction, route: Route) -> Self {
        // Get a unique ID using the atomic counter
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);

        // Set initial position based on direction
        let (pos_x, pos_y) = match direction {
            Direction::North => {
                let x = intersection_center().0 - (LANE_WIDTH as i32 / 2);
                (x as f64, 0.0)
            },
            Direction::South => {
                let x = intersection_center().0 + (LANE_WIDTH as i32 / 2);
                (x as f64, crate::WINDOW_HEIGHT as f64)
            },
            Direction::East => {
                let y = intersection_center().1 - (LANE_WIDTH as i32 / 2);
                (crate::WINDOW_WIDTH as f64, y as f64)
            },
            Direction::West => {
                let y = intersection_center().1 + (LANE_WIDTH as i32 / 2);
                (0.0, y as f64)
            },
        };

        // Create the Point for rendering
        let position = Point::new(pos_x as i32, pos_y as i32);

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

        // Initialize physics properties with slight randomization for variety
        let physics = VehiclePhysics {
            max_acceleration: rng.gen_range(25.0..35.0),  // pixels/second²
            max_deceleration: rng.gen_range(50.0..70.0),  // braking is stronger than acceleration
            current_acceleration: 0.0,
            mass: rng.gen_range(800.0..2000.0),  // kg (affects acceleration)
            drag_coefficient: rng.gen_range(0.25..0.35), // Air resistance factor
            engine_power: rng.gen_range(90.0..110.0),    // Power factor (percentage of efficiency)
        };

        Vehicle {
            id,
            position,
            position_f: (pos_x, pos_y),
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
            turning_path: None,
            physics,
        }
    }

    // Update the vehicle's position and state
    pub fn update(&mut self, delta_time: u32, intersection: &Intersection) {
        let dt = delta_time as f64 / 1000.0; // Convert to seconds

        // Debug print
        println!("Vehicle {} update: dt={}, velocity={}, position=({},{}), position_f=({},{})",
                 self.id, dt, self.current_velocity, self.position.x, self.position.y,
                 self.position_f.0, self.position_f.1);

        // Calculate appropriate acceleration based on target velocity
        self.calculate_acceleration();

        // Apply acceleration to velocity (with physics simulation)
        self.apply_physics(dt);

        // Check if vehicle is turning via bezier curve
        if self.state == VehicleState::Turning && self.turning_path.is_some() {
            // Update position along bezier curve
            self.update_position_along_curve(dt);
        } else {
            // Move vehicle based on current direction and velocity
            let distance = self.current_velocity * dt;
            match self.direction {
                Direction::North => {
                    self.position_f.1 -= distance;
                }
                Direction::South => {
                    self.position_f.1 += distance;
                }
                Direction::East => {
                    self.position_f.0 -= distance;
                }
                Direction::West => {
                    self.position_f.0 += distance;
                }
            }

            // Update integer position for rendering
            self.position = Point::new(self.position_f.0.round() as i32, self.position_f.1.round() as i32);
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

    // Calculate appropriate acceleration based on target velocity and current conditions
    fn calculate_acceleration(&mut self) {
        let velocity_diff = self.target_velocity - self.current_velocity;

        if velocity_diff.abs() < 1.0 {
            // Close enough to target, stabilize
            self.physics.current_acceleration = 0.0;
            self.current_velocity = self.target_velocity;
        } else if velocity_diff > 0.0 {
            // Need to accelerate
            // Apply gradual acceleration with diminishing returns as we approach max speed
            let acceleration_factor = 1.0 - (self.current_velocity / Vehicle::FAST_VELOCITY).min(0.9);

            // Engine power affects acceleration capability (percentage efficiency)
            let power_factor = self.physics.engine_power / 100.0;

            self.physics.current_acceleration = self.physics.max_acceleration * acceleration_factor * power_factor;
        } else {
            // Need to decelerate
            // Braking force increases as speed increases (air resistance + mechanical braking)
            let braking_factor = 0.5 + (self.current_velocity / Vehicle::FAST_VELOCITY).min(0.5);
            self.physics.current_acceleration = -self.physics.max_deceleration * braking_factor;
        }
    }

    // Apply physics calculations to update velocity
    fn apply_physics(&mut self, dt: f64) {
        // Calculate air resistance (drag increases with square of velocity)
        let air_resistance = self.physics.drag_coefficient * self.current_velocity * self.current_velocity * 0.01;

        // Apply air resistance as a negative acceleration
        let drag_deceleration = if self.current_velocity > 0.0 { -air_resistance } else { 0.0 };

        // Calculate net acceleration (including air resistance)
        let net_acceleration = self.physics.current_acceleration + drag_deceleration;

        // Apply mass factor (F = ma, so a = F/m)
        let mass_factor = 1000.0 / self.physics.mass; // Normalize to a reasonable range
        let effective_acceleration = net_acceleration * mass_factor;

        // Apply acceleration to velocity
        self.current_velocity += effective_acceleration * dt;

        // Ensure velocity stays within bounds
        self.current_velocity = self.current_velocity.max(0.0).min(Vehicle::FAST_VELOCITY);

        // Debug physics info
        println!("Vehicle {} physics: target={}, current={}, accel={}, drag={}, mass={}",
                 self.id, self.target_velocity, self.current_velocity,
                 self.physics.current_acceleration, drag_deceleration, self.physics.mass);
    }

    // Use quadratic Bezier curve for smooth turning
    fn update_position_along_curve(&mut self, dt: f64) {
        if let Some(path) = &mut self.turning_path {
            // Increment progress based on velocity and time
            path.progress += (self.current_velocity / 300.0) * dt;
            path.progress = path.progress.min(1.0);

            // Calculate position using Bezier formula
            let t = path.progress;
            let x = (1.0-t)*(1.0-t)*path.start_point.0 +
                2.0*(1.0-t)*t*path.control_point.0 +
                t*t*path.end_point.0;

            let y = (1.0-t)*(1.0-t)*path.start_point.1 +
                2.0*(1.0-t)*t*path.control_point.1 +
                t*t*path.end_point.1;

            // Update position
            self.position_f = (x, y);
            self.position = Point::new(x as i32, y as i32);

            // Calculate tangent angle for realistic orientation
            let dx = 2.0*(1.0-t)*(path.control_point.0 - path.start_point.0) +
                2.0*t*(path.end_point.0 - path.control_point.0);

            let dy = 2.0*(1.0-t)*(path.control_point.1 - path.start_point.1) +
                2.0*t*(path.end_point.1 - path.control_point.1);

            // Set angle based on tangent direction (if defined)
            if dx != 0.0 || dy != 0.0 {
                self.angle = (dy.atan2(dx) * 180.0 / std::f64::consts::PI + 90.0) % 360.0;
            }

            // Check if we've completed the turn
            if path.progress >= 1.0 {
                self.state = VehicleState::Exiting;
                self.complete_turning();
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
                        self.initialize_turning_path();
                    }
                }
            }
            VehicleState::Turning => {
                // If going straight or no turning path, check if we've completed the turn
                if self.route == Route::Straight || self.turning_path.is_none() {
                    if self.has_completed_turn() {
                        self.state = VehicleState::Exiting;
                        self.complete_turning();
                    } else {
                        // Legacy turning logic (should be unused with Bezier curves active)
                        self.continue_turning();
                    }
                }
                // If using Bezier curves, update_position_along_curve handles state transition
            }
            VehicleState::Exiting => {
                if self.has_left_intersection(intersection) {
                    self.state = VehicleState::Completed;
                }
            }
            VehicleState::Completed => {}
        }
    }

    // Initialize the Bezier curve turning path based on direction and route
    fn initialize_turning_path(&mut self) {
        let center = intersection_center();
        let center_x = center.0 as f64;
        let center_y = center.1 as f64;
        let lane_offset = LANE_WIDTH as f64 / 2.0;

        // Create turning path based on direction and route
        match (self.direction, self.route) {
            // Left turns (90° counterclockwise)
            (Direction::North, Route::Left) => {
                self.turning_path = Some(TurningPath {
                    start_point: (center_x - lane_offset, center_y - lane_offset),
                    control_point: (center_x - lane_offset * 2.0, center_y - lane_offset * 2.0),
                    end_point: (center_x - lane_offset * 3.0, center_y),
                    progress: 0.0,
                });
            }
            (Direction::South, Route::Left) => {
                self.turning_path = Some(TurningPath {
                    start_point: (center_x + lane_offset, center_y + lane_offset),
                    control_point: (center_x + lane_offset * 2.0, center_y + lane_offset * 2.0),
                    end_point: (center_x + lane_offset * 3.0, center_y),
                    progress: 0.0,
                });
            }
            (Direction::East, Route::Left) => {
                self.turning_path = Some(TurningPath {
                    start_point: (center_x + lane_offset, center_y - lane_offset),
                    control_point: (center_x + lane_offset * 2.0, center_y - lane_offset * 2.0),
                    end_point: (center_x, center_y - lane_offset * 3.0),
                    progress: 0.0,
                });
            }
            (Direction::West, Route::Left) => {
                self.turning_path = Some(TurningPath {
                    start_point: (center_x - lane_offset, center_y + lane_offset),
                    control_point: (center_x - lane_offset * 2.0, center_y + lane_offset * 2.0),
                    end_point: (center_x, center_y + lane_offset * 3.0),
                    progress: 0.0,
                });
            }

            // Right turns (90° clockwise)
            (Direction::North, Route::Right) => {
                self.turning_path = Some(TurningPath {
                    start_point: (center_x - lane_offset, center_y - lane_offset),
                    control_point: (center_x - lane_offset * 0.5, center_y - lane_offset * 2.0),
                    end_point: (center_x, center_y - lane_offset * 3.0),
                    progress: 0.0,
                });
            }
            (Direction::South, Route::Right) => {
                self.turning_path = Some(TurningPath {
                    start_point: (center_x + lane_offset, center_y + lane_offset),
                    control_point: (center_x + lane_offset * 0.5, center_y + lane_offset * 2.0),
                    end_point: (center_x, center_y + lane_offset * 3.0),
                    progress: 0.0,
                });
            }
            (Direction::East, Route::Right) => {
                self.turning_path = Some(TurningPath {
                    start_point: (center_x + lane_offset, center_y - lane_offset),
                    control_point: (center_x + lane_offset * 2.0, center_y - lane_offset * 0.5),
                    end_point: (center_x + lane_offset * 3.0, center_y),
                    progress: 0.0,
                });
            }
            (Direction::West, Route::Right) => {
                self.turning_path = Some(TurningPath {
                    start_point: (center_x - lane_offset, center_y + lane_offset),
                    control_point: (center_x - lane_offset * 2.0, center_y + lane_offset * 0.5),
                    end_point: (center_x - lane_offset * 3.0, center_y),
                    progress: 0.0,
                });
            }

            // Straight paths (no curve needed)
            (_, Route::Straight) => {
                self.turning_path = None;
            }
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

    // Initialize turning (legacy method kept for compatibility)
    fn start_turning(&mut self) {
        // This method is now mostly replaced by initialize_turning_path
        // Kept for compatibility
    }

    // Continue turning process (legacy method - will be used if turning_path is None)
    fn continue_turning(&mut self) {
        // This method is now mostly replaced by update_position_along_curve
        // But we keep it for backward compatibility and for straight paths

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
        // If using Bezier curve, check progress
        if let Some(path) = &self.turning_path {
            return path.progress >= 1.0;
        }

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
        // Clear turning path
        self.turning_path = None;

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

        // Calculate time based on current velocity and acceleration
        if self.current_velocity > 0.0 {
            // Simple time calculation for constant velocity
            let time_at_current_speed = distance / self.current_velocity;

            // If accelerating/decelerating, adjust the time estimate
            if self.physics.current_acceleration.abs() > 0.001 {
                // Quadratic formula: d = v₀t + ½at²
                // Solve for t: at² + 2v₀t - 2d = 0
                let a = self.physics.current_acceleration;
                let b = 2.0 * self.current_velocity;
                let c = -2.0 * distance;

                if a.abs() > 0.001 {
                    let discriminant = b * b - 4.0 * a * c;
                    if discriminant >= 0.0 {
                        let t1 = (-b + discriminant.sqrt()) / (2.0 * a);
                        let t2 = (-b - discriminant.sqrt()) / (2.0 * a);

                        // Return the positive time
                        if t1 > 0.0 {
                            return t1;
                        } else if t2 > 0.0 {
                            return t2;
                        }
                    }
                }
            }

            return time_at_current_speed;
        } else {
            return f64::MAX; // Avoid division by zero
        }
    }
}