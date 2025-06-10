// src/intersection.rs - FIXED: Enhanced intersection with proper zone detection
use crate::vehicle::{Vehicle, Direction, Route, VehicleState, VelocityLevel};
use std::collections::VecDeque;

pub struct Intersection {
    pub center_x: i32,
    pub center_y: i32,
    pub size: i32,
    pub approach_distance: i32,
    pub core_radius: i32,
}

impl Intersection {
    pub fn new() -> Self {
        Intersection {
            center_x: crate::WINDOW_WIDTH as i32 / 2,
            center_y: crate::WINDOW_HEIGHT as i32 / 2,
            size: 180, // Total intersection area
            approach_distance: 150, // Distance to start slowing down
            core_radius: 90, // Core intersection radius
        }
    }

    pub fn is_point_in_intersection(&self, x: i32, y: i32) -> bool {
        let dx = (x - self.center_x).abs();
        let dy = (y - self.center_y).abs();
        dx < self.size / 2 && dy < self.size / 2
    }

    pub fn is_point_in_core(&self, x: i32, y: i32) -> bool {
        let distance = self.distance_to_center(x, y);
        distance < self.core_radius as f64
    }

    pub fn is_point_in_approach_zone(&self, x: i32, y: i32) -> bool {
        let distance = self.distance_to_center(x, y);
        distance < (self.core_radius + self.approach_distance) as f64 &&
            distance >= self.core_radius as f64
    }

    pub fn distance_to_center(&self, x: i32, y: i32) -> f64 {
        let dx = x - self.center_x;
        let dy = y - self.center_y;
        ((dx * dx + dy * dy) as f64).sqrt()
    }

    // FIXED: Get intersection zone for a vehicle
    pub fn get_vehicle_zone(&self, vehicle: &Vehicle) -> IntersectionZone {
        let distance = self.distance_to_center(vehicle.position.x, vehicle.position.y);

        if distance < self.core_radius as f64 - 20.0 {
            IntersectionZone::Core
        } else if distance < self.core_radius as f64 + 30.0 {
            if vehicle.state == VehicleState::Turning {
                IntersectionZone::Turning
            } else {
                IntersectionZone::Entry
            }
        } else if distance < (self.core_radius + self.approach_distance) as f64 {
            IntersectionZone::Approach
        } else {
            IntersectionZone::Clear
        }
    }

    // FIXED: Check if a vehicle's path will intersect with another
    pub fn do_paths_intersect(&self, vehicle1: &Vehicle, vehicle2: &Vehicle) -> bool {
        // Same direction same lane = same path
        if vehicle1.direction == vehicle2.direction && vehicle1.lane == vehicle2.lane {
            return true;
        }

        // FIXED: Straight traffic in separated lanes never intersects
        if vehicle1.route == Route::Straight && vehicle2.route == Route::Straight {
            return false; // Separated by design in 6-lane system
        }

        // Check specific turning conflicts
        self.check_turning_conflicts(vehicle1, vehicle2)
    }

