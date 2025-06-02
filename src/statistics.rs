// src/statistics.rs - ENHANCED VERSION WITH AUDIT COMPLIANCE
use std::collections::VecDeque;
use std::time::Instant;
use crate::vehicle::{Vehicle, VehicleState, Direction, Route};

pub struct Statistics {
    pub total_vehicles_spawned: u32,
    pub vehicles_completed: u32,
    pub max_velocity: f64,
    pub min_velocity: f64,
    pub max_congestion: usize,
    pub close_calls: u32,

    // Enhanced statistics for audit compliance
    pub max_time_in_intersection: f64,
    pub min_time_in_intersection: f64,
    pub total_intersection_time: f64,
    pub vehicles_with_intersection_time: u32,

    // Traffic flow statistics
    pub vehicles_by_direction: [u32; 4], // North, South, East, West
    pub vehicles_by_route: [u32; 3],     // Left, Straight, Right

    // Performance tracking
    simulation_start: Instant,
    velocity_samples: Vec<f64>,
    congestion_samples: Vec<usize>,

    // Detailed timing statistics
    intersection_times: Vec<f64>,
    completion_times: Vec<f64>,
}

impl Statistics {
    pub fn new() -> Self {
        Statistics {
            total_vehicles_spawned: 0,
            vehicles_completed: 0,
            max_velocity: 0.0,
            min_velocity: f64::INFINITY,
            max_congestion: 0,
            close_calls: 0,

            max_time_in_intersection: 0.0,
            min_time_in_intersection: f64::INFINITY,
            total_intersection_time: 0.0,
            vehicles_with_intersection_time: 0,

            vehicles_by_direction: [0; 4],
            vehicles_by_route: [0; 3],

            simulation_start: Instant::now(),
            velocity_samples: Vec::new(),
            congestion_samples: Vec::new(),
            intersection_times: Vec::new(),
            completion_times: Vec::new(),
        }
    }

    pub fn update(&mut self, vehicles: &VecDeque<Vehicle>) {
        // Track maximum congestion
        if vehicles.len() > self.max_congestion {
            self.max_congestion = vehicles.len();
        }

        // Sample congestion for statistics
        self.congestion_samples.push(vehicles.len());

        // Update velocity statistics
        for vehicle in vehicles {
            // Track velocity ranges
            if vehicle.current_velocity > self.max_velocity {
                self.max_velocity = vehicle.current_velocity;
            }
            if vehicle.current_velocity > 0.0 && vehicle.current_velocity < self.min_velocity {
                self.min_velocity = vehicle.current_velocity;
            }

            // Sample velocities for analysis
            self.velocity_samples.push(vehicle.current_velocity);

            // Track intersection timing for vehicles currently in intersection
            if matches!(vehicle.state, VehicleState::Entering | VehicleState::Turning | VehicleState::Exiting) {
                let time_in_intersection = vehicle.time_in_intersection as f64 / 1000.0; // Convert to seconds

                if time_in_intersection > 0.0 {
                    if time_in_intersection > self.max_time_in_intersection {
                        self.max_time_in_intersection = time_in_intersection;
                    }
                    if time_in_intersection < self.min_time_in_intersection {
                        self.min_time_in_intersection = time_in_intersection;
                    }
                }
            }
        }
    }

    pub fn record_vehicle_spawn(&mut self, direction: Direction, route: Route) {
        self.total_vehicles_spawned += 1;

        // Track by direction
        let dir_index = match direction {
            Direction::North => 0,
            Direction::South => 1,
            Direction::East => 2,
            Direction::West => 3,
        };
        self.vehicles_by_direction[dir_index] += 1;

        // Track by route
        let route_index = match route {
            Route::Left => 0,
            Route::Straight => 1,
            Route::Right => 2,
        };
        self.vehicles_by_route[route_index] += 1;
    }

    pub fn record_vehicle_completion(&mut self, intersection_time_ms: u32) {
        self.vehicles_completed += 1;

        let intersection_time = intersection_time_ms as f64 / 1000.0;
        self.intersection_times.push(intersection_time);

        if intersection_time > 0.0 {
            self.total_intersection_time += intersection_time;
            self.vehicles_with_intersection_time += 1;

            if intersection_time > self.max_time_in_intersection {
                self.max_time_in_intersection = intersection_time;
            }
            if intersection_time < self.min_time_in_intersection {
                self.min_time_in_intersection = intersection_time;
            }
        }

        // Record completion time
        let total_time = self.simulation_start.elapsed().as_secs_f64();
        self.completion_times.push(total_time);
    }

    pub fn add_spawned_vehicle(&mut self) {
        self.total_vehicles_spawned += 1;
    }

    pub fn add_completed_vehicles(&mut self, count: usize) {
        self.vehicles_completed += count as u32;
    }

