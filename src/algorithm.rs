use crate::intersection::Intersection;
use crate::vehicle::{Vehicle, VehicleState, VelocityLevel};
use std::collections::{HashMap, VecDeque};

pub struct SmartIntersection {
    // Keep track of close calls
    pub close_calls: u32,
    // Track safe distance violations
    safe_distance_violations: HashMap<u32, Vec<u32>>, // Vehicle ID to list of vehicles it had close calls with
}

impl SmartIntersection {
    pub fn new() -> Self {
        SmartIntersection {
            close_calls: 0,
            safe_distance_violations: HashMap::new(),
        }
    }

    // Process all vehicles and manage their velocities to avoid collisions
    pub fn process_vehicles(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection, delta_time: u32) {
        // First, update all vehicle positions
        for vehicle in vehicles.iter_mut() {
            vehicle.update(delta_time, intersection);
        }

        // Then check for potential collisions and adjust velocities
        self.manage_velocities(vehicles, intersection);

        // Check for safe distance violations
        self.check_safe_distances(vehicles);
    }

    // Adjust vehicle velocities to avoid collisions
    fn manage_velocities(&mut self, vehicles: &mut VecDeque<Vehicle>, intersection: &Intersection) {
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