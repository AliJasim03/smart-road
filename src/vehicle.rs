// src/vehicle.rs - COMPLETELY NEW: Perfect lane positioning mathematics
use crate::intersection::Intersection;
use sdl2::rect::Point;
use std::time::Instant;

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

// PERFECT MATHEMATICS CONSTANTS
const LANE_WIDTH: i32 = 30;           // Must match main.rs
const SPAWN_DISTANCE: i32 = 200;

pub struct Vehicle {
    pub id: u32,
    pub position: Point,
    position_f: (f64, f64),
    pub direction: Direction,    // Where vehicle is coming from
    pub destination: Direction,  // Where vehicle is going to
    pub lane: usize,            // Lane number (0, 1, 2)
    pub route: Route,           // Left, Straight, Right
    pub color: VehicleColor,    // Visual color
    pub state: VehicleState,    // Current state
    pub velocity_level: VelocityLevel,
    pub current_velocity: f64,
    pub target_velocity: f64,
    pub width: u32,
    pub height: u32,
    pub start_time: Instant,
    pub time_in_intersection: u32,
    turning_progress: f64,
    pub angle: f64,
    current_movement_direction: Direction,
    has_reserved_intersection: bool,
    target_lane_x: i32,  // Target X position for this vehicle's lane
    target_lane_y: i32,  // Target Y position for this vehicle's lane
}

impl Vehicle {
    // Conservative velocity constants
    pub const SLOW_VELOCITY: f64 = 20.0;
    pub const MEDIUM_VELOCITY: f64 = 35.0;
    pub const FAST_VELOCITY: f64 = 50.0;
    pub const SAFE_DISTANCE: f64 = 80.0;

    pub const WIDTH: u32 = 16;
    pub const HEIGHT: u32 = 16;

    // PERFECT: Create vehicle with exact mathematics
    pub fn new_perfect(
        id: u32,
        incoming_direction: Direction,
        destination: Direction,
        lane: usize,
        route: Route,
        color: VehicleColor,
    ) -> Self {
        let (spawn_x, spawn_y, target_lane_x, target_lane_y) =
            Self::calculate_perfect_positions(incoming_direction, lane);

        use rand::Rng;
        let mut rng = rand::thread_rng();
        let velocity_level = match rng.gen_range(0..10) {
            0..=6 => VelocityLevel::Slow,    // 70% slow
            7..=8 => VelocityLevel::Medium,  // 20% medium
            _ => VelocityLevel::Fast,        // 10% fast
        };

        let base_velocity = match velocity_level {
            VelocityLevel::Slow => Self::SLOW_VELOCITY,
            VelocityLevel::Medium => Self::MEDIUM_VELOCITY,
            VelocityLevel::Fast => Self::FAST_VELOCITY,
        };

        let variation = rng.gen_range(-3.0..3.0);
        let initial_velocity = (base_velocity + variation).max(10.0).min(Self::FAST_VELOCITY);

        let initial_angle = match incoming_direction {
            Direction::North => 0.0,
            Direction::East => 90.0,
            Direction::South => 180.0,
            Direction::West => 270.0,
        };

        Vehicle {
            id,
            position: Point::new(spawn_x, spawn_y),
            position_f: (spawn_x as f64, spawn_y as f64),
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
            turning_progress: 0.0,
            angle: initial_angle,
            current_movement_direction: incoming_direction,
            has_reserved_intersection: false,
            target_lane_x,
            target_lane_y,
        }
    }