    pub fn add_close_call(&mut self) {
        self.close_calls += 1;
    }

    pub fn get_average_velocity(&self) -> f64 {
        if self.velocity_samples.is_empty() {
            return 0.0;
        }
        self.velocity_samples.iter().sum::<f64>() / self.velocity_samples.len() as f64
    }

    pub fn get_average_congestion(&self) -> f64 {
        if self.congestion_samples.is_empty() {
            return 0.0;
        }
        self.congestion_samples.iter().sum::<usize>() as f64 / self.congestion_samples.len() as f64
    }

    pub fn get_average_intersection_time(&self) -> f64 {
        if self.vehicles_with_intersection_time == 0 {
            return 0.0;
        }
        self.total_intersection_time / self.vehicles_with_intersection_time as f64
    }

    pub fn get_throughput_per_minute(&self) -> f64 {
        let elapsed_minutes = self.simulation_start.elapsed().as_secs_f64() / 60.0;
        if elapsed_minutes <= 0.0 {
            return 0.0;
        }
        self.vehicles_completed as f64 / elapsed_minutes
    }

    pub fn get_close_call_rate(&self) -> f64 {
        if self.vehicles_completed == 0 {
            return 0.0;
        }
        (self.close_calls as f64 / self.vehicles_completed as f64) * 100.0
    }

    pub fn get_completion_rate(&self) -> f64 {
        if self.total_vehicles_spawned == 0 {
            return 0.0;
        }
        (self.vehicles_completed as f64 / self.total_vehicles_spawned as f64) * 100.0
    }

    // Audit compliance check
    pub fn check_audit_requirements(&self) -> AuditResult {
        let mut result = AuditResult::new();

        // Check if all three velocity levels are being used
        result.has_three_velocities = self.velocity_samples.iter().any(|&v| v <= 30.0) &&
            self.velocity_samples.iter().any(|&v| v > 30.0 && v <= 60.0) &&
            self.velocity_samples.iter().any(|&v| v > 60.0);

        // Check congestion levels (should not exceed 8 for long periods)
        result.low_congestion = self.max_congestion <= 8;

        // Check collision avoidance (close call rate should be low)
        result.good_collision_avoidance = self.get_close_call_rate() < 15.0; // Less than 15% close call rate

        // Check if vehicles are completing their journeys
        result.vehicles_completing = self.get_completion_rate() > 80.0; // At least 80% completion rate

        // Check intersection efficiency (average time should be reasonable)
        result.efficient_intersection = self.get_average_intersection_time() < 5.0; // Less than 5 seconds average

        // Check if all directions are being used
        result.all_directions_used = self.vehicles_by_direction.iter().all(|&count| count > 0);

        // Check if all routes are being used
        result.all_routes_used = self.vehicles_by_route.iter().all(|&count| count > 0);

        result
    }

    pub fn display_comprehensive(&self) -> Result<(), String> {
        let elapsed = self.simulation_start.elapsed();

        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║                    COMPREHENSIVE STATISTICS                  ║");
        println!("╠══════════════════════════════════════════════════════════════╣");

        // Basic statistics
        println!("║ Simulation Duration: {:>8.1}s                           ║", elapsed.as_secs_f32());
        println!("║ Total Vehicles Spawned: {:>8}                           ║", self.total_vehicles_spawned);
        println!("║ Vehicles Completed: {:>12}                           ║", self.vehicles_completed);
        println!("║ Completion Rate: {:>15.1}%                          ║", self.get_completion_rate());
        println!("║ Throughput: {:>12.1} vehicles/min                    ║", self.get_throughput_per_minute());

        println!("╠══════════════════════════════════════════════════════════════╣");

        // Velocity statistics
        println!("║ Max Velocity: {:>18.1} px/s                        ║", self.max_velocity);
        println!("║ Min Velocity: {:>18.1} px/s                        ║",
                 if self.min_velocity == f64::INFINITY { 0.0 } else { self.min_velocity });
        println!("║ Average Velocity: {:>14.1} px/s                        ║", self.get_average_velocity());

        println!("╠══════════════════════════════════════════════════════════════╣");

        // Intersection timing
        println!("║ Max Time in Intersection: {:>8.1}s                      ║", self.max_time_in_intersection);
        println!("║ Min Time in Intersection: {:>8.1}s                      ║",
                 if self.min_time_in_intersection == f64::INFINITY { 0.0 } else { self.min_time_in_intersection });
        println!("║ Avg Time in Intersection: {:>8.1}s                      ║", self.get_average_intersection_time());

        println!("╠══════════════════════════════════════════════════════════════╣");

        // Safety and efficiency
        println!("║ Close Calls: {:>20}                           ║", self.close_calls);
        println!("║ Close Call Rate: {:>15.1}%                          ║", self.get_close_call_rate());
        println!("║ Max Congestion: {:>16} vehicles                   ║", self.max_congestion);
        println!("║ Average Congestion: {:>12.1} vehicles                   ║", self.get_average_congestion());

        println!("╠══════════════════════════════════════════════════════════════╣");

        // Distribution statistics
        println!("║ VEHICLES BY DIRECTION:                                      ║");
        println!("║   North: {:>3} | South: {:>3} | East: {:>3} | West: {:>3}          ║",
                 self.vehicles_by_direction[0], self.vehicles_by_direction[1],
                 self.vehicles_by_direction[2], self.vehicles_by_direction[3]);

        println!("║ VEHICLES BY ROUTE:                                          ║");
        println!("║   Left: {:>4} | Straight: {:>4} | Right: {:>4}                  ║",
                 self.vehicles_by_route[0], self.vehicles_by_route[1], self.vehicles_by_route[2]);

        println!("╚══════════════════════════════════════════════════════════════╝");

        // Audit compliance
        let audit_result = self.check_audit_requirements();
        self.display_audit_compliance(&audit_result);

        Ok(())
    }

