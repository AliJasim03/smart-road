// src/intersection.rs - ENHANCED: Floating-point geometry and improved lane calculations
use crate::vehicle::{Vehicle, Direction, Route, VehicleState, VelocityLevel, Vec2};
use std::collections::VecDeque;

pub struct Intersection {
    pub center_x: f32,
    pub center_y: f32,
    pub size: f32,
    pub approach_distance: f32,
    pub core_radius: f32,
}

impl Intersection {
    pub fn new() -> Self {
        Intersection {
            center_x: crate::WINDOW_WIDTH as f32 / 2.0,
            center_y: crate::WINDOW_HEIGHT as f32 / 2.0,
            size: 180.0, // Total intersection area
            approach_distance: 150.0, // Distance to start slowing down
            core_radius: 90.0, // Core intersection radius
        }
    }

    // ENHANCED: Support floating-point positions
    pub fn is_point_in_intersection(&self, x: i32, y: i32) -> bool {
        self.is_point_in_intersection_f(x as f32, y as f32)
    }

    pub fn is_point_in_intersection_f(&self, x: f32, y: f32) -> bool {
        let dx = (x - self.center_x).abs();
        let dy = (y - self.center_y).abs();
        dx < self.size / 2.0 && dy < self.size / 2.0
    }

    pub fn is_point_in_core(&self, x: f32, y: f32) -> bool {
        let distance = self.distance_to_center(x, y);
        distance < self.core_radius
    }

    pub fn is_point_in_approach_zone(&self, x: f32, y: f32) -> bool {
        let distance = self.distance_to_center(x, y);
        distance < (self.core_radius + self.approach_distance) &&
            distance >= self.core_radius
    }

    pub fn distance_to_center(&self, x: f32, y: f32) -> f32 {
        let dx = x - self.center_x;
        let dy = y - self.center_y;
        (dx * dx + dy * dy).sqrt()
    }

    // ENHANCED: Get intersection zone for a vehicle with floating-point precision
    pub fn get_vehicle_zone(&self, vehicle: &Vehicle) -> IntersectionZone {
        let distance = self.distance_to_center(vehicle.position.x, vehicle.position.y);

        if distance < self.core_radius - 20.0 {
            IntersectionZone::Core
        } else if distance < self.core_radius + 30.0 {
            if vehicle.state == VehicleState::Turning {
                IntersectionZone::Turning
            } else {
                IntersectionZone::Entry
            }
        } else if distance < (self.core_radius + self.approach_distance) {
            IntersectionZone::Approach
        } else {
            IntersectionZone::Clear
        }
    }

    // ENHANCED: Check if vehicle paths will intersect with improved collision detection
    pub fn do_paths_intersect(&self, vehicle1: &Vehicle, vehicle2: &Vehicle) -> bool {
        // Same direction same lane = same path
        if vehicle1.direction == vehicle2.direction && vehicle1.lane == vehicle2.lane {
            return true;
        }

        // ENHANCED: Straight traffic in separated lanes never intersects
        if vehicle1.route == Route::Straight && vehicle2.route == Route::Straight {
            return false; // Separated by design in 6-lane system
        }

        // Check specific turning conflicts using improved geometry
        self.check_turning_conflicts_enhanced(vehicle1, vehicle2)
    }

