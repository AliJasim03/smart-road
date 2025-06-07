// src/vehicle.rs - FINAL VERSION WITH ROAD-TO-ROAD MAPPING
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

// NEW: Fixed road-to-road mapping system
pub struct RoadMapping {
    routes: HashMap<(Direction, usize), Direction>,
}

impl RoadMapping {
    pub fn new() -> Self {
        let mut routes = HashMap::new();

        // FIXED: Explicit road-to-road mapping based on README diagram
        // Each incoming direction has 3 lanes that go to specific outgoing roads

        // North-bound (coming from South) - lanes go to:
        routes.insert((Direction::North, 0), Direction::West);  // Lane 0 → West road
        routes.insert((Direction::North, 1), Direction::North); // Lane 1 → Continue North
        routes.insert((Direction::North, 2), Direction::East);  // Lane 2 → East road

        // South-bound (coming from North) - lanes go to:
        routes.insert((Direction::South, 0), Direction::West);  // Lane 0 → West road
        routes.insert((Direction::South, 1), Direction::South); // Lane 1 → Continue South
        routes.insert((Direction::South, 2), Direction::East);  // Lane 2 → East road

        // East-bound (coming from West) - lanes go to:
        routes.insert((Direction::East, 0), Direction::North);  // Lane 0 → North road
        routes.insert((Direction::East, 1), Direction::East);   // Lane 1 → Continue East
        routes.insert((Direction::East, 2), Direction::South);  // Lane 2 → South road

        // West-bound (coming from East) - lanes go to:
        routes.insert((Direction::West, 0), Direction::North);  // Lane 0 → North road
        routes.insert((Direction::West, 1), Direction::West);   // Lane 1 → Continue West
        routes.insert((Direction::West, 2), Direction::South);  // Lane 2 → South road

        RoadMapping { routes }
    }

    pub fn get_destination(&self, incoming: Direction, lane: usize) -> Direction {
        *self.routes.get(&(incoming, lane)).unwrap_or(&incoming)
    }

    pub fn print_mapping(&self) {
        println!("=== ROAD-TO-ROAD MAPPING ===");
        for direction in [Direction::North, Direction::South, Direction::East, Direction::West] {
            println!("{:?}-bound lanes:", direction);
            for lane in 0..3 {
                let destination = self.get_destination(direction, lane);
                println!("  Lane {} → {:?} road", lane, destination);
            }
        }
        println!("=============================");
    }
}

pub struct Vehicle {
    pub id: u32,
    pub position: Point,
    position_f: (f64, f64), // For smooth movement calculations
    pub direction: Direction, // Original incoming direction (never changes)
    pub destination: Direction, // Final destination road
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
    turning_progress: f64, // 0.0 to 1.0 for turning animation
    pub angle: f64, // For proper rendering rotation
    last_slow_down_time: std::time::Instant,
    stuck_timer: f32, // Track how long vehicle has been slow/stopped
    original_velocity: f64, // Remember the vehicle's preferred speed
    current_movement_direction: Direction, // Direction vehicle is currently moving (changes during turns)
}

impl Vehicle {
    // Enhanced velocity constants with more variation
    pub const SLOW_VELOCITY: f64 = 25.0;   // pixels per second
    pub const MEDIUM_VELOCITY: f64 = 55.0; // pixels per second
    pub const FAST_VELOCITY: f64 = 85.0;   // pixels per second
    pub const SAFE_DISTANCE: f64 = 50.0;   // pixels

    // Lane constants for intersection
    pub const LANE_WIDTH: f64 = 30.0;      // 30px per lane
    pub const ROAD_WIDTH: f64 = 180.0;     // Total road width (3 lanes each direction)

    // Vehicle dimensions
    pub const WIDTH: u32 = 24;
    pub const HEIGHT: u32 = 24;

