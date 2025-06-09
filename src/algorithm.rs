// src/algorithm.rs - FIXED VERSION WITH IMPROVED COLLISION DETECTION
use crate::intersection::Intersection;
use crate::vehicle::{Vehicle, VehicleState, VelocityLevel, Direction};
use std::collections::{HashMap, VecDeque};

pub struct SmartIntersection {
    // Keep track of close calls
    pub close_calls: u32,
    // Track safe distance violations
    safe_distance_violations: HashMap<u32, Vec<u32>>, // Vehicle ID to list of vehicles it had close calls with
    // Track congestion by direction and lane
    congestion_levels: HashMap<(Direction, usize), u32>, // (Direction, lane) to number of vehicles
    // Enable adaptive mode for high traffic
    adaptive_mode: bool,
    // Track throughput for each direction
    direction_priority: [u32; 4], // Priority counter for [North, South, East, West]
    // Track current vehicle flows
    current_flows: Vec<(Direction, usize)>, // Currently prioritized (direction, lane) pairs
    // FIXED: Add reservation system
    intersection_reservations: HashMap<u32, f64>, // Vehicle ID -> expiration time
    current_time: f64,
    // FIXED: More conservative intersection management
    max_simultaneous_vehicles: usize,
    critical_collision_distance: f64,
}

impl SmartIntersection {
    pub fn new() -> Self {
        SmartIntersection {
            close_calls: 0,
            safe_distance_violations: HashMap::new(),
            congestion_levels: HashMap::new(),
            adaptive_mode: false,
            direction_priority: [0; 4],
            current_flows: Vec::new(),
            intersection_reservations: HashMap::new(),
            current_time: 0.0,
            max_simultaneous_vehicles: 4, // FIXED: Reduced from implicit higher number
            critical_collision_distance: 80.0, // FIXED: Increased critical distance
        }
    }

    // FIXED: Process all vehicles with much stricter collision avoidance
    pub fn process_vehicles(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection, delta_time: u32) {
        self.current_time += delta_time as f64 / 1000.0;

        // Clean up expired reservations
        self.cleanup_expired_reservations();

        // First, update all vehicle positions
        for vehicle in vehicles.iter_mut() {
            vehicle.update(delta_time, intersection);
        }

        // Analyze congestion levels
        self.analyze_congestion(vehicles);

        // FIXED: Apply much stricter collision detection and prevention
        self.apply_strict_collision_prevention(vehicles, intersection);

        // Check for safe distance violations
        self.check_safe_distances(vehicles);

        // Manage intersection access more strictly
        self.manage_intersection_access_strict(vehicles, intersection);
    }

