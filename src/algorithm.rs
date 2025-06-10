// src/algorithm.rs - FIXED: Smart collision prevention for synchronized lane system
use crate::intersection::Intersection;
use crate::vehicle::{Vehicle, VehicleState, VelocityLevel, Direction, Route};
use std::collections::{HashMap, VecDeque};

pub struct SmartIntersection {
    // Keep track of close calls
    pub close_calls: u32,
    // Track safe distance violations with timestamps
    safe_distance_violations: HashMap<u32, Vec<(u32, f64)>>, // Vehicle ID to list of (other_vehicle_id, timestamp)
    // Track congestion by direction and lane
    congestion_levels: HashMap<(Direction, usize), u32>,
    // Enable adaptive mode for high traffic
    adaptive_mode: bool,
    // Track throughput for each direction
    direction_priority: [u32; 4], // Priority counter for [North, South, East, West]
    // FIXED: Reservation system for intersection access
    intersection_reservations: HashMap<u32, f64>, // Vehicle ID -> expiration time
    current_time: f64,
    // FIXED: Conservative intersection management
    max_simultaneous_vehicles: usize,
    critical_collision_distance: f64,
    safe_following_distance: f64,
    // Performance tracking
    last_cleanup_time: f64,
    throughput_counter: u32,
}

impl SmartIntersection {
    pub fn new() -> Self {
        SmartIntersection {
            close_calls: 0,
            safe_distance_violations: HashMap::new(),
            congestion_levels: HashMap::new(),
            adaptive_mode: false,
            direction_priority: [0; 4],
            intersection_reservations: HashMap::new(),
            current_time: 0.0,
            max_simultaneous_vehicles: 3, // Very conservative
            critical_collision_distance: 100.0, // Increased safety distance
            safe_following_distance: 120.0, // Even larger following distance
            last_cleanup_time: 0.0,
            throughput_counter: 0,
        }
    }

    // FIXED: Process all vehicles with smart collision prevention
    pub fn process_vehicles(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection, delta_time: u32) {
        self.current_time += delta_time as f64 / 1000.0;

        // Clean up expired data periodically
        if self.current_time - self.last_cleanup_time > 5.0 {
            self.cleanup_expired_data();
            self.last_cleanup_time = self.current_time;
        }

        // First, update all vehicle positions
        for vehicle in vehicles.iter_mut() {
            vehicle.update(delta_time, intersection);
        }

        // Analyze traffic patterns
        self.analyze_traffic_patterns(vehicles);

        // FIXED: Smart collision prevention with lane awareness
        self.apply_smart_collision_prevention(vehicles, intersection);

        // Manage intersection access intelligently
        self.manage_smart_intersection_access(vehicles, intersection);

        // Check for safety violations
        self.check_safety_violations(vehicles);
    }

    // FIXED: Smart collision prevention that understands lane separation
    fn apply_smart_collision_prevention(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection) {
        // Group vehicles by potential conflict zones (immutable access)
        let conflict_groups = self.group_vehicles_by_conflict_zones(vehicles, intersection);

        // Process each conflict group separately (mutable access)
        for group in conflict_groups.iter() {
            if group.len() <= 1 {
                continue;
            }

            // Create a sorted copy of the group
            let mut sorted_group = group.clone();

            // Sort by priority (distance to intersection, route type, etc.)
            sorted_group.sort_by(|&a, &b| {
                let vehicle_a = &vehicles[a];
                let vehicle_b = &vehicles[b];

                // Priority rules:
                // 1. Vehicles already in intersection have highest priority
                let a_in_intersection = vehicle_a.is_in_intersection(intersection);
                let b_in_intersection = vehicle_b.is_in_intersection(intersection);

                if a_in_intersection && !b_in_intersection {
                    return std::cmp::Ordering::Less;
                }
                if !a_in_intersection && b_in_intersection {
                    return std::cmp::Ordering::Greater;
                }

                // 2. Straight traffic has priority over turning traffic
                let a_straight = vehicle_a.route == Route::Straight;
                let b_straight = vehicle_b.route == Route::Straight;

                if a_straight && !b_straight {
                    return std::cmp::Ordering::Less;
                }
                if !a_straight && b_straight {
                    return std::cmp::Ordering::Greater;
                }

                // 3. Closer to intersection has priority
                let dist_a = self.distance_to_intersection_center(vehicle_a);
                let dist_b = self.distance_to_intersection_center(vehicle_b);
                dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
            });

            // Apply conflict resolution
            self.resolve_conflicts_in_group(vehicles, intersection, &sorted_group);
        }
    }

