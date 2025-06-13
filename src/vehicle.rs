// src/vehicle.rs - FIXED: Smooth Bezier curve animations and perfect physics
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

// SMOOTH MATHEMATICS: Vec2 for floating-point precision
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

// SMOOTH TURNING: Bezier curve implementation
#[derive(Debug, Clone)]
pub struct TurningPath {
    pub start: Vec2,
    pub control1: Vec2,
    pub control2: Vec2,
    pub end: Vec2,
    pub total_length: f32,
}

impl TurningPath {
    pub fn new(start: Vec2, end: Vec2, incoming_dir: Direction, outgoing_dir: Direction) -> Self {
        let (control1, control2) = Self::calculate_control_points(start, end, incoming_dir, outgoing_dir);
        let total_length = Self::estimate_curve_length(start, control1, control2, end);

        TurningPath {
            start,
            control1,
            control2,
            end,
            total_length,
        }
    }

    fn calculate_control_points(start: Vec2, end: Vec2, incoming_dir: Direction, outgoing_dir: Direction) -> (Vec2, Vec2) {
        let control_distance = 60.0; // Distance for smooth curves

        let incoming_vector = match incoming_dir {
            Direction::North => Vec2::new(0.0, -1.0),
            Direction::South => Vec2::new(0.0, 1.0),
            Direction::East => Vec2::new(1.0, 0.0),
            Direction::West => Vec2::new(-1.0, 0.0),
        };

        let outgoing_vector = match outgoing_dir {
            Direction::North => Vec2::new(0.0, -1.0),
            Direction::South => Vec2::new(0.0, 1.0),
            Direction::East => Vec2::new(1.0, 0.0),
            Direction::West => Vec2::new(-1.0, 0.0),
        };

        let control1 = start + incoming_vector * control_distance;
        let control2 = end - outgoing_vector * control_distance;

        (control1, control2)
    }

    fn estimate_curve_length(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2) -> f32 {
        // Estimate using chord length
        let chord = (p3 - p0).length();
        let net = (p1 - p0).length() + (p2 - p1).length() + (p3 - p2).length();
        (chord + net) / 2.0
    }

    pub fn cubic_bezier(&self, t: f32) -> Vec2 {
        let u = 1.0 - t;
        let tt = t * t;
        let uu = u * u;
        let uuu = uu * u;
        let ttt = tt * t;

        self.start * uuu +
            self.control1 * (3.0 * uu * t) +
            self.control2 * (3.0 * u * tt) +
            self.end * ttt
    }

    pub fn get_tangent(&self, t: f32) -> Vec2 {
        let u = 1.0 - t;
        let derivative =
            (self.control1 - self.start) * (3.0 * u * u) +
                (self.control2 - self.control1) * (6.0 * u * t) +
                (self.end - self.control2) * (3.0 * t * t);
        derivative.normalize()
    }

    pub fn get_angle_at(&self, t: f32) -> f32 {
        let tangent = self.get_tangent(t);
        tangent.y.atan2(tangent.x).to_degrees()
    }
}

// PERFECT MATHEMATICS CONSTANTS
const LANE_WIDTH: f32 = 30.0;           // Must match main.rs
const SPAWN_DISTANCE: f32 = 200.0;

pub struct Vehicle {
    pub id: u32,
    pub position: Vec2,               // Physics position
    pub interpolated_position: Vec2,  // Rendered position (interpolated)
    pub direction: Direction,         // Where vehicle is coming from
    pub destination: Direction,       // Where vehicle is going to
    pub lane: usize,                 // Lane number (0, 1, 2)
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

    // SMOOTH ANIMATION FIELDS
    pub angle: f32,                  // Physics angle
    pub interpolated_angle: f32,     // Rendered angle (interpolated)
    pub turning_progress: f32,       // Progress through turn (0.0 to 1.0)
    pub turning_path: Option<TurningPath>, // Bezier curve for smooth turns
    current_movement_direction: Direction,

    // PHYSICS FIELDS
    previous_position: Vec2,         // For interpolation
    previous_angle: f32,            // For interpolation
    velocity: Vec2,                 // Current velocity vector

    // LANE POSITIONING
    target_lane_x: f32,             // Target X position for this vehicle's lane
    target_lane_y: f32,             // Target Y position for this vehicle's lane

    // INTERSECTION MANAGEMENT
    has_reserved_intersection: bool,

    // DEBUG TRACKING
    pub path_history: Vec<Vec2>,    // For debugging smooth paths
}

