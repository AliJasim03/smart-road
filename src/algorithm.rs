// src/algorithm.rs
use crate::intersection::Intersection;
use crate::vehicle::{Direction, Route, Vec2, Vehicle, VehicleState, VelocityLevel};
use std::collections::{HashMap, VecDeque};

// Represents a vehicle's claim to cross the intersection at a certain time.
#[derive(Debug)]
struct IntersectionReservation {
    vehicle_id: u32,
    path_key: (Direction, Route), // The path this reservation is for.
    entry_time: f64,              // The simulation time the vehicle expects to enter.
    clear_time: f64,              // The time the path is expected to be clear.
}

pub struct SmartIntersection {
    pub close_calls: u32,
    safe_distance_violations: HashMap<(u32, u32), f64>,
    current_time: f64,
    reservations: Vec<IntersectionReservation>,

    // These constants are for the BUMPER-TO-BUMPER following logic.
    critical_gap: f32,
    safe_following_gap: f32,
}

impl SmartIntersection {
    pub fn new() -> Self {
        SmartIntersection {
            close_calls: 0,
            safe_distance_violations: HashMap::new(),
            current_time: 0.0,
            reservations: Vec::new(),
            // --- These are for same-lane following and are likely fine ---
            critical_gap: 5.0,        // A 5px emergency gap
            safe_following_gap: 40.0, // Aim for a gap of one car length
        }
    }

    // --- Bumper helpers remain the same ---
    fn get_front_bumper_pos(&self, vehicle: &Vehicle) -> Vec2 {
        let offset = vehicle.height / 2.0;
        let mut pos = vehicle.position;
        match vehicle.get_current_movement_direction() {
            Direction::North => pos.y -= offset,
            Direction::South => pos.y += offset,
            Direction::East  => pos.x += offset,
            Direction::West  => pos.x -= offset,
        }
        pos
    }

    fn get_rear_bumper_pos(&self, vehicle: &Vehicle) -> Vec2 {
        let offset = vehicle.height / 2.0;
        let mut pos = vehicle.position;
        match vehicle.get_current_movement_direction() {
            Direction::North => pos.y += offset,
            Direction::South => pos.y -= offset,
            Direction::East  => pos.x -= offset,
            Direction::West  => pos.x += offset,
        }
        pos
    }

    pub fn process_vehicles(
        &mut self,
        vehicles: &mut VecDeque<Vehicle>,
        intersection: &Intersection,
        delta_time: u32,
    ) {
        self.current_time += delta_time as f64 / 1000.0;
        self.clear_stale_reservations();

        for i in 0..vehicles.len() {
            let mut desired_velocity = self.get_leading_vehicle_decision(i, vehicles);
            if desired_velocity == VelocityLevel::Medium {
                let v = &vehicles[i];
                if v.has_passage_grant {
                    desired_velocity = if v.state == VehicleState::Exiting { VelocityLevel::Fast } else { VelocityLevel::Medium };
                } else if v.is_approaching_intersection(intersection) {
                    desired_velocity = VelocityLevel::Slow;
                }
            }
            vehicles[i].set_target_velocity(desired_velocity);
        }

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
                    let bumper_to_bumper_distance = (self.get_front_bumper_pos(follower) - self.get_rear_bumper_pos(leader)).length();
                    if bumper_to_bumper_distance < self.critical_gap { return VelocityLevel::Stop; }
                    if bumper_to_bumper_distance < self.safe_following_gap { return leader.velocity_level; }
                }
            }
        }
        VelocityLevel::Medium
    }

    fn try_grant_passage(&mut self, vehicle: &mut Vehicle, intersection: &Intersection) {
        let dist_to_intersection = distance_to_core(self.get_front_bumper_pos(vehicle), vehicle.direction, intersection);
        if vehicle.current_velocity < 1.0 { vehicle.time_to_intersection = f32::MAX; return; }

        let tti = dist_to_intersection / vehicle.current_velocity;
        vehicle.time_to_intersection = tti;

        let time_to_cross_intersection = (intersection.size + vehicle.height) / vehicle.current_velocity;

        let requested_entry_time = self.current_time + tti as f64;
        let requested_clear_time = requested_entry_time + time_to_cross_intersection as f64;

        let vehicle_path_key = (vehicle.direction, vehicle.route);

        // --- CORE LOGIC FIX: Check for time-window overlaps ---
        for res in &self.reservations {
            if intersection_paths_cross(vehicle_path_key, res.path_key) {
                // Conflict occurs if the time windows overlap.
                // Window A [a,b] overlaps with Window B [c,d] if (a < d) and (c < b)
                let windows_overlap = requested_entry_time < res.clear_time && res.entry_time < requested_clear_time;
                if windows_overlap {
                    return; // DENIED: Conflict detected, must wait.
                }
            }
        }
        // --- END OF FIX ---

        println!("✅ Vehicle {} GRANTED passage. Crossing from {:.1}s to {:.1}s", vehicle.id, requested_entry_time, requested_clear_time);
        vehicle.has_passage_grant = true;
        self.reservations.push(IntersectionReservation {
            vehicle_id: vehicle.id,
            path_key: vehicle_path_key,
            entry_time: requested_entry_time,
            clear_time: requested_clear_time,
        });
    }

    pub fn clear_reservation_for_vehicle(&mut self, vehicle_id: u32) {
        if self.reservations.iter().any(|r| r.vehicle_id == vehicle_id) {
            self.reservations.retain(|r| r.vehicle_id != vehicle_id);
        }
    }

    fn clear_stale_reservations(&mut self) {
        self.reservations.retain(|r| r.clear_time > self.current_time);
    }

    fn check_for_close_calls_stat(&mut self, vehicles: &VecDeque<Vehicle>) {
        let vehicle_list: Vec<_> = vehicles.iter().collect();
        for i in 0..vehicle_list.len() {
            for j in (i + 1)..vehicle_list.len() {
                let v1 = vehicle_list[i];
                let v2 = vehicle_list[j];
                let center_distance = (v1.position - v2.position).length();
                if center_distance < (v1.height + v2.height) { // Only check if close
                    if (self.get_front_bumper_pos(v1) - self.get_rear_bumper_pos(v2)).length() < self.critical_gap ||
                        (self.get_front_bumper_pos(v2) - self.get_rear_bumper_pos(v1)).length() < self.critical_gap {
                        let pair = if v1.id < v2.id { (v1.id, v2.id) } else { (v2.id, v1.id) };
                        if !self.safe_distance_violations.contains_key(&pair) {
                            self.close_calls += 1;
                            self.safe_distance_violations.insert(pair, self.current_time);
                            println!("⚠️ CLOSE CALL #{}: Vehicles {} and {} too close!", self.close_calls, v1.id, v2.id);
                        }
                    }
                }
            }
        }
        self.safe_distance_violations.retain(|_, time| self.current_time - *time < 3.0);
    }
}