    // PERFECT: Calculate exact spawn and target positions
    fn calculate_perfect_positions(direction: Direction, lane: usize) -> (i32, i32, i32, i32) {
        let center_x = crate::WINDOW_WIDTH as i32 / 2;
        let center_y = crate::WINDOW_HEIGHT as i32 / 2;

        match direction {
            Direction::North => {
                // Coming from South, moving North
                // Right side of vertical road: lanes 0, 1, 2 from left to right
                let target_x = center_x + 15 + (lane as i32 * LANE_WIDTH); // 527, 557, 587
                let spawn_y = crate::WINDOW_HEIGHT as i32 + SPAWN_DISTANCE;
                (target_x, spawn_y, target_x, center_y)
            }
            Direction::South => {
                // Coming from North, moving South
                // Left side of vertical road: lanes 0, 1, 2 from right to left
                let target_x = center_x - 15 - (lane as i32 * LANE_WIDTH); // 497, 467, 437
                let spawn_y = -SPAWN_DISTANCE;
                (target_x, spawn_y, target_x, center_y)
            }
            Direction::East => {
                // Coming from West, moving East
                // Bottom side of horizontal road: lanes 0, 1, 2 from top to bottom
                let target_y = center_y + 15 + (lane as i32 * LANE_WIDTH); // 399, 429, 459
                let spawn_x = -SPAWN_DISTANCE;
                (spawn_x, target_y, center_x, target_y)
            }
            Direction::West => {
                // Coming from East, moving West
                // Top side of horizontal road: lanes 0, 1, 2 from bottom to top
                let target_y = center_y - 15 - (lane as i32 * LANE_WIDTH); // 369, 339, 309
                let spawn_x = crate::WINDOW_WIDTH as i32 + SPAWN_DISTANCE;
                (spawn_x, target_y, center_x, target_y)
            }
        }
    }

    pub fn update(&mut self, delta_time: u32, intersection: &Intersection) {
        let dt = delta_time as f64 / 1000.0;

        if self.is_in_intersection(intersection) {
            self.time_in_intersection += delta_time;
        }

        self.adjust_velocity(dt);

        match self.state {
            VehicleState::Approaching | VehicleState::Entering => {
                self.move_in_lane(dt);
            }
            VehicleState::Turning => {
                self.move_turning(dt);
            }
            VehicleState::Exiting | VehicleState::Completed => {
                self.move_toward_destination(dt);
            }
        }

        self.update_state(intersection);
        self.position = Point::new(self.position_f.0 as i32, self.position_f.1 as i32);
    }

    fn adjust_velocity(&mut self, dt: f64) {
        let velocity_diff = self.target_velocity - self.current_velocity;

        if velocity_diff.abs() < 0.5 {
            self.current_velocity = self.target_velocity;
        } else {
            let acceleration = if velocity_diff > 0.0 { 25.0 } else { -80.0 };
            self.current_velocity += acceleration * dt;
            self.current_velocity = self.current_velocity.max(0.0).min(Self::FAST_VELOCITY);
        }
    }

    // PERFECT: Move exactly in lane center
    fn move_in_lane(&mut self, dt: f64) {
        let distance = self.current_velocity * dt;

        match self.current_movement_direction {
            Direction::North => {
                self.position_f.1 -= distance;
                self.position_f.0 = self.target_lane_x as f64; // Stay perfectly in lane
                self.angle = 0.0;
            }
            Direction::South => {
                self.position_f.1 += distance;
                self.position_f.0 = self.target_lane_x as f64; // Stay perfectly in lane
                self.angle = 180.0;
            }
            Direction::East => {
                self.position_f.0 += distance;
                self.position_f.1 = self.target_lane_y as f64; // Stay perfectly in lane
                self.angle = 90.0;
            }
            Direction::West => {
                self.position_f.0 -= distance;
                self.position_f.1 = self.target_lane_y as f64; // Stay perfectly in lane
                self.angle = 270.0;
            }
        }
    }

