// src/statistics.rs - ENHANCED: Comprehensive statistics for smooth animation system
use std::collections::VecDeque;
use std::time::Instant;
use crate::vehicle::{Vehicle, VehicleState, Direction, Route, Vec2};

pub struct Statistics {
    pub total_vehicles_spawned: u32,
    pub vehicles_completed: u32,
    pub max_velocity: f32,        // Now f32 for consistency
    pub min_velocity: f32,        // Now f32 for consistency
    pub max_congestion: usize,
    pub close_calls: u32,

    // Enhanced statistics for smooth animation system
    pub max_time_in_intersection: f32,    // Now f32 for consistency
    pub min_time_in_intersection: f32,    // Now f32 for consistency
    pub total_intersection_time: f32,     // Now f32 for consistency
    pub vehicles_with_intersection_time: u32,

    // Traffic flow statistics
    pub vehicles_by_direction: [u32; 4], // North, South, East, West
    pub vehicles_by_route: [u32; 3],     // Left, Straight, Right

    // Performance tracking with floating-point precision
    simulation_start: Instant,
    velocity_samples: Vec<f32>,           // Now f32 for consistency
    congestion_samples: Vec<usize>,

    // Detailed timing statistics
    intersection_times: Vec<f32>,         // Now f32 for consistency
    completion_times: Vec<f32>,           // Now f32 for consistency

    // NEW: Smooth animation quality metrics
    pub animation_smoothness_score: f32,
    pub turning_accuracy_score: f32,
    pub lane_adherence_score: f32,

    // NEW: Advanced performance metrics
    frame_rate_samples: Vec<f32>,
    physics_timing_samples: Vec<f32>,
    rendering_timing_samples: Vec<f32>,

    // NEW: Path analysis for smooth animations
    vehicle_paths: std::collections::HashMap<u32, Vec<Vec2>>,
    path_deviation_scores: Vec<f32>,

    // NEW: Velocity transition analysis
    velocity_changes: Vec<VelocityTransition>,
    smooth_transitions: u32,
    jerky_transitions: u32,

    // NEW: Intersection performance
    intersection_efficiency: IntersectionEfficiency,
}

// NEW: Velocity transition tracking
#[derive(Debug, Clone)]
struct VelocityTransition {
    vehicle_id: u32,
    from_velocity: f32,
    to_velocity: f32,
    transition_time: f32,
    smoothness_score: f32,
    timestamp: f32,
}

// NEW: Intersection efficiency metrics
#[derive(Debug, Clone)]
struct IntersectionEfficiency {
    total_throughput: u32,
    average_wait_time: f32,
    capacity_utilization: f32,
    conflict_resolution_time: f32,
    successful_merges: u32,
    failed_merges: u32,
}

impl Statistics {
    pub fn new() -> Self {
        Statistics {
            total_vehicles_spawned: 0,
            vehicles_completed: 0,
            max_velocity: 0.0,
            min_velocity: f32::INFINITY,
            max_congestion: 0,
            close_calls: 0,

            max_time_in_intersection: 0.0,
            min_time_in_intersection: f32::INFINITY,
            total_intersection_time: 0.0,
            vehicles_with_intersection_time: 0,

            vehicles_by_direction: [0; 4],
            vehicles_by_route: [0; 3],

            simulation_start: Instant::now(),
            velocity_samples: Vec::new(),
            congestion_samples: Vec::new(),
            intersection_times: Vec::new(),
            completion_times: Vec::new(),

            // NEW: Smooth animation metrics
            animation_smoothness_score: 100.0,
            turning_accuracy_score: 100.0,
            lane_adherence_score: 100.0,

            frame_rate_samples: Vec::new(),
            physics_timing_samples: Vec::new(),
            rendering_timing_samples: Vec::new(),

            vehicle_paths: std::collections::HashMap::new(),
            path_deviation_scores: Vec::new(),

            velocity_changes: Vec::new(),
            smooth_transitions: 0,
            jerky_transitions: 0,

            intersection_efficiency: IntersectionEfficiency::new(),
        }
    }