    // FIXED: Group vehicles that could potentially conflict
    fn group_vehicles_by_conflict_zones(&self, vehicles: &VecDeque<Vehicle>, intersection: &Intersection) -> Vec<Vec<usize>> {
        let mut groups = Vec::new();
        let mut processed = vec![false; vehicles.len()];

        for i in 0..vehicles.len() {
            if processed[i] || vehicles[i].state == VehicleState::Completed {
                continue;
            }

            let mut group = vec![i];
            processed[i] = true;

            // Find all vehicles that could conflict with vehicle i
            for j in (i + 1)..vehicles.len() {
                if processed[j] || vehicles[j].state == VehicleState::Completed {
                    continue;
                }

                if self.could_vehicles_conflict(&vehicles[i], &vehicles[j], intersection) {
                    group.push(j);
                    processed[j] = true;
                }
            }

            if group.len() > 1 {
                groups.push(group);
            }
        }

        groups
    }

    // FIXED: Determine if two vehicles could conflict
    fn could_vehicles_conflict(&self, vehicle_a: &Vehicle, vehicle_b: &Vehicle, intersection: &Intersection) -> bool {
        // Check physical proximity
        let distance = self.calculate_distance(vehicle_a, vehicle_b);
        if distance > 200.0 {
            return false; // Too far apart to conflict
        }

        // Same lane same direction - always potential conflict
        if vehicle_a.direction == vehicle_b.direction && vehicle_a.lane == vehicle_b.lane {
            return true;
        }

        // FIXED: Only check intersection conflicts for vehicles actually approaching/in intersection
        let a_near_intersection = vehicle_a.is_approaching_intersection(intersection) || vehicle_a.is_in_intersection(intersection);
        let b_near_intersection = vehicle_b.is_approaching_intersection(intersection) || vehicle_b.is_in_intersection(intersection);

        if !a_near_intersection || !b_near_intersection {
            return false;
        }

        // FIXED: Straight traffic in properly separated lanes should NEVER conflict
        if vehicle_a.route == Route::Straight && vehicle_b.route == Route::Straight {
            // Check if they're in truly separated lanes
            match (vehicle_a.direction, vehicle_b.direction) {
                (Direction::North, Direction::South) | (Direction::South, Direction::North) => false, // Vertical separation
                (Direction::East, Direction::West) | (Direction::West, Direction::East) => false,   // Horizontal separation
                _ => false, // Perpendicular straight traffic is separated by design
            }
        } else {
            // Only specific turning scenarios can conflict
            self.do_turning_paths_intersect(vehicle_a, vehicle_b)
        }
    }

    // FIXED: Check if turning paths actually intersect
    fn do_turning_paths_intersect(&self, vehicle_a: &Vehicle, vehicle_b: &Vehicle) -> bool {
        match (vehicle_a.direction, vehicle_a.route, vehicle_b.direction, vehicle_b.route) {
            // Left turner vs straight from perpendicular direction
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

            // Right turns generally don't conflict with others (tight turns)
            _ => false,
        }
    }

