// src/statistics.rs - SIMPLIFIED VERSION
use std::collections::VecDeque;
use crate::vehicle::Vehicle;

pub struct Statistics {
    pub total_vehicles_spawned: u32,
    pub vehicles_completed: u32,
    pub max_velocity: f64,
    pub min_velocity: f64,
    pub max_congestion: usize,
    pub close_calls: u32,
}

impl Statistics {
    pub fn new() -> Self {
        Statistics {
            total_vehicles_spawned: 0,
            vehicles_completed: 0,
            max_velocity: 0.0,
            min_velocity: f64::MAX,
            max_congestion: 0,
            close_calls: 0,
        }
    }

    pub fn update(&mut self, vehicles: &VecDeque<Vehicle>) {
        // Track maximum congestion
        if vehicles.len() > self.max_congestion {
            self.max_congestion = vehicles.len();
        }

        // Update velocity statistics
        for vehicle in vehicles {
            if vehicle.current_velocity > self.max_velocity {
                self.max_velocity = vehicle.current_velocity;
            }
            if vehicle.current_velocity > 0.0 && vehicle.current_velocity < self.min_velocity {
                self.min_velocity = vehicle.current_velocity;
            }
        }
    }

    pub fn add_completed_vehicles(&mut self, count: usize) {
        self.vehicles_completed += count as u32;
    }

    pub fn add_spawned_vehicle(&mut self) {
        self.total_vehicles_spawned += 1;
    }

    pub fn add_close_call(&mut self) {
        self.close_calls += 1;
    }

    pub fn display_summary(&self) {
        println!("\n=== SIMULATION STATISTICS ===");
        println!("Total vehicles spawned: {}", self.total_vehicles_spawned);
        println!("Vehicles completed: {}", self.vehicles_completed);
        println!("Max velocity: {:.1} px/s", self.max_velocity);
        println!("Min velocity: {:.1} px/s", if self.min_velocity == f64::MAX { 0.0 } else { self.min_velocity });
        println!("Max congestion: {} vehicles", self.max_congestion);
        println!("Close calls: {}", self.close_calls);

        if self.total_vehicles_spawned > 0 {
            let completion_rate = (self.vehicles_completed as f64 / self.total_vehicles_spawned as f64) * 100.0;
            println!("Completion rate: {:.1}%", completion_rate);
        }

        println!("=============================\n");
    }
}