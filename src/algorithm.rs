// src/algorithm.rs - ENHANCED: Smart collision prevention for smooth animation system
use crate::intersection::Intersection;
use crate::vehicle::{Vehicle, VehicleState, VelocityLevel, Direction, Route, Vec2};
use std::collections::{HashMap, VecDeque};

pub struct SmartIntersection {
    // Enhanced tracking for smooth animations
    pub close_calls: u32,
    safe_distance_violations: HashMap<u32, Vec<(u32, f64)>>,
    congestion_levels: HashMap<(Direction, usize), u32>,
    adaptive_mode: bool,
    direction_priority: [u32; 4],

    // ENHANCED: Smooth animation support
    intersection_reservations: HashMap<u32, f64>,
    current_time: f64,
    max_simultaneous_vehicles: usize,
    critical_collision_distance: f32,  // Now f32 for floating-point precision
    safe_following_distance: f32,      // Now f32 for floating-point precision

    // Performance and analytics
    last_cleanup_time: f64,
    throughput_counter: u32,

    // NEW: Smooth movement prediction
    vehicle_trajectories: HashMap<u32, Vec<Vec2>>,  // Vehicle ID -> predicted path
    turning_conflict_zones: Vec<ConflictZone>,      // Precomputed conflict areas

    // NEW: Advanced velocity management
    velocity_smoother: VelocitySmoother,

    // NEW: Dynamic intersection timing
    intersection_timing: IntersectionTiming,
}

// NEW: Conflict zone for smooth prediction
#[derive(Debug, Clone)]
struct ConflictZone {
    center: Vec2,
    radius: f32,
    affected_routes: Vec<(Direction, Route)>,
}

// NEW: Velocity smoothing for natural deceleration/acceleration
struct VelocitySmoother {
    velocity_history: HashMap<u32, Vec<f32>>,  // Vehicle ID -> velocity history
    smoothing_factor: f32,
    max_history_length: usize,
}