    // NEW: Create vehicle with explicit destination using road mapping
    pub fn new_with_destination(incoming_direction: Direction, lane: usize, road_mapping: &RoadMapping) -> Self {
        let destination = road_mapping.get_destination(incoming_direction, lane);
        let route = Self::calculate_route(incoming_direction, destination);

        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let (spawn_x, spawn_y) = Self::calculate_spawn_position(incoming_direction, lane);

        // Assign color based on route for better visualization
        let color = match route {
            Route::Left => VehicleColor::Red,
            Route::Straight => VehicleColor::Blue,
            Route::Right => VehicleColor::Green,
        };

        // Better velocity distribution with more realistic variation
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let velocity_level = match rng.gen_range(0..9) {
            0..=2 => VelocityLevel::Slow,   // 33% slow
            3..=5 => VelocityLevel::Medium, // 33% medium
            _ => VelocityLevel::Fast,       // 33% fast
        };

        let base_velocity = match velocity_level {
            VelocityLevel::Slow => Self::SLOW_VELOCITY,
            VelocityLevel::Medium => Self::MEDIUM_VELOCITY,
            VelocityLevel::Fast => Self::FAST_VELOCITY,
        };

        // Add some random variation but keep within reasonable bounds
        let variation = rng.gen_range(-8.0..8.0);
        let initial_velocity = (base_velocity + variation).max(15.0).min(95.0);

        let initial_angle = match incoming_direction {
            Direction::North => 0.0,
            Direction::East => 90.0,
            Direction::South => 180.0,
            Direction::West => 270.0,
        };

        println!("🚗 Vehicle {}: {:?} Lane {} → {:?} road ({:?} route, {:.0} px/s) at ({:.0}, {:.0})",
                 id, incoming_direction, lane, destination, route, initial_velocity, spawn_x, spawn_y);

        Vehicle {
            id,
            position: Point::new(spawn_x as i32, spawn_y as i32),
            position_f: (spawn_x, spawn_y),
            direction: incoming_direction, // Original direction never changes
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
            current_movement_direction: incoming_direction, // Starts same as original
        }
    }

    // Legacy constructor for compatibility
    pub fn new(direction: Direction, _lane: usize, route: Route) -> Self {
        // Create a default road mapping and use random lane
        let road_mapping = RoadMapping::new();
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let lane = rng.gen_range(0..3);

        Self::new_with_destination(direction, lane, &road_mapping)
    }

    // Calculate route based on incoming and destination directions
    fn calculate_route(from: Direction, to: Direction) -> Route {
        if from == to {
            return Route::Straight;
        }

        match (from, to) {
            // Left turns (90 degrees counterclockwise)
            (Direction::North, Direction::West) |
            (Direction::West, Direction::South) |
            (Direction::South, Direction::East) |
            (Direction::East, Direction::North) => Route::Left,

            // Right turns (90 degrees clockwise)
            (Direction::North, Direction::East) |
            (Direction::East, Direction::South) |
            (Direction::South, Direction::West) |
            (Direction::West, Direction::North) => Route::Right,

            // U-turns (treat as left for now)
            _ => Route::Left,
        }
    }

    // Get target direction (final destination)
    pub fn get_target_direction(&self) -> Direction {
        self.destination
    }

    // Spawn positioning to align with drawn lane markings
    fn calculate_spawn_position(direction: Direction, lane: usize) -> (f64, f64) {
        let center_x = crate::WINDOW_WIDTH as f64 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f64 / 2.0;

        match direction {
            Direction::North => {
                // North-bound vehicles spawn from SOUTH (bottom of screen)
                // Right side of vertical road: lanes at center_x + 15, +45, +75
                let x = center_x + 15.0 + (lane as f64 * 30.0);
                let y = crate::WINDOW_HEIGHT as f64 + 150.0;
                (x, y)
            }
            Direction::South => {
                // South-bound vehicles spawn from NORTH (top of screen)
                // Left side of vertical road: lanes at center_x - 15, -45, -75
                let x = center_x - 15.0 - (lane as f64 * 30.0);
                let y = -150.0;
                (x, y)
            }
            Direction::East => {
                // East-bound vehicles spawn from WEST (left side of screen)
                // Bottom side of horizontal road: lanes at center_y + 15, +45, +75
                let x = -150.0;
                let y = center_y + 15.0 + (lane as f64 * 30.0);
                (x, y)
            }
            Direction::West => {
                // West-bound vehicles spawn from EAST (right side of screen)
                // Top side of horizontal road: lanes at center_y - 15, -45, -75
                let x = crate::WINDOW_WIDTH as f64 + 150.0;
                let y = center_y - 15.0 - (lane as f64 * 30.0);
                (x, y)
            }
        }
    }

    // Validation method to ensure vehicles spawn from correct edges
    pub fn is_spawning_from_correct_edge(&self) -> bool {
        let center_x = crate::WINDOW_WIDTH as f64 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f64 / 2.0;

        match self.direction {
            Direction::North => {
                // Should spawn from south (bottom) - y should be > center_y
                self.position_f.1 > center_y + 100.0
            }
            Direction::South => {
                // Should spawn from north (top) - y should be < center_y
                self.position_f.1 < center_y - 100.0
            }
            Direction::East => {
                // Should spawn from west (left) - x should be < center_x
                self.position_f.0 < center_x - 100.0
            }
            Direction::West => {
                // Should spawn from east (right) - x should be > center_x
                self.position_f.0 > center_x + 100.0
            }
        }
    }