    fn check_turning_conflicts_enhanced(&self, vehicle1: &Vehicle, vehicle2: &Vehicle) -> bool {
        // Enhanced conflict detection considering actual vehicle positions and timing
        let time1 = vehicle1.time_to_intersection(self);
        let time2 = vehicle2.time_to_intersection(self);

        // If vehicles arrive at very different times, no conflict
        if (time1 - time2).abs() > 5.0 {
            return false;
        }

        match (vehicle1.direction, vehicle1.route, vehicle2.direction, vehicle2.route) {
            // Left turner vs straight traffic from perpendicular direction
            (Direction::North, Route::Left, Direction::East, Route::Straight) => true,
            (Direction::South, Route::Left, Direction::West, Route::Straight) => true,
            (Direction::East, Route::Left, Direction::South, Route::Straight) => true,
            (Direction::West, Route::Left, Direction::North, Route::Straight) => true,

            // Reverse cases
            (Direction::East, Route::Straight, Direction::North, Route::Left) => true,
            (Direction::West, Route::Straight, Direction::South, Route::Left) => true,
            (Direction::South, Route::Straight, Direction::East, Route::Left) => true,
            (Direction::North, Route::Straight, Direction::West, Route::Left) => true,

            // Opposing left turns can conflict in intersection center
            (Direction::North, Route::Left, Direction::South, Route::Left) => true,
            (Direction::East, Route::Left, Direction::West, Route::Left) => true,

            // Right turns generally don't conflict (tight radius)
            _ => false,
        }
    }

    // ENHANCED: Get safe following distance based on zone with floating-point precision
    pub fn get_safe_distance_for_zone(&self, zone: IntersectionZone) -> f32 {
        match zone {
            IntersectionZone::Core | IntersectionZone::Turning => 60.0,  // Closer in intersection
            IntersectionZone::Entry => 80.0,                             // Medium distance
            IntersectionZone::Approach => 120.0,                         // Longer distance when approaching
            IntersectionZone::Clear => 100.0,                            // Standard distance
        }
    }

    // ENHANCED: Calculate time for vehicle to clear intersection with precise geometry
    pub fn time_to_clear_intersection(&self, vehicle: &Vehicle) -> f32 {
        if vehicle.current_velocity <= 0.0 {
            return f32::INFINITY;
        }

        let distance_to_exit = match vehicle.route {
            Route::Straight => self.size,                    // Just go through
            Route::Left | Route::Right => self.size * 1.4,   // Turning path is longer
        };

        distance_to_exit / vehicle.current_velocity
    }

    // ENHANCED: Check if intersection is clear for a vehicle to enter with improved geometry
    pub fn is_clear_for_entry(&self, entering_vehicle: &Vehicle, all_vehicles: &VecDeque<Vehicle>) -> bool {
        for other in all_vehicles {
            if other.id == entering_vehicle.id {
                continue;
            }

            // Check vehicles currently in intersection
            if self.is_point_in_intersection_f(other.position.x, other.position.y) {
                if self.do_paths_intersect(entering_vehicle, other) {
                    return false;
                }
            }

            // Check vehicles about to enter
            if self.is_point_in_approach_zone(other.position.x, other.position.y) {
                if self.do_paths_intersect(entering_vehicle, other) {
                    let time_diff = (entering_vehicle.time_to_intersection(self) - other.time_to_intersection(self)).abs();
                    if time_diff < 3.0 { // 3 second buffer
                        return false;
                    }
                }
            }
        }

        true
    }

    // ENHANCED: Get recommended velocity for intersection zone
    pub fn get_recommended_velocity(&self, vehicle: &Vehicle) -> crate::vehicle::VelocityLevel {
        let zone = self.get_vehicle_zone(vehicle);

        match zone {
            IntersectionZone::Clear => {
                if vehicle.state == VehicleState::Completed {
                    VelocityLevel::Fast
                } else {
                    VelocityLevel::Medium
                }
            }
            IntersectionZone::Approach => VelocityLevel::Medium,
            IntersectionZone::Entry => VelocityLevel::Slow,
            IntersectionZone::Core | IntersectionZone::Turning => VelocityLevel::Slow,
        }
    }

    // ENHANCED: Check for potential conflicts ahead with improved precision
    pub fn check_conflicts_ahead(&self, vehicle: &Vehicle, look_ahead_distance: f32, all_vehicles: &VecDeque<Vehicle>) -> Vec<u32> {
        let mut conflicts = Vec::new();

        for other in all_vehicles {
            if other.id == vehicle.id {
                continue;
            }

            let distance = vehicle.distance_from_spawn() as f32;
            if distance < look_ahead_distance {
                if self.do_paths_intersect(vehicle, other) {
                    conflicts.push(other.id);
                }
            }
        }

        conflicts
    }