    // PERFECT: Smooth turning with proper geometry
    fn move_turning(&mut self, dt: f64) {
        let turn_speed = self.current_velocity * 0.6; // Slower when turning
        self.turning_progress += turn_speed * dt / 100.0; // Adjust for smooth turning

        let center_x = crate::WINDOW_WIDTH as f64 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f64 / 2.0;

        // Simple linear interpolation for smooth turning
        let start_pos = (self.target_lane_x as f64, self.target_lane_y as f64);
        let end_pos = self.calculate_end_position_after_turn();

        let progress = self.turning_progress.min(1.0);

        // Interpolate position smoothly
        self.position_f.0 = start_pos.0 + (end_pos.0 - start_pos.0) * progress;
        self.position_f.1 = start_pos.1 + (end_pos.1 - start_pos.1) * progress;

        // Update angle smoothly
        let start_angle = self.get_angle_for_direction(self.direction);
        let end_angle = self.get_angle_for_direction(self.destination);
        self.angle = start_angle + (end_angle - start_angle) * progress;

        if self.turning_progress >= 1.0 {
            self.complete_turn();
        }
    }

    fn calculate_end_position_after_turn(&self) -> (f64, f64) {
        // FIXED: Use exact lane positioning mathematics
        match self.destination {
            Direction::North => {
                let target_x = self.get_perfect_lane_center_x(Direction::North, 1) as f64; // Lane 1 = straight lane
                let target_y = crate::WINDOW_HEIGHT as f64 / 2.0;
                (target_x, target_y)
            }
            Direction::South => {
                let target_x = self.get_perfect_lane_center_x(Direction::South, 1) as f64; // Lane 1 = straight lane
                let target_y = crate::WINDOW_HEIGHT as f64 / 2.0;
                (target_x, target_y)
            }
            Direction::East => {
                let target_x = crate::WINDOW_WIDTH as f64 / 2.0;
                let target_y = self.get_perfect_lane_center_y(Direction::East, 1) as f64; // Lane 1 = straight lane
                (target_x, target_y)
            }
            Direction::West => {
                let target_x = crate::WINDOW_WIDTH as f64 / 2.0;
                let target_y = self.get_perfect_lane_center_y(Direction::West, 1) as f64; // Lane 1 = straight lane
                (target_x, target_y)
            }
        }
    }

    fn get_angle_for_direction(&self, direction: Direction) -> f64 {
        match direction {
            Direction::North => 0.0,
            Direction::East => 90.0,
            Direction::South => 180.0,
            Direction::West => 270.0,
        }
    }

    fn complete_turn(&mut self) {
        self.current_movement_direction = self.destination;
        self.state = VehicleState::Exiting;
        self.turning_progress = 0.0;
        self.has_reserved_intersection = false;

        // FIXED: Update target lane position using perfect mathematics
        self.update_target_lane_for_destination();

        // FIXED: Smoothly position vehicle in the exact center of new lane
        match self.destination {
            Direction::North | Direction::South => {
                self.position_f.0 = self.target_lane_x as f64; // Snap to exact lane center
            }
            Direction::East | Direction::West => {
                self.position_f.1 = self.target_lane_y as f64; // Snap to exact lane center
            }
        }

        println!("Vehicle {} completed turn to {:?} - positioned at ({:.0}, {:.0})",
                 self.id, self.destination, self.position_f.0, self.position_f.1);
    }

