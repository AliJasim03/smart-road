// src/algorithm.rs - FIXED: Simple and robust collision prevention
use crate::intersection::Intersection;
use crate::vehicle::{Vehicle, VehicleState, VelocityLevel, Direction, Route, Vec2};
use std::collections::{HashMap, VecDeque};

pub struct SmartIntersection {
    // Core tracking
    pub close_calls: u32,
    safe_distance_violations: HashMap<u32, Vec<(u32, f64)>>,
    current_time: f64,

    // Simple intersection management
    intersection_occupancy: Vec<u32>,  // Vehicle IDs currently in intersection
    max_intersection_capacity: usize,

    // Performance tracking
    last_cleanup_time: f64,
    throughput_counter: u32,

    // Safe distances
    critical_distance: f32,
    safe_following_distance: f32,

    // Simple conflict detection
    conflict_cooldowns: HashMap<(u32, u32), f64>,  // Vehicle pair -> cooldown time
}

impl SmartIntersection {
    pub fn new() -> Self {
        SmartIntersection {
            close_calls: 0,
            safe_distance_violations: HashMap::new(),
            current_time: 0.0,
            intersection_occupancy: Vec::new(),
            max_intersection_capacity: 2, // Only 2 vehicles in intersection at once
            last_cleanup_time: 0.0,
            throughput_counter: 0,
            critical_distance: 35.0,  // Very close
            safe_following_distance: 80.0, // Following distance
            conflict_cooldowns: HashMap::new(),
        }
    }

    // SIMPLIFIED: Process vehicles with robust collision prevention
    pub fn process_vehicles(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection, delta_time: u32) {
        self.current_time += delta_time as f64 / 1000.0;

        // Periodic cleanup
        if self.current_time - self.last_cleanup_time > 3.0 {
            self.cleanup_expired_data();
            self.last_cleanup_time = self.current_time;
        }

        // Update intersection occupancy
        self.update_intersection_occupancy(vehicles, intersection);

        // Apply simple collision prevention
        self.apply_simple_collision_prevention(vehicles, intersection);

        // Manage intersection access
        self.manage_intersection_access(vehicles, intersection);

        // Check for safety violations
        self.check_safety_violations(vehicles);
    }

    fn update_intersection_occupancy(&mut self, vehicles: &VecDeque<Vehicle>, intersection: &Intersection) {
        self.intersection_occupancy.clear();

        for vehicle in vehicles {
            if vehicle.is_in_intersection(intersection) {
                self.intersection_occupancy.push(vehicle.id);
            }
        }
    }

    // SIMPLE: Robust collision prevention
    fn apply_simple_collision_prevention(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection) {
        for i in 0..vehicles.len() {
            let vehicle_id = vehicles[i].id;
            let mut should_slow = false;
            let mut should_stop = false;

            // Check conflicts with other vehicles
            for j in 0..vehicles.len() {
                if i == j {
                    continue;
                }

                let distance = self.calculate_distance(&vehicles[i], &vehicles[j]);

                // Critical distance - emergency stop
                if distance < self.critical_distance {
                    if self.vehicles_will_conflict(&vehicles[i], &vehicles[j], intersection) {
                        should_stop = true;
                        self.record_close_call(vehicle_id, vehicles[j].id);
                        break;
                    }
                }

                // Safe following distance - slow down
                if distance < self.safe_following_distance {
                    if self.vehicles_will_conflict(&vehicles[i], &vehicles[j], intersection) {
                        should_slow = true;
                    }
                }
            }

            // Apply velocity adjustments
            if should_stop {
                vehicles[i].set_target_velocity(VelocityLevel::Slow);
                // Further reduce velocity for emergency
                if vehicles[i].current_velocity > Vehicle::SLOW_VELOCITY * 0.5 {
                    vehicles[i].current_velocity *= 0.7;
                }
            } else if should_slow {
                vehicles[i].set_target_velocity(VelocityLevel::Slow);
            } else {
                // Allow normal speeds based on state
                match vehicles[i].state {
                    VehicleState::Approaching => {
                        vehicles[i].set_target_velocity(VelocityLevel::Medium);
                    }
                    VehicleState::Entering => {
                        vehicles[i].set_target_velocity(VelocityLevel::Slow);
                    }
                    VehicleState::Exiting | VehicleState::Completed => {
                        vehicles[i].set_target_velocity(VelocityLevel::Fast);
                    }
                    // Remove AtTurnPoint and Turning states - not used anymore
                    _ => {
                        vehicles[i].set_target_velocity(VelocityLevel::Medium);
                    }
                }
            }
        }
    }