    // FIXED: Resolve conflicts within a group of vehicles
    fn resolve_conflicts_in_group(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection, group: &[usize]) {
        if group.len() <= 1 {
            return;
        }

        let priority_vehicle_idx = group[0];

        // Check if priority vehicle can proceed safely (immutable borrow)
        let can_proceed = self.is_safe_to_proceed(vehicles, intersection, priority_vehicle_idx, group);

        // Apply decisions (mutable borrow)
        if can_proceed {
            self.allow_vehicle_to_proceed(vehicles, priority_vehicle_idx);
        } else {
            // Even priority vehicle must slow down
            self.apply_conservative_slowdown(vehicles, priority_vehicle_idx);
        }

        // Other vehicles must yield
        for &vehicle_idx in &group[1..] {
            self.apply_yielding_behavior(vehicles, intersection, vehicle_idx, priority_vehicle_idx);
        }
    }

    // FIXED: Check if it's safe for a vehicle to proceed
    fn is_safe_to_proceed(&self, vehicles: &VecDeque<Vehicle>, intersection: &Intersection, vehicle_idx: usize, group: &[usize]) -> bool {
        let vehicle = &vehicles[vehicle_idx];

        // Check if path is clear ahead
        for &other_idx in group {
            if other_idx == vehicle_idx {
                continue;
            }

            let other = &vehicles[other_idx];
            let distance = self.calculate_distance(vehicle, other);

            // Too close - not safe
            if distance < self.safe_following_distance {
                return false;
            }

            // Check if other vehicle is blocking the path
            if self.is_vehicle_blocking_path(vehicle, other, intersection) {
                return false;
            }
        }

        // Check intersection capacity
        let vehicles_in_intersection = vehicles.iter()
            .filter(|v| v.is_in_intersection(intersection))
            .count();

        if vehicles_in_intersection >= self.max_simultaneous_vehicles && !vehicle.is_in_intersection(intersection) {
            return false;
        }

        true
    }

    // FIXED: Check if one vehicle is blocking another's path
    fn is_vehicle_blocking_path(&self, vehicle: &Vehicle, other: &Vehicle, intersection: &Intersection) -> bool {
        // Same lane same direction
        if vehicle.direction == other.direction && vehicle.lane == other.lane {
            return self.is_vehicle_ahead_in_same_lane(vehicle, other);
        }

        // Different paths - check if they intersect spatially
        if vehicle.could_collide_with(other, intersection) {
            let time_to_intersection_self = vehicle.time_to_intersection(intersection);
            let time_to_intersection_other = other.time_to_intersection(intersection);

            // If arrival times are too close, it's blocking
            return (time_to_intersection_self - time_to_intersection_other).abs() < 2.0;
        }

        false
    }

    fn is_vehicle_ahead_in_same_lane(&self, vehicle: &Vehicle, other: &Vehicle) -> bool {
        if vehicle.direction != other.direction || vehicle.lane != other.lane {
            return false;
        }

        let distance_ahead = match vehicle.direction {
            Direction::North => other.position.y - vehicle.position.y,
            Direction::South => vehicle.position.y - other.position.y,
            Direction::East => other.position.x - vehicle.position.x,
            Direction::West => vehicle.position.x - other.position.x,
        };

        distance_ahead > 0 && distance_ahead < self.safe_following_distance as i32
    }

    fn allow_vehicle_to_proceed(&mut self, vehicles: &mut VecDeque<Vehicle>, vehicle_idx: usize) {
        let vehicle = &mut vehicles[vehicle_idx];

        match vehicle.state {
            VehicleState::Approaching => {
                vehicle.set_target_velocity(VelocityLevel::Medium);
            }
            VehicleState::Entering | VehicleState::Turning => {
                vehicle.set_target_velocity(VelocityLevel::Slow); // Conservative in intersection
            }
            VehicleState::Exiting => {
                vehicle.set_target_velocity(VelocityLevel::Medium);
            }
            VehicleState::Completed => {
                vehicle.set_target_velocity(VelocityLevel::Fast);
            }
        }
    }

