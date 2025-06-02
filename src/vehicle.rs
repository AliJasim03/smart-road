// src/vehicle.rs - FIXED VERSION WITH PROPER LANE-TO-DIRECTION MAPPING
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

pub struct Vehicle {
    pub id: u32,
    pub position: Point,
    position_f: (f64, f64), // For smooth movement calculations
    pub direction: Direction,
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
}

impl Vehicle {
    // Enhanced velocity constants with more variation
    pub const SLOW_VELOCITY: f64 = 25.0;   // pixels per second
    pub const MEDIUM_VELOCITY: f64 = 55.0; // pixels per second
    pub const FAST_VELOCITY: f64 = 85.0;   // pixels per second
    pub const SAFE_DISTANCE: f64 = 50.0;   // pixels

    // FIXED: Proper lane constants for intersection
    pub const LANE_WIDTH: f64 = 30.0;      // 30px per lane
    pub const ROAD_WIDTH: f64 = 180.0;     // Total road width (3 lanes each direction)

    // Vehicle dimensions
    pub const WIDTH: u32 = 24;
    pub const HEIGHT: u32 = 24;

    pub fn new(direction: Direction, lane: usize, route: Route) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);

        // FIXED: Validate that the lane-route combination is valid for this direction
        let validated_lane = Self::validate_lane_for_route(direction, lane, route);

        // Calculate spawn position with proper lane alignment
        let (spawn_x, spawn_y) = Self::calculate_spawn_position(direction, validated_lane);

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

        let initial_angle = match direction {
            Direction::North => 0.0,
            Direction::East => 90.0,
            Direction::South => 180.0,
            Direction::West => 270.0,
        };

        Vehicle {
            id,
            position: Point::new(spawn_x as i32, spawn_y as i32),
            position_f: (spawn_x, spawn_y),
            direction,
            lane: validated_lane,
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
        }
    }

    // FIXED: Strict lane validation based on direction and route
    fn validate_lane_for_route(direction: Direction, requested_lane: usize, route: Route) -> usize {
        // PROPER LANE-TO-DIRECTION MAPPING:
        // Each direction has 3 lanes (0=left, 1=middle, 2=right from driver's perspective)

        let valid_lane = match (direction, route) {
            // North-bound vehicles (coming from South):
            (Direction::North, Route::Left) => 0,    // Lane 0 -> turn to West
            (Direction::North, Route::Straight) => 1, // Lane 1 -> continue North
            (Direction::North, Route::Right) => 2,   // Lane 2 -> turn to East

            // South-bound vehicles (coming from North):
            (Direction::South, Route::Left) => 0,    // Lane 0 -> turn to East
            (Direction::South, Route::Straight) => 1, // Lane 1 -> continue South
            (Direction::South, Route::Right) => 2,   // Lane 2 -> turn to West

            // East-bound vehicles (coming from West):
            (Direction::East, Route::Left) => 0,     // Lane 0 -> turn to North
            (Direction::East, Route::Straight) => 1,  // Lane 1 -> continue East
            (Direction::East, Route::Right) => 2,    // Lane 2 -> turn to South

            // West-bound vehicles (coming from East):
            (Direction::West, Route::Left) => 0,     // Lane 0 -> turn to South
            (Direction::West, Route::Straight) => 1,  // Lane 1 -> continue West
            (Direction::West, Route::Right) => 2,    // Lane 2 -> turn to North
        };

        println!("🛣️  Vehicle {:?} {:?} assigned to correct lane {}", direction, route, valid_lane);
        valid_lane
    }

    // FIXED: Get the correct outgoing direction for this vehicle's lane and route
    pub fn get_target_direction(&self) -> Direction {
        match (self.direction, self.route) {
            (Direction::North, Route::Left) => Direction::West,
            (Direction::North, Route::Straight) => Direction::North,
            (Direction::North, Route::Right) => Direction::East,

            (Direction::South, Route::Left) => Direction::East,
            (Direction::South, Route::Straight) => Direction::South,
            (Direction::South, Route::Right) => Direction::West,

            (Direction::East, Route::Left) => Direction::North,
            (Direction::East, Route::Straight) => Direction::East,
            (Direction::East, Route::Right) => Direction::South,

            (Direction::West, Route::Left) => Direction::South,
            (Direction::West, Route::Straight) => Direction::West,
            (Direction::West, Route::Right) => Direction::North,
        }
    }

    // FIXED: Proper lane positioning for 4-way intersection with strict lane discipline
    fn calculate_spawn_position(direction: Direction, lane: usize) -> (f64, f64) {
        let center_x = crate::WINDOW_WIDTH as f64 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f64 / 2.0;
        let half_road_width = Self::ROAD_WIDTH / 2.0; // 90px from center

        // Calculate exact lane position (lanes 0, 1, 2 with 30px spacing)
        let lane_offset = (lane as f64 * Self::LANE_WIDTH) + (Self::LANE_WIDTH / 2.0); // 15, 45, 75 from road edge

        match direction {
            Direction::North => {
                // North-bound vehicles use right side of vertical road (when viewed from above)
                let x = center_x + half_road_width - Self::ROAD_WIDTH + lane_offset;
                let y = crate::WINDOW_HEIGHT as f64 + 100.0; // Spawn from bottom
                (x, y)
            }
            Direction::South => {
                // South-bound vehicles use left side of vertical road
                let x = center_x - half_road_width + Self::ROAD_WIDTH - lane_offset;
                let y = -100.0; // Spawn from top
                (x, y)
            }
            Direction::East => {
                // East-bound vehicles use bottom side of horizontal road
                let x = -100.0; // Spawn from left
                let y = center_y + half_road_width - Self::ROAD_WIDTH + lane_offset;
                (x, y)
            }
            Direction::West => {
                // West-bound vehicles use top side of horizontal road
                let x = crate::WINDOW_WIDTH as f64 + 100.0; // Spawn from right
                let y = center_y - half_road_width + Self::ROAD_WIDTH - lane_offset;
                (x, y)
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

        // Auto-recovery for stuck vehicles
        if self.stuck_timer > 2.0 { // Reduced recovery time
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

    fn adjust_velocity(&mut self, dt: f64) {
        let velocity_diff = self.target_velocity - self.current_velocity;

        if velocity_diff.abs() < 1.0 {
            self.current_velocity = self.target_velocity;
        } else {
            let acceleration = if velocity_diff > 0.0 {
                60.0 // Faster acceleration when speeding up
            } else {
                -100.0 // Faster deceleration when slowing down
            };

            self.current_velocity += acceleration * dt;
            self.current_velocity = self.current_velocity.max(3.0).min(Self::FAST_VELOCITY * 1.3);
        }
    }

    fn move_straight(&mut self, dt: f64) {
        let distance = self.current_velocity * dt;

        match self.direction {
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
        let start_angle = match self.direction {
            Direction::North => 0.0,
            Direction::East => 90.0,
            Direction::South => 180.0,
            Direction::West => 270.0,
        };

        let end_angle = match self.get_turn_direction() {
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

        // IMPROVED: Better turn path calculation
        if self.turning_progress < 0.5 {
            // First half of turn - continue in original direction
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
            (_, Route::Straight) => self.direction,
        }
    }

    fn complete_turn(&mut self) {
        self.direction = self.get_turn_direction();
        self.state = VehicleState::Exiting;
        self.turning_progress = 0.0;

        self.angle = match self.direction {
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

    pub fn try_speed_up(&mut self) {
        let time_since_slowdown = self.last_slow_down_time.elapsed().as_secs_f32();

        if time_since_slowdown > 1.0 { // Reduced from 1.2
            match self.state {
                VehicleState::Approaching => {
                    let target_speed = self.original_velocity.min(Self::MEDIUM_VELOCITY);
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
                    if self.target_velocity < target_speed * 0.9 { // More aggressive
                        self.target_velocity = target_speed * 0.9;
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
            Direction::North => (crate::WINDOW_HEIGHT as f64 + 100.0) - self.position_f.1,
            Direction::South => self.position_f.1 + 100.0,
            Direction::East => self.position_f.0 + 100.0,
            Direction::West => (crate::WINDOW_WIDTH as f64 + 100.0) - self.position_f.0,
        }
    }

    pub fn is_approaching_intersection(&self, _intersection: &Intersection) -> bool {
        let center_x = crate::WINDOW_WIDTH as i32 / 2;
        let center_y = crate::WINDOW_HEIGHT as i32 / 2;
        let approach_distance = 200;

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

        let distance = match self.direction {
            Direction::North => (self.position.y - center_y).max(0) as f64,
            Direction::South => (center_y - self.position.y).max(0) as f64,
            Direction::East => (center_x - self.position.x).max(0) as f64,
            Direction::West => (self.position.x - center_x).max(0) as f64,
        };

        distance / self.current_velocity
    }

    pub fn could_collide_with(&self, other: &Vehicle, intersection: &Intersection) -> bool {
        if self.id == other.id {
            return false;
        }

        let dx = (self.position.x - other.position.x) as f64;
        let dy = (self.position.y - other.position.y) as f64;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance > 300.0 {
            return false;
        }

        if self.is_approaching_intersection(intersection) ||
            other.is_approaching_intersection(intersection) ||
            self.is_in_intersection(intersection) ||
            other.is_in_intersection(intersection) {
            return self.paths_intersect(other);
        }

        false
    }

    // IMPROVED: More accurate path intersection logic based on target destinations
    fn paths_intersect(&self, other: &Vehicle) -> bool {
        // Get the target (outgoing) directions for both vehicles
        let my_target = self.get_target_direction();
        let other_target = other.get_target_direction();

        // Check for path conflicts based on target directions
        match (self.direction, my_target, other.direction, other_target) {
            // Same incoming direction - no conflict if proper lane discipline
            (d1, _, d2, _) if d1 == d2 => {
                // Only conflict if in same lane (which shouldn't happen with proper lane assignment)
                self.lane == other.lane
            }

            // Opposite directions - check for crossing paths
            (Direction::North, target1, Direction::South, target2) |
            (Direction::South, target1, Direction::North, target2) => {
                // Conflict if either vehicle turns left (crossing path)
                target1 == Direction::West || target1 == Direction::East ||
                    target2 == Direction::West || target2 == Direction::East
            }

            (Direction::East, target1, Direction::West, target2) |
            (Direction::West, target1, Direction::East, target2) => {
                // Conflict if either vehicle turns left (crossing path)
                target1 == Direction::North || target1 == Direction::South ||
                    target2 == Direction::North || target2 == Direction::South
            }

            // Perpendicular directions - check for crossing paths
            (Direction::North, target1, Direction::East, target2) => {
                // Conflict if paths cross
                (target1 == Direction::East && target2 == Direction::North) ||
                    (target1 == Direction::West && target2 == Direction::South) ||
                    (self.route == Route::Straight && other.route == Route::Straight)
            }

            (Direction::North, target1, Direction::West, target2) => {
                (target1 == Direction::West && target2 == Direction::North) ||
                    (target1 == Direction::East && target2 == Direction::South) ||
                    (self.route == Route::Straight && other.route == Route::Straight)
            }

            (Direction::South, target1, Direction::East, target2) => {
                (target1 == Direction::East && target2 == Direction::South) ||
                    (target1 == Direction::West && target2 == Direction::North) ||
                    (self.route == Route::Straight && other.route == Route::Straight)
            }

            (Direction::South, target1, Direction::West, target2) => {
                (target1 == Direction::West && target2 == Direction::South) ||
                    (target1 == Direction::East && target2 == Direction::North) ||
                    (self.route == Route::Straight && other.route == Route::Straight)
            }

            // Handle remaining combinations
            _ => {
                // Default: check if any paths actually cross in intersection
                self.route == Route::Left || other.route == Route::Left ||
                    (self.route == Route::Straight && other.route == Route::Straight &&
                        self.direction != other.direction &&
                        self.direction != Self::opposite_direction(other.direction))
            }
        }
    }

    fn opposite_direction(dir: Direction) -> Direction {
        match dir {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        }
    }
}