impl Vehicle {
    // Conservative velocity constants
    pub const SLOW_VELOCITY: f32 = 20.0;
    pub const MEDIUM_VELOCITY: f32 = 35.0;
    pub const FAST_VELOCITY: f32 = 50.0;
    pub const SAFE_DISTANCE: f32 = 80.0;

    pub const WIDTH: f32 = 16.0;
    pub const HEIGHT: f32 = 16.0;

    // SMOOTH: Create vehicle with Bezier curve support
    pub fn new_smooth(
        id: u32,
        incoming_direction: Direction,
        destination: Direction,
        lane: usize,
        route: Route,
        color: VehicleColor,
    ) -> Self {
        let (spawn_pos, target_lane_x, target_lane_y) =
            Self::calculate_smooth_positions(incoming_direction, lane);

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
            Direction::North => 0.0,   // Moving up
            Direction::East => 90.0,   // Moving right
            Direction::South => 180.0, // Moving down
            Direction::West => 270.0,  // Moving left
        };

        // Calculate initial velocity vector
        let velocity_direction = match incoming_direction {
            Direction::North => Vec2::new(0.0, -1.0),
            Direction::South => Vec2::new(0.0, 1.0),
            Direction::East => Vec2::new(1.0, 0.0),
            Direction::West => Vec2::new(-1.0, 0.0),
        };