    fn apply_conservative_slowdown(&mut self, vehicles: &mut VecDeque<Vehicle>, vehicle_idx: usize) {
        let vehicle = &mut vehicles[vehicle_idx];
        vehicle.set_target_velocity(VelocityLevel::Slow);

        // Additional emergency braking if very close to conflict
        if vehicle.current_velocity > Vehicle::SLOW_VELOCITY {
            vehicle.current_velocity *= 0.8;
        }
    }

    fn apply_yielding_behavior(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection, vehicle_idx: usize, priority_vehicle_idx: usize) {
        // Calculate distance first (immutable access)
        let distance_to_priority = {
            let vehicle = &vehicles[vehicle_idx];
            let priority_vehicle = &vehicles[priority_vehicle_idx];
            self.calculate_distance(vehicle, priority_vehicle)
        };

        // Apply yielding behavior based on distance (mutable access)
        if distance_to_priority < self.critical_collision_distance {
            // Emergency stop
            vehicles[vehicle_idx].set_target_velocity(VelocityLevel::Slow);
            vehicles[vehicle_idx].current_velocity *= 0.5; // Emergency braking

            let vehicle_id = vehicles[vehicle_idx].id;
            let priority_id = vehicles[priority_vehicle_idx].id;
            self.record_close_call(vehicle_id, priority_id);
        } else if distance_to_priority < self.safe_following_distance {
            // Gradual slowdown
            vehicles[vehicle_idx].set_target_velocity(VelocityLevel::Slow);
        } else {
            // Cautious approach
            match vehicles[vehicle_idx].state {
                VehicleState::Approaching => {
                    vehicles[vehicle_idx].set_target_velocity(VelocityLevel::Slow);
                }
                _ => {
                    vehicles[vehicle_idx].set_target_velocity(VelocityLevel::Medium);
                }
            }
        }
    }

    // FIXED: Smart intersection access management
    fn manage_smart_intersection_access(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection) {
        // Count vehicles currently in intersection
        let vehicles_in_intersection: Vec<usize> = vehicles.iter()
            .enumerate()
            .filter(|(_, v)| v.is_in_intersection(intersection))
            .map(|(i, _)| i)
            .collect();

        // If at capacity, deny new entries
        if vehicles_in_intersection.len() >= self.max_simultaneous_vehicles {
            for (i, vehicle) in vehicles.iter_mut().enumerate() {
                if vehicle.is_approaching_intersection(intersection) && !vehicles_in_intersection.contains(&i) {
                    vehicle.set_target_velocity(VelocityLevel::Slow);
                }
            }
            return;
        }

        // FIXED: Collect access decisions first, then apply them
        let mut access_decisions = Vec::new();

        // First pass: decide who gets access (immutable borrow)
        for (i, vehicle) in vehicles.iter().enumerate() {
            if vehicle.is_approaching_intersection(intersection) && !vehicle.has_intersection_reservation() {
                let can_grant = self.can_grant_safe_access(i, vehicles, intersection);
                access_decisions.push((i, vehicle.id, can_grant));
            }
        }

        // Second pass: apply decisions (mutable borrow)
        for (vehicle_idx, vehicle_id, should_grant) in access_decisions {
            if should_grant {
                vehicles[vehicle_idx].set_intersection_reservation(true);
                self.intersection_reservations.insert(vehicle_id, self.current_time + 15.0);
                println!("🎫 Granted intersection access to vehicle {}", vehicle_id);
            }
        }
    }

    fn can_grant_safe_access(&self, vehicle_idx: usize, vehicles: &VecDeque<Vehicle>, intersection: &Intersection) -> bool {
        let vehicle = &vehicles[vehicle_idx];

        // Check for conflicts with vehicles already in intersection
        for (i, other) in vehicles.iter().enumerate() {
            if i == vehicle_idx {
                continue;
            }

            if other.is_in_intersection(intersection) || other.has_intersection_reservation() {
                if vehicle.could_collide_with(other, intersection) {
                    let time_diff = (vehicle.time_to_intersection(intersection) - other.time_to_intersection(intersection)).abs();
                    if time_diff < 4.0 { // Increased safety margin
                        return false;
                    }
                }
            }
        }

        true
    }

