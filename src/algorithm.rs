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

        // Check if any direction is heavily congested (threshold: 12 vehicles across all lanes)
        self.adaptive_mode = direction_congestion.iter().any(|&count| count > 12);

        // Update direction priority based on throughput deficit
        for i in 0..4 {
            // Increase priority if congested
            if direction_congestion[i] > 8 {
                self.direction_priority[i] += 2;
            } else if direction_congestion[i] > 4 {
                self.direction_priority[i] += 1;
            }

            // Cap priority
            if self.direction_priority[i] > 10 {
                self.direction_priority[i] = 10;
            }
        }

        // Debug output (optional, can be removed for performance)
        if direction_congestion.iter().any(|&count| count > 5) {
            println!("Direction congestion: {:?}", direction_congestion);
            println!("Adaptive mode: {}", self.adaptive_mode);
        }
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

                    // Check if vehicles could collide and are in conflicting lanes
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
        // Group vehicles by direction and lane
        let mut direction_lane_groups: HashMap<(Direction, usize), Vec<usize>> = HashMap::new();

        for (i, vehicle) in vehicles.iter().enumerate() {
            if vehicle.state != VehicleState::Completed {
                direction_lane_groups.entry((vehicle.direction, vehicle.lane)).or_default().push(i);
            }
        }

        // Find the direction with highest priority
        let mut max_priority = 0;
        let mut priority_direction = None;

        for (i, &priority) in self.direction_priority.iter().enumerate() {
            if priority > max_priority {
                max_priority = priority;
                priority_direction = Some(match i {
                    0 => Direction::North,
                    1 => Direction::South,
                    2 => Direction::East,
                    _ => Direction::West,
                });
            }
        }

        // Update current flows if needed
        if self.current_flows.is_empty() && priority_direction.is_some() {
            // Add all lanes from the priority direction to the flow
            let dir = priority_direction.unwrap();
            for lane in 0..6 {
                self.current_flows.push((dir, lane));
            }

            // Decrease the priority of this direction
            let dir_index = match dir {
                Direction::North => 0,
                Direction::South => 1,
                Direction::East => 2,
                Direction::West => 3,
            };
            self.direction_priority[dir_index] = self.direction_priority[dir_index].saturating_sub(3);
        }

        // Sort each direction-lane group by distance to intersection
        for (_, indices) in direction_lane_groups.iter_mut() {
            indices.sort_by(|&a, &b| {
                let time_a = vehicles[a].time_to_intersection(intersection);
                let time_b = vehicles[b].time_to_intersection(intersection);
                time_a.partial_cmp(&time_b).unwrap()
            });
        }

        // Process vehicles in prioritized flows
        for &(direction, lane) in &self.current_flows {
            if let Some(indices) = direction_lane_groups.get(&(direction, lane)) {
                // Process vehicles in this flow lane
                for (position, &idx) in indices.iter().enumerate() {
                    if position < 2 {
                        // First 2 vehicles get fast
                        vehicles[idx].set_target_velocity(VelocityLevel::Fast);
                    } else if position < 4 {
                        // Next 2 get medium
                        vehicles[idx].set_target_velocity(VelocityLevel::Medium);
                    } else {
                        // Rest get slow
                        vehicles[idx].set_target_velocity(VelocityLevel::Slow);
                    }
                }
            }
        }

        // Process crossing traffic - slow down if they could conflict with prioritized flows
        for ((dir, lane), indices) in &direction_lane_groups {
            // Skip if this is a prioritized flow
            if self.current_flows.contains(&(*dir, *lane)) {
                continue;
            }

            // Check if this flow conflicts with any prioritized flow
            let conflicts = self.current_flows.iter().any(|&(pdir, _)| {
                Self::could_conflict(*dir, pdir)
            });

            if conflicts {
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
                    }
                }
            }
        }

        // Handle vehicles in the intersection - collect conflicts first, then adjust speeds
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
    }

    // Determine if vehicle_a should yield to vehicle_b
    fn should_yield(&self, vehicle_a: &Vehicle, vehicle_b: &Vehicle, intersection: &Intersection) -> bool {
        // Priority rules:
        // 1. Vehicles already in the intersection have priority
        // 2. Vehicles coming from the right have priority (right-hand rule)
        // 3. Vehicles in prioritized flows have priority
        // 4. Vehicles that are closer to the intersection have priority

        // Rule 1: Vehicles in the intersection have priority
        if vehicle_b.state == VehicleState::Turning && vehicle_a.state == VehicleState::Approaching {
            return true;
        }
        if vehicle_a.state == VehicleState::Turning && vehicle_b.state == VehicleState::Approaching {
            return false;
        }

        // Rule 2: Right-hand rule (vehicle coming from the right has priority)
        if vehicle_a.state == VehicleState::Approaching && vehicle_b.state == VehicleState::Approaching {
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

        // Rule 3: Prioritized flows have priority
        let a_is_prioritized = self.current_flows.contains(&(vehicle_a.direction, vehicle_a.lane));
        let b_is_prioritized = self.current_flows.contains(&(vehicle_b.direction, vehicle_b.lane));

        if b_is_prioritized && !a_is_prioritized {
            return true;
        }
        if a_is_prioritized && !b_is_prioritized {
            return false;
        }

        // Rule 4: Vehicle closer to the intersection has priority
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
            (Direction::North, Direction::South) | (Direction::South, Direction::North) => true,
            (Direction::East, Direction::West) | (Direction::West, Direction::East) => true,
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
                if vehicle_a.direction == vehicle_b.direction && vehicle_a.lane == vehicle_b.lane {
                    // Calculate distance between vehicles
                    let distance = match vehicle_a.direction {
                        Direction::North => {
                            if vehicle_a.position.y < vehicle_b.position.y {
                                (vehicle_b.position.y - vehicle_a.position.y) as f64
                            } else {
                                continue; // vehicle_a is behind vehicle_b
                            }
                        }
                        Direction::South => {
                            if vehicle_a.position.y > vehicle_b.position.y {
                                (vehicle_a.position.y - vehicle_b.position.y) as f64
                            } else {
                                continue; // vehicle_a is behind vehicle_b
                            }
                        }
                        Direction::East => {
                            if vehicle_a.position.x > vehicle_b.position.x {
                                (vehicle_a.position.x - vehicle_b.position.x) as f64
                            } else {
                                continue; // vehicle_a is behind vehicle_b
                            }
                        }
                        Direction::West => {
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