    // NEW: Calculate lane geometry for perfect lane rendering
    pub fn calculate_lane_geometry(&self, direction: Direction, lane: usize) -> LaneGeometry {
        let lane_width = 30.0;
        let road_center = match direction {
            Direction::North | Direction::South => Vec2::new(self.center_x, self.center_y),
            Direction::East | Direction::West => Vec2::new(self.center_x, self.center_y),
        };

        let road_direction = match direction {
            Direction::North => Vec2::new(0.0, -1.0),
            Direction::South => Vec2::new(0.0, 1.0),
            Direction::East => Vec2::new(1.0, 0.0),
            Direction::West => Vec2::new(-1.0, 0.0),
        };

        let perpendicular = Vec2::new(-road_direction.y, road_direction.x);

        // Calculate lane center offset from road center
        let lane_offset = match direction {
            Direction::North => {
                // Right side of vertical road
                15.0 + (lane as f32 * lane_width)
            }
            Direction::South => {
                // Left side of vertical road
                -15.0 - (lane as f32 * lane_width)
            }
            Direction::East => {
                // Bottom side of horizontal road
                15.0 + (lane as f32 * lane_width)
            }
            Direction::West => {
                // Top side of horizontal road
                -15.0 - (lane as f32 * lane_width)
            }
        };

        let lane_center = match direction {
            Direction::North | Direction::South => {
                Vec2::new(self.center_x + lane_offset, self.center_y)
            }
            Direction::East | Direction::West => {
                Vec2::new(self.center_x, self.center_y + lane_offset)
            }
        };

        // Calculate divider positions
        let half_width = lane_width / 2.0;
        let left_divider = lane_center + perpendicular * half_width;
        let right_divider = lane_center - perpendicular * half_width;

        LaneGeometry {
            center_line: lane_center,
            left_divider,
            right_divider,
            width: lane_width,
            direction,
            lane_index: lane,
        }
    }

    // NEW: Get all lane geometries for rendering
    pub fn get_all_lane_geometries(&self) -> Vec<LaneGeometry> {
        let mut geometries = Vec::new();

        for direction in [Direction::North, Direction::South, Direction::East, Direction::West] {
            for lane in 0..3 {
                geometries.push(self.calculate_lane_geometry(direction, lane));
            }
        }

        geometries
    }

