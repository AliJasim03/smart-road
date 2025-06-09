// src/vehicle.rs - FINAL FIX: Equal lane spacing and proper positioning
use crate::intersection::Intersection;
use sdl2::rect::Point;
use std::sync::atomic::{AtomicU32, Ordering};
use std::collections::HashMap;

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

// FINAL FIX: Corrected road mapping system
pub struct RoadMapping {
    routes: HashMap<(Direction, usize), (Direction, Route)>,
}

impl RoadMapping {
    pub fn new() -> Self {
        let mut routes = HashMap::new();

        // FINAL FIX: Each direction gets 3 lanes with consistent routing
        // Lane 0 = Left turn, Lane 1 = Straight, Lane 2 = Right turn

        // North-bound traffic (coming from South, moving North)
        routes.insert((Direction::North, 0), (Direction::West, Route::Left));     // Left turn to West
        routes.insert((Direction::North, 1), (Direction::North, Route::Straight)); // Straight North
        routes.insert((Direction::North, 2), (Direction::East, Route::Right));     // Right turn to East

        // South-bound traffic (coming from North, moving South)
        routes.insert((Direction::South, 0), (Direction::East, Route::Left));      // Left turn to East
        routes.insert((Direction::South, 1), (Direction::South, Route::Straight)); // Straight South
        routes.insert((Direction::South, 2), (Direction::West, Route::Right));     // Right turn to West

        // East-bound traffic (coming from West, moving East)
        routes.insert((Direction::East, 0), (Direction::North, Route::Left));     // Left turn to North
        routes.insert((Direction::East, 1), (Direction::East, Route::Straight));  // Straight East
        routes.insert((Direction::East, 2), (Direction::South, Route::Right));    // Right turn to South

        // West-bound traffic (coming from East, moving West)
        routes.insert((Direction::West, 0), (Direction::South, Route::Left));     // Left turn to South
        routes.insert((Direction::West, 1), (Direction::West, Route::Straight));  // Straight West
        routes.insert((Direction::West, 2), (Direction::North, Route::Right));    // Right turn to North

        RoadMapping { routes }
    }

    pub fn get_destination_and_route(&self, incoming: Direction, lane: usize) -> (Direction, Route) {
        *self.routes.get(&(incoming, lane)).unwrap_or(&(incoming, Route::Straight))
    }

    pub fn print_mapping(&self) {
        println!("=== FINAL FIXED 6-LANE SYSTEM ===");
        println!("Perfect lane spacing: Each lane exactly 40px wide");
        println!("Lane 0=Left, Lane 1=Straight(MIDDLE), Lane 2=Right\n");

        for direction in [Direction::North, Direction::South, Direction::East, Direction::West] {
            println!("{:?}-bound traffic:", direction);
            for lane in 0..3 {
                let (destination, route) = self.get_destination_and_route(direction, lane);
                println!("  Lane {}: {:?} turn → {:?} road", lane, route, destination);
            }
            println!();
        }
        println!("=====================================");
    }
}

pub struct Vehicle {
    pub id: u32,
    pub position: Point,
    position_f: (f64, f64),
    pub direction: Direction,
    pub destination: Direction,
    pub lane: usize,
    pub route: Route,
    pub color: VehicleColor,
    pub state: VehicleState,
    pub velocity_level: VelocityLevel,
    pub current_velocity: f64,
    pub target_velocity: f64,
    pub width: u32,
    pub height: u32,
    pub start_time: std::time::Instant,
    pub time_in_intersection: u32,
    turning_progress: f64,
    pub angle: f64,
    last_slow_down_time: std::time::Instant,
    stuck_timer: f32,
    original_velocity: f64,
    current_movement_direction: Direction,
    has_reserved_intersection: bool,
    lane_center_offset: f64,
}

impl Vehicle {
    // Conservative velocity constants
    pub const SLOW_VELOCITY: f64 = 15.0;
    pub const MEDIUM_VELOCITY: f64 = 35.0;
    pub const FAST_VELOCITY: f64 = 55.0;
    pub const SAFE_DISTANCE: f64 = 80.0;