    pub fn update(&mut self, delta_time: u32, intersection: &Intersection) {
        let dt = delta_time as f64 / 1000.0; // Convert to seconds

        // Update intersection time
        if self.is_in_intersection(intersection) {
            self.time_in_intersection += delta_time;
        }

        // Track stuck timer for vehicles that are moving too slowly
        if self.current_velocity < Self::SLOW_VELOCITY * 0.6 {
            self.stuck_timer += dt as f32;
        } else {
            self.stuck_timer = 0.0;
        }

        // Auto-recovery for stuck vehicles - less aggressive
        if self.stuck_timer > 3.0 { // Increased from 2.0 to 3.0
            println!("Vehicle {} auto-recovering from stuck state", self.id);
            self.target_velocity = self.original_velocity.min(Self::MEDIUM_VELOCITY);
            self.stuck_timer = 0.0;
        }

        // Smooth velocity adjustment with better acceleration
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
                self.move_straight(dt);
            }
        }

        // Update state based on position
        self.update_state(intersection);

        // Update integer position for rendering
        self.position = Point::new(self.position_f.0 as i32, self.position_f.1 as i32);
    }

    // Less aggressive velocity adjustment
    fn adjust_velocity(&mut self, dt: f64) {
        let velocity_diff = self.target_velocity - self.current_velocity;

        if velocity_diff.abs() < 2.0 { // Increased tolerance
            self.current_velocity = self.target_velocity;
        } else {
            let acceleration = if velocity_diff > 0.0 {
                80.0 // Faster acceleration when speeding up (was 60.0)
            } else {
                -80.0 // Less aggressive deceleration (was -100.0)
            };

            self.current_velocity += acceleration * dt;
            self.current_velocity = self.current_velocity.max(5.0).min(Self::FAST_VELOCITY * 1.2);
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
        let turn_speed = self.current_velocity * 0.9; // Less speed reduction during turns
        let distance = turn_speed * dt;

        let turn_rate = 2.5; // Faster turn completion
        self.turning_progress += dt * turn_rate;

        // Smooth angle interpolation during turn
        let start_angle = match self.current_movement_direction {
            Direction::North => 0.0,
            Direction::East => 90.0,
            Direction::South => 180.0,
            Direction::West => 270.0,
        };

        let end_angle = match self.destination {
            Direction::North => 0.0,
            Direction::East => 90.0,
            Direction::South => 180.0,
            Direction::West => 270.0,
        };

        // Handle angle wrapping for smooth interpolation
        let mut angle_diff = end_angle - start_angle;
        if angle_diff > 180.0 { angle_diff -= 360.0; }
        if angle_diff < -180.0 { angle_diff += 360.0; }

        self.angle = start_angle + angle_diff * self.turning_progress.min(1.0);

        // Better turn path calculation
        if self.turning_progress < 0.5 {
            // First half of turn - continue in original direction
            match self.current_movement_direction {
                Direction::North => self.position_f.1 -= distance,
                Direction::South => self.position_f.1 += distance,
                Direction::East => self.position_f.0 += distance,
                Direction::West => self.position_f.0 -= distance,
            }
        } else {
            // Second half of turn - move in new direction
            match self.destination {
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

    fn complete_turn(&mut self) {
        self.current_movement_direction = self.destination;
        self.state = VehicleState::Exiting;
        self.turning_progress = 0.0;

        self.angle = match self.destination {
            Direction::North => 0.0,
            Direction::East => 90.0,
            Direction::South => 180.0,
            Direction::West => 270.0,
        };
    }

    fn update_state(&mut self, _intersection: &Intersection) {
        let center_x = crate::WINDOW_WIDTH as i32 / 2;
        let center_y = crate::WINDOW_HEIGHT as i32 / 2;
        let intersection_radius = 120;

        let distance_to_center = (
            (self.position.x - center_x).pow(2) +
                (self.position.y - center_y).pow(2)
        ) as f64;
        let distance_to_center = distance_to_center.sqrt();

        match self.state {
            VehicleState::Approaching => {
                if distance_to_center < intersection_radius as f64 + 30.0 {
                    self.state = VehicleState::Entering;
                }
            }
            VehicleState::Entering => {
                if distance_to_center < 80.0 {
                    if self.route != Route::Straight {
                        self.state = VehicleState::Turning;
                        self.turning_progress = 0.0;
                    } else {
                        self.state = VehicleState::Exiting;
                    }
                }
            }
            VehicleState::Turning => {
                // Handled in move_turning method
            }
            VehicleState::Exiting => {
                if distance_to_center > intersection_radius as f64 + 40.0 {
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
            if time_since_slowdown > 0.8 || // Reduced recovery time
                matches!(self.state, VehicleState::Exiting | VehicleState::Completed) {
                self.target_velocity = new_target.min(self.original_velocity);
            }
        }
    }

    // More aggressive recovery from slowdowns
    pub fn try_speed_up(&mut self) {
        let time_since_slowdown = self.last_slow_down_time.elapsed().as_secs_f32();

        if time_since_slowdown > 0.6 { // Reduced from 1.0 - faster recovery
            match self.state {
                VehicleState::Approaching => {
                    let target_speed = self.original_velocity.min(Self::FAST_VELOCITY); // Increased from MEDIUM
                    if self.target_velocity < target_speed {
                        self.target_velocity = target_speed;
                    }
                }
                VehicleState::Exiting | VehicleState::Completed => {
                    let target_speed = self.original_velocity.min(Self::FAST_VELOCITY);
                    if self.target_velocity < target_speed {
                        self.target_velocity = target_speed;
                    }
                }
                VehicleState::Entering => {
                    let target_speed = self.original_velocity.min(Self::MEDIUM_VELOCITY);
                    if self.target_velocity < target_speed * 0.95 { // More aggressive
                        self.target_velocity = target_speed;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn is_on_screen(&self) -> bool {
        self.position.x >= -150 &&
            self.position.x <= (crate::WINDOW_WIDTH as i32 + 150) &&
            self.position.y >= -150 &&
            self.position.y <= (crate::WINDOW_HEIGHT as i32 + 150)
    }

    pub fn distance_from_spawn(&self) -> f64 {
        match self.direction {
            Direction::North => (crate::WINDOW_HEIGHT as f64 + 150.0) - self.position_f.1,
            Direction::South => self.position_f.1 + 150.0,
            Direction::East => self.position_f.0 + 150.0,
            Direction::West => (crate::WINDOW_WIDTH as f64 + 150.0) - self.position_f.0,
        }
    }

    pub fn is_approaching_intersection(&self, _intersection: &Intersection) -> bool {
        let center_x = crate::WINDOW_WIDTH as i32 / 2;
        let center_y = crate::WINDOW_HEIGHT as i32 / 2;
        let approach_distance = 200;

        match self.current_movement_direction {
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

    // More robust collision detection
    pub fn could_collide_with(&self, other: &Vehicle, intersection: &Intersection) -> bool {
        if self.id == other.id {
            return false;
        }

        let dx = (self.position.x - other.position.x) as f64;
        let dy = (self.position.y - other.position.y) as f64;
        let distance = (dx * dx + dy * dy).sqrt();

        // More reasonable collision detection distance
        if distance > 150.0 {
            return false;
        }

        // Immediate collision check - if vehicles are very close
        if distance < 40.0 {
            return true;
        }

        // Same lane following collision check
        if self.direction == other.direction && self.lane == other.lane {
            return self.is_vehicle_ahead_in_same_lane(other);
        }

        // Intersection-based collision detection
        if self.is_approaching_intersection(intersection) ||
            other.is_approaching_intersection(intersection) ||
            self.is_in_intersection(intersection) ||
            other.is_in_intersection(intersection) {
            return self.paths_intersect_improved(other);
        }

        false
    }

    // Check if other vehicle is directly ahead in same lane
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

        distance_ahead > 0 && distance_ahead < 80 // Vehicle is ahead and close
    }

    // Improved path intersection logic
    fn paths_intersect_improved(&self, other: &Vehicle) -> bool {
        // If both vehicles are going to the same destination, no conflict
        if self.destination == other.destination {
            return false;
        }

        // Same incoming direction with proper lane discipline
        if self.direction == other.direction {
            // Only conflict if somehow in same lane (shouldn't happen)
            return self.lane == other.lane;
        }

        // Check if paths actually cross in intersection based on destinations
        match (self.direction, self.destination, other.direction, other.destination) {
            // Vehicles with conflicting turn paths
            (Direction::North, Direction::West, Direction::East, Direction::North) => true,
            (Direction::North, Direction::East, Direction::West, Direction::South) => true,
            (Direction::South, Direction::East, Direction::West, Direction::North) => true,
            (Direction::South, Direction::West, Direction::East, Direction::South) => true,

            // Straight through conflicts
            (Direction::North, Direction::North, Direction::South, Direction::South) => true,
            (Direction::East, Direction::East, Direction::West, Direction::West) => true,

            // Most turns don't conflict with proper lane discipline
            _ => false,
        }
    }
}