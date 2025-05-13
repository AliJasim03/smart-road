use crate::intersection::Intersection;
use crate::vehicle::{Vehicle, VehicleState, VelocityLevel, Direction};
use std::collections::{HashMap, VecDeque};

pub struct SmartIntersection {
    // Keep track of close calls
    pub close_calls: u32,
    // Track safe distance violations
    safe_distance_violations: HashMap<u32, Vec<u32>>, // Vehicle ID to list of vehicles it had close calls with
    // Track congestion by direction
    congestion_levels: HashMap<Direction, u32>, // Direction to number of vehicles
    // Enable adaptive mode for high traffic
    adaptive_mode: bool,
}

impl SmartIntersection {
    pub fn new() -> Self {
        SmartIntersection {
            close_calls: 0,
            safe_distance_violations: HashMap::new(),
            congestion_levels: HashMap::new(),
            adaptive_mode: false,
        }
    }

    // Process all vehicles and manage their velocities to avoid collisions
    pub fn process_vehicles(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection, delta_time: u32) {
        // First, update all vehicle positions
        for vehicle in vehicles.iter_mut() {
            vehicle.update(delta_time, intersection);
        }

        // Analyze congestion levels
        self.analyze_congestion(vehicles);

        // Then check for potential collisions and adjust velocities
        self.manage_velocities(vehicles, intersection);

        // Check for safe distance violations
        self.check_safe_distances(vehicles);
    }

    // Analyze congestion levels for each direction
    fn analyze_congestion(&mut self, vehicles: &VecDeque<Vehicle>) {
        // Reset congestion counts
        self.congestion_levels.clear();

        // Count vehicles per direction
        for vehicle in vehicles {
            if vehicle.state == VehicleState::Approaching || vehicle.state == VehicleState::Entering {
                *self.congestion_levels.entry(vehicle.direction).or_insert(0) += 1;
            }
        }

        // Check if any direction is congested (threshold: 5 vehicles)
        self.adaptive_mode = self.congestion_levels.values().any(|&count| count > 5);

        // Debug print of congestion levels
        for (dir, count) in &self.congestion_levels {
            println!("Congestion {:?}: {} vehicles", dir, count);
        }
        println!("Adaptive mode: {}", self.adaptive_mode);
    }

    // Adjust vehicle velocities to avoid collisions
    fn manage_velocities(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection) {
        // If in adaptive mode, use batch processing for efficiency
        if self.adaptive_mode {
            self.batch_process_vehicles(vehicles, intersection);
        } else {
            // Original logic for normal traffic conditions
            // First, identify vehicles that are approaching or in the intersection
            let mut active_vehicles: Vec<usize> = vehicles
                .iter()
                .enumerate()
                .filter(|(_, v)| v.state != VehicleState::Completed)
                .map(|(i, _)| i)
                .collect();

            // Sort by time to intersection (vehicles closer to intersection get priority)
            active_vehicles.sort_by(|&a, &b| {
                let time_a = vehicles[a].time_to_intersection(intersection);
                let time_b = vehicles[b].time_to_intersection(intersection);
                time_a.partial_cmp(&time_b).unwrap()
            });

            // Process each vehicle in order of priority
            for i in 0..active_vehicles.len() {
                let idx_a = active_vehicles[i];
                let vehicle_a = &vehicles[idx_a];

                // Skip vehicles that have completed the intersection
                if vehicle_a.state == VehicleState::Completed {
                    continue;
                }

                // Check for potential collisions with other vehicles
                let mut should_slow_down = false;

                for j in 0..active_vehicles.len() {
                    if i == j {
                        continue; // Skip self
                    }

                    let idx_b = active_vehicles[j];
                    let vehicle_b = &vehicles[idx_b];

                    // Skip vehicles that have completed the intersection
                    if vehicle_b.state == VehicleState::Completed {
                        continue;
                    }

                    // Check if vehicles could collide
                    if vehicle_a.could_collide_with(vehicle_b, intersection) {
                        // Check if vehicle_a should yield to vehicle_b
                        if self.should_yield(vehicle_a, vehicle_b, intersection) {
                            should_slow_down = true;
                            break;
                        }
                    }
                }

                // Adjust velocity based on collision risk
                if should_slow_down {
                    // Slow down the vehicle
                    vehicles[idx_a].set_target_velocity(VelocityLevel::Slow);
                } else if vehicle_a.state == VehicleState::Approaching ||
                    vehicle_a.state == VehicleState::Exiting {
                    // Resume normal speed when approaching or exiting the intersection
                    vehicles[idx_a].set_target_velocity(VelocityLevel::Medium);
                }
            }
        }
    }

