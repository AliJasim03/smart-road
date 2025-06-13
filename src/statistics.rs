// src/statistics.rs - COMPATIBILITY: Verified with new turn system
use std::collections::{VecDeque, HashMap};
use std::time::Instant;
use crate::vehicle::{Vehicle, Vec2, Direction, Route};

pub struct Statistics {
    pub total_vehicles_spawned: u32,
    pub vehicles_completed: u32,
    pub max_velocity: f32,
    pub min_velocity: f32,
    pub max_congestion: usize,
    pub close_calls: u32,
    pub max_time_in_intersection: f32,
    pub min_time_in_intersection: f32,

    vehicles_by_direction: [u32; 4], // N, S, E, W
    vehicles_by_route: [u32; 3], // L, S, R

    simulation_start: Instant,
    velocity_samples: Vec<f32>,
    intersection_times: Vec<f32>,
    vehicle_paths: HashMap<u32, Vec<Vec2>>,
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
            vehicles_by_direction: [0; 4],
            vehicles_by_route: [0; 3],
            simulation_start: Instant::now(),
            velocity_samples: Vec::new(),
            intersection_times: Vec::new(),
            vehicle_paths: HashMap::new(),
        }
    }

    pub fn update(&mut self, vehicles: &VecDeque<Vehicle>, close_calls: u32) {
        self.max_congestion = self.max_congestion.max(vehicles.len());
        self.close_calls = close_calls;

        for vehicle in vehicles {
            if vehicle.current_velocity > self.max_velocity {
                self.max_velocity = vehicle.current_velocity;
            }
            if vehicle.current_velocity > 0.0 && vehicle.current_velocity < self.min_velocity {
                self.min_velocity = vehicle.current_velocity;
            }
            self.velocity_samples.push(vehicle.current_velocity);
        }
    }

    pub fn record_vehicle_spawn(&mut self, direction: Direction, route: Route) {
        self.total_vehicles_spawned += 1;
        self.vehicles_by_direction[direction as usize] += 1;
        self.vehicles_by_route[route as usize] += 1;
    }

    pub fn record_vehicle_completion(&mut self, intersection_time_ms: u32) {
        self.vehicles_completed += 1;
        let time_sec = intersection_time_ms as f32 / 1000.0;
        self.intersection_times.push(time_sec);

        if time_sec > self.max_time_in_intersection {
            self.max_time_in_intersection = time_sec;
        }
        if time_sec < self.min_time_in_intersection {
            self.min_time_in_intersection = time_sec;
        }
    }

    pub fn cleanup_completed_vehicle_data(&mut self, active_ids: &[u32]) {
        self.vehicle_paths.retain(|&id, _| active_ids.contains(&id));
    }

    pub fn get_average_intersection_time(&self) -> f32 {
        if self.intersection_times.is_empty() { return 0.0; }
        self.intersection_times.iter().sum::<f32>() / self.intersection_times.len() as f32
    }

    pub fn display(&self) -> Result<(), String> {
        let elapsed_sec = self.simulation_start.elapsed().as_secs_f32();

        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║                      FINAL STATISTICS                        ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Simulation Duration: {:>8.1}s                             ║", elapsed_sec);
        println!("║ Total Vehicles Spawned: {:<8}                              ║", self.total_vehicles_spawned);
        println!("║ Vehicles Completed: {:<12}                              ║", self.vehicles_completed);
        let throughput = if elapsed_sec > 0.0 { self.vehicles_completed as f32 * 60.0 / elapsed_sec} else {0.0};
        println!("║ Throughput: {:>16.1} veh/min                      ║", throughput);

        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Max Velocity: {:>18.1} px/s                        ║", self.max_velocity);
        let min_vel = if self.min_velocity == f32::INFINITY { 0.0 } else { self.min_velocity };
        println!("║ Min Velocity: {:>18.1} px/s                        ║", min_vel);

        println!("╠══════════════════════════════════════════════════════════════╣");
        let max_time = self.max_time_in_intersection;
        let min_time = if self.min_time_in_intersection == f32::INFINITY { 0.0 } else {self.min_time_in_intersection};
        println!("║ Max Time in Intersection: {:>8.1}s                      ║", max_time);
        println!("║ Min Time in Intersection: {:>8.1}s                      ║", min_time);
        println!("║ Avg Time in Intersection: {:>8.1}s                      ║", self.get_average_intersection_time());

        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Close Calls: {:<20}                              ║", self.close_calls);
        println!("║ Max Congestion: {:<16} vehicles                   ║", self.max_congestion);

        println!("╚══════════════════════════════════════════════════════════════╝");

        Ok(())
    }
}