fn distance_to_core(pos: Vec2, dir: Direction, intersection: &Intersection) -> f32 {
    let half_size = intersection.size / 2.0;
    match dir {
        Direction::North => pos.y - (intersection.center_y + half_size),
        Direction::South => (intersection.center_y - half_size) - pos.y,
        Direction::East => (intersection.center_x - half_size) - pos.x,
        Direction::West => pos.x - (intersection.center_x + half_size),
    }.max(0.0)
}

// --- CORE LOGIC FIX: A more exhaustive and correct path-crossing logic ---
fn intersection_paths_cross(path1: (Direction, Route), path2: (Direction, Route)) -> bool {
    use Direction::*;
    use Route::*;
    let (d1, r1) = path1;
    let (d2, r2) = path2;

    if d1 == d2 { return false; } // Same-lane conflicts are handled by following logic

    // To simplify, we can canonicalize the check by always having d1 be the 'smaller' enum value
    let (d1, r1, d2, r2) = if d1 as u8 > d2 as u8 {
        (d2, r2, d1, r1)
    } else {
        (d1, r1, d2, r2)
    };

    match (d1, r1, d2, r2) {
        // --- North vs East ---
        (North, Straight, East, Straight) => true,
        (North, Straight, East, Left)     => true,
        (North, Left,     East, Straight) => true,
        (North, Left,     East, Left)     => true,
        (North, Left,     East, Right)    => true, // Conflict
        (North, Right,    East, Straight) => true, // Conflict
        // Other (North, _, East, _) pairs do not conflict.

        // --- North vs West ---
        (North, Straight, West, Straight) => true,
        (North, Straight, West, Left)     => true,
        (North, Right,    West, Straight) => true,
        (North, Right,    West, Left)     => true,
        (North, Right,    West, Right)    => true,
        (North, Left,     West, Right)    => true,

        // --- North vs South (Opposite) ---
        (North, Left, South, Left) => true, // Head-on left turn conflict

        // --- East vs South ---
        (East, Straight, South, Straight) => true,
        (East, Straight, South, Left)     => true,
        (East, Right,    South, Straight) => true,
        (East, Right,    South, Left)     => true,
        (East, Right,    South, Right)    => true,
        (East, Left,     South, Right)    => true,

        // --- East vs West (Opposite) ---
        (East, Left, West, Left) => true,

        // --- South vs West ---
        (South, Straight, West, Straight) => true,
        (South, Straight, West, Left)     => true,
        (South, Left,     West, Straight) => true,
        (South, Left,     West, Left)     => true,
        (South, Left,     West, Right)    => true,
        (South, Right,    West, Straight) => true,

        // If we haven't matched a conflicting rule, they don't conflict
        _ => false,
    }
}