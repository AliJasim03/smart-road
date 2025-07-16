use crate::intersection::Intersection;
use crate::vehicle::{Direction, Route, Vec2, Vehicle, VehicleState, VelocityLevel};
use std::collections::{HashMap, VecDeque};

// A simplified struct to hold a vehicle's intent, used for grant prioritization.
struct GrantCandidate {
    vehicle_index: usize,
    tti: f64, // Time to intersection
}

#[derive(Debug, Clone)]
struct IntersectionReservation {
    vehicle_id: u32,
    path_key: (Direction, Route),
    entry_time: f64,
    clear_time: f64,
}

pub struct SmartIntersection {
    pub close_calls: u32,
    safe_distance_violations: HashMap<(u32, u32), f64>,
    current_time: f64,
    reservations: Vec<IntersectionReservation>,
    critical_gap: f32,
    reservation_safety_gap: f64,
}

impl SmartIntersection {
    pub fn new() -> Self {
        SmartIntersection {
            close_calls: 0,
            safe_distance_violations: HashMap::new(),
            current_time: 0.0,
            reservations: Vec::new(),
            critical_gap: 15.0,
            reservation_safety_gap: 0.6,
        }
    }

    fn get_time_to_travel(&self, v_initial: f32, v_target: f32, acceleration: f32, distance: f32) -> f64 {
        if v_initial >= v_target || acceleration <= 0.0 {
            return if v_initial > 0.0 { (distance / v_initial) as f64 } else { f64::MAX };
        }
        let time_to_accel = (v_target - v_initial) / acceleration;
        let dist_covered_during_accel = (v_initial * time_to_accel) + 0.5 * acceleration * time_to_accel.powi(2);
        if dist_covered_during_accel >= distance {
            let a = 0.5 * acceleration; let b = v_initial; let c = -distance;
            let discriminant = b.powi(2) - 4.0 * a * c;
            if discriminant >= 0.0 {
                return ((-b + discriminant.sqrt()) / (2.0 * a)) as f64;
            } else {
                return f64::MAX;
            }
        } else {
            let remaining_dist = distance - dist_covered_during_accel;
            let time_at_const_speed = remaining_dist / v_target;
            return time_to_accel as f64 + time_at_const_speed as f64;
        }
    }

    fn get_front_bumper_pos(&self, vehicle: &Vehicle) -> Vec2 {
        let offset = vehicle.height / 2.0; let mut pos = vehicle.position;
        match vehicle.get_current_movement_direction() {
            Direction::North => pos.y -= offset, Direction::South => pos.y += offset,
            Direction::East => pos.x += offset, Direction::West => pos.x -= offset,
        }
        pos
    }

    fn get_rear_bumper_pos(&self, vehicle: &Vehicle) -> Vec2 {
        let offset = vehicle.height / 2.0; let mut pos = vehicle.position;
        match vehicle.get_current_movement_direction() {
            Direction::North => pos.y += offset, Direction::South => pos.y -= offset,
            Direction::East => pos.x -= offset, Direction::West => pos.x += offset,
        }
        pos
    }

    // --- FINAL LOGIC: Decide, then let main loop act ---
    pub fn process_vehicles(
        &mut self,
        vehicles: &mut VecDeque<Vehicle>,
        intersection: &Intersection,
        delta_time: u32,
    ) {
        let dt_seconds = delta_time as f64 / 1000.0;
        self.current_time += dt_seconds;
        self.clear_stale_reservations();

        let mut candidates = Vec::new();

        // 1. Find candidates for grants based on who needs one.
        for (i, v) in vehicles.iter().enumerate() {
            if !v.has_passage_grant && v.state == VehicleState::Approaching {
                let dist = distance_to_core(self.get_front_bumper_pos(v), v.direction, intersection);
                if dist > 0.0 {
                    let tti = self.get_time_to_travel(v.current_velocity, Vehicle::MEDIUM_VELOCITY, Vehicle::ACCELERATION, dist);
                    if tti.is_finite() {
                        candidates.push(GrantCandidate { vehicle_index: i, tti });
                    }
                }
            }
        }

        // 2. Prioritize candidates by sorting by Time To Intersection.
        candidates.sort_by(|a, b| a.tti.partial_cmp(&b.tti).unwrap_or(std::cmp::Ordering::Equal));

        // 3. Attempt to grant passage to the prioritized candidates.
        for candidate in &candidates {
            self.try_grant_passage(
                &mut vehicles[candidate.vehicle_index],
                candidate.tti,
            );
        }

        // 4. Set the final target velocity for every car for this frame.
        // The main loop will call update_physics to actually use this value.
        for i in 0..vehicles.len() {
            let final_vel = self.get_final_velocity_decision(i, &vehicles, intersection);
            vehicles[i].set_target_velocity(final_vel);
            // Also update TTI for debug view
            for cand in &candidates {
                if cand.vehicle_index == i {
                    vehicles[i].time_to_intersection = cand.tti as f32;
                }
            }
        }

        self.check_for_close_calls_stat(vehicles);
    }