    // NEW: Calculate intersection boundary points for lane rendering
    pub fn get_intersection_boundary_points(&self, direction: Direction) -> (Vec2, Vec2) {
        let half_size = self.size / 2.0;

        match direction {
            Direction::North => {
                // Bottom edge of intersection
                (Vec2::new(self.center_x - half_size, self.center_y + half_size),
                 Vec2::new(self.center_x + half_size, self.center_y + half_size))
            }
            Direction::South => {
                // Top edge of intersection
                (Vec2::new(self.center_x - half_size, self.center_y - half_size),
                 Vec2::new(self.center_x + half_size, self.center_y - half_size))
            }
            Direction::East => {
                // Left edge of intersection
                (Vec2::new(self.center_x - half_size, self.center_y - half_size),
                 Vec2::new(self.center_x - half_size, self.center_y + half_size))
            }
            Direction::West => {
                // Right edge of intersection
                (Vec2::new(self.center_x + half_size, self.center_y - half_size),
                 Vec2::new(self.center_x + half_size, self.center_y + half_size))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntersectionZone {
    Clear,      // Outside intersection influence
    Approach,   // Approaching intersection
    Entry,      // Entering intersection
    Core,       // In core intersection area
    Turning,    // Currently turning
}

// NEW: Lane geometry structure for improved rendering
#[derive(Debug, Clone)]
pub struct LaneGeometry {
    pub center_line: Vec2,
    pub left_divider: Vec2,
    pub right_divider: Vec2,
    pub width: f32,
    pub direction: Direction,
    pub lane_index: usize,
}

impl LaneGeometry {
    pub fn get_lane_color(&self) -> (u8, u8, u8) {
        match self.lane_index {
            0 => (255, 100, 100), // Red - Left turn
            1 => (100, 100, 255), // Blue - Straight
            2 => (100, 255, 100), // Green - Right turn
            _ => (128, 128, 128),  // Gray - Unknown
        }
    }

    pub fn get_route_type(&self) -> Route {
        match self.lane_index {
            0 => Route::Left,
            1 => Route::Straight,
            2 => Route::Right,
            _ => Route::Straight,
        }
    }
}

// ENHANCED: Traffic flow analysis with floating-point precision
pub struct TrafficFlowAnalyzer {
    throughput_by_direction: [u32; 4],
    congestion_by_direction: [f32; 4],
    average_wait_times: [f32; 4],
    last_analysis_time: std::time::Instant,
    vehicle_positions: Vec<(u32, Vec2, f32)>, // (id, position, velocity) for tracking
}

impl TrafficFlowAnalyzer {
    pub fn new() -> Self {
        TrafficFlowAnalyzer {
            throughput_by_direction: [0; 4],
            congestion_by_direction: [0.0; 4],
            average_wait_times: [0.0; 4],
            last_analysis_time: std::time::Instant::now(),
            vehicle_positions: Vec::new(),
        }
    }

    pub fn analyze_flow(&mut self, vehicles: &VecDeque<Vehicle>, intersection: &Intersection) {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_analysis_time).as_secs_f32();

        if elapsed < 1.0 {
            return; // Analyze every second
        }

        // Reset counters
        self.congestion_by_direction = [0.0; 4];
        let mut wait_times = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];

        // Update vehicle positions for velocity tracking
        self.vehicle_positions.clear();
        for vehicle in vehicles {
            self.vehicle_positions.push((vehicle.id, vehicle.position, vehicle.current_velocity));
        }

        // Analyze current vehicles
        for vehicle in vehicles {
            let dir_index = match vehicle.direction {
                Direction::North => 0,
                Direction::South => 1,
                Direction::East => 2,
                Direction::West => 3,
            };

            // Count congestion (vehicles approaching or in intersection)
            if vehicle.is_approaching_intersection(intersection) || vehicle.is_in_intersection(intersection) {
                self.congestion_by_direction[dir_index] += 1.0;
            }

            // Calculate wait time (slow vehicles are "waiting")
            if vehicle.current_velocity < Vehicle::SLOW_VELOCITY * 0.8 {
                let wait_time = vehicle.start_time.elapsed().as_secs_f32();
                wait_times[dir_index].push(wait_time);
            }
        }

        // Calculate average wait times
        for i in 0..4 {
            if !wait_times[i].is_empty() {
                self.average_wait_times[i] = wait_times[i].iter().sum::<f32>() / wait_times[i].len() as f32;
            }
        }

        self.last_analysis_time = now;
    }

    pub fn get_most_congested_direction(&self) -> Direction {
        let max_index = self.congestion_by_direction
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        match max_index {
            0 => Direction::North,
            1 => Direction::South,
            2 => Direction::East,
            _ => Direction::West,
        }
    }

    pub fn get_congestion_level(&self, direction: Direction) -> f32 {
        let index = match direction {
            Direction::North => 0,
            Direction::South => 1,
            Direction::East => 2,
            Direction::West => 3,
        };
        self.congestion_by_direction[index]
    }

    pub fn record_vehicle_completion(&mut self, direction: Direction) {
        let index = match direction {
            Direction::North => 0,
            Direction::South => 1,
            Direction::East => 2,
            Direction::West => 3,
        };
        self.throughput_by_direction[index] += 1;
    }

    pub fn get_throughput(&self, direction: Direction) -> u32 {
        let index = match direction {
            Direction::North => 0,
            Direction::South => 1,
            Direction::East => 2,
            Direction::West => 3,
        };
        self.throughput_by_direction[index]
    }

    pub fn print_flow_analysis(&self) {
        println!("\n=== ENHANCED TRAFFIC FLOW ANALYSIS ===");
        for (i, direction) in [Direction::North, Direction::South, Direction::East, Direction::West].iter().enumerate() {
            println!("{:?}: Congestion: {:.1}, Throughput: {}, Avg Wait: {:.1}s",
                     direction,
                     self.congestion_by_direction[i],
                     self.throughput_by_direction[i],
                     self.average_wait_times[i]);
        }
        println!("======================================\n");
    }

    // NEW: Get efficiency metrics
    pub fn get_efficiency_metrics(&self) -> (f32, f32, f32) {
        let total_throughput = self.throughput_by_direction.iter().sum::<u32>() as f32;
        let average_congestion = self.congestion_by_direction.iter().sum::<f32>() / 4.0;
        let average_wait_time = self.average_wait_times.iter().sum::<f32>() / 4.0;

        (total_throughput, average_congestion, average_wait_time)
    }
}

// ENHANCED: Helper functions for intersection management with floating-point precision
pub fn intersection_center() -> (f32, f32) {
    (crate::WINDOW_WIDTH as f32 / 2.0, crate::WINDOW_HEIGHT as f32 / 2.0)
}

pub fn intersection_area() -> (f32, f32, f32, f32) {
    let (center_x, center_y) = intersection_center();
    let size = 180.0;
    (center_x - size / 2.0, center_y - size / 2.0, size, size)
}

// NEW: Lane calculation utilities
pub fn calculate_lane_center_position(direction: Direction, lane: usize) -> Vec2 {
    let (center_x, center_y) = intersection_center();
    let lane_width = 30.0;

    match direction {
        Direction::North => {
            let x = center_x + 15.0 + (lane as f32 * lane_width);
            Vec2::new(x, center_y)
        }
        Direction::South => {
            let x = center_x - 15.0 - (lane as f32 * lane_width);
            Vec2::new(x, center_y)
        }
        Direction::East => {
            let y = center_y + 15.0 + (lane as f32 * lane_width);
            Vec2::new(center_x, y)
        }
        Direction::West => {
            let y = center_y - 15.0 - (lane as f32 * lane_width);
            Vec2::new(center_x, y)
        }
    }
}

pub const ROAD_WIDTH: f32 = 180.0; // Total road width (6 lanes × 30px)
pub const LANE_WIDTH: f32 = 30.0;  // Individual lane width

// NEW: Intersection geometry validation
pub fn validate_intersection_geometry() -> bool {
    // Verify that lane mathematics are consistent
    let total_lanes = 6;
    let calculated_width = total_lanes as f32 * LANE_WIDTH;

    if (calculated_width - ROAD_WIDTH).abs() < 0.1 {
        println!("✅ Intersection geometry validated: {}px width", ROAD_WIDTH);
        true
    } else {
        println!("❌ Intersection geometry error: expected {}, got {}", ROAD_WIDTH, calculated_width);
        false
    }
}

// NEW: Distance calculation utilities
pub fn calculate_distance_between_points(p1: Vec2, p2: Vec2) -> f32 {
    let dx = p1.x - p2.x;
    let dy = p1.y - p2.y;
    (dx * dx + dy * dy).sqrt()
}

pub fn point_to_line_distance(point: Vec2, line_start: Vec2, line_end: Vec2) -> f32 {
    let line_vec = line_end - line_start;
    let point_vec = point - line_start;

    let line_len = line_vec.length();
    if line_len < 0.001 {
        return calculate_distance_between_points(point, line_start);
    }

    let line_unit = line_vec / line_len;
    let proj_length = point_vec.dot(&line_unit);

    if proj_length < 0.0 {
        calculate_distance_between_points(point, line_start)
    } else if proj_length > line_len {
        calculate_distance_between_points(point, line_end)
    } else {
        let proj_point = line_start + line_unit * proj_length;
        calculate_distance_between_points(point, proj_point)
    }
}