    // SIMPLE: Basic conflict detection
    fn vehicles_will_conflict(&self, vehicle1: &Vehicle, vehicle2: &Vehicle, intersection: &Intersection) -> bool {
        // Same lane same direction = always conflict
        if vehicle1.direction == vehicle2.direction && vehicle1.lane == vehicle2.lane {
            return self.is_vehicle_ahead(vehicle1, vehicle2);
        }

        // Both must be near intersection for intersection conflicts
        if !((vehicle1.is_approaching_intersection(intersection) || vehicle1.is_in_intersection(intersection)) &&
            (vehicle2.is_approaching_intersection(intersection) || vehicle2.is_in_intersection(intersection))) {
            return false;
        }

        // Simple intersection conflict rules
        match (vehicle1.direction, vehicle1.route, vehicle2.direction, vehicle2.route) {
            // Same direction conflicts
            (d1, _, d2, _) if d1 == d2 => true,

            // Left turn vs straight from perpendicular direction
            (Direction::North, Route::Left, Direction::East, Route::Straight) => true,
            (Direction::East, Route::Straight, Direction::North, Route::Left) => true,

            (Direction::South, Route::Left, Direction::West, Route::Straight) => true,
            (Direction::West, Route::Straight, Direction::South, Route::Left) => true,

            (Direction::East, Route::Left, Direction::South, Route::Straight) => true,
            (Direction::South, Route::Straight, Direction::East, Route::Left) => true,

            (Direction::West, Route::Left, Direction::North, Route::Straight) => true,
            (Direction::North, Route::Straight, Direction::West, Route::Left) => true,

            // Opposing left turns
            (Direction::North, Route::Left, Direction::South, Route::Left) => true,
            (Direction::East, Route::Left, Direction::West, Route::Left) => true,

            // No other conflicts
            _ => false,
        }
    }

    fn is_vehicle_ahead(&self, vehicle1: &Vehicle, vehicle2: &Vehicle) -> bool {
        if vehicle1.direction != vehicle2.direction || vehicle1.lane != vehicle2.lane {
            return false;
        }

        let distance_ahead = match vehicle1.direction {
            Direction::North => vehicle2.position.y - vehicle1.position.y,
            Direction::South => vehicle1.position.y - vehicle2.position.y,
            Direction::East => vehicle2.position.x - vehicle1.position.x,
            Direction::West => vehicle1.position.x - vehicle2.position.x,
        };

        distance_ahead > 0.0 && distance_ahead < self.safe_following_distance
    }

    // SIMPLE: Intersection access management
    fn manage_intersection_access(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection) {
        // If intersection is at capacity, stop approaching vehicles
        if self.intersection_occupancy.len() >= self.max_intersection_capacity {
            for vehicle in vehicles.iter_mut() {
                if vehicle.is_approaching_intersection(intersection) && !vehicle.has_intersection_reservation() {
                    vehicle.set_target_velocity(VelocityLevel::Slow);
                }
            }
            return;
        }

        // First, collect vehicles that need access decisions
        let mut access_decisions = Vec::new();

        for (i, vehicle) in vehicles.iter().enumerate() {
            if vehicle.is_approaching_intersection(intersection) && !vehicle.has_intersection_reservation() {
                let can_enter = self.can_safely_enter_by_index(i, vehicles, intersection);
                access_decisions.push((i, vehicle.id, can_enter));
            }
        }

        // Then apply the decisions
        for (vehicle_index, vehicle_id, should_grant) in access_decisions {
            if should_grant {
                vehicles[vehicle_index].set_intersection_reservation(true);
                println!("🎫 Granted intersection access to vehicle {}", vehicle_id);
            }
        }
    }

    fn can_safely_enter_by_index(&self, entering_vehicle_index: usize, all_vehicles: &VecDeque<Vehicle>, intersection: &Intersection) -> bool {
        let entering_vehicle = &all_vehicles[entering_vehicle_index];

        // Check conflicts with vehicles in intersection
        for (i, other) in all_vehicles.iter().enumerate() {
            if i == entering_vehicle_index {
                continue;
            }

            if other.is_in_intersection(intersection) || other.has_intersection_reservation() {
                if self.vehicles_will_conflict(entering_vehicle, other, intersection) {
                    // Check timing - if other vehicle will clear soon, allow entry
                    let time_diff = (entering_vehicle.time_to_intersection(intersection) -
                        other.time_to_intersection(intersection)).abs();
                    if time_diff < 2.0 { // 2 second safety margin
                        return false;
                    }
                }
            }
        }

        true
    }

    fn can_safely_enter(&self, entering_vehicle: &Vehicle, all_vehicles: &VecDeque<Vehicle>, intersection: &Intersection) -> bool {
        // Check conflicts with vehicles in intersection
        for other in all_vehicles {
            if other.id == entering_vehicle.id {
                continue;
            }

            if other.is_in_intersection(intersection) || other.has_intersection_reservation() {
                if self.vehicles_will_conflict(entering_vehicle, other, intersection) {
                    // Check timing - if other vehicle will clear soon, allow entry
                    let time_diff = (entering_vehicle.time_to_intersection(intersection) -
                        other.time_to_intersection(intersection)).abs();
                    if time_diff < 2.0 { // 2 second safety margin
                        return false;
                    }
                }
            }
        }

        true
    }

