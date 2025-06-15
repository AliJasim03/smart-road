// src/algorithm.rs - UPDATED for more cautious, larger vehicles
use crate::intersection::Intersection;
use crate::vehicle::{Direction, Route, Vec2, Vehicle, VehicleState, VelocityLevel};
use std::collections::{HashMap, VecDeque};

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
            // --- KEY CHANGE: Increased distances for larger cars ---
            critical_distance: 60.0,
            safe_following_distance: 120.0,
            // --- END KEY CHANGE ---
        }
    }

    pub fn process_vehicles(
        &mut self,
        vehicles: &mut VecDeque<Vehicle>,
        intersection: &Intersection,
        delta_time: u32,
    ) {
        self.current_time += delta_time as f64 / 1000.0;
        self.update_intersection_occupancy(vehicles, intersection);

        for i in 0..vehicles.len() {
            let mut should_slow = false;
            let mut should_stop = false;

            // This temporary borrow is necessary because we need mutable access later.
            let vehicle_i_state = (
                vehicles[i].direction,
                vehicles[i].route,
                vehicles[i].lane,
                vehicles[i].id,
                vehicles[i].position,
                vehicles[i].is_in_intersection(intersection),
                vehicles[i].is_approaching_intersection(intersection),
            );

            for j in 0..vehicles.len() {
                if i == j {
                    continue;
                }

                let v_j = &vehicles[j];

                // CASE 1: Following another car in the same lane.
                if vehicle_i_state.0 == v_j.direction && vehicle_i_state.2 == v_j.lane {
                    if self.is_vehicle_ahead_state(vehicle_i_state.0, vehicle_i_state.4, v_j) {
                        let distance = (vehicle_i_state.4 - v_j.position).length();
                        if distance < self.critical_distance {
                            should_stop = true;
                        } else if distance < self.safe_following_distance {
                            should_slow = true;
                        }
                    }
                }
                // CASE 2: Paths cross at the intersection.
                else if intersection_paths_cross(
                    vehicle_i_state.0,
                    vehicle_i_state.1,
                    v_j.direction,
                    v_j.route,
                ) {
                    let v_i_is_near = vehicle_i_state.5 || vehicle_i_state.6;
                    let v_j_is_near = v_j.is_in_intersection(intersection)
                        || v_j.is_approaching_intersection(intersection);

                    if v_i_is_near && v_j_is_near {
                        // Higher ID vehicle yields to prevent deadlock
                        if vehicle_i_state.3 > v_j.id {
                            should_slow = true;
                            // Emergency stop if about to touch
                            if (vehicle_i_state.4 - v_j.position).length()
                                < self.critical_distance + 10.0
                            {
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
            if should_stop {
                vehicle.set_target_velocity(VelocityLevel::Stop);
            } else if should_slow {
                vehicle.set_target_velocity(VelocityLevel::Slow);
            } else {
                if vehicle.is_in_intersection(intersection) {
                    vehicle.set_target_velocity(VelocityLevel::Slow);
                } else if vehicle.state == VehicleState::Exiting {
                    vehicle.set_target_velocity(VelocityLevel::Fast);
                } else {
                    vehicle.set_target_velocity(VelocityLevel::Medium);
                }
            }
        }

        self.check_for_close_calls_stat(vehicles);
    }

    // --- The rest of the functions are unchanged ---
    fn is_vehicle_ahead_state(
        &self,
        follower_dir: Direction,
        follower_pos: Vec2,
        leader: &Vehicle,
    ) -> bool {
        /* ... Re-pasting for completeness ... */
        match follower_dir {
            Direction::North => leader.position.y < follower_pos.y,
            Direction::South => leader.position.y > follower_pos.y,
            Direction::East => leader.position.x > follower_pos.x,
            Direction::West => leader.position.x < follower_pos.x,
        }
    }
}
// --- Pasting unchanged functions for completeness ---
impl SmartIntersection {
    fn update_intersection_occupancy(
        &mut self,
        vehicles: &VecDeque<Vehicle>,
        intersection: &Intersection,
    ) {
        self.intersection_occupancy.clear();
        for v in vehicles {
            if v.is_in_intersection(intersection) {
                self.intersection_occupancy.push(v.id);
            }
        }
    }
    fn should_wait_for_intersection_capacity(&self, vehicle: &Vehicle) -> bool {
        if vehicle.state != VehicleState::Approaching {
            return false;
        }
        if self.intersection_occupancy.len() >= self.max_intersection_capacity {
            return true;
        }
        false
    }
    fn check_for_close_calls_stat(&mut self, vehicles: &VecDeque<Vehicle>) {
        let vehicle_list: Vec<_> = vehicles.iter().collect();
        for i in 0..vehicle_list.len() {
            for j in (i + 1)..vehicle_list.len() {
                let v1 = vehicle_list[i];
                let v2 = vehicle_list[j];
                let distance = (v1.position - v2.position).length();
                if distance < self.critical_distance {
                    let pair = if v1.id < v2.id {
                        (v1.id, v2.id)
                    } else {
                        (v2.id, v1.id)
                    };
                    if !self.safe_distance_violations.contains_key(&pair) {
                        self.close_calls += 1;
                        self.safe_distance_violations
                            .insert(pair, self.current_time);
                        println!(
                            "⚠️ CLOSE CALL #{}: Vehicles {} and {} too close!",
                            self.close_calls, v1.id, v2.id
                        );
                    }
                }
            }
        }
        self.safe_distance_violations
            .retain(|_, time| self.current_time - *time < 3.0);
    }
}
fn intersection_paths_cross(d1: Direction, r1: Route, d2: Direction, r2: Route) -> bool {
    if d1 == d2 {
        return r1 == r2;
    }
    let is_opposite = (d1 as i32).abs_diff(d2 as i32) == 2;
    if is_opposite {
        return r1 == Route::Left && r2 == Route::Left;
    }
    if r1 == Route::Right && r2 == Route::Right {
        return false;
    }
    if (r1 == Route::Straight && r2 == Route::Right)
        || (r2 == Route::Straight && r1 == Route::Right)
    {
        return false;
    }
    true
}