// NEW: Dynamic intersection timing management
struct IntersectionTiming {
    average_crossing_time: f32,
    crossing_times_by_route: HashMap<Route, f32>,
    last_update: f64,
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
            max_simultaneous_vehicles: 3,
            critical_collision_distance: 100.0,  // Increased for smooth animations
            safe_following_distance: 120.0,      // Increased for smooth animations
            last_cleanup_time: 0.0,
            throughput_counter: 0,
            vehicle_trajectories: HashMap::new(),
            turning_conflict_zones: Self::initialize_conflict_zones(),
            velocity_smoother: VelocitySmoother::new(),
            intersection_timing: IntersectionTiming::new(),
        }
    }

    // ENHANCED: Process all vehicles with smooth collision prevention
    pub fn process_vehicles(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection, delta_time: u32) {
        self.current_time += delta_time as f64 / 1000.0;

        // Periodic cleanup
        if self.current_time - self.last_cleanup_time > 5.0 {
            self.cleanup_expired_data();
            self.last_cleanup_time = self.current_time;
        }

        // Update vehicle trajectories for prediction
        self.update_vehicle_trajectories(vehicles, intersection);

        // Analyze traffic patterns
        self.analyze_traffic_patterns(vehicles);

        // ENHANCED: Smart collision prevention with smooth transitions
        self.apply_smooth_collision_prevention(vehicles, intersection);

        // Manage intersection access intelligently
        self.manage_smart_intersection_access(vehicles, intersection);

        // Check for safety violations
        self.check_safety_violations(vehicles, intersection);

        // Update intersection timing statistics
        self.intersection_timing.update(vehicles, self.current_time);

        // Apply velocity smoothing
        self.velocity_smoother.smooth_all_velocities(vehicles);
    }

    // NEW: Initialize conflict zones for smooth prediction
    fn initialize_conflict_zones() -> Vec<ConflictZone> {
        let center_x = crate::WINDOW_WIDTH as f32 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f32 / 2.0;
        let intersection_center = Vec2::new(center_x, center_y);

        vec![
            // Central conflict zone (all left turns)
            ConflictZone {
                center: intersection_center,
                radius: 50.0,
                affected_routes: vec![
                    (Direction::North, Route::Left),
                    (Direction::South, Route::Left),
                    (Direction::East, Route::Left),
                    (Direction::West, Route::Left),
                ],
            },
            // North-East conflict zone (North-left vs East-straight)
            ConflictZone {
                center: Vec2::new(center_x + 30.0, center_y - 30.0),
                radius: 40.0,
                affected_routes: vec![
                    (Direction::North, Route::Left),
                    (Direction::East, Route::Straight),
                ],
            },
            // South-West conflict zone (South-left vs West-straight)
            ConflictZone {
                center: Vec2::new(center_x - 30.0, center_y + 30.0),
                radius: 40.0,
                affected_routes: vec![
                    (Direction::South, Route::Left),
                    (Direction::West, Route::Straight),
                ],
            },
            // East-South conflict zone (East-left vs South-straight)
            ConflictZone {
                center: Vec2::new(center_x + 30.0, center_y + 30.0),
                radius: 40.0,
                affected_routes: vec![
                    (Direction::East, Route::Left),
                    (Direction::South, Route::Straight),
                ],
            },
            // West-North conflict zone (West-left vs North-straight)
            ConflictZone {
                center: Vec2::new(center_x - 30.0, center_y - 30.0),
                radius: 40.0,
                affected_routes: vec![
                    (Direction::West, Route::Left),
                    (Direction::North, Route::Straight),
                ],
            },
        ]
    }

    // NEW: Update vehicle trajectories for smooth prediction
    fn update_vehicle_trajectories(&mut self, vehicles: &VecDeque<Vehicle>, intersection: &Intersection) {
        for vehicle in vehicles {
            if vehicle.state == VehicleState::Completed {
                self.vehicle_trajectories.remove(&vehicle.id);
                continue;
            }

            let trajectory = self.predict_vehicle_trajectory(vehicle, intersection);
            self.vehicle_trajectories.insert(vehicle.id, trajectory);
        }
    }

    fn predict_vehicle_trajectory(&self, vehicle: &Vehicle, intersection: &Intersection) -> Vec<Vec2> {
        let mut trajectory = Vec::new();
        let prediction_time = 5.0; // Predict 5 seconds ahead
        let time_step = 0.2; // Every 0.2 seconds

        let mut current_pos = vehicle.position;
        let mut current_velocity = vehicle.current_velocity;

        for step in 0..((prediction_time / time_step) as usize) {
            trajectory.push(current_pos);

            // Simple linear prediction (could be enhanced with turning prediction)
            let direction_vector = match vehicle.get_current_movement_direction() {
                Direction::North => Vec2::new(0.0, -1.0),
                Direction::South => Vec2::new(0.0, 1.0),
                Direction::East => Vec2::new(1.0, 0.0),
                Direction::West => Vec2::new(-1.0, 0.0),
            };

            current_pos = current_pos + direction_vector * (current_velocity * time_step);

            // Adjust velocity prediction based on intersection proximity
            if intersection.is_point_in_approach_zone(current_pos.x, current_pos.y) {
                current_velocity *= 0.8; // Predict slowdown near intersection
            }
        }

        trajectory
    }

    // ENHANCED: Smart collision prevention with smooth velocity transitions
    fn apply_smooth_collision_prevention(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection) {
        // Group vehicles by potential conflict zones
        let conflict_groups = self.group_vehicles_by_smooth_conflicts(vehicles, intersection);

        for group in conflict_groups.iter() {
            if group.len() <= 1 {
                continue;
            }

            // Sort by priority with enhanced criteria
            let mut sorted_group = group.clone();
            sorted_group.sort_by(|&a, &b| {
                let vehicle_a = &vehicles[a];
                let vehicle_b = &vehicles[b];

                // Enhanced priority system
                self.compare_vehicle_priority(vehicle_a, vehicle_b, intersection)
            });

            // Apply smooth conflict resolution
            self.resolve_conflicts_smoothly(vehicles, intersection, &sorted_group);
        }
    }

    fn group_vehicles_by_smooth_conflicts(&self, vehicles: &VecDeque<Vehicle>, intersection: &Intersection) -> Vec<Vec<usize>> {
        let mut groups = Vec::new();
        let mut processed = vec![false; vehicles.len()];

        for i in 0..vehicles.len() {
            if processed[i] || vehicles[i].state == VehicleState::Completed {
                continue;
            }

            let mut group = vec![i];
            processed[i] = true;

            // Find vehicles in same conflict zones
            for j in (i + 1)..vehicles.len() {
                if processed[j] || vehicles[j].state == VehicleState::Completed {
                    continue;
                }

                if self.vehicles_share_conflict_zone(&vehicles[i], &vehicles[j], intersection) {
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

    fn vehicles_share_conflict_zone(&self, vehicle1: &Vehicle, vehicle2: &Vehicle, intersection: &Intersection) -> bool {
        // Check if vehicles will pass through same conflict zones
        for zone in &self.turning_conflict_zones {
            let vehicle1_affected = zone.affected_routes.contains(&(vehicle1.direction, vehicle1.route));
            let vehicle2_affected = zone.affected_routes.contains(&(vehicle2.direction, vehicle2.route));

            if vehicle1_affected && vehicle2_affected {
                // Check if they'll arrive at similar times
                let dist1 = self.calculate_distance_to_point(vehicle1, zone.center);
                let dist2 = self.calculate_distance_to_point(vehicle2, zone.center);

                let time1 = if vehicle1.current_velocity > 0.0 { dist1 / vehicle1.current_velocity } else { f32::INFINITY };
                let time2 = if vehicle2.current_velocity > 0.0 { dist2 / vehicle2.current_velocity } else { f32::INFINITY };

                if (time1 - time2).abs() < 4.0 { // Within 4 seconds
                    return true;
                }
            }
        }

        // Check traditional collision detection as fallback
        vehicle1.could_collide_with(vehicle2, intersection)
    }

    fn calculate_distance_to_point(&self, vehicle: &Vehicle, point: Vec2) -> f32 {
        let dx = vehicle.position.x - point.x;
        let dy = vehicle.position.y - point.y;
        (dx * dx + dy * dy).sqrt()
    }

    fn compare_vehicle_priority(&self, vehicle_a: &Vehicle, vehicle_b: &Vehicle, intersection: &Intersection) -> std::cmp::Ordering {
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

        // 3. Right turns have priority over left turns (easier/faster)
        if vehicle_a.route == Route::Right && vehicle_b.route == Route::Left {
            return std::cmp::Ordering::Less;
        }
        if vehicle_a.route == Route::Left && vehicle_b.route == Route::Right {
            return std::cmp::Ordering::Greater;
        }

        // 4. Closer to intersection has priority
        let dist_a = self.distance_to_intersection_center(vehicle_a);
        let dist_b = self.distance_to_intersection_center(vehicle_b);
        dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
    }

    fn resolve_conflicts_smoothly(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection, group: &[usize]) {
        if group.is_empty() {
            return;
        }

        let priority_vehicle_idx = group[0];

        // Check if priority vehicle can proceed safely
        let can_proceed = self.is_safe_to_proceed_smooth(vehicles, intersection, priority_vehicle_idx, group);

        // Apply smooth decisions
        if can_proceed {
            self.allow_vehicle_to_proceed_smooth(vehicles, priority_vehicle_idx);
        } else {
            self.apply_conservative_slowdown_smooth(vehicles, priority_vehicle_idx);
        }

        // Other vehicles must yield with smooth transitions
        for &vehicle_idx in &group[1..] {
            self.apply_smooth_yielding_behavior(vehicles, intersection, vehicle_idx, priority_vehicle_idx);
        }
    }

    fn is_safe_to_proceed_smooth(&self, vehicles: &VecDeque<Vehicle>, intersection: &Intersection, vehicle_idx: usize, group: &[usize]) -> bool {
        let vehicle = &vehicles[vehicle_idx];

        // Enhanced safety checks with trajectory prediction
        for &other_idx in group {
            if other_idx == vehicle_idx {
                continue;
            }

            let other = &vehicles[other_idx];
            let distance = self.calculate_distance_f32(vehicle, other);

            // Check current distance
            if distance < self.safe_following_distance {
                return false;
            }

            // Check predicted trajectory overlap
            if let (Some(traj1), Some(traj2)) = (self.vehicle_trajectories.get(&vehicle.id), self.vehicle_trajectories.get(&other.id)) {
                if self.trajectories_will_conflict(traj1, traj2) {
                    return false;
                }
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

    fn trajectories_will_conflict(&self, traj1: &[Vec2], traj2: &[Vec2]) -> bool {
        for (i, point1) in traj1.iter().enumerate() {
            if let Some(point2) = traj2.get(i) {
                let distance = self.calculate_distance_between_points(*point1, *point2);
                if distance < 30.0 { // Minimum safe distance
                    return true;
                }
            }
        }
        false
    }

    fn allow_vehicle_to_proceed_smooth(&mut self, vehicles: &mut VecDeque<Vehicle>, vehicle_idx: usize) {
        let vehicle = &mut vehicles[vehicle_idx];

        // Smooth velocity transitions based on state
        match vehicle.state {
            VehicleState::Approaching => {
                self.velocity_smoother.set_target_velocity(vehicle.id, VelocityLevel::Medium);
            }
            VehicleState::Entering | VehicleState::Turning => {
                self.velocity_smoother.set_target_velocity(vehicle.id, VelocityLevel::Slow);
            }
            VehicleState::Exiting => {
                self.velocity_smoother.set_target_velocity(vehicle.id, VelocityLevel::Medium);
            }
            VehicleState::Completed => {
                self.velocity_smoother.set_target_velocity(vehicle.id, VelocityLevel::Fast);
            }
        }
    }

    fn apply_conservative_slowdown_smooth(&mut self, vehicles: &mut VecDeque<Vehicle>, vehicle_idx: usize) {
        let vehicle = &mut vehicles[vehicle_idx];
        self.velocity_smoother.set_target_velocity(vehicle.id, VelocityLevel::Slow);

        // Additional smooth emergency braking if very close to conflict
        if vehicle.current_velocity > Vehicle::SLOW_VELOCITY {
            self.velocity_smoother.apply_emergency_braking(vehicle.id, 0.9); // 90% of current velocity
        }
    }

    fn apply_smooth_yielding_behavior(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection, vehicle_idx: usize, priority_vehicle_idx: usize) {
        let distance_to_priority = {
            let vehicle = &vehicles[vehicle_idx];
            let priority_vehicle = &vehicles[priority_vehicle_idx];
            self.calculate_distance_f32(vehicle, priority_vehicle)
        };

        // Smooth yielding based on distance
        if distance_to_priority < self.critical_collision_distance {
            // Emergency smooth stop
            self.velocity_smoother.set_target_velocity(vehicles[vehicle_idx].id, VelocityLevel::Slow);
            self.velocity_smoother.apply_emergency_braking(vehicles[vehicle_idx].id, 0.6);

            let vehicle_id = vehicles[vehicle_idx].id;
            let priority_id = vehicles[priority_vehicle_idx].id;
            self.record_close_call(vehicle_id, priority_id);
        } else if distance_to_priority < self.safe_following_distance {
            // Gradual smooth slowdown
            self.velocity_smoother.set_target_velocity(vehicles[vehicle_idx].id, VelocityLevel::Slow);
        } else {
            // Cautious approach with smooth transitions
            match vehicles[vehicle_idx].state {
                VehicleState::Approaching => {
                    self.velocity_smoother.set_target_velocity(vehicles[vehicle_idx].id, VelocityLevel::Slow);
                }
                _ => {
                    self.velocity_smoother.set_target_velocity(vehicles[vehicle_idx].id, VelocityLevel::Medium);
                }
            }
        }
    }

    // ENHANCED: Smart intersection access management
    fn manage_smart_intersection_access(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection) {
        let vehicles_in_intersection: Vec<usize> = vehicles.iter()
            .enumerate()
            .filter(|(_, v)| v.is_in_intersection(intersection))
            .map(|(i, _)| i)
            .collect();

        if vehicles_in_intersection.len() >= self.max_simultaneous_vehicles {
            for (i, vehicle) in vehicles.iter_mut().enumerate() {
                if vehicle.is_approaching_intersection(intersection) && !vehicles_in_intersection.contains(&i) {
                    self.velocity_smoother.set_target_velocity(vehicle.id, VelocityLevel::Slow);
                }
            }
            return;
        }

        // Enhanced access decisions with smooth transitions
        let mut access_decisions = Vec::new();

        for (i, vehicle) in vehicles.iter().enumerate() {
            if vehicle.is_approaching_intersection(intersection) && !vehicle.has_intersection_reservation() {
                let can_grant = self.can_grant_smooth_access(i, vehicles, intersection);
                access_decisions.push((i, vehicle.id, can_grant));
            }
        }

        for (vehicle_idx, vehicle_id, should_grant) in access_decisions {
            if should_grant {
                vehicles[vehicle_idx].set_intersection_reservation(true);
                self.intersection_reservations.insert(vehicle_id, self.current_time + 15.0);
                println!("🎫 Granted smooth intersection access to vehicle {}", vehicle_id);
            }
        }
    }

    fn can_grant_smooth_access(&self, vehicle_idx: usize, vehicles: &VecDeque<Vehicle>, intersection: &Intersection) -> bool {
        let vehicle = &vehicles[vehicle_idx];

        // Enhanced conflict checking with trajectory prediction
        for (i, other) in vehicles.iter().enumerate() {
            if i == vehicle_idx {
                continue;
            }

            if other.is_in_intersection(intersection) || other.has_intersection_reservation() {
                if vehicle.could_collide_with(other, intersection) {
                    let time_diff = (vehicle.time_to_intersection(intersection) - other.time_to_intersection(intersection)).abs();
                    if time_diff < 4.0 { // Increased safety margin for smooth animations
                        return false;
                    }
                }

                // Check trajectory prediction
                if let (Some(traj1), Some(traj2)) = (self.vehicle_trajectories.get(&vehicle.id), self.vehicle_trajectories.get(&other.id)) {
                    if self.trajectories_will_conflict(traj1, traj2) {
                        return false;
                    }
                }
            }
        }

        true
    }

    // Enhanced analysis and utility functions
    fn analyze_traffic_patterns(&mut self, vehicles: &VecDeque<Vehicle>) {
        self.congestion_levels.clear();

        for vehicle in vehicles {
            if matches!(vehicle.state, VehicleState::Approaching | VehicleState::Entering) {
                let key = (vehicle.direction, vehicle.lane);
                *self.congestion_levels.entry(key).or_insert(0) += 1;
            }
        }

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

    fn check_safety_violations(&mut self, vehicles: &VecDeque<Vehicle>, intersection: &Intersection) {
        for (i, vehicle_a) in vehicles.iter().enumerate() {
            for (j, vehicle_b) in vehicles.iter().enumerate() {
                if i == j || vehicle_a.state == VehicleState::Completed || vehicle_b.state == VehicleState::Completed {
                    continue;
                }

                let distance = self.calculate_distance_f32(vehicle_a, vehicle_b);

                if distance < Vehicle::SAFE_DISTANCE {
                    self.record_safe_distance_violation(vehicle_a.id, vehicle_b.id);
                }

                if distance < 30.0 && vehicle_a.could_collide_with(vehicle_b, intersection) {
                    self.record_close_call(vehicle_a.id, vehicle_b.id);
                }
            }
        }
    }

    fn record_close_call(&mut self, vehicle1_id: u32, vehicle2_id: u32) {
        let violations = self.safe_distance_violations
            .entry(vehicle1_id)
            .or_insert(Vec::new());

        let already_recorded = violations.iter()
            .any(|(id, time)| *id == vehicle2_id && self.current_time - time < 5.0);

        if !already_recorded {
            self.close_calls += 1;
            violations.push((vehicle2_id, self.current_time));
            println!("⚠️ CLOSE CALL #{}: Vehicles {} and {} nearly collided during smooth operation!",
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
        for violations in self.safe_distance_violations.values_mut() {
            violations.retain(|(_, time)| self.current_time - time < 10.0);
        }

        self.intersection_reservations.retain(|_, &mut expiration_time| {
            expiration_time > self.current_time
        });

        self.velocity_smoother.cleanup_old_data();
    }

    // Utility functions with floating-point precision
    fn distance_to_intersection_center(&self, vehicle: &Vehicle) -> f64 {
        let center_x = crate::WINDOW_WIDTH as f64 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f64 / 2.0;

        let dx = vehicle.position.x as f64 - center_x;
        let dy = vehicle.position.y as f64 - center_y;

        (dx * dx + dy * dy).sqrt()
    }

    fn calculate_distance_f32(&self, vehicle1: &Vehicle, vehicle2: &Vehicle) -> f32 {
        let dx = vehicle1.position.x - vehicle2.position.x;
        let dy = vehicle1.position.y - vehicle2.position.y;
        (dx * dx + dy * dy).sqrt()
    }

    fn calculate_distance_between_points(&self, p1: Vec2, p2: Vec2) -> f32 {
        let dx = p1.x - p2.x;
        let dy = p1.y - p2.y;
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

// NEW: Velocity smoother for natural transitions
impl VelocitySmoother {
    fn new() -> Self {
        VelocitySmoother {
            velocity_history: HashMap::new(),
            smoothing_factor: 0.1,
            max_history_length: 10,
        }
    }

    fn smooth_all_velocities(&mut self, vehicles: &mut VecDeque<Vehicle>) {
        for vehicle in vehicles {
            self.smooth_vehicle_velocity(vehicle);
        }
    }

    fn smooth_vehicle_velocity(&mut self, vehicle: &mut Vehicle) {
        let history = self.velocity_history.entry(vehicle.id).or_insert(Vec::new());

        // Add current velocity to history
        history.push(vehicle.current_velocity);
        if history.len() > self.max_history_length {
            history.remove(0);
        }

        // Apply smoothing if we have enough history
        if history.len() > 3 {
            let average = history.iter().sum::<f32>() / history.len() as f32;
            let smoothed = vehicle.current_velocity * (1.0 - self.smoothing_factor) + average * self.smoothing_factor;
            vehicle.current_velocity = smoothed;
        }
    }

    fn set_target_velocity(&mut self, vehicle_id: u32, level: VelocityLevel) {
        // This would be used in conjunction with the vehicle's own velocity management
        // Implementation depends on how you want to integrate smoothing with target velocity
    }

    fn apply_emergency_braking(&mut self, vehicle_id: u32, factor: f32) {
        // Apply immediate velocity reduction for emergency situations
        if let Some(history) = self.velocity_history.get_mut(&vehicle_id) {
            if let Some(last_velocity) = history.last_mut() {
                *last_velocity *= factor;
            }
        }
    }

    fn cleanup_old_data(&mut self) {
        // Remove velocity history for vehicles that no longer exist
        // This would need vehicle lifetime tracking
    }
}

// NEW: Intersection timing management
impl IntersectionTiming {
    fn new() -> Self {
        IntersectionTiming {
            average_crossing_time: 3.0,
            crossing_times_by_route: HashMap::new(),
            last_update: 0.0,
        }
    }

    fn update(&mut self, vehicles: &VecDeque<Vehicle>, current_time: f64) {
        if current_time - self.last_update < 1.0 {
            return;
        }

        // Update crossing time statistics
        for vehicle in vehicles {
            if vehicle.state == VehicleState::Completed {
                let crossing_time = vehicle.time_in_intersection as f32 / 1000.0;
                if crossing_time > 0.0 {
                    let route_time = self.crossing_times_by_route.entry(vehicle.route).or_insert(crossing_time);
                    *route_time = (*route_time * 0.9) + (crossing_time * 0.1); // Exponential moving average
                }
            }
        }

        self.last_update = current_time;
    }

    fn get_estimated_crossing_time(&self, route: Route) -> f32 {
        *self.crossing_times_by_route.get(&route).unwrap_or(&self.average_crossing_time)
    }
}