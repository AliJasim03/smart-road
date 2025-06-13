// src/algorithm.rs - FINAL FIX: Context-aware collision prevention
use crate::intersection::Intersection;
use crate::vehicle::{Vehicle, VehicleState, VelocityLevel, Direction, Route};
use std::collections::{HashMap, VecDeque};
use std::ops::Sub;

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
            max_intersection_capacity: 4,
            critical_distance: 25.0,
            safe_following_distance: 60.0,
        }
    }

    pub fn process_vehicles(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection, delta_time: u32) {
        self.current_time += delta_time as f64 / 1000.0;
        self.update_intersection_occupancy(vehicles, intersection);

        for i in 0..vehicles.len() {
            let mut should_slow = false;
            let mut should_stop = false;

            for j in 0..vehicles.len() {
                if i == j { continue; }

                let v_i = &vehicles[i];
                let v_j = &vehicles[j];

                // CASE 1: Following another car in the same lane.
                // This logic is simple and does not need to change.
                if v_i.direction == v_j.direction && v_i.lane == v_j.lane {
                    if self.is_vehicle_ahead(v_i, v_j) {
                        let distance = (v_i.position - v_j.position).length();
                        if distance < self.critical_distance { should_stop = true; }
                        else if distance < self.safe_following_distance { should_slow = true; }
                    }
                }
                // CASE 2: Paths cross at the intersection.
                else if intersection_paths_cross(v_i.direction, v_i.route, v_j.direction, v_j.route) {
                    // --- THIS IS THE NEW CRITICAL LOGIC ---
                    // A vehicle should only yield if BOTH vehicles are near the point of conflict.
                    let v_i_is_near = v_i.is_in_intersection(intersection) || v_i.is_approaching_intersection(intersection);
                    let v_j_is_near = v_j.is_in_intersection(intersection) || v_j.is_approaching_intersection(intersection);

                    if v_i_is_near && v_j_is_near {
                        // Both cars are in the danger zone. Apply a priority rule to decide who yields.
                        // Rule: Higher ID vehicle yields. This is deterministic and prevents deadlocks.
                        if v_i.id > v_j.id {
                            should_slow = true; // Be cautious when a conflict is developing.
                            // If they are about to touch, perform an emergency stop.
                            if (v_i.position - v_j.position).length() < self.critical_distance + 10.0 {
                                should_stop = true;
                            }
                        }
                    }
                }
            }

            if !should_stop && self.should_wait_for_intersection_capacity(&vehicles[i]) {
                should_stop = true;
            }

            let vehicle = &mut vehicles[i];
            if should_stop { vehicle.set_target_velocity(VelocityLevel::Stop); }
            else if should_slow { vehicle.set_target_velocity(VelocityLevel::Slow); }
            else {
                if vehicle.is_in_intersection(intersection) { vehicle.set_target_velocity(VelocityLevel::Slow); }
                else if vehicle.state == VehicleState::Exiting { vehicle.set_target_velocity(VelocityLevel::Fast); }
                else { vehicle.set_target_velocity(VelocityLevel::Medium); }
            }
        }

        self.check_for_close_calls_stat(vehicles);
    }

    fn update_intersection_occupancy(&mut self, vehicles: &VecDeque<Vehicle>, intersection: &Intersection) {
        self.intersection_occupancy.clear();
        for v in vehicles {
            if v.is_in_intersection(intersection) { self.intersection_occupancy.push(v.id); }
        }
    }

    fn is_vehicle_ahead(&self, follower: &Vehicle, leader: &Vehicle) -> bool {
        match follower.direction {
            Direction::North => leader.position.y < follower.position.y,
            Direction::South => leader.position.y > follower.position.y,
            Direction::East => leader.position.x > follower.position.x,
            Direction::West => leader.position.x < follower.position.x,
        }
    }

    fn should_wait_for_intersection_capacity(&self, vehicle: &Vehicle) -> bool {
        if vehicle.state != VehicleState::Approaching { return false; }
        if self.intersection_occupancy.len() >= self.max_intersection_capacity { return true; }
        false
    }

    fn check_for_close_calls_stat(&mut self, vehicles: &VecDeque<Vehicle>) {
        let vehicle_list: Vec<_> = vehicles.iter().collect();
        for i in 0..vehicle_list.len() {
            for j in (i + 1)..vehicle_list.len() {
                let v1 = vehicle_list[i]; let v2 = vehicle_list[j];
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
        self.safe_distance_violations.retain(|_, time| self.current_time - *time < 3.0);
    }
}

fn intersection_paths_cross(d1: Direction, r1: Route, d2: Direction, r2: Route) -> bool {
    if d1 == d2 { return r1 == r2; } // Only conflict if in same lane (handled by same-lane check)
    let is_opposite = (d1 as i32).abs_diff(d2 as i32) == 2;
    if is_opposite { return r1 == Route::Left && r2 == Route::Left; }
    if r1 == Route::Right && r2 == Route::Right { return false; }
    if (r1 == Route::Straight && r2 == Route::Right) || (r2 == Route::Straight && r1 == Route::Right) { return false; }
    true
}