// src/vehicle.rs - FIXED: One-frame stop and turn system
use crate::intersection::Intersection;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North, // Moving from south to north (up)
    South, // Moving from north to south (down)
    East,  // Moving from west to east (right)
    West,  // Moving from east to west (left)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Route {
    Left,     // Will turn left at intersection
    Straight, // Will go straight through intersection
    Right,    // Will turn right at intersection
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VehicleState {
    Approaching, // Moving towards the intersection
    Entering,    // Just entered the intersection area
    AtTurnPoint, // Stopped at turn point (one frame stop)
    Turning,     // Currently turning (immediate direction change)
    Exiting,     // Leaving the intersection
    Completed,   // Has left the intersection
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VelocityLevel {
    Slow,
    Medium,
    Fast,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VehicleColor {
    Red,    // Left turn
    Blue,   // Straight
    Green,  // Right turn
    Yellow, // Special/emergency
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Vec2::new(self.x / len, self.y / len)
        } else {
            Vec2::new(0.0, 0.0)
        }
    }

    pub fn dot(&self, other: &Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Vec2::new(self.x - other.x, self.y - other.y)
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Vec2::new(self.x * scalar, self.y * scalar)
    }
}

impl std::ops::Div<f32> for Vec2 {
    type Output = Self;
    fn div(self, scalar: f32) -> Self {
        Vec2::new(self.x / scalar, self.y / scalar)
    }
}

pub struct Vehicle {
    pub id: u32,
    pub position: Vec2,
    pub direction: Direction,         // Where vehicle came from
    pub destination: Direction,       // Where vehicle is going to
    pub lane: usize,                 // Lane number (0=LEFT, 1=STRAIGHT, 2=RIGHT)
    pub route: Route,                // Left, Straight, Right
    pub color: VehicleColor,         // Visual color
    pub state: VehicleState,         // Current state
    pub velocity_level: VelocityLevel,
    pub current_velocity: f32,
    pub target_velocity: f32,
    pub width: f32,
    pub height: f32,
    pub start_time: Instant,
    pub time_in_intersection: u32,

    // RENDERING
    pub angle: f32,                  // Current angle (0, 90, 180, 270)

    // LANE POSITIONING
    pub target_lane_x: f32,          // Target X position for current lane
    pub target_lane_y: f32,          // Target Y position for current lane

    // INTERSECTION MANAGEMENT
    has_reserved_intersection: bool,
    has_completed_turn: bool,
    at_turn_point: bool,             // NEW: Flag for one-frame stop

    // PHYSICS
    velocity: Vec2,

    // DEBUG
    pub path_history: Vec<Vec2>,
    pub turning_path: Option<()>,    // For compatibility
}

impl Vehicle {
    pub const SLOW_VELOCITY: f32 = 25.0;
    pub const MEDIUM_VELOCITY: f32 = 40.0;
    pub const FAST_VELOCITY: f32 = 55.0;
    pub const SAFE_DISTANCE: f32 = 80.0;

    pub const WIDTH: f32 = 16.0;
    pub const HEIGHT: f32 = 16.0;

    pub fn new_simple(
        id: u32,
        incoming_direction: Direction,
        destination: Direction,
        lane: usize,
        route: Route,
        color: VehicleColor,
    ) -> Self {
        let (spawn_pos, target_lane_x, target_lane_y) =
            Self::calculate_perfect_spawn_position(incoming_direction, lane);

        use rand::Rng;
        let mut rng = rand::thread_rng();
        let velocity_level = match rng.gen_range(0..10) {
            0..=6 => VelocityLevel::Slow,
            7..=8 => VelocityLevel::Medium,
            _ => VelocityLevel::Fast,
        };

        let base_velocity = match velocity_level {
            VelocityLevel::Slow => Self::SLOW_VELOCITY,
            VelocityLevel::Medium => Self::MEDIUM_VELOCITY,
            VelocityLevel::Fast => Self::FAST_VELOCITY,
        };

        let variation = rng.gen_range(-3.0..3.0);
        let initial_velocity = (base_velocity + variation).max(15.0).min(Self::FAST_VELOCITY);

        let initial_angle = match incoming_direction {
            Direction::North => 0.0,
            Direction::East => 90.0,
            Direction::South => 180.0,
            Direction::West => 270.0,
        };

        let velocity_direction = match incoming_direction {
            Direction::North => Vec2::new(0.0, -1.0),
            Direction::South => Vec2::new(0.0, 1.0),
            Direction::East => Vec2::new(1.0, 0.0),
            Direction::West => Vec2::new(-1.0, 0.0),
        };

        Vehicle {
            id,
            position: spawn_pos,
            direction: incoming_direction,
            destination,
            lane,
            route,
            color,
            state: VehicleState::Approaching,
            velocity_level,
            current_velocity: initial_velocity,
            target_velocity: initial_velocity,
            width: Self::WIDTH,
            height: Self::HEIGHT,
            start_time: Instant::now(),
            time_in_intersection: 0,
            angle: initial_angle,
            target_lane_x,
            target_lane_y,
            has_reserved_intersection: false,
            has_completed_turn: false,
            at_turn_point: false,
            velocity: velocity_direction * initial_velocity,
            path_history: vec![spawn_pos],
            turning_path: None,
        }
    }