    fn check_safety_violations(&mut self, vehicles: &VecDeque<Vehicle>) {
        for (i, vehicle_a) in vehicles.iter().enumerate() {
            for (j, vehicle_b) in vehicles.iter().enumerate() {
                if i >= j || vehicle_a.state == VehicleState::Completed ||
                    vehicle_b.state == VehicleState::Completed {
                    continue;
                }

                let distance = self.calculate_distance(vehicle_a, vehicle_b);

                if distance < Vehicle::SAFE_DISTANCE {
                    self.record_safe_distance_violation(vehicle_a.id, vehicle_b.id);
                }

                if distance < self.critical_distance {
                    self.record_close_call(vehicle_a.id, vehicle_b.id);
                }
            }
        }
    }

    fn record_close_call(&mut self, vehicle1_id: u32, vehicle2_id: u32) {
        let pair = if vehicle1_id < vehicle2_id {
            (vehicle1_id, vehicle2_id)
        } else {
            (vehicle2_id, vehicle1_id)
        };

        // Check cooldown to prevent spam
        if let Some(&last_time) = self.conflict_cooldowns.get(&pair) {
            if self.current_time - last_time < 3.0 {
                return; // Still in cooldown
            }
        }

        self.close_calls += 1;
        self.conflict_cooldowns.insert(pair, self.current_time);

        println!("⚠️ CLOSE CALL #{}: Vehicles {} and {} too close!",
                 self.close_calls, vehicle1_id, vehicle2_id);
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

        // Clean up old conflict cooldowns
        self.conflict_cooldowns.retain(|_, &mut time| self.current_time - time < 5.0);

        // Remove vehicles that no longer have reservations
        // This would need additional tracking, simplified for now
    }

    fn calculate_distance(&self, vehicle1: &Vehicle, vehicle2: &Vehicle) -> f32 {
        let dx = vehicle1.position.x - vehicle2.position.x;
        let dy = vehicle1.position.y - vehicle2.position.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn get_statistics(&self) -> (u32, f64, usize, usize) {
        (
            self.throughput_counter,
            self.intersection_occupancy.len() as f64,
            self.intersection_occupancy.len(),
            self.safe_distance_violations.len(),
        )
    }

    // Record vehicle completion
    pub fn record_completion(&mut self, _vehicle_id: u32) {
        self.throughput_counter += 1;
    }

    // Get intersection status
    pub fn get_intersection_occupancy(&self) -> usize {
        self.intersection_occupancy.len()
    }

    pub fn is_intersection_clear(&self) -> bool {
        self.intersection_occupancy.is_empty()
    }

    // Get close call rate for statistics
    pub fn get_close_call_rate(&self) -> f32 {
        if self.throughput_counter == 0 {
            return 0.0;
        }
        (self.close_calls as f32 / self.throughput_counter as f32) * 100.0
    }

    // Simple priority system
    fn get_vehicle_priority(&self, vehicle: &Vehicle, intersection: &Intersection) -> u32 {
        let mut priority = 0;

        // Vehicles in intersection have highest priority
        if vehicle.is_in_intersection(intersection) {
            priority += 1000;
        }

        // Straight traffic has higher priority than turns
        if vehicle.route == Route::Straight {
            priority += 100;
        }

        // Right turns have priority over left turns
        if vehicle.route == Route::Right {
            priority += 50;
        }

        // Closer to intersection has higher priority
        let distance_to_center = self.distance_to_intersection_center(vehicle);
        priority += (1000.0 - distance_to_center.min(1000.0)) as u32;

        priority
    }

    fn distance_to_intersection_center(&self, vehicle: &Vehicle) -> f64 {
        let center_x = crate::WINDOW_WIDTH as f64 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f64 / 2.0;

        let dx = vehicle.position.x as f64 - center_x;
        let dy = vehicle.position.y as f64 - center_y;

        (dx * dx + dy * dy).sqrt()
    }

    // Enhanced debugging
    pub fn print_debug_status(&self, vehicles: &VecDeque<Vehicle>) {
        println!("\n=== ALGORITHM DEBUG STATUS ===");
        println!("Intersection occupancy: {}/{}", self.intersection_occupancy.len(), self.max_intersection_capacity);
        println!("Vehicles in intersection: {:?}", self.intersection_occupancy);
        println!("Total close calls: {}", self.close_calls);
        println!("Active conflict cooldowns: {}", self.conflict_cooldowns.len());

        for vehicle in vehicles {
            if vehicle.is_approaching_intersection(&crate::intersection::Intersection::new()) ||
                vehicle.is_in_intersection(&crate::intersection::Intersection::new()) {
                println!("  Vehicle {}: {:?} {:?} at ({:.1}, {:.1}) vel={:.1}",
                         vehicle.id, vehicle.state, vehicle.route,
                         vehicle.position.x, vehicle.position.y, vehicle.current_velocity);
            }
        }
        println!("==============================\n");
    }
}