    // FIXED: Much stricter collision prevention
    fn apply_strict_collision_prevention(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection) {
        // Process vehicles in order of proximity to intersection
        let mut vehicle_indices: Vec<usize> = (0..vehicles.len()).collect();

        // Sort by distance to intersection (closest first)
        vehicle_indices.sort_by(|&a, &b| {
            let dist_a = self.distance_to_intersection_center(&vehicles[a]);
            let dist_b = self.distance_to_intersection_center(&vehicles[b]);
            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Check each vehicle against all others for potential collisions
        for i in 0..vehicle_indices.len() {
            let idx_a = vehicle_indices[i];
            let mut should_stop = false;
            let mut should_slow = false;

            if vehicles[idx_a].state == VehicleState::Completed {
                continue;
            }

            // Check against all other vehicles
            for j in 0..vehicle_indices.len() {
                if i == j {
                    continue;
                }

                let idx_b = vehicle_indices[j];
                if vehicles[idx_b].state == VehicleState::Completed {
                    continue;
                }

                let vehicle_a = &vehicles[idx_a];
                let vehicle_b = &vehicles[idx_b];

                // FIXED: Much more conservative collision detection
                if vehicle_a.could_collide_with(vehicle_b, intersection) {
                    let distance = self.calculate_distance(vehicle_a, vehicle_b);
                    let time_to_collision = self.estimate_time_to_collision(vehicle_a, vehicle_b);

                    // FIXED: Much more conservative thresholds
                    if distance < self.critical_collision_distance && time_to_collision < 2.0 {
                        // Determine who should yield based on strict priority rules
                        if self.should_yield_strict(vehicle_a, vehicle_b, intersection) {
                            if distance < 40.0 || time_to_collision < 0.8 {
                                should_stop = true; // Complete stop for critical situations
                                self.record_close_call(vehicle_a.id, vehicle_b.id);
                            } else {
                                should_slow = true;
                            }
                            break; // Found a conflict, no need to check others
                        }
                    } else if distance < 120.0 && time_to_collision < 3.0 {
                        // Preventive slowing
                        if self.should_yield_strict(vehicle_a, vehicle_b, intersection) {
                            should_slow = true;
                        }
                    }
                }
            }

            // Apply the most restrictive response
            if should_stop {
                vehicles[idx_a].set_target_velocity(VelocityLevel::Slow);
                vehicles[idx_a].current_velocity *= 0.3; // Emergency braking
                println!("🚨 Emergency stop for vehicle {} to avoid collision", vehicles[idx_a].id);
            } else if should_slow {
                vehicles[idx_a].set_target_velocity(VelocityLevel::Slow);
                vehicles[idx_a].current_velocity *= 0.7; // Significant slowdown
            } else {
                // FIXED: More conservative speed recovery
                self.allow_safe_speedup(idx_a, vehicles, intersection);
            }
        }
    }

    // FIXED: Much stricter yielding rules
    fn should_yield_strict(&self, vehicle_a: &Vehicle, vehicle_b: &Vehicle, intersection: &Intersection) -> bool {
        // Rule 1: Vehicles already in the intersection have absolute priority
        if vehicle_b.is_in_intersection(intersection) && !vehicle_a.is_in_intersection(intersection) {
            return true;
        }
        if vehicle_a.is_in_intersection(intersection) && !vehicle_b.is_in_intersection(intersection) {
            return false;
        }

        // Rule 2: Same lane - vehicle behind yields
        if vehicle_a.direction == vehicle_b.direction && vehicle_a.lane == vehicle_b.lane {
            return self.is_vehicle_behind(vehicle_a, vehicle_b);
        }

        // Rule 3: Intersection conflicts - use right-of-way rules
        if vehicle_a.is_approaching_intersection(intersection) && vehicle_b.is_approaching_intersection(intersection) {
            // Vehicles with reservations have priority
            if vehicle_b.has_intersection_reservation() && !vehicle_a.has_intersection_reservation() {
                return true;
            }
            if vehicle_a.has_intersection_reservation() && !vehicle_b.has_intersection_reservation() {
                return false;
            }

            // FIXED: Stricter right-of-way rules based on actual traffic rules
            match (vehicle_a.direction, vehicle_b.direction) {
                // Opposing traffic - left turns yield to straight/right
                (Direction::North, Direction::South) | (Direction::South, Direction::North) => {
                    if vehicle_a.route == crate::vehicle::Route::Left && vehicle_b.route != crate::vehicle::Route::Left {
                        return true;
                    }
                    if vehicle_b.route == crate::vehicle::Route::Left && vehicle_a.route != crate::vehicle::Route::Left {
                        return false;
                    }
                }
                (Direction::East, Direction::West) | (Direction::West, Direction::East) => {
                    if vehicle_a.route == crate::vehicle::Route::Left && vehicle_b.route != crate::vehicle::Route::Left {
                        return true;
                    }
                    if vehicle_b.route == crate::vehicle::Route::Left && vehicle_a.route != crate::vehicle::Route::Left {
                        return false;
                    }
                }

                // Perpendicular traffic - right-hand rule (vehicle from right has priority)
                (Direction::North, Direction::East) => return true,  // East (right) has priority
                (Direction::East, Direction::South) => return true,  // South (right) has priority
                (Direction::South, Direction::West) => return true,  // West (right) has priority
                (Direction::West, Direction::North) => return true,  // North (right) has priority
                (Direction::East, Direction::North) => return false,
                (Direction::South, Direction::East) => return false,
                (Direction::West, Direction::South) => return false,
                (Direction::North, Direction::West) => return false,
                _ => {}
            }
        }

        // Rule 4: Time to intersection (closer vehicle has priority)
        let time_a = vehicle_a.time_to_intersection(intersection);
        let time_b = vehicle_b.time_to_intersection(intersection);

        // Vehicle that will reach intersection first has right of way
        time_a > time_b + 0.5 // Add buffer for safety
    }

    // Check if vehicle_a is behind vehicle_b in the same lane
    fn is_vehicle_behind(&self, vehicle_a: &Vehicle, vehicle_b: &Vehicle) -> bool {
        if vehicle_a.direction != vehicle_b.direction || vehicle_a.lane != vehicle_b.lane {
            return false;
        }

        match vehicle_a.direction {
            Direction::North => vehicle_a.position.y > vehicle_b.position.y,
            Direction::South => vehicle_a.position.y < vehicle_b.position.y,
            Direction::East => vehicle_a.position.x < vehicle_b.position.x,
            Direction::West => vehicle_a.position.x > vehicle_b.position.x,
        }
    }

    // FIXED: Much more conservative speed-up conditions
    fn allow_safe_speedup(&mut self, vehicle_idx: usize, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection) {
        // First, check safety without borrowing conflicts
        let mut safe_to_speedup = true;
        let safety_radius = 150.0; // Large safety radius
        let vehicle_pos = vehicles[vehicle_idx].position;
        let vehicle_state = vehicles[vehicle_idx].state;
        let vehicle_id = vehicles[vehicle_idx].id;
        let vehicle_is_in_intersection = vehicles[vehicle_idx].is_in_intersection(intersection);

        // Only allow speedup if there are no conflicts and vehicle is not in intersection
        if !vehicle_is_in_intersection {
            // Check all other vehicles within safety radius
            for (i, other) in vehicles.iter().enumerate() {
                if i == vehicle_idx || other.state == VehicleState::Completed {
                    continue;
                }

                let dx = (vehicle_pos.x - other.position.x) as f64;
                let dy = (vehicle_pos.y - other.position.y) as f64;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < safety_radius {
                    // Simple collision check to avoid complex borrowing
                    if vehicles[vehicle_idx].could_collide_with(other, intersection) {
                        safe_to_speedup = false;
                        break;
                    }
                }
            }

            if safe_to_speedup {
                match vehicle_state {
                    VehicleState::Approaching => {
                        vehicles[vehicle_idx].set_target_velocity(VelocityLevel::Medium);
                    }
                    VehicleState::Exiting | VehicleState::Completed => {
                        vehicles[vehicle_idx].set_target_velocity(VelocityLevel::Fast);
                    }
                    _ => {
                        // Don't speed up in intersection
                    }
                }
            }
        }
    }

    // FIXED: Strict intersection access management
    fn manage_intersection_access_strict(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection) {
        // Count vehicles currently in intersection
        let vehicles_in_intersection: Vec<usize> = vehicles.iter()
            .enumerate()
            .filter(|(_, v)| v.is_in_intersection(intersection))
            .map(|(i, _)| i)
            .collect();

        // If intersection is at capacity, stop approaching vehicles
        if vehicles_in_intersection.len() >= self.max_simultaneous_vehicles {
            for (i, vehicle) in vehicles.iter_mut().enumerate() {
                if vehicle.is_approaching_intersection(intersection) && !vehicles_in_intersection.contains(&i) {
                    vehicle.set_target_velocity(VelocityLevel::Slow);
                    vehicle.current_velocity *= 0.6; // Significant slowdown
                }
            }
            return;
        }

        // FIXED: Collect reservation decisions first, then apply them
        let mut reservation_decisions = Vec::new();

        // First pass: decide who gets reservations
        for (i, vehicle) in vehicles.iter().enumerate() {
            if vehicle.is_approaching_intersection(intersection) && !vehicle.has_intersection_reservation() {
                let can_grant = self.can_grant_intersection_reservation_safe(i, vehicles, intersection);
                reservation_decisions.push((i, vehicle.id, can_grant));
            }
        }

        // Second pass: apply reservation decisions
        for (vehicle_idx, vehicle_id, should_grant) in reservation_decisions {
            if should_grant {
                vehicles[vehicle_idx].set_intersection_reservation(true);
                self.intersection_reservations.insert(vehicle_id, self.current_time + 10.0);
                println!("🎫 Granted intersection reservation to vehicle {}", vehicle_id);
            } else {
                vehicles[vehicle_idx].set_target_velocity(VelocityLevel::Slow);
            }
        }

        // Allow reserved vehicles to proceed at normal speed
        for vehicle in vehicles.iter_mut() {
            if vehicle.has_intersection_reservation() && vehicle.is_approaching_intersection(intersection) {
                if vehicles_in_intersection.len() < self.max_simultaneous_vehicles {
                    vehicle.set_target_velocity(VelocityLevel::Medium);
                }
            }
        }
    }

    // FIXED: Check if it's safe to grant intersection reservation (immutable version)
    fn can_grant_intersection_reservation_safe(&self, vehicle_idx: usize, vehicles: &VecDeque<Vehicle>, intersection: &Intersection) -> bool {
        let vehicle = &vehicles[vehicle_idx];

        // Check conflicts with vehicles already in intersection
        for (i, other) in vehicles.iter().enumerate() {
            if i == vehicle_idx {
                continue;
            }

            if other.is_in_intersection(intersection) || other.has_intersection_reservation() {
                if vehicle.could_collide_with(other, intersection) {
                    let time_to_conflict = vehicle.time_to_intersection(intersection);
                    let other_time = other.time_to_intersection(intersection);

                    // Don't grant if conflict would occur within 3 seconds
                    if (time_to_conflict - other_time).abs() < 3.0 {
                        return false;
                    }
                }
            }
        }

        true
    }

    fn cleanup_expired_reservations(&mut self) {
        self.intersection_reservations.retain(|_, &mut expiration_time| {
            expiration_time > self.current_time
        });
    }

    fn record_close_call(&mut self, vehicle1_id: u32, vehicle2_id: u32) {
        self.close_calls += 1;
        println!("⚠️ CLOSE CALL #{}: Vehicles {} and {} nearly collided!", self.close_calls, vehicle1_id, vehicle2_id);

        // Record in violations map
        self.safe_distance_violations
            .entry(vehicle1_id)
            .or_insert(Vec::new())
            .push(vehicle2_id);
    }

    // Analyze congestion levels for each direction and lane
    fn analyze_congestion(&mut self, vehicles: &VecDeque<Vehicle>) {
        // Reset congestion counts
        self.congestion_levels.clear();

        // Count vehicles per direction and lane
        for vehicle in vehicles {
            if vehicle.state == VehicleState::Approaching || vehicle.state == VehicleState::Entering {
                let key = (vehicle.direction, vehicle.lane);
                *self.congestion_levels.entry(key).or_insert(0) += 1;
            }
        }

        // Calculate total congestion per direction
        let mut direction_congestion = [0; 4];
        for ((direction, _), count) in &self.congestion_levels {
            let dir_index = match direction {
                Direction::North => 0,
                Direction::South => 1,
                Direction::East => 2,
                Direction::West => 3,
            };
            direction_congestion[dir_index] += count;
        }

        // FIXED: More conservative congestion threshold
        self.adaptive_mode = direction_congestion.iter().any(|&count| count > 6); // Reduced from 12

        // Update direction priority based on throughput deficit
        for i in 0..4 {
            if direction_congestion[i] > 4 { // Reduced threshold
                self.direction_priority[i] += 2;
            } else if direction_congestion[i] > 2 {
                self.direction_priority[i] += 1;
            }

            // Cap priority
            if self.direction_priority[i] > 8 { // Reduced cap
                self.direction_priority[i] = 8;
            }
        }

        if self.adaptive_mode {
            println!("🚨 ADAPTIVE MODE: High congestion detected - {:?}", direction_congestion);
        }
    }

    // FIXED: Stricter safe distance checking
    fn check_safe_distances(&mut self, vehicles: &VecDeque<Vehicle>) {
        for (i, vehicle_a) in vehicles.iter().enumerate() {
            for (j, vehicle_b) in vehicles.iter().enumerate() {
                if i == j {
                    continue;
                }

                if vehicle_a.state == VehicleState::Completed || vehicle_b.state == VehicleState::Completed {
                    continue;
                }

                // Check if vehicles are in the same lane and direction
                if vehicle_a.direction == vehicle_b.direction && vehicle_a.lane == vehicle_b.lane {
                    let distance = self.calculate_distance(vehicle_a, vehicle_b);

                    // FIXED: Stricter safe distance checking
                    if distance < Vehicle::SAFE_DISTANCE {
                        // Record safe distance violation
                        let violations = self.safe_distance_violations
                            .entry(vehicle_a.id)
                            .or_insert(Vec::new());

                        if !violations.contains(&vehicle_b.id) {
                            self.close_calls += 1;
                            violations.push(vehicle_b.id);
                            println!("⚠️ Safe distance violation between vehicles {} and {} (distance: {:.1})",
                                     vehicle_a.id, vehicle_b.id, distance);
                        }
                    }
                }

                // Also check for general collision proximity
                let distance = self.calculate_distance(vehicle_a, vehicle_b);
                if distance < 35.0 && vehicle_a.could_collide_with(vehicle_b, &Intersection::new()) {
                    let violations = self.safe_distance_violations
                        .entry(vehicle_a.id)
                        .or_insert(Vec::new());

                    if !violations.contains(&vehicle_b.id) {
                        self.close_calls += 1;
                        violations.push(vehicle_b.id);
                        println!("⚠️ Proximity warning: vehicles {} and {} very close (distance: {:.1})",
                                 vehicle_a.id, vehicle_b.id, distance);
                    }
                }
            }
        }
    }

    fn distance_to_intersection_center(&self, vehicle: &Vehicle) -> f64 {
        let center_x = crate::WINDOW_WIDTH as f64 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f64 / 2.0;

        let dx = vehicle.position.x as f64 - center_x;
        let dy = vehicle.position.y as f64 - center_y;

        (dx * dx + dy * dy).sqrt()
    }

    fn calculate_distance(&self, vehicle1: &Vehicle, vehicle2: &Vehicle) -> f64 {
        let dx = (vehicle1.position.x - vehicle2.position.x) as f64;
        let dy = (vehicle1.position.y - vehicle2.position.y) as f64;
        (dx * dx + dy * dy).sqrt()
    }

    fn estimate_time_to_collision(&self, vehicle1: &Vehicle, vehicle2: &Vehicle) -> f64 {
        let distance = self.calculate_distance(vehicle1, vehicle2);

        let relative_speed = if vehicle1.direction == vehicle2.direction {
            (vehicle1.current_velocity - vehicle2.current_velocity).abs().max(1.0)
        } else {
            (vehicle1.current_velocity + vehicle2.current_velocity).max(1.0)
        };

        distance / relative_speed
    }
}