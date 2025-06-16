// src/algorithm.rs
use crate::intersection::Intersection;
use crate::vehicle::{Direction, Route, Vec2, Vehicle, VehicleState, VelocityLevel};
use std::collections::{HashMap, VecDeque};

// Represents a vehicle's claim to cross the intersection at a certain time.
#[derive(Debug)]
struct IntersectionReservation {
    vehicle_id: u32,
    path_key: (Direction, Route), // The path this reservation is for.
    projected_entry_time: f64,    // The simulation time the vehicle expects to enter.
    clearing_time: f64,           // The time the path is expected to be clear.
}

pub struct SmartIntersection {
    pub close_calls: u32,
    safe_distance_violations: HashMap<(u32, u32), f64>,
    current_time: f64,
    reservations: Vec<IntersectionReservation>,

    // Tunable constants for the new algorithm
    critical_distance: f32,       // For close-call stat & failsafe stop.
    safe_following_distance: f32, // For cars in the same lane.
    prediction_time_horizon: f64, // How far in the future to consider conflicts (in seconds).
}

impl SmartIntersection {
    pub fn new() -> Self {
        SmartIntersection {
            close_calls: 0,
            safe_distance_violations: HashMap::new(),
            current_time: 0.0,
            reservations: Vec::new(),
            // --- UPDATED: Increased safety buffers to prevent visual touching ---
            critical_distance: 55.0,        // A more generous buffer beyond the car's length of 39px.
            safe_following_distance: 110.0, // Gives cars more space to decelerate smoothly.
            prediction_time_horizon: 2.8,   // Looks a bit further into the future to be more cautious.
        }
    }

    pub fn process_vehicles(
        &mut self,
        vehicles: &mut VecDeque<Vehicle>,
        intersection: &Intersection,
        delta_time: u32,
    ) {
        self.current_time += delta_time as f64 / 1000.0;
        self.clear_stale_reservations();

        // Phase 1: All vehicles determine their desired velocity based on their leader and intersection grant status.
        for i in 0..vehicles.len() {
            // First, check for a leading vehicle. This has the highest priority.
            let mut desired_velocity = self.get_leading_vehicle_decision(i, vehicles);

            // If not affected by a leader, check intersection logic.
            if desired_velocity == VelocityLevel::Medium {
                let v = &vehicles[i];
                if v.has_passage_grant {
                    // You have a green light, go! Use Medium for crossing, Fast for exiting.
                    desired_velocity = if v.state == VehicleState::Exiting {
                        VelocityLevel::Fast
                    } else {
                        VelocityLevel::Medium
                    };
                } else if v.is_approaching_intersection(intersection) {
                    // You're approaching but have no grant, slow down to yield and wait.
                    desired_velocity = VelocityLevel::Slow;
                }
            }
            vehicles[i].set_target_velocity(desired_velocity);
        }

        // Phase 2: Iterate again to grant intersection passage to eligible vehicles.
        // This is done after all velocity decisions are made to ensure we use up-to-date TTI calculations.
        for v in vehicles.iter_mut() {
            if !v.has_passage_grant && v.state == VehicleState::Approaching {
                self.try_grant_passage(v, intersection);
            }
        }

        self.check_for_close_calls_stat(vehicles);
    }

    fn get_leading_vehicle_decision(&self, i: usize, vehicles: &VecDeque<Vehicle>) -> VelocityLevel {
        let follower = &vehicles[i];
        for j in 0..vehicles.len() {
            if i == j { continue; }
            let leader = &vehicles[j];
            if follower.direction == leader.direction && follower.lane == leader.lane {
                let is_ahead = match follower.direction {
                    Direction::North => leader.position.y < follower.position.y,
                    Direction::South => leader.position.y > follower.position.y,
                    Direction::East => leader.position.x > follower.position.x,
                    Direction::West => leader.position.x < follower.position.x,
                };
                if is_ahead {
                    let distance = (follower.position - leader.position).length();
                    if distance < self.critical_distance { return VelocityLevel::Stop; }
                    if distance < self.safe_following_distance { return leader.velocity_level; }
                }
            }
        }
        VelocityLevel::Medium
    }

    fn try_grant_passage(&mut self, vehicle: &mut Vehicle, intersection: &Intersection) {
        let dist_to_intersection = distance_to_core(vehicle.position, vehicle.direction, intersection);
        if vehicle.current_velocity < 1.0 { vehicle.time_to_intersection = f32::MAX; return; }

        let tti = dist_to_intersection / vehicle.current_velocity; // Time To Intersection
        vehicle.time_to_intersection = tti;

        // Don't check for vehicles that are very far away.
        if tti as f64 > self.prediction_time_horizon * 1.5 { return; }

        let projected_entry_time = self.current_time + tti as f64;
        let vehicle_path_key = (vehicle.direction, vehicle.route);

        for res in &self.reservations {
            if intersection_paths_cross(vehicle_path_key, res.path_key) {
                // Paths conflict. Check times.
                let time_diff = (projected_entry_time - res.projected_entry_time).abs();
                if time_diff < self.prediction_time_horizon {
                    return; // CONFLICT! Must wait.
                }
            }
        }

        println!("✅ Vehicle {} GRANTED passage. TTI: {:.1}s", vehicle.id, tti);
        vehicle.has_passage_grant = true;
        let time_to_cross = (intersection.size / vehicle.current_velocity) as f64;
        self.reservations.push(IntersectionReservation {
            vehicle_id: vehicle.id,
            path_key: vehicle_path_key,
            projected_entry_time,
            clearing_time: projected_entry_time + time_to_cross,
        });
    }

    pub fn clear_reservation_for_vehicle(&mut self, vehicle_id: u32) {
        if self.reservations.iter().any(|r| r.vehicle_id == vehicle_id) {
            println!("🗑️ Clearing reservation for completed Vehicle {}", vehicle_id);
            self.reservations.retain(|r| r.vehicle_id != vehicle_id);
        }
    }

    fn clear_stale_reservations(&mut self) {
        self.reservations.retain(|r| r.clearing_time > self.current_time);
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

/// Helper to calculate the distance from a vehicle to the edge of the intersection core.
fn distance_to_core(pos: Vec2, dir: Direction, intersection: &Intersection) -> f32 {
    let half_size = intersection.size / 2.0;
    match dir {
        Direction::North => pos.y - (intersection.center_y + half_size),
        Direction::South => (intersection.center_y - half_size) - pos.y,
        Direction::East => (intersection.center_x - half_size) - pos.x,
        Direction::West => pos.x - (intersection.center_x + half_size),
    }.max(0.0)
}

/// Determines if two paths will cross inside the intersection. (Simplified and more robust)
fn intersection_paths_cross(path1: (Direction, Route), path2: (Direction, Route)) -> bool {
    let (d1, r1) = path1; let (d2, r2) = path2;
    if d1 == d2 { return false; } // Same incoming direction is handled by leader logic
    let is_opposite = (d1 as i32).abs_diff(d2 as i32) == 2;

    // Rule 1: Opposing left turns always cross
    if is_opposite { return r1 == Route::Left && r2 == Route::Left; }

    // From adjacent (non-opposite) directions:
    // Rule 2: A left turn crosses a straight path
    if (r1 == Route::Left && r2 == Route::Straight) || (r2 == Route::Left && r1 == Route::Straight) { return true; }

    // Rule 3: A left turn crosses an opposing right turn
    if (r1 == Route::Left && r2 == Route::Right) || (r2 == Route::Left && r1 == Route::Right) { return true; }

    // All other cases (Right/Right, Right/Straight from adjacent directions) are non-conflicting
    false
}