    fn check_turning_conflicts(&self, vehicle1: &Vehicle, vehicle2: &Vehicle) -> bool {
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

            // Opposing left turns
            (Direction::North, Route::Left, Direction::South, Route::Left) => true,
            (Direction::East, Route::Left, Direction::West, Route::Left) => true,

            // Right turns generally don't conflict (tight radius)
            _ => false,
        }
    }

    // FIXED: Get safe following distance based on zone
    pub fn get_safe_distance_for_zone(&self, zone: IntersectionZone) -> f64 {
        match zone {
            IntersectionZone::Core | IntersectionZone::Turning => 60.0,  // Closer in intersection
            IntersectionZone::Entry => 80.0,                             // Medium distance
            IntersectionZone::Approach => 120.0,                         // Longer distance when approaching
            IntersectionZone::Clear => 100.0,                            // Standard distance
        }
    }

    // FIXED: Calculate time for vehicle to clear intersection
    pub fn time_to_clear_intersection(&self, vehicle: &Vehicle) -> f64 {
        if vehicle.current_velocity <= 0.0 {
            return f64::INFINITY;
        }

        let distance_to_exit = match vehicle.route {
            Route::Straight => self.size as f64,                    // Just go through
            Route::Left | Route::Right => self.size as f64 * 1.4,   // Turning path is longer
        };

        distance_to_exit / vehicle.current_velocity
    }

    // FIXED: Check if intersection is clear for a vehicle to enter
    pub fn is_clear_for_entry(&self, entering_vehicle: &Vehicle, all_vehicles: &std::collections::VecDeque<Vehicle>) -> bool {
        for other in all_vehicles {
            if other.id == entering_vehicle.id {
                continue;
            }

            // Check vehicles currently in intersection
            if self.is_point_in_intersection(other.position.x, other.position.y) {
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

    // FIXED: Get recommended velocity for intersection zone
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

    // FIXED: Check for potential conflicts ahead
    pub fn check_conflicts_ahead(&self, vehicle: &Vehicle, look_ahead_distance: f64, all_vehicles: &std::collections::VecDeque<Vehicle>) -> Vec<u32> {
        let mut conflicts = Vec::new();

        for other in all_vehicles {
            if other.id == vehicle.id {
                continue;
            }

            let distance = vehicle.distance_from_spawn(); // Use existing distance calculation
            if distance < look_ahead_distance {
                if self.do_paths_intersect(vehicle, other) {
                    conflicts.push(other.id);
                }
            }
        }

        conflicts
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

// FIXED: Traffic flow analysis
pub struct TrafficFlowAnalyzer {
    throughput_by_direction: [u32; 4],
    congestion_by_direction: [f64; 4],
    average_wait_times: [f64; 4],
    last_analysis_time: std::time::Instant,
}

impl TrafficFlowAnalyzer {
    pub fn new() -> Self {
        TrafficFlowAnalyzer {
            throughput_by_direction: [0; 4],
            congestion_by_direction: [0.0; 4],
            average_wait_times: [0.0; 4],
            last_analysis_time: std::time::Instant::now(),
        }
    }

    pub fn analyze_flow(&mut self, vehicles: &std::collections::VecDeque<Vehicle>, intersection: &Intersection) {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_analysis_time).as_secs_f64();

        if elapsed < 1.0 {
            return; // Analyze every second
        }

        // Reset counters
        self.congestion_by_direction = [0.0; 4];
        let mut wait_times = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];

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
                let wait_time = vehicle.start_time.elapsed().as_secs_f64();
                wait_times[dir_index].push(wait_time);
            }
        }

        // Calculate average wait times
        for i in 0..4 {
            if !wait_times[i].is_empty() {
                self.average_wait_times[i] = wait_times[i].iter().sum::<f64>() / wait_times[i].len() as f64;
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

    pub fn get_congestion_level(&self, direction: Direction) -> f64 {
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
        println!("\n=== TRAFFIC FLOW ANALYSIS ===");
        for (i, direction) in [Direction::North, Direction::South, Direction::East, Direction::West].iter().enumerate() {
            println!("{:?}: Congestion: {:.1}, Throughput: {}, Avg Wait: {:.1}s",
                     direction,
                     self.congestion_by_direction[i],
                     self.throughput_by_direction[i],
                     self.average_wait_times[i]);
        }
        println!("=============================\n");
    }
}

// FIXED: Helper functions for intersection management
pub fn intersection_center() -> (i32, i32) {
    (crate::WINDOW_WIDTH as i32 / 2, crate::WINDOW_HEIGHT as i32 / 2)
}

pub fn intersection_area() -> (i32, i32, u32, u32) {
    let (center_x, center_y) = intersection_center();
    let size = 180u32;
    (center_x - size as i32 / 2, center_y - size as i32 / 2, size, size)
}

pub const ROAD_WIDTH: u32 = 180; // Total road width (6 lanes × 30px)
pub const LANE_WIDTH: u32 = 30;  // Individual lane width