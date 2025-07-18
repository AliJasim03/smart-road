use crate::intersection::Intersection;
use crate::{SpawnType, HALF_ROAD_WIDTH, LANE_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North = 0,
    South = 2,
    East = 1,
    West = 3,
}
// +++ ADD THIS IMPLEMENTATION BLOCK +++
impl Direction {
    pub fn is_opposite(self, other: Direction) -> bool {
        (self as i32 - other as i32).abs() == 2
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Route {
    Left,
    Straight,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VehicleState {
    Approaching,
    Entering,
    Turning,
    Exiting,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VelocityLevel {
    Stop,
    Slow,
    Medium,
    Fast,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VehicleColor {
    Red,
    Blue,
    Green,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self { Vec2 { x, y } }
    pub fn length(&self) -> f32 { (self.x * self.x + self.y * self.y).sqrt() }
}
impl std::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, other: Self) -> Self { Vec2::new(self.x - other.x, self.y - other.y) }
}

pub struct Vehicle {
    pub id: u32,
    pub position: Vec2,
    pub direction: Direction,
    pub destination: Direction,
    pub lane: usize,
    pub route: Route,
    pub color: VehicleColor,
    pub state: VehicleState,
    pub velocity_level: VelocityLevel,
    pub current_velocity: f32,
    pub target_velocity: f32,
    pub width: f32,
    pub height: f32,
    pub time_in_intersection: u32,
    pub turn_point: Vec2,
    pub target_lane_pos: Vec2,
    current_movement_dir: Direction,
    pub has_passage_grant: bool,
    pub time_to_intersection: f32,
}

impl Vehicle {
    pub const STOP_VELOCITY: f32 = 0.0;
    pub const SLOW_VELOCITY: f32 = 40.0;
    pub const MEDIUM_VELOCITY: f32 = 90.0;
    pub const FAST_VELOCITY: f32 = 120.0;

    // --- MODIFIED: Increased the physics dimensions of the car. ---
    // This creates a larger "safety bubble" for all calculations and drawing.
    pub const ACCELERATION: f32 = 60.0;

    pub const WIDTH: f32 = 30.0;
    pub const HEIGHT: f32 = 48.0;

    pub fn new(
        id: u32,
        direction: Direction,
        destination: Direction,
        lane: usize,
        route: Route,
        color: VehicleColor,
        spawn_type: SpawnType,
    ) -> Self {
        let (spawn_pos, initial_target) = Self::calculate_spawn_and_target(direction, lane);
        let turn_point = Self::calculate_turn_point(direction, lane, route);

        let (initial_velocity, initial_target_velocity, initial_velocity_level) = match spawn_type {
            SpawnType::Manual => (Self::STOP_VELOCITY, Self::STOP_VELOCITY, VelocityLevel::Stop),
            SpawnType::Automatic => (Self::MEDIUM_VELOCITY, Self::MEDIUM_VELOCITY, VelocityLevel::Medium),
        };

        Vehicle {
            id,
            position: spawn_pos,
            direction,
            destination,
            lane,
            route,
            color,
            state: VehicleState::Approaching,
            velocity_level: initial_velocity_level,
            current_velocity: initial_velocity,
            target_velocity: initial_target_velocity,
            width: Self::WIDTH,
            height: Self::HEIGHT,
            time_in_intersection: 0,
            turn_point,
            target_lane_pos: initial_target,
            current_movement_dir: direction,
            has_passage_grant: false,
            time_to_intersection: f32::MAX,
        }
    }

    pub fn get_current_movement_direction(&self) -> Direction {
        self.current_movement_dir
    }

    pub fn is_in_intersection(&self, intersection: &Intersection) -> bool {
        intersection.is_point_in_core(self.position.x, self.position.y)
    }

    pub fn is_approaching_intersection(&self, intersection: &Intersection) -> bool {
        self.state == VehicleState::Approaching
            && intersection.is_point_in_approach_zone(self.position.x, self.position.y)
    }

    fn calculate_spawn_and_target(direction: Direction, lane: usize) -> (Vec2, Vec2) {
        let center_x = WINDOW_WIDTH as f32 / 2.0;
        let center_y = WINDOW_HEIGHT as f32 / 2.0;
        let spawn_margin = 100.0;
        let offset_from_road_edge = LANE_WIDTH * (lane as f32 + 0.5);

        let (lane_pos_x, lane_pos_y) = match direction {
            Direction::North => (center_x + HALF_ROAD_WIDTH - offset_from_road_edge, 0.0),
            Direction::South => (center_x - HALF_ROAD_WIDTH + offset_from_road_edge, 0.0),
            Direction::East => (0.0, center_y + HALF_ROAD_WIDTH - offset_from_road_edge),
            Direction::West => (0.0, center_y - HALF_ROAD_WIDTH + offset_from_road_edge),
        };

        match direction {
            Direction::North => (
                Vec2::new(lane_pos_x, WINDOW_HEIGHT as f32 + spawn_margin),
                Vec2::new(lane_pos_x, -spawn_margin),
            ),
            Direction::South => (
                Vec2::new(lane_pos_x, -spawn_margin),
                Vec2::new(lane_pos_x, WINDOW_HEIGHT as f32 + spawn_margin),
            ),
            Direction::East => (
                Vec2::new(-spawn_margin, lane_pos_y),
                Vec2::new(WINDOW_WIDTH as f32 + spawn_margin, lane_pos_y),
            ),
            Direction::West => (
                Vec2::new(WINDOW_WIDTH as f32 + spawn_margin, lane_pos_y),
                Vec2::new(-spawn_margin, lane_pos_y),
            ),
        }
    }

    fn calculate_turn_point(direction: Direction, lane: usize, route: Route) -> Vec2 {
        if route == Route::Straight { return Vec2::new(-1000.0, -1000.0); }
        let center_x = WINDOW_WIDTH as f32 / 2.0;
        let center_y = WINDOW_HEIGHT as f32 / 2.0;
        let lane_center_offset = HALF_ROAD_WIDTH - LANE_WIDTH * (lane as f32 + 0.5);

        match direction {
            Direction::North => Vec2::new(
                center_x + lane_center_offset,
                if route == Route::Right { center_y + lane_center_offset } else { center_y - lane_center_offset },
            ),
            Direction::South => Vec2::new(
                center_x - lane_center_offset,
                if route == Route::Right { center_y - lane_center_offset } else { center_y + lane_center_offset },
            ),
            Direction::East => Vec2::new(
                if route == Route::Right { center_x - lane_center_offset } else { center_x + lane_center_offset },
                center_y + lane_center_offset,
            ),
            Direction::West => Vec2::new(
                if route == Route::Right { center_x + lane_center_offset } else { center_x - lane_center_offset },
                center_y - lane_center_offset,
            ),
        }
    }

    pub fn update_physics(&mut self, dt: f64, intersection: &Intersection) {
        if self.is_in_intersection(intersection) && self.state != VehicleState::Completed {
            self.time_in_intersection += (dt * 1000.0) as u32;
        }

        let accel = 60.0;
        let decel = 120.0;
        let diff = self.target_velocity - self.current_velocity;

        if diff > 1.0 { self.current_velocity += accel * dt as f32; }
        else if diff < -1.0 { self.current_velocity -= decel * dt as f32; }
        else { self.current_velocity = self.target_velocity; }

        self.current_velocity = self.current_velocity.max(0.0);
        let distance = self.current_velocity * dt as f32;

        match self.current_movement_dir {
            Direction::North => self.position.y -= distance,
            Direction::South => self.position.y += distance,
            Direction::East => self.position.x += distance,
            Direction::West => self.position.x -= distance,
        }
        self.update_state(intersection);
    }

    fn update_state(&mut self, intersection: &Intersection) {
        let distance_to_turn_point = (self.position - self.turn_point).length();
        let is_at_turn_point = distance_to_turn_point < (self.current_velocity * 0.05).max(3.0);

        match self.state {
            VehicleState::Approaching => {
                if self.is_in_intersection(intersection) { self.state = VehicleState::Entering; }
            }
            VehicleState::Entering => {
                if self.route != Route::Straight && is_at_turn_point { self.state = VehicleState::Turning; }
                else if self.route == Route::Straight && !self.is_in_intersection(intersection) { self.state = VehicleState::Exiting; }
            }
            VehicleState::Turning => {
                self.position = self.turn_point;
                self.current_movement_dir = self.destination;
                self.target_lane_pos = Self::calculate_spawn_and_target(self.destination, self.lane).1;
                self.state = VehicleState::Exiting;
            }
            VehicleState::Exiting => {
                if self.position.x < -150.0 || self.position.x > WINDOW_WIDTH as f32 + 150.0 ||
                    self.position.y < -150.0 || self.position.y > WINDOW_HEIGHT as f32 + 150.0 {
                    self.state = VehicleState::Completed;
                }
            }
            _ => {}
        }

        if self.state == VehicleState::Exiting {
            match self.current_movement_dir {
                Direction::North | Direction::South => {
                    if (self.position.x - self.target_lane_pos.x).abs() > 0.5 {
                        self.position.x += (self.target_lane_pos.x - self.position.x) * 0.1;
                    }
                }
                Direction::East | Direction::West => {
                    if (self.position.y - self.target_lane_pos.y).abs() > 0.5 {
                        self.position.y += (self.target_lane_pos.y - self.position.y) * 0.1;
                    }
                }
            }
        }
    }

    pub fn set_target_velocity(&mut self, level: VelocityLevel) {
        self.target_velocity = match level {
            VelocityLevel::Stop => Self::STOP_VELOCITY,
            VelocityLevel::Slow => Self::SLOW_VELOCITY,
            VelocityLevel::Medium => Self::MEDIUM_VELOCITY,
            VelocityLevel::Fast => Self::FAST_VELOCITY,
        };
        self.velocity_level = level;
    }
}