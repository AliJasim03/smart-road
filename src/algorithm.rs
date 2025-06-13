// src/algorithm.rs - COMPATIBILITY: Verified with new turn system
use crate::intersection::Intersection;
use crate::vehicle::{Vehicle, VehicleState, VelocityLevel, Direction, Route, Vec2};
use std::collections::{HashMap, VecDeque};
use std::ops::Sub; // You can add this line...

pub struct SmartIntersection {
    pub close_calls: u32,
    safe_distance_violations: HashMap<(u32, u32), f64>,
    current_time: f64,
    intersection_occupancy: Vec<u32>,
    max_intersection_capacity: usize,
    critical_distance: f32,
    safe_following_distance: f32,
}

impl SmartIntersection {
    pub fn new() -> Self {
        SmartIntersection {
            close_calls: 0,
            safe_distance_violations: HashMap::new(),
            current_time: 0.0,
            intersection_occupancy: Vec::new(),
            max_intersection_capacity: 4, // Allow a few cars in the large intersection box
            critical_distance: 25.0,      // Critical collision distance
            safe_following_distance: 60.0, // Standard following distance
        }
    }

    pub fn process_vehicles(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection, delta_time: u32) {
        self.current_time += delta_time as f64 / 1000.0;

        self.update_intersection_occupancy(vehicles, intersection);

        // This is the main logic loop
        for i in 0..vehicles.len() {
            let mut should_slow = false;
            let mut should_stop = false;

            for j in 0..vehicles.len() {
                if i == j { continue; }

                // Check for conflicts
                if self.vehicles_conflict(&vehicles[i], &vehicles[j], intersection) {
                    // --- FIXED ---
                    // Using the '-' operator for subtraction instead of .sub()
                    let distance = (vehicles[i].position - vehicles[j].position).length();

                    // Vehicle ahead logic (follower should slow/stop)
                    if self.is_vehicle_ahead(&vehicles[i], &vehicles[j]) {
                        if distance < self.critical_distance { should_stop = true; }
                        else if distance < self.safe_following_distance { should_slow = true; }
                    }
                }
            }

            // Intersection entry management
            if self.should_wait_for_intersection(&vehicles[i], intersection) {
                should_stop = true;
            }

            // Apply velocity changes
            let vehicle = &mut vehicles[i];
            if should_stop {
                vehicle.set_target_velocity(VelocityLevel::Stop);
            } else if should_slow {
                vehicle.set_target_velocity(VelocityLevel::Slow);
            } else {
                // If no hazards, try to resume normal speed
                if vehicle.is_in_intersection(intersection) {
                    vehicle.set_target_velocity(VelocityLevel::Slow);
                } else if vehicle.state == VehicleState::Exiting {
                    vehicle.set_target_velocity(VelocityLevel::Fast);
                } else {
                    vehicle.set_target_velocity(VelocityLevel::Medium);
                }
            }
        }
        self.check_for_close_calls(vehicles);
    }

    fn update_intersection_occupancy(&mut self, vehicles: &VecDeque<Vehicle>, intersection: &Intersection) {
        self.intersection_occupancy.clear();
        for v in vehicles {
            if v.is_in_intersection(intersection) {
                self.intersection_occupancy.push(v.id);
            }
        }
    }

    // Main conflict detection logic
    fn vehicles_conflict(&self, v1: &Vehicle, v2: &Vehicle, intersection: &Intersection) -> bool {
        // Same lane conflict
        if v1.direction == v2.direction && v1.lane == v2.lane {
            return true;
        }

        // Both must be near or in intersection to have an intersection conflict
        if !(v1.is_approaching_intersection(intersection) || v1.is_in_intersection(intersection)) ||
            !(v2.is_approaching_intersection(intersection) || v2.is_in_intersection(intersection)) {
            return false;
        }

        // Broad phase: Do their destination paths cross?
        return intersection_paths_cross(v1.direction, v1.route, v2.direction, v2.route);
    }

    // Check if v2 is physically ahead of v1 in the same lane
    fn is_vehicle_ahead(&self, v1: &Vehicle, v2: &Vehicle) -> bool {
        if v1.direction != v2.direction || v1.lane != v2.lane {
            return false;
        }

        let is_ahead = match v1.direction {
            Direction::North => v2.position.y < v1.position.y,
            Direction::South => v2.position.y > v1.position.y,
            Direction::East   => v2.position.x > v1.position.x,
            Direction::West  => v2.position.x < v1.position.x,
        };
        is_ahead
    }

    // Logic to decide if a vehicle should wait before entering the intersection
    fn should_wait_for_intersection(&self, vehicle: &Vehicle, intersection: &Intersection) -> bool {
        // Only applies to vehicles approaching, not those already inside
        if !vehicle.is_approaching_intersection(intersection) {
            return false;
        }

        // If intersection is full, wait
        if self.intersection_occupancy.len() >= self.max_intersection_capacity {
            return true;
        }

        // This is a placeholder for more advanced logic where you might check
        // for conflicts with specific vehicles already in the intersection.
        // for _id in &self.intersection_occupancy {}

        false // Default to allow entry
    }

    // Check for close calls for statistical purposes
    fn check_for_close_calls(&mut self, vehicles: &VecDeque<Vehicle>) {
        let vehicle_list: Vec<_> = vehicles.iter().collect();
        for i in 0..vehicle_list.len() {
            for j in (i + 1)..vehicle_list.len() {
                let v1 = vehicle_list[i];
                let v2 = vehicle_list[j];

                // --- FIXED ---
                // Using the '-' operator for subtraction instead of .sub()
                let distance = (v1.position - v2.position).length();

                if distance < self.critical_distance {
                    let pair = if v1.id < v2.id { (v1.id, v2.id) } else { (v2.id, v1.id) };
                    if !self.safe_distance_violations.contains_key(&pair) {
                        self.close_calls += 1;
                        self.safe_distance_violations.insert(pair, self.current_time);
                        println!("⚠️ CLOSE CALL #{}: Vehicles {} and {} too close!", self.close_calls, v1.id, v2.id);
                    }
                }
            }
        }
        // Clean up old violations so they can be triggered again after some time
        self.safe_distance_violations.retain(|_, time| self.current_time - *time < 3.0);
    }
}

// Helper to determine if two paths conflict based on their direction and route.
fn intersection_paths_cross(d1: Direction, r1: Route, d2: Direction, r2: Route) -> bool {
    if d1 == d2 { return false; } // Parallel paths don't cross

    // Check for opposite directions (e.g., North vs. South)
    let is_opposite = (d1 as i32 - d2 as i32).abs() == 2;
    if is_opposite {
        // Opposing traffic only conflicts if both are turning left.
        return r1 == Route::Left && r2 == Route::Left;
    }

    // Perpendicular traffic rules
    // Two right-turning vehicles from perpendicular roads do not conflict.
    if r1 == Route::Right && r2 == Route::Right { return false; }

    // A straight vehicle does not conflict with a vehicle from a perpendicular
    // road that is turning right (away from the straight path).
    if r1 == Route::Straight && r2 == Route::Right {
        // e.g., N-Straight vs W-Right (safe) or E-Right (safe)
        // Check relative geometry. A simple way is to know they don't cross.
        return false;
    }
    // And the reverse
    if r2 == Route::Straight && r1 == Route::Right {
        return false;
    }

    // In most other perpendicular cases, assume a potential conflict.
    // e.g., a left turn vs. a straight vehicle.
    true
}