    // Batch process vehicles in high congestion scenario
    fn batch_process_vehicles(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection) {
        // Group vehicles by direction
        let mut direction_groups: HashMap<Direction, Vec<usize>> = HashMap::new();

        for (i, vehicle) in vehicles.iter().enumerate() {
            if vehicle.state != VehicleState::Completed {
                direction_groups.entry(vehicle.direction).or_default().push(i);
            }
        }

        // Find most congested direction
        let mut max_congestion = 0;
        let mut priority_direction = None;

        for (dir, count) in &self.congestion_levels {
            if *count > max_congestion {
                max_congestion = *count;
                priority_direction = Some(*dir);
            }
        }

        // Sort each direction group by distance to intersection
        for (_, indices) in direction_groups.iter_mut() {
            indices.sort_by(|&a, &b| {
                let time_a = vehicles[a].time_to_intersection(intersection);
                let time_b = vehicles[b].time_to_intersection(intersection);
                time_a.partial_cmp(&time_b).unwrap()
            });
        }

        // Give green wave to priority direction
        if let Some(priority_dir) = priority_direction {
            println!("Priority direction: {:?} with {} vehicles", priority_dir, max_congestion);

            if let Some(indices) = direction_groups.get(&priority_dir) {
                // Process vehicles in priority direction
                for (position, &idx) in indices.iter().enumerate() {
                    if position < 3 {
                        // First 3 vehicles get fast
                        vehicles[idx].set_target_velocity(VelocityLevel::Fast);
                        println!("Vehicle {} in priority direction set to FAST", vehicles[idx].id);
                    } else {
                        // Rest get medium
                        vehicles[idx].set_target_velocity(VelocityLevel::Medium);
                        println!("Vehicle {} in priority direction set to MEDIUM", vehicles[idx].id);
                    }
                }
            }

            // Process crossing traffic - slow down if they could conflict
            for (dir, indices) in &direction_groups {
                if *dir != priority_dir && Self::could_conflict(*dir, priority_dir) {
                    for &idx in indices {
                        // Skip vehicles already in the intersection
                        if vehicles[idx].state == VehicleState::Turning {
                            continue;
                        }

                        // Check if this vehicle is close to entering the intersection
                        let time_to_intersection = vehicles[idx].time_to_intersection(intersection);

                        if time_to_intersection < 2.0 {
                            // Vehicle is about to enter - slow down
                            vehicles[idx].set_target_velocity(VelocityLevel::Slow);
                            println!("Vehicle {} in crossing direction {:?} set to SLOW", vehicles[idx].id, dir);
                        }
                    }
                }
            }

            // Handle vehicles in the intersection - collect conflicts first, then adjust speeds
            // Create a data structure to track vehicles that need to slow down
            let mut vehicles_to_slow_down = Vec::new();

            // First pass: identify vehicles that need to slow down
            for i in 0..vehicles.len() {
                let vehicle = &vehicles[i];

                if vehicle.state == VehicleState::Turning {
                    // Check for potential conflicts
                    for j in 0..vehicles.len() {
                        if i != j {
                            let other = &vehicles[j];

                            if other.state != VehicleState::Completed &&
                                vehicle.could_collide_with(other, intersection) {
                                // Add to the list that needs to slow down
                                vehicles_to_slow_down.push(i);
                                println!("Potential collision between vehicles {} and {} - will slow",
                                         vehicle.id, other.id);
                                break;
                            }
                        }
                    }
                }
            }

            // Second pass: apply medium speed to all turning vehicles
            for i in 0..vehicles.len() {
                if vehicles[i].state == VehicleState::Turning {
                    vehicles[i].set_target_velocity(VelocityLevel::Medium);
                }
            }

            // Third pass: slow down vehicles with potential conflicts
            for idx in vehicles_to_slow_down {
                vehicles[idx].set_target_velocity(VelocityLevel::Slow);
            }

        } else {
            // Fallback to standard processing if no priority direction
            println!("No priority direction detected");
        }
    }
    // Determine if vehicle_a should yield to vehicle_b
    fn should_yield(&self, vehicle_a: &Vehicle, vehicle_b: &Vehicle, intersection: &Intersection) -> bool {
        // Priority rules:
        // 1. Vehicles already in the intersection have priority
        // 2. Vehicles coming from the right have priority (right-hand rule)
        // 3. Vehicles that are closer to the intersection have priority

        // Rule 1: Vehicles in the intersection have priority
        if vehicle_b.state == VehicleState::Turning && vehicle_a.state == VehicleState::Approaching {
            return true;
        }
        if vehicle_a.state == VehicleState::Turning && vehicle_b.state == VehicleState::Approaching {
            return false;
        }

        // Rule 2: Right-hand rule (vehicle coming from the right has priority)
        if vehicle_a.state == VehicleState::Approaching && vehicle_b.state == VehicleState::Approaching {
            use crate::vehicle::Direction;

            match (vehicle_a.direction, vehicle_b.direction) {
                (Direction::North, Direction::East) => return true,
                (Direction::East, Direction::South) => return true,
                (Direction::South, Direction::West) => return true,
                (Direction::West, Direction::North) => return true,
                (Direction::East, Direction::North) => return false,
                (Direction::South, Direction::East) => return false,
                (Direction::West, Direction::South) => return false,
                (Direction::North, Direction::West) => return false,
                _ => {}
            }
        }

        // Rule 3: Vehicle closer to the intersection has priority
        let time_a = vehicle_a.time_to_intersection(intersection);
        let time_b = vehicle_b.time_to_intersection(intersection);

        // If vehicle_b will reach the intersection sooner, vehicle_a should yield
        time_a > time_b
    }