        Vehicle {
            id,
            position: spawn_pos,
            interpolated_position: spawn_pos,
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
            interpolated_angle: initial_angle,
            turning_progress: 0.0,
            turning_path: None,
            current_movement_direction: incoming_direction,
            previous_position: spawn_pos,
            previous_angle: initial_angle,
            velocity: velocity_direction * initial_velocity,
            target_lane_x,
            target_lane_y,
            has_reserved_intersection: false,
            path_history: vec![spawn_pos],
        }
    }

    // SMOOTH: Calculate exact spawn and target positions with floating-point precision
    fn calculate_smooth_positions(direction: Direction, lane: usize) -> (Vec2, f32, f32) {
        let center_x = crate::WINDOW_WIDTH as f32 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f32 / 2.0;

        match direction {
            Direction::North => {
                // Coming from South, moving North
                let target_x = center_x + 15.0 + (lane as f32 * LANE_WIDTH);
                let spawn_y = crate::WINDOW_HEIGHT as f32 + SPAWN_DISTANCE;
                (Vec2::new(target_x, spawn_y), target_x, center_y)
            }
            Direction::South => {
                // Coming from North, moving South
                let target_x = center_x - 15.0 - (lane as f32 * LANE_WIDTH);
                let spawn_y = -SPAWN_DISTANCE;
                (Vec2::new(target_x, spawn_y), target_x, center_y)
            }
            Direction::East => {
                // Coming from West, moving East
                let target_y = center_y + 15.0 + (lane as f32 * LANE_WIDTH);
                let spawn_x = -SPAWN_DISTANCE;
                (Vec2::new(spawn_x, target_y), center_x, target_y)
            }
            Direction::West => {
                // Coming from East, moving West
                let target_y = center_y - 15.0 - (lane as f32 * LANE_WIDTH);
                let spawn_x = crate::WINDOW_WIDTH as f32 + SPAWN_DISTANCE;
                (Vec2::new(spawn_x, target_y), center_x, target_y)
            }
        }
    }

    // FIXED TIMESTEP: Separate physics from rendering
    pub fn update_physics(&mut self, dt: f64, intersection: &Intersection) {
        // Store previous state for interpolation
        self.previous_position = self.position;
        self.previous_angle = self.angle;

        if self.is_in_intersection(intersection) {
            self.time_in_intersection += (dt * 1000.0) as u32;
        }

        self.adjust_velocity(dt);

        match self.state {
            VehicleState::Approaching | VehicleState::Entering => {
                self.move_in_lane_smooth(dt);
            }
            VehicleState::Turning => {
                self.move_turning_smooth(dt);
            }
            VehicleState::Exiting | VehicleState::Completed => {
                self.move_toward_destination_smooth(dt);
            }
        }

        self.update_state(intersection);

        // Record path history for debugging
        if self.path_history.len() > 50 {
            self.path_history.remove(0);
        }
        self.path_history.push(self.position);
    }

    // SMOOTH INTERPOLATION: Update rendered position between physics steps
    pub fn update_interpolation(&mut self, alpha: f32) {
        self.interpolated_position = self.previous_position + (self.position - self.previous_position) * alpha;
        self.interpolated_angle = lerp_angle(self.previous_angle, self.angle, alpha);
    }

    fn adjust_velocity(&mut self, dt: f64) {
        let velocity_diff = self.target_velocity - self.current_velocity;

        if velocity_diff.abs() < 0.5 {
            self.current_velocity = self.target_velocity;
        } else {
            let acceleration = if velocity_diff > 0.0 { 25.0 } else { -80.0 };
            self.current_velocity += acceleration * dt as f32;
            self.current_velocity = self.current_velocity.max(0.0).min(Self::FAST_VELOCITY);
        }

        // Update velocity vector
        let direction_vector = match self.current_movement_direction {
            Direction::North => Vec2::new(0.0, -1.0),
            Direction::South => Vec2::new(0.0, 1.0),
            Direction::East => Vec2::new(1.0, 0.0),
            Direction::West => Vec2::new(-1.0, 0.0),
        };
        self.velocity = direction_vector * self.current_velocity;
    }

    // SMOOTH: Move exactly in lane center with floating-point precision
    fn move_in_lane_smooth(&mut self, dt: f64) {
        let distance = self.current_velocity * dt as f32;

        match self.current_movement_direction {
            Direction::North => {
                self.position.y -= distance;
                self.position.x = self.target_lane_x; // Stay perfectly in lane
                self.angle = 0.0;
            }
            Direction::South => {
                self.position.y += distance;
                self.position.x = self.target_lane_x; // Stay perfectly in lane
                self.angle = 180.0;
            }
            Direction::East => {
                self.position.x += distance;
                self.position.y = self.target_lane_y; // Stay perfectly in lane
                self.angle = 90.0;
            }
            Direction::West => {
                self.position.x -= distance;
                self.position.y = self.target_lane_y; // Stay perfectly in lane
                self.angle = 270.0;
            }
        }
    }

    // SMOOTH: Bezier curve turning with perfect mathematics
    fn move_turning_smooth(&mut self, dt: f64) {
        if let Some(ref turning_path) = self.turning_path {
            let turn_speed = self.current_velocity * 0.7; // Slightly slower when turning
            let progress_increment = turn_speed * dt as f32 / turning_path.total_length;

            self.turning_progress += progress_increment;
            self.turning_progress = self.turning_progress.min(1.0);

            // Smooth position along Bezier curve
            self.position = turning_path.cubic_bezier(self.turning_progress);

            // Smooth rotation aligned with path tangent
            self.angle = turning_path.get_angle_at(self.turning_progress);

            if self.turning_progress >= 1.0 {
                self.complete_turn_smooth();
            }
        }
    }

    fn complete_turn_smooth(&mut self) {
        self.current_movement_direction = self.destination;
        self.state = VehicleState::Exiting;
        self.turning_progress = 0.0;
        self.turning_path = None;
        self.has_reserved_intersection = false;

        // SMOOTH: Update target lane position using perfect mathematics
        self.update_target_lane_for_destination_smooth();

        println!("Vehicle {} completed smooth turn to {:?} - positioned at ({:.1}, {:.1})",
                 self.id, self.destination, self.position.x, self.position.y);
    }

    fn update_target_lane_for_destination_smooth(&mut self) {
        // Calculate appropriate lanes after turning
        let destination_lane = match (self.direction, self.route) {
            // Left turns use right-side lanes in new direction
            (Direction::North, Route::Left) => 2,
            (Direction::South, Route::Left) => 2,
            (Direction::East, Route::Left) => 2,
            (Direction::West, Route::Left) => 2,

            // Right turns use left-side lanes in new direction
            (Direction::North, Route::Right) => 0,
            (Direction::South, Route::Right) => 0,
            (Direction::East, Route::Right) => 0,
            (Direction::West, Route::Right) => 0,

            // Straight traffic keeps middle lane
            _ => 1,
        };

        match self.destination {
            Direction::North => {
                self.target_lane_x = self.get_perfect_lane_center_x(Direction::North, destination_lane);
                self.target_lane_y = crate::WINDOW_HEIGHT as f32 / 2.0;
            }
            Direction::South => {
                self.target_lane_x = self.get_perfect_lane_center_x(Direction::South, destination_lane);
                self.target_lane_y = crate::WINDOW_HEIGHT as f32 / 2.0;
            }
            Direction::East => {
                self.target_lane_x = crate::WINDOW_WIDTH as f32 / 2.0;
                self.target_lane_y = self.get_perfect_lane_center_y(Direction::East, destination_lane);
            }
            Direction::West => {
                self.target_lane_x = crate::WINDOW_WIDTH as f32 / 2.0;
                self.target_lane_y = self.get_perfect_lane_center_y(Direction::West, destination_lane);
            }
        }
    }

    // PERFECT: Same mathematical functions as main.rs for consistency
    fn get_perfect_lane_center_x(&self, direction: Direction, lane: usize) -> f32 {
        let center_x = crate::WINDOW_WIDTH as f32 / 2.0;
        match direction {
            Direction::North => {
                center_x + 15.0 + (lane as f32 * LANE_WIDTH)
            }
            Direction::South => {
                center_x - 15.0 - (lane as f32 * LANE_WIDTH)
            }
            _ => center_x,
        }
    }

    fn get_perfect_lane_center_y(&self, direction: Direction, lane: usize) -> f32 {
        let center_y = crate::WINDOW_HEIGHT as f32 / 2.0;
        match direction {
            Direction::East => {
                center_y + 15.0 + (lane as f32 * LANE_WIDTH)
            }
            Direction::West => {
                center_y - 15.0 - (lane as f32 * LANE_WIDTH)
            }
            _ => center_y,
        }
    }

    // SMOOTH: Move toward final destination with gradual lane correction
    fn move_toward_destination_smooth(&mut self, dt: f64) {
        let distance = self.current_velocity * dt as f32;

        match self.destination {
            Direction::North => {
                self.position.y -= distance;
                // Gradual lane correction
                let target_x = self.target_lane_x;
                let diff_x = target_x - self.position.x;
                if diff_x.abs() > 1.0 {
                    self.position.x += diff_x * 0.1;
                } else {
                    self.position.x = target_x;
                }
                self.angle = 0.0;
            }
            Direction::South => {
                self.position.y += distance;
                let target_x = self.target_lane_x;
                let diff_x = target_x - self.position.x;
                if diff_x.abs() > 1.0 {
                    self.position.x += diff_x * 0.1;
                } else {
                    self.position.x = target_x;
                }
                self.angle = 180.0;
            }
            Direction::East => {
                self.position.x += distance;
                let target_y = self.target_lane_y;
                let diff_y = target_y - self.position.y;
                if diff_y.abs() > 1.0 {
                    self.position.y += diff_y * 0.1;
                } else {
                    self.position.y = target_y;
                }
                self.angle = 90.0;
            }
            Direction::West => {
                self.position.x -= distance;
                let target_y = self.target_lane_y;
                let diff_y = target_y - self.position.y;
                if diff_y.abs() > 1.0 {
                    self.position.y += diff_y * 0.1;
                } else {
                    self.position.y = target_y;
                }
                self.angle = 270.0;
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
                if distance_to_center < intersection_radius + 70.0 {
                    self.state = VehicleState::Entering;
                    println!("Vehicle {} entering intersection", self.id);
                }
            }
            VehicleState::Entering => {
                if distance_to_center < 50.0 {
                    if self.route != Route::Straight {
                        self.state = VehicleState::Turning;
                        self.turning_progress = 0.0;
                        self.create_turning_path();
                        println!("Vehicle {} starting smooth {:?} turn", self.id, self.route);
                    } else {
                        self.state = VehicleState::Exiting;
                        println!("Vehicle {} going straight through", self.id);
                    }
                }
            }
            VehicleState::Turning => {
                // Handled in move_turning_smooth method
            }
            VehicleState::Exiting => {
                if distance_to_center > intersection_radius + 90.0 {
                    self.state = VehicleState::Completed;
                    println!("Vehicle {} completed intersection", self.id);
                }
            }
            VehicleState::Completed => {
                // Continue moving until off screen
            }
        }
    }

    // SMOOTH: Create Bezier curve for turning
    fn create_turning_path(&mut self) {
        let center_x = crate::WINDOW_WIDTH as f32 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f32 / 2.0;
        let intersection_center = Vec2::new(center_x, center_y);

        // Calculate start and end points for the turn
        let start_point = self.position;
        let end_point = self.calculate_turn_end_point();

        // Create smooth Bezier path
        self.turning_path = Some(TurningPath::new(
            start_point,
            end_point,
            self.direction,
            self.destination,
        ));

        println!("Vehicle {} created smooth turning path from ({:.1}, {:.1}) to ({:.1}, {:.1})",
                 self.id, start_point.x, start_point.y, end_point.x, end_point.y);
    }

    fn calculate_turn_end_point(&self) -> Vec2 {
        let center_x = crate::WINDOW_WIDTH as f32 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f32 / 2.0;

        // Calculate exit point based on destination direction
        match self.destination {
            Direction::North => {
                let target_x = self.get_perfect_lane_center_x(Direction::North, 1);
                Vec2::new(target_x, center_y - 45.0) // Exit north side
            }
            Direction::South => {
                let target_x = self.get_perfect_lane_center_x(Direction::South, 1);
                Vec2::new(target_x, center_y + 45.0) // Exit south side
            }
            Direction::East => {
                let target_y = self.get_perfect_lane_center_y(Direction::East, 1);
                Vec2::new(center_x + 45.0, target_y) // Exit east side
            }
            Direction::West => {
                let target_y = self.get_perfect_lane_center_y(Direction::West, 1);
                Vec2::new(center_x - 45.0, target_y) // Exit west side
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
        self.position.x >= -300.0 &&
            self.position.x <= (crate::WINDOW_WIDTH as f32 + 300.0) &&
            self.position.y >= -300.0 &&
            self.position.y <= (crate::WINDOW_HEIGHT as f32 + 300.0)
    }

    pub fn distance_from_spawn(&self) -> f64 {
        match self.direction {
            Direction::North => (crate::WINDOW_HEIGHT as f64 + SPAWN_DISTANCE as f64) - self.position.y as f64,
            Direction::South => self.position.y as f64 + SPAWN_DISTANCE as f64,
            Direction::East => self.position.x as f64 + SPAWN_DISTANCE as f64,
            Direction::West => (crate::WINDOW_WIDTH as f64 + SPAWN_DISTANCE as f64) - self.position.x as f64,
        }
    }

    pub fn is_approaching_intersection(&self, intersection: &Intersection) -> bool {
        let center_x = crate::WINDOW_WIDTH as f32 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f32 / 2.0;
        let approach_distance = 150.0;

        match self.current_movement_direction {
            Direction::North => {
                self.position.y > center_y &&
                    self.position.y < center_y + approach_distance &&
                    (self.position.x - center_x).abs() < 120.0
            }
            Direction::South => {
                self.position.y < center_y &&
                    self.position.y > center_y - approach_distance &&
                    (self.position.x - center_x).abs() < 120.0
            }
            Direction::East => {
                self.position.x < center_x &&
                    self.position.x > center_x - approach_distance &&
                    (self.position.y - center_y).abs() < 120.0
            }
            Direction::West => {
                self.position.x > center_x &&
                    self.position.x < center_x + approach_distance &&
                    (self.position.y - center_y).abs() < 120.0
            }
        }
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

        let distance = match self.current_movement_direction {
            Direction::North => (self.position.y - center_y).max(0.0),
            Direction::South => (center_y - self.position.y).max(0.0),
            Direction::East => (center_x - self.position.x).max(0.0),
            Direction::West => (self.position.x - center_x).max(0.0),
        };

        distance as f64 / self.current_velocity as f64
    }

    // SMOOTH: Collision detection based on exact lane mathematics
    pub fn could_collide_with(&self, other: &Vehicle, intersection: &Intersection) -> bool {
        if self.id == other.id {
            return false;
        }

        let dx = self.position.x - other.position.x;
        let dy = self.position.y - other.position.y;
        let distance = (dx * dx + dy * dy).sqrt();

        // Critical proximity
        if distance < 25.0 {
            return true;
        }

        // Same lane same direction
        if self.direction == other.direction && self.lane == other.lane {
            return self.is_vehicle_ahead_in_same_lane(other);
        }

        // Only actual path intersections for intersection conflicts
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

        distance_ahead > 0.0 && distance_ahead < Self::SAFE_DISTANCE
    }

    // PERFECT: Only real intersecting paths
    fn will_paths_actually_intersect(&self, other: &Vehicle) -> bool {
        // Same direction same lane = conflict
        if self.direction == other.direction {
            return self.lane == other.lane;
        }

        // Right turns use tight curves - no conflicts
        if self.route == Route::Right || other.route == Route::Right {
            return false;
        }

        // Straight traffic is completely separated
        if self.route == Route::Straight && other.route == Route::Straight {
            return false;
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

// SMOOTH: Angle interpolation with proper wrapping
fn lerp_angle(start: f32, end: f32, t: f32) -> f32 {
    let mut diff = end - start;

    // Handle angle wrapping
    if diff > 180.0 {
        diff -= 360.0;
    } else if diff < -180.0 {
        diff += 360.0;
    }

    let result = start + diff * t;

    // Normalize to 0-360 range
    if result < 0.0 {
        result + 360.0
    } else if result >= 360.0 {
        result - 360.0
    } else {
        result
    }
}