    fn calculate_perfect_spawn_position(direction: Direction, lane: usize) -> (Vec2, f32, f32) {
        let center_x = crate::WINDOW_WIDTH as f32 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f32 / 2.0;
        let lane_width = 30.0;
        let spawn_distance = 250.0;

        match direction {
            Direction::North => {
                let target_x = center_x + 15.0 + (lane as f32 * lane_width);
                let spawn_y = crate::WINDOW_HEIGHT as f32 + spawn_distance;
                (Vec2::new(target_x, spawn_y), target_x, center_y)
            }
            Direction::South => {
                let target_x = center_x - 15.0 - (lane as f32 * lane_width);
                let spawn_y = -spawn_distance;
                (Vec2::new(target_x, spawn_y), target_x, center_y)
            }
            Direction::East => {
                let target_y = center_y + 15.0 + (lane as f32 * lane_width);
                let spawn_x = -spawn_distance;
                (Vec2::new(spawn_x, target_y), center_x, target_y)
            }
            Direction::West => {
                let target_y = center_y - 15.0 - (lane as f32 * lane_width);
                let spawn_x = crate::WINDOW_WIDTH as f32 + spawn_distance;
                (Vec2::new(spawn_x, target_y), center_x, target_y)
            }
        }
    }

    // SIMPLIFIED: No complex state machine, just move and flip when needed
    pub fn update_physics(&mut self, dt: f64, intersection: &Intersection) {
        if self.is_in_intersection(intersection) {
            self.time_in_intersection += (dt * 1000.0) as u32;
        }

        self.adjust_velocity(dt);

        // Simple movement - always move toward current direction
        match self.state {
            VehicleState::Approaching | VehicleState::Entering | VehicleState::Exiting | VehicleState::Completed => {
                if self.state == VehicleState::Completed || self.state == VehicleState::Exiting {
                    // After turn, move toward destination
                    self.move_toward_direction(dt, self.destination);
                } else {
                    // Before turn, move toward original direction
                    self.move_toward_direction(dt, self.direction);
                }
            }
            _ => {
                // Keep old behavior for other states
                self.move_in_lane(dt);
            }
        }

        self.update_state(intersection);

        // Update path history
        if self.path_history.len() > 20 {
            self.path_history.remove(0);
        }
        self.path_history.push(self.position);
    }

    fn adjust_velocity(&mut self, dt: f64) {
        let velocity_diff = self.target_velocity - self.current_velocity;

        if velocity_diff.abs() < 0.5 {
            self.current_velocity = self.target_velocity;
        } else {
            let acceleration = if velocity_diff > 0.0 { 30.0 } else { -60.0 };
            self.current_velocity += acceleration * dt as f32;
            self.current_velocity = self.current_velocity.max(0.0).min(Self::FAST_VELOCITY);
        }
    }

    fn move_in_lane(&mut self, dt: f64) {
        let distance = self.current_velocity * dt as f32;

        match self.direction {
            Direction::North => {
                self.position.y -= distance;
                self.position.x = self.target_lane_x;
            }
            Direction::South => {
                self.position.y += distance;
                self.position.x = self.target_lane_x;
            }
            Direction::East => {
                self.position.x += distance;
                self.position.y = self.target_lane_y;
            }
            Direction::West => {
                self.position.x -= distance;
                self.position.y = self.target_lane_y;
            }
        }
    }