    fn update_target_lane_for_destination(&mut self) {
        // FIXED: Assign vehicles to appropriate lanes after turning, not all to lane 1
        let destination_lane = match (self.direction, self.route) {
            // Left turns should use right-side lanes (closer to center) in new direction
            (Direction::North, Route::Left) => 2,  // West-bound lane 2 (right lane)
            (Direction::South, Route::Left) => 2,  // East-bound lane 2 (right lane)
            (Direction::East, Route::Left) => 2,   // North-bound lane 2 (right lane)
            (Direction::West, Route::Left) => 2,   // South-bound lane 2 (right lane)

            // Right turns should use left-side lanes (further from center) in new direction
            (Direction::North, Route::Right) => 0, // East-bound lane 0 (left lane)
            (Direction::South, Route::Right) => 0, // West-bound lane 0 (left lane)
            (Direction::East, Route::Right) => 0,  // South-bound lane 0 (left lane)
            (Direction::West, Route::Right) => 0,  // North-bound lane 0 (left lane)

            // Straight traffic (shouldn't happen in turning, but just in case)
            _ => 1, // Default to middle lane
        };

        match self.destination {
            Direction::North => {
                self.target_lane_x = self.get_perfect_lane_center_x(Direction::North, destination_lane);
                self.target_lane_y = crate::WINDOW_HEIGHT as i32 / 2;
            }
            Direction::South => {
                self.target_lane_x = self.get_perfect_lane_center_x(Direction::South, destination_lane);
                self.target_lane_y = crate::WINDOW_HEIGHT as i32 / 2;
            }
            Direction::East => {
                self.target_lane_x = crate::WINDOW_WIDTH as i32 / 2;
                self.target_lane_y = self.get_perfect_lane_center_y(Direction::East, destination_lane);
            }
            Direction::West => {
                self.target_lane_x = crate::WINDOW_WIDTH as i32 / 2;
                self.target_lane_y = self.get_perfect_lane_center_y(Direction::West, destination_lane);
            }
        }

        println!("Vehicle {} will use destination lane {} in {:?} direction (pos: {}, {})",
                 self.id, destination_lane, self.destination, self.target_lane_x, self.target_lane_y);
    }

    // PERFECT: Same mathematical functions as main.rs for consistency
    fn get_perfect_lane_center_x(&self, direction: Direction, lane: usize) -> i32 {
        let center_x = crate::WINDOW_WIDTH as i32 / 2;
        match direction {
            Direction::North => {
                // Right side of vertical road: lanes 0, 1, 2 from left to right
                center_x + 15 + (lane as i32 * LANE_WIDTH) // 527, 557, 587
            }
            Direction::South => {
                // Left side of vertical road: lanes 0, 1, 2 from right to left
                center_x - 15 - (lane as i32 * LANE_WIDTH) // 497, 467, 437
            }
            _ => center_x,
        }
    }

    fn get_perfect_lane_center_y(&self, direction: Direction, lane: usize) -> i32 {
        let center_y = crate::WINDOW_HEIGHT as i32 / 2;
        match direction {
            Direction::East => {
                // Bottom side of horizontal road: lanes 0, 1, 2 from top to bottom
                center_y + 15 + (lane as i32 * LANE_WIDTH) // 399, 429, 459
            }
            Direction::West => {
                // Top side of horizontal road: lanes 0, 1, 2 from bottom to top
                center_y - 15 - (lane as i32 * LANE_WIDTH) // 369, 339, 309
            }
            _ => center_y,
        }
    }

    // PERFECT: Move toward final destination with smooth lane correction
    fn move_toward_destination(&mut self, dt: f64) {
        let distance = self.current_velocity * dt;

        match self.destination {
            Direction::North => {
                self.position_f.1 -= distance;
                // FIXED: Gradually correct to lane center instead of snapping
                let target_x = self.target_lane_x as f64;
                let diff_x = target_x - self.position_f.0;
                if diff_x.abs() > 1.0 {
                    self.position_f.0 += diff_x * 0.1; // Gradual correction
                } else {
                    self.position_f.0 = target_x; // Snap when very close
                }
                self.angle = 0.0;
            }
            Direction::South => {
                self.position_f.1 += distance;
                let target_x = self.target_lane_x as f64;
                let diff_x = target_x - self.position_f.0;
                if diff_x.abs() > 1.0 {
                    self.position_f.0 += diff_x * 0.1; // Gradual correction
                } else {
                    self.position_f.0 = target_x; // Snap when very close
                }
                self.angle = 180.0;
            }
            Direction::East => {
                self.position_f.0 += distance;
                let target_y = self.target_lane_y as f64;
                let diff_y = target_y - self.position_f.1;
                if diff_y.abs() > 1.0 {
                    self.position_f.1 += diff_y * 0.1; // Gradual correction
                } else {
                    self.position_f.1 = target_y; // Snap when very close
                }
                self.angle = 90.0;
            }
            Direction::West => {
                self.position_f.0 -= distance;
                let target_y = self.target_lane_y as f64;
                let diff_y = target_y - self.position_f.1;
                if diff_y.abs() > 1.0 {
                    self.position_f.1 += diff_y * 0.1; // Gradual correction
                } else {
                    self.position_f.1 = target_y; // Snap when very close
                }
                self.angle = 270.0;
            }
        }
    }