    // ENHANCED: Update with smooth animation quality analysis
    pub fn update(&mut self, vehicles: &VecDeque<Vehicle>) {
        // Track maximum congestion
        if vehicles.len() > self.max_congestion {
            self.max_congestion = vehicles.len();
        }

        // Sample congestion for statistics
        self.congestion_samples.push(vehicles.len());

        // Update velocity statistics and analyze smoothness
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

            // NEW: Track path quality for smooth animations
            self.update_path_tracking(vehicle);

            // NEW: Analyze velocity transitions
            self.analyze_velocity_transitions(vehicle);

            // Track intersection timing for vehicles currently in intersection
            if matches!(vehicle.state, VehicleState::Entering | VehicleState::Turning | VehicleState::Exiting) {
                let time_in_intersection = vehicle.time_in_intersection as f32 / 1000.0;

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

        // NEW: Update animation quality scores
        self.update_animation_quality_scores(vehicles);

        // Update intersection efficiency
        self.intersection_efficiency.update(vehicles, self.simulation_start.elapsed().as_secs_f32());
    }

    // NEW: Track vehicle paths for smoothness analysis
    fn update_path_tracking(&mut self, vehicle: &Vehicle) {
        let path = self.vehicle_paths.entry(vehicle.id).or_insert(Vec::new());

        // Add current position to path
        path.push(vehicle.position);

        // Limit path history to last 50 points for performance
        if path.len() > 50 {
            path.remove(0);
        }

        // Analyze path smoothness if we have enough points
        if path.len() > 5 {
            // Clone the path to avoid borrowing issues
            let path_clone = path.clone();
            let smoothness = Self::calculate_path_smoothness_static(&path_clone);
            self.path_deviation_scores.push(smoothness);
        }
    }

    fn calculate_path_smoothness(&self, path: &[Vec2]) -> f32 {
        Self::calculate_path_smoothness_static(path)
    }

    fn calculate_path_smoothness_static(path: &[Vec2]) -> f32 {
        if path.len() < 3 {
            return 100.0; // Perfect score for insufficient data
        }

        let mut total_deviation = 0.0;
        let mut measurements = 0;

        // Calculate curvature deviation
        for i in 1..path.len()-1 {
            let prev = path[i-1];
            let curr = path[i];
            let next = path[i+1];

            // Calculate expected position based on linear interpolation
            let expected = prev + (next - prev) * 0.5;
            let deviation = (curr - expected).length();

            total_deviation += deviation;
            measurements += 1;
        }

        if measurements == 0 {
            return 100.0;
        }

        let average_deviation = total_deviation / measurements as f32;

        // Convert to score (lower deviation = higher score)
        (100.0 - (average_deviation * 10.0)).max(0.0).min(100.0)
    }

    // NEW: Analyze velocity transitions for smoothness
    fn analyze_velocity_transitions(&mut self, vehicle: &Vehicle) {
        // This would need previous velocity tracking - simplified for now
        let current_time = self.simulation_start.elapsed().as_secs_f32();

        // Check for jerky transitions (large velocity changes)
        if let Some(last_sample) = self.velocity_samples.last() {
            let velocity_change = (vehicle.current_velocity - last_sample).abs();

            if velocity_change > 20.0 { // Threshold for "jerky" transition
                self.jerky_transitions += 1;

                self.velocity_changes.push(VelocityTransition {
                    vehicle_id: vehicle.id,
                    from_velocity: *last_sample,
                    to_velocity: vehicle.current_velocity,
                    transition_time: 0.1, // Approximate frame time
                    smoothness_score: 0.0, // Poor score for jerky transition
                    timestamp: current_time,
                });
            } else if velocity_change > 0.1 {
                self.smooth_transitions += 1;

                self.velocity_changes.push(VelocityTransition {
                    vehicle_id: vehicle.id,
                    from_velocity: *last_sample,
                    to_velocity: vehicle.current_velocity,
                    transition_time: 0.1,
                    smoothness_score: (20.0 - velocity_change) / 20.0 * 100.0,
                    timestamp: current_time,
                });
            }
        }
    }

    // NEW: Update animation quality scores
    fn update_animation_quality_scores(&mut self, vehicles: &VecDeque<Vehicle>) {
        if vehicles.is_empty() {
            return;
        }

        // Calculate animation smoothness score
        if !self.path_deviation_scores.is_empty() {
            let average_smoothness = self.path_deviation_scores.iter().sum::<f32>() / self.path_deviation_scores.len() as f32;
            self.animation_smoothness_score = self.animation_smoothness_score * 0.95 + average_smoothness * 0.05;
        }

        // Calculate turning accuracy score (vehicles staying in correct lanes)
        let mut lane_adherence_total = 0.0;
        let mut measurements = 0;

        for vehicle in vehicles {
            // Simplified lane adherence check
            let expected_lane_center = self.get_expected_lane_center(vehicle);
            let distance_from_center = (vehicle.position - expected_lane_center).length();

            let adherence = (50.0 - distance_from_center.min(50.0)) / 50.0 * 100.0;
            lane_adherence_total += adherence;
            measurements += 1;
        }

        if measurements > 0 {
            let current_adherence = lane_adherence_total / measurements as f32;
            self.lane_adherence_score = self.lane_adherence_score * 0.95 + current_adherence * 0.05;
        }

        // Calculate turning accuracy based on smooth transitions through turns
        let turning_vehicles = vehicles.iter().filter(|v| v.state == VehicleState::Turning).count();
        if turning_vehicles > 0 {
            // Simplified turning accuracy - could be enhanced with actual turn path analysis
            let smooth_ratio = if self.smooth_transitions + self.jerky_transitions > 0 {
                self.smooth_transitions as f32 / (self.smooth_transitions + self.jerky_transitions) as f32 * 100.0
            } else {
                100.0
            };
            self.turning_accuracy_score = self.turning_accuracy_score * 0.98 + smooth_ratio * 0.02;
        }
    }

    fn get_expected_lane_center(&self, vehicle: &Vehicle) -> Vec2 {
        // Simplified calculation - should match the lane mathematics from main.rs
        let center_x = crate::WINDOW_WIDTH as f32 / 2.0;
        let center_y = crate::WINDOW_HEIGHT as f32 / 2.0;
        let lane_width = 30.0;

        match vehicle.direction {
            Direction::North => {
                let x = center_x + 15.0 + (vehicle.lane as f32 * lane_width);
                Vec2::new(x, vehicle.position.y)
            }
            Direction::South => {
                let x = center_x - 15.0 - (vehicle.lane as f32 * lane_width);
                Vec2::new(x, vehicle.position.y)
            }
            Direction::East => {
                let y = center_y + 15.0 + (vehicle.lane as f32 * lane_width);
                Vec2::new(vehicle.position.x, y)
            }
            Direction::West => {
                let y = center_y - 15.0 - (vehicle.lane as f32 * lane_width);
                Vec2::new(vehicle.position.x, y)
            }
        }
    }

    // NEW: Record performance metrics
    pub fn record_frame_rate(&mut self, fps: f32) {
        self.frame_rate_samples.push(fps);
        if self.frame_rate_samples.len() > 100 {
            self.frame_rate_samples.remove(0);
        }
    }

    pub fn record_physics_timing(&mut self, timing_ms: f32) {
        self.physics_timing_samples.push(timing_ms);
        if self.physics_timing_samples.len() > 100 {
            self.physics_timing_samples.remove(0);
        }
    }

    pub fn record_rendering_timing(&mut self, timing_ms: f32) {
        self.rendering_timing_samples.push(timing_ms);
        if self.rendering_timing_samples.len() > 100 {
            self.rendering_timing_samples.remove(0);
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

        let intersection_time = intersection_time_ms as f32 / 1000.0;
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
        let total_time = self.simulation_start.elapsed().as_secs_f32();
        self.completion_times.push(total_time);

        self.intersection_efficiency.record_completion();
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

    // ENHANCED: Get performance metrics with floating-point precision
    pub fn get_average_velocity(&self) -> f32 {
        if self.velocity_samples.is_empty() {
            return 0.0;
        }
        self.velocity_samples.iter().sum::<f32>() / self.velocity_samples.len() as f32
    }

    pub fn get_average_congestion(&self) -> f32 {
        if self.congestion_samples.is_empty() {
            return 0.0;
        }
        self.congestion_samples.iter().sum::<usize>() as f32 / self.congestion_samples.len() as f32
    }

    pub fn get_average_intersection_time(&self) -> f32 {
        if self.vehicles_with_intersection_time == 0 {
            return 0.0;
        }
        self.total_intersection_time / self.vehicles_with_intersection_time as f32
    }

    pub fn get_throughput_per_minute(&self) -> f32 {
        let elapsed_minutes = self.simulation_start.elapsed().as_secs_f32() / 60.0;
        if elapsed_minutes <= 0.0 {
            return 0.0;
        }
        self.vehicles_completed as f32 / elapsed_minutes
    }

    pub fn get_close_call_rate(&self) -> f32 {
        if self.vehicles_completed == 0 {
            return 0.0;
        }
        (self.close_calls as f32 / self.vehicles_completed as f32) * 100.0
    }

    pub fn get_completion_rate(&self) -> f32 {
        if self.total_vehicles_spawned == 0 {
            return 0.0;
        }
        (self.vehicles_completed as f32 / self.total_vehicles_spawned as f32) * 100.0
    }

    // NEW: Get animation quality metrics
    pub fn get_animation_quality_metrics(&self) -> AnimationQualityMetrics {
        let average_fps = if !self.frame_rate_samples.is_empty() {
            self.frame_rate_samples.iter().sum::<f32>() / self.frame_rate_samples.len() as f32
        } else {
            60.0
        };

        let average_physics_time = if !self.physics_timing_samples.is_empty() {
            self.physics_timing_samples.iter().sum::<f32>() / self.physics_timing_samples.len() as f32
        } else {
            0.0
        };

        let smooth_transition_rate = if self.smooth_transitions + self.jerky_transitions > 0 {
            self.smooth_transitions as f32 / (self.smooth_transitions + self.jerky_transitions) as f32 * 100.0
        } else {
            100.0
        };

        AnimationQualityMetrics {
            smoothness_score: self.animation_smoothness_score,
            turning_accuracy: self.turning_accuracy_score,
            lane_adherence: self.lane_adherence_score,
            average_fps,
            average_physics_time,
            smooth_transition_rate,
            total_transitions: self.smooth_transitions + self.jerky_transitions,
        }
    }

    // Enhanced audit compliance check
    pub fn check_audit_requirements(&self) -> AuditResult {
        let mut result = AuditResult::new();

        // Check if all three velocity levels are being used
        result.has_three_velocities = self.velocity_samples.iter().any(|&v| v <= 30.0) &&
            self.velocity_samples.iter().any(|&v| v > 30.0 && v <= 60.0) &&
            self.velocity_samples.iter().any(|&v| v > 60.0);

        // Check congestion levels
        result.low_congestion = self.max_congestion <= 8;

        // Check collision avoidance
        result.good_collision_avoidance = self.get_close_call_rate() < 15.0;

        // Check completion rate
        result.vehicles_completing = self.get_completion_rate() > 80.0;

        // Check intersection efficiency
        result.efficient_intersection = self.get_average_intersection_time() < 5.0;

        // Check direction usage
        result.all_directions_used = self.vehicles_by_direction.iter().all(|&count| count > 0);

        // Check route usage
        result.all_routes_used = self.vehicles_by_route.iter().all(|&count| count > 0);

        // NEW: Check animation quality
        result.smooth_animations = self.animation_smoothness_score > 80.0;
        result.accurate_turning = self.turning_accuracy_score > 85.0;
        result.good_lane_adherence = self.lane_adherence_score > 90.0;

        result
    }

    // ENHANCED: Comprehensive display with animation metrics
    pub fn display_comprehensive(&self) -> Result<(), String> {
        let elapsed = self.simulation_start.elapsed();
        let animation_metrics = self.get_animation_quality_metrics();

        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║              COMPREHENSIVE STATISTICS - SMOOTH SYSTEM        ║");
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
                 if self.min_velocity == f32::INFINITY { 0.0 } else { self.min_velocity });
        println!("║ Average Velocity: {:>14.1} px/s                        ║", self.get_average_velocity());

        println!("╠══════════════════════════════════════════════════════════════╣");

        // Intersection timing
        println!("║ Max Time in Intersection: {:>8.1}s                      ║", self.max_time_in_intersection);
        println!("║ Min Time in Intersection: {:>8.1}s                      ║",
                 if self.min_time_in_intersection == f32::INFINITY { 0.0 } else { self.min_time_in_intersection });
        println!("║ Avg Time in Intersection: {:>8.1}s                      ║", self.get_average_intersection_time());

        println!("╠══════════════════════════════════════════════════════════════╣");

        // Safety and efficiency
        println!("║ Close Calls: {:>20}                           ║", self.close_calls);
        println!("║ Close Call Rate: {:>15.1}%                          ║", self.get_close_call_rate());
        println!("║ Max Congestion: {:>16} vehicles                   ║", self.max_congestion);
        println!("║ Average Congestion: {:>12.1} vehicles                   ║", self.get_average_congestion());

        println!("╠══════════════════════════════════════════════════════════════╣");

        // NEW: Animation quality metrics
        println!("║ SMOOTH ANIMATION QUALITY:                                  ║");
        println!("║ Animation Smoothness: {:>13.1}%                        ║", animation_metrics.smoothness_score);
        println!("║ Turning Accuracy: {:>17.1}%                        ║", animation_metrics.turning_accuracy);
        println!("║ Lane Adherence: {:>19.1}%                        ║", animation_metrics.lane_adherence);
        println!("║ Average FPS: {:>23.1}                           ║", animation_metrics.average_fps);
        println!("║ Smooth Transitions: {:>14.1}%                        ║", animation_metrics.smooth_transition_rate);

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

        // Enhanced audit compliance
        let audit_result = self.check_audit_requirements();
        self.display_enhanced_audit_compliance(&audit_result);

        Ok(())
    }

    fn display_enhanced_audit_compliance(&self, result: &AuditResult) {
        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║                   ENHANCED AUDIT COMPLIANCE                  ║");
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

        // NEW: Animation quality checks
        println!("║ {} Smooth animations (>80% smoothness score)             ║",
                 if result.smooth_animations { "✅" } else { "❌" });
        println!("║ {} Accurate turning (>85% accuracy score)                ║",
                 if result.accurate_turning { "✅" } else { "❌" });
        println!("║ {} Good lane adherence (>90% adherence)                  ║",
                 if result.good_lane_adherence { "✅" } else { "❌" });

        let passed_checks = result.count_passed();
        let total_checks = 10; // Updated total

        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ OVERALL COMPLIANCE: {}/{} checks passed                     ║", passed_checks, total_checks);

        if passed_checks == total_checks {
            println!("║ 🎉 EXCELLENT! All enhanced requirements met!              ║");
        } else if passed_checks >= total_checks - 1 {
            println!("║ 👍 VERY GOOD! Almost all requirements met.                ║");
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

    // Get percentile statistics for detailed analysis
    pub fn get_velocity_percentiles(&self) -> (f32, f32, f32) {
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

    pub fn get_intersection_time_percentiles(&self) -> (f32, f32, f32) {
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

    // NEW: Cleanup old path data to prevent memory issues
    pub fn cleanup_completed_vehicle_data(&mut self, active_vehicle_ids: &[u32]) {
        let active_ids: std::collections::HashSet<_> = active_vehicle_ids.iter().collect();
        self.vehicle_paths.retain(|id, _| active_ids.contains(id));
    }
}

// NEW: Animation quality metrics structure
pub struct AnimationQualityMetrics {
    pub smoothness_score: f32,
    pub turning_accuracy: f32,
    pub lane_adherence: f32,
    pub average_fps: f32,
    pub average_physics_time: f32,
    pub smooth_transition_rate: f32,
    pub total_transitions: u32,
}

// ENHANCED: Audit result with animation quality
pub struct AuditResult {
    pub has_three_velocities: bool,
    pub low_congestion: bool,
    pub good_collision_avoidance: bool,
    pub vehicles_completing: bool,
    pub efficient_intersection: bool,
    pub all_directions_used: bool,
    pub all_routes_used: bool,
    // NEW: Animation quality checks
    pub smooth_animations: bool,
    pub accurate_turning: bool,
    pub good_lane_adherence: bool,
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
            smooth_animations: false,
            accurate_turning: false,
            good_lane_adherence: false,
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
        if self.smooth_animations { count += 1; }
        if self.accurate_turning { count += 1; }
        if self.good_lane_adherence { count += 1; }
        count
    }
}

// NEW: Intersection efficiency implementation
impl IntersectionEfficiency {
    fn new() -> Self {
        IntersectionEfficiency {
            total_throughput: 0,
            average_wait_time: 0.0,
            capacity_utilization: 0.0,
            conflict_resolution_time: 0.0,
            successful_merges: 0,
            failed_merges: 0,
        }
    }

    fn update(&mut self, vehicles: &VecDeque<Vehicle>, elapsed_time: f32) {
        // Calculate capacity utilization
        let active_vehicles = vehicles.len();
        let max_capacity = 20; // Theoretical maximum
        self.capacity_utilization = (active_vehicles as f32 / max_capacity as f32 * 100.0).min(100.0);

        // Update average wait time
        let mut total_wait_time = 0.0;
        let mut waiting_vehicles = 0;

        for vehicle in vehicles {
            if vehicle.current_velocity < Vehicle::SLOW_VELOCITY * 0.5 {
                total_wait_time += vehicle.start_time.elapsed().as_secs_f32();
                waiting_vehicles += 1;
            }
        }

        if waiting_vehicles > 0 {
            let current_avg_wait = total_wait_time / waiting_vehicles as f32;
            self.average_wait_time = self.average_wait_time * 0.9 + current_avg_wait * 0.1;
        }
    }

    fn record_completion(&mut self) {
        self.total_throughput += 1;
        self.successful_merges += 1;
    }
}