    // Helper to determine if directions could have conflicting paths
    fn could_conflict(dir1: Direction, dir2: Direction) -> bool {
        match (dir1, dir2) {
            (Direction::North, Direction::East) | (Direction::East, Direction::North) => true,
            (Direction::North, Direction::West) | (Direction::West, Direction::North) => true,
            (Direction::South, Direction::East) | (Direction::East, Direction::South) => true,
            (Direction::South, Direction::West) | (Direction::West, Direction::South) => true,
            _ => false,
        }
    }

    // Check for safe distance violations between vehicles
    fn check_safe_distances(&mut self, vehicles: &VecDeque<Vehicle>) {
        for (i, vehicle_a) in vehicles.iter().enumerate() {
            for (j, vehicle_b) in vehicles.iter().enumerate() {
                if i == j {
                    continue; // Skip self
                }

                // Skip completed vehicles
                if vehicle_a.state == VehicleState::Completed || vehicle_b.state == VehicleState::Completed {
                    continue;
                }

                // Check if vehicles are in the same lane and direction
                if vehicle_a.direction == vehicle_b.direction {
                    // Calculate distance between vehicles
                    let distance = match vehicle_a.direction {
                        crate::vehicle::Direction::North => {
                            if vehicle_a.position.y < vehicle_b.position.y {
                                (vehicle_b.position.y - vehicle_a.position.y) as f64
                            } else {
                                continue; // vehicle_a is behind vehicle_b
                            }
                        }
                        crate::vehicle::Direction::South => {
                            if vehicle_a.position.y > vehicle_b.position.y {
                                (vehicle_a.position.y - vehicle_b.position.y) as f64
                            } else {
                                continue; // vehicle_a is behind vehicle_b
                            }
                        }
                        crate::vehicle::Direction::East => {
                            if vehicle_a.position.x > vehicle_b.position.x {
                                (vehicle_a.position.x - vehicle_b.position.x) as f64
                            } else {
                                continue; // vehicle_a is behind vehicle_b
                            }
                        }
                        crate::vehicle::Direction::West => {
                            if vehicle_a.position.x < vehicle_b.position.x {
                                (vehicle_b.position.x - vehicle_a.position.x) as f64
                            } else {
                                continue; // vehicle_a is behind vehicle_b
                            }
                        }
                    };

                    // Check if distance is less than safe distance
                    if distance < Vehicle::SAFE_DISTANCE {
                        // Record safe distance violation
                        let violations = self.safe_distance_violations
                            .entry(vehicle_a.id)
                            .or_insert(Vec::new());

                        // Only count as a close call if this specific pair hasn't been recorded yet
                        if !violations.contains(&vehicle_b.id) {
                            self.close_calls += 1;
                            violations.push(vehicle_b.id);
                        }
                    }
                }
            }
        }
    }
}