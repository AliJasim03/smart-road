// src/statistics.rs - With stats display function
use std::collections::VecDeque;
use std::time::Instant;
use crate::vehicle::{Vehicle, Direction, Route};

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
    intersection_times: Vec<f32>,
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
            intersection_times: Vec::new(),
        }
    }

    pub fn update(&mut self, vehicles: &VecDeque<Vehicle>, close_calls: u32) {
        self.max_congestion = self.max_congestion.max(vehicles.len());
        self.close_calls = close_calls;

        let mut current_max_vel = 0.0;
        let mut current_min_vel = f32::INFINITY;

        for vehicle in vehicles {
            if vehicle.current_velocity > current_max_vel {
                current_max_vel = vehicle.current_velocity;
            }
            if vehicle.current_velocity < current_min_vel {
                current_min_vel = vehicle.current_velocity;
            }
        }

        self.max_velocity = self.max_velocity.max(current_max_vel);
        if current_min_vel != f32::INFINITY {
            self.min_velocity = self.min_velocity.min(current_min_vel);
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

        if time_sec > 0.0 {
            self.intersection_times.push(time_sec);
            if time_sec > self.max_time_in_intersection {
                self.max_time_in_intersection = time_sec;
            }
            if time_sec < self.min_time_in_intersection {
                self.min_time_in_intersection = time_sec;
            }
        }
    }

    pub fn get_display_string(&self) -> String {
        let elapsed_sec = self.simulation_start.elapsed().as_secs_f32();
        let throughput = if elapsed_sec > 0.0 { self.vehicles_completed as f32 * 60.0 / elapsed_sec } else { 0.0 };
        let min_vel = if self.min_velocity == f32::INFINITY { 0.0 } else { self.min_velocity };
        let avg_time = if self.intersection_times.is_empty() { 0.0 } else { self.intersection_times.iter().sum::<f32>() / self.intersection_times.len() as f32 };
        let min_time = if self.min_time_in_intersection == f32::INFINITY { 0.0 } else { self.min_time_in_intersection };

        format!(
            "--- FINAL STATISTICS ---\n\n\
            Simulation Duration: {:.1}s\n\
            Vehicles Completed: {}\n\
            Throughput: {:.1} veh/min\n\n\
            --- Performance ---\n\
            Max Velocity: {:.1} px/s\n\
            Min Velocity: {:.1} px/s\n\
            Avg Intersection Time: {:.1}s\n\
            Min/Max Intersection Time: {:.1}s / {:.1}s\n\n\
            --- Safety & Congestion ---\n\
            Close Calls: {}\n\
            Max Concurrent Vehicles: {}",
            elapsed_sec, self.vehicles_completed, throughput,
            self.max_velocity, min_vel,
            avg_time, min_time, self.max_time_in_intersection,
            self.close_calls, self.max_congestion
        )
    }
}