    // FINAL FIX: Perfect lane spacing
    pub const LANE_WIDTH: f64 = 40.0;      // Each lane exactly 40px
    pub const ROAD_WIDTH: f64 = 240.0;     // Total road width (6 lanes × 40px)

    pub const WIDTH: u32 = 20;
    pub const HEIGHT: u32 = 20;

    // FINAL FIX: Perfect spawn positioning with equal lane spacing
    pub fn new_with_destination(incoming_direction: Direction, lane: usize, road_mapping: &RoadMapping) -> Self {
        let (destination, route) = road_mapping.get_destination_and_route(incoming_direction, lane);

        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let (spawn_x, spawn_y, lane_offset) = Self::calculate_perfect_spawn_position(incoming_direction, lane);

        // FINAL FIX: Color based on actual route, not lane
        let color = match route {
            Route::Left => VehicleColor::Red,
            Route::Straight => VehicleColor::Blue,
            Route::Right => VehicleColor::Green,
        };

        use rand::Rng;
        let mut rng = rand::thread_rng();
        let velocity_level = match rng.gen_range(0..10) {
            0..=4 => VelocityLevel::Slow,
            5..=8 => VelocityLevel::Medium,
            _ => VelocityLevel::Fast,
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

        println!("🚗 Vehicle {}: {:?} Lane {} → {:?} road ({:?}, {:.0} px/s) at ({:.0}, {:.0})",
                 id, incoming_direction, lane, destination, route, initial_velocity, spawn_x, spawn_y);

        Vehicle {
            id,
            position: Point::new(spawn_x as i32, spawn_y as i32),
            position_f: (spawn_x, spawn_y),
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
            start_time: std::time::Instant::now(),
            time_in_intersection: 0,
            turning_progress: 0.0,
            angle: initial_angle,
            last_slow_down_time: std::time::Instant::now(),
            stuck_timer: 0.0,
            original_velocity: initial_velocity,
            current_movement_direction: incoming_direction,
            has_reserved_intersection: false,
            lane_center_offset: lane_offset,
        }
    }

    // FINAL FIX: Perfect lane positioning with exactly equal spacing
    fn calculate_perfect_spawn_position(direction: Direction, lane: usize) -> (f64, f64, f64) {
        let center_x = crate::WINDOW_WIDTH as f64 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f64 / 2.0;
        let spawn_distance = 250.0;

        // Perfect lane calculation: each lane is exactly 40px wide
        // Lane positions from center of their road section:
        // Lane 0: -40px (left/outer)
        // Lane 1:   0px (center/middle)
        // Lane 2: +40px (right/inner)
        let lane_offset = (lane as f64 - 1.0) * Self::LANE_WIDTH; // -40, 0, +40

        match direction {
            Direction::North => {
                // North-bound: spawn from south, travel north
                // Use RIGHT side of vertical road (positive X from center)
                let base_x = center_x + (Self::ROAD_WIDTH / 4.0); // 60px right of center
                let x = base_x + lane_offset; // Add lane offset
                let y = crate::WINDOW_HEIGHT as f64 + spawn_distance;
                (x, y, lane_offset)
            }
            Direction::South => {
                // South-bound: spawn from north, travel south
                // Use LEFT side of vertical road (negative X from center)
                let base_x = center_x - (Self::ROAD_WIDTH / 4.0); // 60px left of center
                let x = base_x - lane_offset; // Subtract lane offset (flip for opposite direction)
                let y = -spawn_distance;
                (x, y, -lane_offset)
            }
            Direction::East => {
                // East-bound: spawn from west, travel east
                // Use BOTTOM side of horizontal road (positive Y from center)
                let base_y = center_y + (Self::ROAD_WIDTH / 4.0); // 60px below center
                let x = -spawn_distance;
                let y = base_y + lane_offset; // Add lane offset
                (x, y, lane_offset)
            }
            Direction::West => {
                // West-bound: spawn from east, travel west
                // Use TOP side of horizontal road (negative Y from center)
                let base_y = center_y - (Self::ROAD_WIDTH / 4.0); // 60px above center
                let x = crate::WINDOW_WIDTH as f64 + spawn_distance;
                let y = base_y - lane_offset; // Subtract lane offset (flip for opposite direction)
                (x, y, -lane_offset)
            }
        }
    }

    // Legacy constructor for compatibility
    pub fn new(direction: Direction, lane: usize, route: Route) -> Self {
        let road_mapping = RoadMapping::new();
        Self::new_with_destination(direction, lane, &road_mapping)
    }

    pub fn get_target_direction(&self) -> Direction {
        self.destination
    }

    pub fn update(&mut self, delta_time: u32, intersection: &Intersection) {
        let dt = delta_time as f64 / 1000.0;

        if self.is_in_intersection(intersection) {
            self.time_in_intersection += delta_time;
        }

        if self.current_velocity < Self::SLOW_VELOCITY * 0.3 {
            self.stuck_timer += dt as f32;
        } else {
            self.stuck_timer = 0.0;
        }

        if self.stuck_timer > 8.0 {
            println!("Vehicle {} auto-recovering from stuck state", self.id);
            self.target_velocity = self.original_velocity.min(Self::MEDIUM_VELOCITY);
            self.stuck_timer = 0.0;
        }

        self.adjust_velocity(dt);

        match self.state {
            VehicleState::Approaching | VehicleState::Entering => {
                self.move_straight(dt);
            }
            VehicleState::Turning => {
                self.move_turning(dt);
            }
            VehicleState::Exiting | VehicleState::Completed => {
                self.move_straight(dt);
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
            let acceleration = if velocity_diff > 0.0 {
                30.0
            } else {
                -150.0
            };

            self.current_velocity += acceleration * dt;
            self.current_velocity = self.current_velocity.max(0.0).min(Self::FAST_VELOCITY);
        }
    }

    fn move_straight(&mut self, dt: f64) {
        let distance = self.current_velocity * dt;

        match self.current_movement_direction {
            Direction::North => {
                self.position_f.1 -= distance;
                self.angle = 0.0;
            }
            Direction::South => {
                self.position_f.1 += distance;
                self.angle = 180.0;
            }
            Direction::East => {
                self.position_f.0 += distance;
                self.angle = 90.0;
            }
            Direction::West => {
                self.position_f.0 -= distance;
                self.angle = 270.0;
            }
        }
    }

    fn move_turning(&mut self, dt: f64) {
        let turn_speed = self.current_velocity * 0.6;
        let distance = turn_speed * dt;

        let turn_rate = 1.5;
        self.turning_progress += dt * turn_rate;

        if self.turning_progress < 0.7 {
            match self.current_movement_direction {
                Direction::North => self.position_f.1 -= distance * 0.7,
                Direction::South => self.position_f.1 += distance * 0.7,
                Direction::East => self.position_f.0 += distance * 0.7,
                Direction::West => self.position_f.0 -= distance * 0.7,
            }
        } else {
            match self.destination {
                Direction::North => self.position_f.1 -= distance * 0.7,
                Direction::South => self.position_f.1 += distance * 0.7,
                Direction::East => self.position_f.0 += distance * 0.7,
                Direction::West => self.position_f.0 -= distance * 0.7,
            }
        }

        if self.turning_progress >= 1.0 {
            self.complete_turn();
        }
    }

    fn complete_turn(&mut self) {
        self.current_movement_direction = self.destination;
        self.state = VehicleState::Exiting;
        self.turning_progress = 0.0;
        self.has_reserved_intersection = false;

        self.angle = match self.destination {
            Direction::North => 0.0,
            Direction::East => 90.0,
            Direction::South => 180.0,
            Direction::West => 270.0,
        };

        println!("Vehicle {} completed turn to {:?}", self.id, self.destination);
    }

    fn update_state(&mut self, _intersection: &Intersection) {
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
                if distance_to_center < intersection_radius as f64 + 50.0 {
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
                if distance_to_center > intersection_radius as f64 + 60.0 {
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
        let new_target = match level {
            VelocityLevel::Slow => Self::SLOW_VELOCITY,
            VelocityLevel::Medium => Self::MEDIUM_VELOCITY,
            VelocityLevel::Fast => Self::FAST_VELOCITY,
        };

        if new_target < self.target_velocity {
            self.target_velocity = new_target;
            self.last_slow_down_time = std::time::Instant::now();
        } else if new_target > self.target_velocity {
            let time_since_slowdown = self.last_slow_down_time.elapsed().as_secs_f32();
            if time_since_slowdown > 3.0 ||
                matches!(self.state, VehicleState::Exiting | VehicleState::Completed) {
                self.target_velocity = new_target.min(self.original_velocity);
            }
        }
    }

    pub fn try_speed_up(&mut self) {
        let time_since_slowdown = self.last_slow_down_time.elapsed().as_secs_f32();

        if time_since_slowdown > 2.5 {
            match self.state {
                VehicleState::Approaching => {
                    let target_speed = self.original_velocity.min(Self::MEDIUM_VELOCITY);
                    if self.target_velocity < target_speed * 0.8 {
                        self.target_velocity = target_speed;
                    }
                }
                VehicleState::Exiting | VehicleState::Completed => {
                    let target_speed = self.original_velocity.min(Self::FAST_VELOCITY);
                    if self.target_velocity < target_speed * 0.8 {
                        self.target_velocity = target_speed;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn is_on_screen(&self) -> bool {
        self.position.x >= -400 &&
            self.position.x <= (crate::WINDOW_WIDTH as i32 + 400) &&
            self.position.y >= -400 &&
            self.position.y <= (crate::WINDOW_HEIGHT as i32 + 400)
    }

    pub fn distance_from_spawn(&self) -> f64 {
        match self.direction {
            Direction::North => (crate::WINDOW_HEIGHT as f64 + 250.0) - self.position_f.1,
            Direction::South => self.position_f.1 + 250.0,
            Direction::East => self.position_f.0 + 250.0,
            Direction::West => (crate::WINDOW_WIDTH as f64 + 250.0) - self.position_f.0,
        }
    }

    pub fn is_approaching_intersection(&self, _intersection: &Intersection) -> bool {
        let center_x = crate::WINDOW_WIDTH as i32 / 2;
        let center_y = crate::WINDOW_HEIGHT as i32 / 2;
        let approach_distance = 120;

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

    pub fn has_left_intersection(&self, intersection: &Intersection) -> bool {
        !self.is_in_intersection(intersection) &&
            matches!(self.state, VehicleState::Exiting | VehicleState::Completed)
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

    // FINAL FIX: Much better collision detection
    pub fn could_collide_with(&self, other: &Vehicle, intersection: &Intersection) -> bool {
        if self.id == other.id {
            return false;
        }

        let dx = (self.position.x - other.position.x) as f64;
        let dy = (self.position.y - other.position.y) as f64;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance < 30.0 {
            return true;
        }

        if self.direction == other.direction && self.lane == other.lane {
            return self.is_vehicle_ahead_in_same_lane(other);
        }

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
            Direction::East => self.position.x - other.position.x,
            Direction::West => other.position.x - self.position.x,
        };

        distance_ahead > 0 && distance_ahead < Self::SAFE_DISTANCE as i32
    }

    // FINAL FIX: Only real path intersections
    fn will_paths_actually_intersect(&self, other: &Vehicle) -> bool {
        if self.direction == other.direction {
            return self.lane == other.lane;
        }

        // FINAL FIX: Straight traffic should NEVER collide in proper 6-lane system
        if self.route == Route::Straight && other.route == Route::Straight {
            return false; // Separated by design
        }

        // Only specific turning conflicts
        match (self.direction, self.route, other.direction, other.route) {
            // Left turn vs straight from perpendicular direction
            (Direction::North, Route::Left, Direction::East, Route::Straight) => true,
            (Direction::South, Route::Left, Direction::West, Route::Straight) => true,
            (Direction::East, Route::Left, Direction::South, Route::Straight) => true,
            (Direction::West, Route::Left, Direction::North, Route::Straight) => true,

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

    pub fn is_spawning_from_correct_edge(&self) -> bool {
        let center_x = crate::WINDOW_WIDTH as f64 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f64 / 2.0;

        match self.direction {
            Direction::North => self.position_f.1 > center_y + 100.0,
            Direction::South => self.position_f.1 < center_y - 100.0,
            Direction::East => self.position_f.0 < center_x - 100.0,
            Direction::West => self.position_f.0 > center_x + 100.0,
        }
    }

    pub fn get_lane_center_offset(&self) -> f64 {
        self.lane_center_offset
    }
}