    fn update_state(&mut self, intersection: &Intersection) {
        let center_x = crate::WINDOW_WIDTH as i32 / 2;
        let center_y = crate::WINDOW_HEIGHT as i32 / 2;
        let intersection_radius = 90;

        let distance_to_center = (
            (self.position.x - center_x).pow(2) +
                (self.position.y - center_y).pow(2)
        ) as f64;
        let distance_to_center = distance_to_center.sqrt();

        match self.state {
            VehicleState::Approaching => {
                if distance_to_center < intersection_radius as f64 + 70.0 {
                    self.state = VehicleState::Entering;
                    println!("Vehicle {} entering intersection", self.id);
                }
            }
            VehicleState::Entering => {
                if distance_to_center < 50.0 {
                    if self.route != Route::Straight {
                        self.state = VehicleState::Turning;
                        self.turning_progress = 0.0;
                        println!("Vehicle {} starting {:?} turn", self.id, self.route);
                    } else {
                        self.state = VehicleState::Exiting;
                        println!("Vehicle {} going straight through", self.id);
                    }
                }
            }
            VehicleState::Turning => {
                // Handled in move_turning method
            }
            VehicleState::Exiting => {
                if distance_to_center > intersection_radius as f64 + 90.0 {
                    self.state = VehicleState::Completed;
                    println!("Vehicle {} completed intersection", self.id);
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
        self.position.x >= -300 &&
            self.position.x <= (crate::WINDOW_WIDTH as i32 + 300) &&
            self.position.y >= -300 &&
            self.position.y <= (crate::WINDOW_HEIGHT as i32 + 300)
    }

    pub fn distance_from_spawn(&self) -> f64 {
        match self.direction {
            Direction::North => (crate::WINDOW_HEIGHT as f64 + SPAWN_DISTANCE as f64) - self.position_f.1,
            Direction::South => self.position_f.1 + SPAWN_DISTANCE as f64,
            Direction::East => self.position_f.0 + SPAWN_DISTANCE as f64,
            Direction::West => (crate::WINDOW_WIDTH as f64 + SPAWN_DISTANCE as f64) - self.position_f.0,
        }
    }

    pub fn is_approaching_intersection(&self, intersection: &Intersection) -> bool {
        let center_x = crate::WINDOW_WIDTH as i32 / 2;
        let center_y = crate::WINDOW_HEIGHT as i32 / 2;
        let approach_distance = 150;

        match self.current_movement_direction {
            Direction::North => {
                self.position.y > center_y &&
                    self.position.y < center_y + approach_distance &&
                    (self.position.x - center_x).abs() < 120
            }
            Direction::South => {
                self.position.y < center_y &&
                    self.position.y > center_y - approach_distance &&
                    (self.position.x - center_x).abs() < 120
            }
            Direction::East => {
                self.position.x < center_x &&
                    self.position.x > center_x - approach_distance &&
                    (self.position.y - center_y).abs() < 120
            }
            Direction::West => {
                self.position.x > center_x &&
                    self.position.x < center_x + approach_distance &&
                    (self.position.y - center_y).abs() < 120
            }
        }
    }

    pub fn is_in_intersection(&self, intersection: &Intersection) -> bool {
        intersection.is_point_in_intersection(self.position.x, self.position.y)
    }

    pub fn time_to_intersection(&self, _intersection: &Intersection) -> f64 {
        if self.current_velocity <= 0.0 {
            return f64::INFINITY;
        }

        let center_x = crate::WINDOW_WIDTH as i32 / 2;
        let center_y = crate::WINDOW_HEIGHT as i32 / 2;

        let distance = match self.current_movement_direction {
            Direction::North => (self.position.y - center_y).max(0) as f64,
            Direction::South => (center_y - self.position.y).max(0) as f64,
            Direction::East => (center_x - self.position.x).max(0) as f64,
            Direction::West => (self.position.x - center_x).max(0) as f64,
        };

        distance / self.current_velocity
    }

    // PERFECT: Collision detection based on exact lane mathematics
    pub fn could_collide_with(&self, other: &Vehicle, intersection: &Intersection) -> bool {
        if self.id == other.id {
            return false;
        }

        let dx = (self.position.x - other.position.x) as f64;
        let dy = (self.position.y - other.position.y) as f64;
        let distance = (dx * dx + dy * dy).sqrt();

        // Critical proximity
        if distance < 25.0 {
            return true;
        }

        // Same lane same direction
        if self.direction == other.direction && self.lane == other.lane {
            return self.is_vehicle_ahead_in_same_lane(other);
        }

        // PERFECT: Only actual path intersections
        if (self.is_approaching_intersection(intersection) || self.is_in_intersection(intersection)) &&
            (other.is_approaching_intersection(intersection) || other.is_in_intersection(intersection)) {
            return self.will_paths_actually_intersect(other);
        }

        false
    }

    fn is_vehicle_ahead_in_same_lane(&self, other: &Vehicle) -> bool {
        if self.direction != other.direction || self.lane != other.lane {
            return false;
        }

        let distance_ahead = match self.current_movement_direction {
            Direction::North => other.position.y - self.position.y,
            Direction::South => self.position.y - other.position.y,
            Direction::East => other.position.x - self.position.x,
            Direction::West => self.position.x - other.position.x,
        };

        distance_ahead > 0 && distance_ahead < Self::SAFE_DISTANCE as i32
    }

    // PERFECT: Only real intersecting paths
    fn will_paths_actually_intersect(&self, other: &Vehicle) -> bool {
        // Same direction same lane = conflict
        if self.direction == other.direction {
            return self.lane == other.lane;
        }

        // PERFECT: Right turns use tight curves - no conflicts with straight traffic
        if self.route == Route::Right || other.route == Route::Right {
            return false; // Right turns are designed to avoid all conflicts
        }

        // PERFECT: Straight traffic is completely separated
        if self.route == Route::Straight && other.route == Route::Straight {
            return false; // Physically separated lanes
        }

        // Only left turns can create conflicts
        match (self.direction, self.route, other.direction, other.route) {
            // Left turner vs straight from perpendicular direction
            (Direction::North, Route::Left, Direction::East, Route::Straight) => true,
            (Direction::South, Route::Left, Direction::West, Route::Straight) => true,
            (Direction::East, Route::Left, Direction::South, Route::Straight) => true,
            (Direction::West, Route::Left, Direction::North, Route::Straight) => true,

            // Reverse cases
            (Direction::East, Route::Straight, Direction::North, Route::Left) => true,
            (Direction::West, Route::Straight, Direction::South, Route::Left) => true,
            (Direction::South, Route::Straight, Direction::East, Route::Left) => true,
            (Direction::North, Route::Straight, Direction::West, Route::Left) => true,

            // Opposing left turns
            (Direction::North, Route::Left, Direction::South, Route::Left) => true,
            (Direction::East, Route::Left, Direction::West, Route::Left) => true,

            _ => false,
        }
    }

    pub fn has_intersection_reservation(&self) -> bool {
        self.has_reserved_intersection
    }

    pub fn set_intersection_reservation(&mut self, reserved: bool) {
        self.has_reserved_intersection = reserved;
    }

    // Getter for current movement direction (needed for rendering)
    pub fn get_current_movement_direction(&self) -> Direction {
        self.current_movement_direction
    }
}