    fn analyze_traffic_patterns(&mut self, vehicles: &VecDeque<Vehicle>) {
        // Reset congestion counts
        self.congestion_levels.clear();

        // Count vehicles per direction and lane
        for vehicle in vehicles {
            if matches!(vehicle.state, VehicleState::Approaching | VehicleState::Entering) {
                let key = (vehicle.direction, vehicle.lane);
                *self.congestion_levels.entry(key).or_insert(0) += 1;
            }
        }

        // Determine if adaptive mode is needed
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

        self.adaptive_mode = direction_congestion.iter().any(|&count| count > 4);

        if self.adaptive_mode {
            println!("🚨 ADAPTIVE MODE: High congestion detected - {:?}", direction_congestion);
        }
    }

    fn check_safety_violations(&mut self, vehicles: &VecDeque<Vehicle>) {
        for (i, vehicle_a) in vehicles.iter().enumerate() {
            for (j, vehicle_b) in vehicles.iter().enumerate() {
                if i == j || vehicle_a.state == VehicleState::Completed || vehicle_b.state == VehicleState::Completed {
                    continue;
                }

                let distance = self.calculate_distance(vehicle_a, vehicle_b);

                // Check safe distance violations
                if distance < Vehicle::SAFE_DISTANCE {
                    self.record_safe_distance_violation(vehicle_a.id, vehicle_b.id);
                }

                // Check critical proximity
                if distance < 30.0 && vehicle_a.could_collide_with(vehicle_b, &Intersection::new()) {
                    self.record_close_call(vehicle_a.id, vehicle_b.id);
                }
            }
        }
    }

    fn record_close_call(&mut self, vehicle1_id: u32, vehicle2_id: u32) {
        // Avoid duplicate close calls between same pair
        let violations = self.safe_distance_violations
            .entry(vehicle1_id)
            .or_insert(Vec::new());

        let already_recorded = violations.iter()
            .any(|(id, time)| *id == vehicle2_id && self.current_time - time < 5.0);

        if !already_recorded {
            self.close_calls += 1;
            violations.push((vehicle2_id, self.current_time));
            println!("⚠️ CLOSE CALL #{}: Vehicles {} and {} nearly collided!",
                     self.close_calls, vehicle1_id, vehicle2_id);
        }
    }

    fn record_safe_distance_violation(&mut self, vehicle1_id: u32, vehicle2_id: u32) {
        let violations = self.safe_distance_violations
            .entry(vehicle1_id)
            .or_insert(Vec::new());

        let already_recorded = violations.iter()
            .any(|(id, time)| *id == vehicle2_id && self.current_time - time < 2.0);

        if !already_recorded {
            violations.push((vehicle2_id, self.current_time));
        }
    }

    fn cleanup_expired_data(&mut self) {
        // Clean up old violations
        for violations in self.safe_distance_violations.values_mut() {
            violations.retain(|(_, time)| self.current_time - time < 10.0);
        }

        // Clean up expired reservations
        self.intersection_reservations.retain(|_, &mut expiration_time| {
            expiration_time > self.current_time
        });
    }

    // Utility functions
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

    pub fn get_statistics(&self) -> (u32, f64, usize, usize) {
        let avg_congestion = if !self.congestion_levels.is_empty() {
            self.congestion_levels.values().sum::<u32>() as f64 / self.congestion_levels.len() as f64
        } else {
            0.0
        };

        (
            self.throughput_counter,
            avg_congestion,
            self.intersection_reservations.len(),
            self.safe_distance_violations.len(),
        )
    }
}