    // INSTANT 90 DEGREE FLIP - no stopping, just immediate turn
    fn execute_instant_90_flip(&mut self) {
        // Update direction immediately
        self.direction = self.destination;

        // Update angle immediately
        self.angle = match self.destination {
            Direction::North => 0.0,
            Direction::East => 90.0,
            Direction::South => 180.0,
            Direction::West => 270.0,
        };

        // Update target lane for new direction
        self.update_target_lane_for_destination();

        println!("🔄 Vehicle {} instant flip: {:?} → {:?} (angle: {:.1}°)",
                 self.id, self.route, self.destination, self.angle);
    }

    fn update_target_lane_for_destination(&mut self) {
        let center_x = crate::WINDOW_WIDTH as f32 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f32 / 2.0;
        let lane_width = 30.0;

        // Use middle lane for all post-turn traffic
        let destination_lane = 1;

        match self.destination {
            Direction::North => {
                self.target_lane_x = center_x + 15.0 + (destination_lane as f32 * lane_width);
                self.target_lane_y = center_y;
            }
            Direction::South => {
                self.target_lane_x = center_x - 15.0 - (destination_lane as f32 * lane_width);
                self.target_lane_y = center_y;
            }
            Direction::East => {
                self.target_lane_x = center_x;
                self.target_lane_y = center_y + 15.0 + (destination_lane as f32 * lane_width);
            }
            Direction::West => {
                self.target_lane_x = center_x;
                self.target_lane_y = center_y - 15.0 - (destination_lane as f32 * lane_width);
            }
        }
    }

    // SIMPLE: Move in any direction
    fn move_toward_direction(&mut self, dt: f64, direction: Direction) {
        let distance = self.current_velocity * dt as f32;

        match direction {
            Direction::North => {
                self.position.y -= distance;
                // Gradual lane correction if needed
                if (self.position.x - self.target_lane_x).abs() > 2.0 {
                    let diff_x = self.target_lane_x - self.position.x;
                    self.position.x += diff_x * 0.1;
                }
            }
            Direction::South => {
                self.position.y += distance;
                if (self.position.x - self.target_lane_x).abs() > 2.0 {
                    let diff_x = self.target_lane_x - self.position.x;
                    self.position.x += diff_x * 0.1;
                }
            }
            Direction::East => {
                self.position.x += distance;
                if (self.position.y - self.target_lane_y).abs() > 2.0 {
                    let diff_y = self.target_lane_y - self.position.y;
                    self.position.y += diff_y * 0.1;
                }
            }
            Direction::West => {
                self.position.x -= distance;
                if (self.position.y - self.target_lane_y).abs() > 2.0 {
                    let diff_y = self.target_lane_y - self.position.y;
                    self.position.y += diff_y * 0.1;
                }
            }
        }
    }

    fn update_state(&mut self, intersection: &Intersection) {
        let center_x = crate::WINDOW_WIDTH as f32 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f32 / 2.0;
        let intersection_radius = 90.0;

        let distance_to_center = ((self.position.x - center_x).powi(2) +
            (self.position.y - center_y).powi(2)).sqrt();

        match self.state {
            VehicleState::Approaching => {
                if distance_to_center < intersection_radius + 50.0 {
                    self.state = VehicleState::Entering;
                    println!("Vehicle {} ({:?}) entering intersection", self.id, self.route);
                }
            }
            VehicleState::Entering => {
                // When vehicle reaches CENTER of intersection, execute turn
                if distance_to_center < 30.0 { // Very close to center
                    if self.route == Route::Left || self.route == Route::Right {
                        // INSTANT 90 DEGREE FLIP
                        self.execute_instant_90_flip();
                        self.state = VehicleState::Exiting;
                        println!("Vehicle {} ({:?}) executed instant 90° flip", self.id, self.route);
                    } else {
                        self.state = VehicleState::Exiting;
                        println!("Vehicle {} ({:?}) going straight through", self.id, self.route);
                    }
                }
            }
            VehicleState::AtTurnPoint => {
                // Not used in this approach
            }
            VehicleState::Turning => {
                // Not used in this approach
            }
            VehicleState::Exiting => {
                if distance_to_center > intersection_radius + 80.0 {
                    self.state = VehicleState::Completed;
                    println!("Vehicle {} completed intersection", self.id);
                }
            }
            VehicleState::Completed => {
                // Continue moving until off screen
            }
        }
    }