    fn get_final_velocity_decision(&self, i: usize, vehicles: &VecDeque<Vehicle>, intersection: &Intersection) -> VelocityLevel {
        let v = &vehicles[i];

        // Rule 1: A car is too close ahead. Must slow/stop.
        for j in 0..vehicles.len() {
            if i == j { continue; }
            let leader = &vehicles[j];
            if v.direction == leader.direction && v.lane == leader.lane {
                let is_ahead = match v.direction {
                    Direction::North => leader.position.y < v.position.y, Direction::South => leader.position.y > v.position.y,
                    Direction::East  => leader.position.x > v.position.x, Direction::West  => leader.position.x < v.position.x,
                };
                if is_ahead {
                    let bumper_to_bumper_distance = (self.get_front_bumper_pos(v) - self.get_rear_bumper_pos(leader)).length();
                    if bumper_to_bumper_distance < self.critical_gap { return VelocityLevel::Stop; }
                    if bumper_to_bumper_distance < v.height * 1.5 { return leader.velocity_level; }
                }
            }
        }

        // Rule 2: Car has a grant. It should proceed.
        if v.has_passage_grant {
            return if v.state == VehicleState::Exiting { VelocityLevel::Fast } else { VelocityLevel::Medium };
        }

        // Rule 3: Car is approaching intersection without a grant. Be cautious.
        if v.is_approaching_intersection(intersection) {
            return VelocityLevel::Slow;
        }

        // Rule 4: Otherwise, no immediate threats. Cruise at medium speed.
        VelocityLevel::Medium
    }

    fn try_grant_passage(&mut self, vehicle: &mut Vehicle, tti: f64) {
        if vehicle.has_passage_grant { return; }

        let effective_velocity = Vehicle::MEDIUM_VELOCITY;
        let base_time_to_cross = (Intersection::new().size + vehicle.height) / effective_velocity;
        let route_travel_time_multiplier = match vehicle.route {
            Route::Left => 1.8, Route::Straight => 1.1, Route::Right => 0.9,
        };
        let time_to_cross_intersection = base_time_to_cross as f64 * route_travel_time_multiplier;
        let requested_entry_time = self.current_time + tti;
        let requested_clear_time = requested_entry_time + time_to_cross_intersection + self.reservation_safety_gap;
        let vehicle_path_key = (vehicle.direction, vehicle.route);

        for res in &self.reservations {
            if intersection_paths_cross(vehicle_path_key, res.path_key) {
                if requested_entry_time < res.clear_time && res.entry_time < requested_clear_time {
                    return;
                }
            }
        }
        println!("✅ Vehicle {} GRANTED passage. Crossing from {:.1}s to {:.1}s", vehicle.id, requested_entry_time, requested_clear_time);
        vehicle.has_passage_grant = true;
        self.reservations.push(IntersectionReservation {
            vehicle_id: vehicle.id, path_key: vehicle_path_key,
            entry_time: requested_entry_time, clear_time: requested_clear_time,
        });
    }

    pub fn clear_reservation_for_vehicle(&mut self, vehicle_id: u32) {
        self.reservations.retain(|r| r.vehicle_id != vehicle_id);
    }

    fn clear_stale_reservations(&mut self) {
        self.reservations.retain(|r| r.clear_time > self.current_time);
    }

    fn check_for_close_calls_stat(&mut self, vehicles: &VecDeque<Vehicle>) {
        let vehicle_list: Vec<_> = vehicles.iter().collect();
        for i in 0..vehicle_list.len() {
            for j in (i + 1)..vehicle_list.len() {
                let v1 = vehicle_list[i]; let v2 = vehicle_list[j];
                let collides_x = (v1.position.x - v2.position.x).abs() * 2.0 < (v1.width + v2.width);
                let collides_y = (v1.position.y - v2.position.y).abs() * 2.0 < (v1.height + v2.height);
                if collides_x && collides_y {
                    let pair = if v1.id < v2.id { (v1.id, v2.id) } else { (v2.id, v1.id) };
                    if !self.safe_distance_violations.contains_key(&pair) {
                        self.close_calls += 1;
                        self.safe_distance_violations.insert(pair, self.current_time);
                        println!("⚠️ CLOSE CALL #{}: Vehicles {} and {} physically overlap!", self.close_calls, v1.id, v2.id);
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

fn intersection_paths_cross(path1: (Direction, Route), path2: (Direction, Route)) -> bool {
    use Direction::*; use Route::*;
    let (d1, r1) = path1;
    let (d2, r2) = path2;

    if d1 == d2 { return false; }

    let dest1 = crate::get_destination_for_route(d1, r1);
    let dest2 = crate::get_destination_for_route(d2, r2);

    if dest1 == dest2 && !d1.is_opposite(d2) {
        return true;
    }

    if !d1.is_opposite(d2) {
        // These are pairs from adjacent directions.
        match (d1, r1, d2, r2) {
            // Safe cases (U-turns from one road to another)
            (North, Left, West, Right) | (West, Right, North, Left) => return false,
            (North, Right, East, Left) | (East, Left, North, Right) => return false,
            (South, Left, East, Right) | (East, Right, South, Left) => return false,
            (South, Right, West, Left) | (West, Left, South, Right) => return false,
            // All other adjacent path combinations conflict.
            _ => return true,
        }
    }

    // These are pairs from opposite directions (e.g., North/South).
    if (r1 == Straight && r2 == Straight) ||
        (r1 == Straight && r2 == Right) ||
        (r1 == Right && r2 == Straight) ||
        (r1 == Right && r2 == Right) {
        return false;
    }

    // All other opposite path combinations (involving a left turn) conflict.
    true
}