    fn display_audit_compliance(&self, result: &AuditResult) {
        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║                      AUDIT COMPLIANCE                        ║");
        println!("╠══════════════════════════════════════════════════════════════╣");

        println!("║ {} Three velocity levels implemented                      ║",
                 if result.has_three_velocities { "✅" } else { "❌" });
        println!("║ {} Traffic congestion kept low (≤8 vehicles)             ║",
                 if result.low_congestion { "✅" } else { "❌" });
        println!("║ {} Good collision avoidance (<15% close calls)           ║",
                 if result.good_collision_avoidance { "✅" } else { "❌" });
        println!("║ {} Vehicles completing journeys (>80%)                   ║",
                 if result.vehicles_completing { "✅" } else { "❌" });
        println!("║ {} Efficient intersection management (<5s avg)           ║",
                 if result.efficient_intersection { "✅" } else { "❌" });
        println!("║ {} All directions utilized                                ║",
                 if result.all_directions_used { "✅" } else { "❌" });
        println!("║ {} All route types utilized                              ║",
                 if result.all_routes_used { "✅" } else { "❌" });

        let passed_checks = result.count_passed();
        let total_checks = 7;

        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ OVERALL COMPLIANCE: {}/{} checks passed                     ║", passed_checks, total_checks);

        if passed_checks == total_checks {
            println!("║ 🎉 EXCELLENT! All audit requirements met!                  ║");
        } else if passed_checks >= total_checks - 1 {
            println!("║ 👍 GOOD! Almost all requirements met.                      ║");
        } else if passed_checks >= total_checks / 2 {
            println!("║ ⚠️  FAIR. Some improvements needed.                        ║");
        } else {
            println!("║ ❌ POOR. Significant improvements required.                ║");
        }

        println!("╚══════════════════════════════════════════════════════════════╝");
    }

    pub fn display(&self) -> Result<(), String> {
        self.display_comprehensive()
    }

    // Get percentile statistics for more detailed analysis
    pub fn get_velocity_percentiles(&self) -> (f64, f64, f64) {
        if self.velocity_samples.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let mut sorted_velocities = self.velocity_samples.clone();
        sorted_velocities.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let len = sorted_velocities.len();
        let p25 = sorted_velocities[len / 4];
        let p50 = sorted_velocities[len / 2];
        let p75 = sorted_velocities[3 * len / 4];

        (p25, p50, p75)
    }

    pub fn get_intersection_time_percentiles(&self) -> (f64, f64, f64) {
        if self.intersection_times.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let mut sorted_times = self.intersection_times.clone();
        sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let len = sorted_times.len();
        let p25 = sorted_times[len / 4];
        let p50 = sorted_times[len / 2];
        let p75 = sorted_times[3 * len / 4];

        (p25, p50, p75)
    }
}

pub struct AuditResult {
    pub has_three_velocities: bool,
    pub low_congestion: bool,
    pub good_collision_avoidance: bool,
    pub vehicles_completing: bool,
    pub efficient_intersection: bool,
    pub all_directions_used: bool,
    pub all_routes_used: bool,
}

impl AuditResult {
    pub fn new() -> Self {
        AuditResult {
            has_three_velocities: false,
            low_congestion: false,
            good_collision_avoidance: false,
            vehicles_completing: false,
            efficient_intersection: false,
            all_directions_used: false,
            all_routes_used: false,
        }
    }

    pub fn count_passed(&self) -> u32 {
        let mut count = 0;
        if self.has_three_velocities { count += 1; }
        if self.low_congestion { count += 1; }
        if self.good_collision_avoidance { count += 1; }
        if self.vehicles_completing { count += 1; }
        if self.efficient_intersection { count += 1; }
        if self.all_directions_used { count += 1; }
        if self.all_routes_used { count += 1; }
        count
    }
}