    // Compatibility methods
    pub fn update_interpolation(&mut self, _alpha: f32) {
        // No interpolation needed for one-frame turns
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
        self.position.x >= -200.0 &&
            self.position.x <= (crate::WINDOW_WIDTH as f32 + 200.0) &&
            self.position.y >= -200.0 &&
            self.position.y <= (crate::WINDOW_HEIGHT as f32 + 200.0)
    }

    pub fn distance_from_spawn(&self) -> f64 {
        match self.direction {
            Direction::North => (crate::WINDOW_HEIGHT as f64 + 250.0) - self.position.y as f64,
            Direction::South => self.position.y as f64 + 250.0,
            Direction::East => self.position.x as f64 + 250.0,
            Direction::West => (crate::WINDOW_WIDTH as f64 + 250.0) - self.position.x as f64,
        }
    }

    pub fn is_approaching_intersection(&self, _intersection: &Intersection) -> bool {
        let center_x = crate::WINDOW_WIDTH as f32 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f32 / 2.0;
        let approach_distance = 120.0;

        let distance_to_center = ((self.position.x - center_x).powi(2) +
            (self.position.y - center_y).powi(2)).sqrt();

        distance_to_center < approach_distance + 50.0 && distance_to_center > 90.0
    }

    pub fn is_in_intersection(&self, intersection: &Intersection) -> bool {
        intersection.is_point_in_intersection(self.position.x as i32, self.position.y as i32)
    }

    pub fn time_to_intersection(&self, _intersection: &Intersection) -> f64 {
        if self.current_velocity <= 0.0 {
            return f64::INFINITY;
        }

        let center_x = crate::WINDOW_WIDTH as f32 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f32 / 2.0;

        let distance = match self.direction {
            Direction::North => (self.position.y - center_y).max(0.0),
            Direction::South => (center_y - self.position.y).max(0.0),
            Direction::East => (center_x - self.position.x).max(0.0),
            Direction::West => (self.position.x - center_x).max(0.0),
        };

        distance as f64 / self.current_velocity as f64
    }

    pub fn could_collide_with(&self, other: &Vehicle, intersection: &Intersection) -> bool {
        if self.id == other.id {
            return false;
        }

        let dx = self.position.x - other.position.x;
        let dy = self.position.y - other.position.y;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance < 30.0 {
            return true;
        }

        if self.direction == other.direction && self.lane == other.lane {
            return self.is_vehicle_ahead_in_same_lane(other);
        }

        if (self.is_approaching_intersection(intersection) || self.is_in_intersection(intersection)) &&
            (other.is_approaching_intersection(intersection) || other.is_in_intersection(intersection)) {
            return self.will_paths_intersect_simple(other);
        }

        false
    }

    fn is_vehicle_ahead_in_same_lane(&self, other: &Vehicle) -> bool {
        if self.direction != other.direction || self.lane != other.lane {
            return false;
        }

        let distance_ahead = match self.direction {
            Direction::North => other.position.y - self.position.y,
            Direction::South => self.position.y - other.position.y,
            Direction::East => other.position.x - self.position.x,
            Direction::West => self.position.x - other.position.x,
        };

        distance_ahead > 0.0 && distance_ahead < Self::SAFE_DISTANCE
    }

    fn will_paths_intersect_simple(&self, other: &Vehicle) -> bool {
        if self.direction == other.direction {
            return self.lane == other.lane;
        }

        if self.route == Route::Left || other.route == Route::Left {
            return match (self.direction, other.direction) {
                (Direction::North, Direction::East) | (Direction::East, Direction::North) => true,
                (Direction::South, Direction::West) | (Direction::West, Direction::South) => true,
                (Direction::East, Direction::South) | (Direction::South, Direction::East) => true,
                (Direction::West, Direction::North) | (Direction::North, Direction::West) => true,
                _ => false,
            };
        }

        false
    }

    pub fn has_intersection_reservation(&self) -> bool {
        self.has_reserved_intersection
    }

    pub fn set_intersection_reservation(&mut self, reserved: bool) {
        self.has_reserved_intersection = reserved;
    }

    pub fn get_current_movement_direction(&self) -> Direction {
        // After exiting intersection, use destination direction
        // Before/during intersection, use original direction
        let current_dir = if self.state == VehicleState::Exiting || self.state == VehicleState::Completed {
            self.destination
        } else {
            self.direction
        };

        current_dir
    }

    pub fn interpolated_position(&self) -> Vec2 {
        self.position
    }

    pub fn interpolated_angle(&self) -> f32 {
        self.angle
    }
}