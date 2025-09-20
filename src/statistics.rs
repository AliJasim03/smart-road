use crate::direction::*;
use crate::vehicle_positions::Position;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

const SAFE_DISTANCE: f32 = 55.0;

#[derive(Debug)]
pub struct VehicleStats {
    entry_time: Instant,
    exit_time: Option<Instant>,
    max_velocity: f32,
    min_velocity: f32,
    in_intersection: bool,
}

impl VehicleStats {
    pub fn new() -> Self {
        Self {
            entry_time: Instant::now(),
            exit_time: None,
            max_velocity: 0.0,
            min_velocity: f32::MAX,
            in_intersection: false,
        }
    }

    pub fn update_velocity(&mut self, velocity: f32) {
        if velocity > 0.0 {
            self.max_velocity = self.max_velocity.max(velocity);
            self.min_velocity = self.min_velocity.min(velocity);
        }
    }

    pub fn record_exit(&mut self) {
        self.exit_time = Some(Instant::now());
    }

    pub fn get_intersection_time(&self) -> Option<f32> {
        self.exit_time
            .map(|exit| (exit.duration_since(self.entry_time)).as_secs_f32())
    }
}

pub struct Statistics {
    pub vehicles_spawned: HashMap<Direction, u32>,
    pub total_vehicles: u32,
    pub simulation_start: Instant,
    pub end_time: Option<f32>,
    pub vehicle_stats: HashMap<usize, VehicleStats>,
    pub max_intersection_time: f32,
    pub min_intersection_time: f32,
    pub total_close_calls: u32,
    pub max_velocity: f32,
    pub min_velocity: f32,
    pub current_vehicles_in_intersection: u32,
    pub max_vehicles_in_intersection: u32,
    vehicle_counter: usize,
    close_call_pairs: HashSet<(usize, usize)>,
}

impl Statistics {
    pub fn new() -> Self {
        Statistics {
            vehicles_spawned: HashMap::new(),
            total_vehicles: 0,
            simulation_start: Instant::now(),
            end_time: None,
            vehicle_stats: HashMap::new(),
            max_intersection_time: 0.0,
            min_intersection_time: f32::MAX,
            total_close_calls: 0,
            max_velocity: 0.0,
            min_velocity: f32::MAX,
            current_vehicles_in_intersection: 0,
            max_vehicles_in_intersection: 0,
            vehicle_counter: 0,
            close_call_pairs: HashSet::new(),
        }
    }

    pub fn add_vehicle(&mut self, direction: Direction) -> usize {
        *self.vehicles_spawned.entry(direction).or_insert(0) += 1;
        self.total_vehicles += 1;

        let vehicle_id = self.vehicle_counter;
        self.vehicle_counter += 1;

        self.vehicle_stats.insert(vehicle_id, VehicleStats::new());
        vehicle_id
    }

    pub fn update_vehicle_stats(&mut self, vehicle_id: usize, position: Position, velocity: f32) {
        if let Some(stats) = self.vehicle_stats.get_mut(&vehicle_id) {
            // Track intersection entry/exit
            let was_in_intersection = stats.in_intersection;
            let now_in_intersection = position.is_in_intersection();

            if !was_in_intersection && now_in_intersection {
                // Vehicle entered intersection
                self.current_vehicles_in_intersection += 1;
                self.max_vehicles_in_intersection = self
                    .max_vehicles_in_intersection
                    .max(self.current_vehicles_in_intersection);
                stats.in_intersection = true;
            } else if was_in_intersection && !now_in_intersection {
                // Vehicle exited intersection
                if self.current_vehicles_in_intersection > 0 {
                    self.current_vehicles_in_intersection -= 1;
                }
                stats.in_intersection = false;
            }

            // Update velocity stats
            if velocity > 0.0 {
                stats.update_velocity(velocity);
                self.max_velocity = self.max_velocity.max(velocity);
                self.min_velocity = self.min_velocity.min(velocity);
            }
        }
    }

    pub fn record_vehicle_exit(&mut self, vehicle_id: usize) {
        if let Some(stats) = self.vehicle_stats.get_mut(&vehicle_id) {
            stats.record_exit();
            if let Some(time) = stats.get_intersection_time() {
                self.max_intersection_time = self.max_intersection_time.max(time);
                self.min_intersection_time = self.min_intersection_time.min(time);
            }

            // Make sure to update intersection count if vehicle is removed while in intersection
            if stats.in_intersection {
                if self.current_vehicles_in_intersection > 0 {
                    self.current_vehicles_in_intersection -= 1;
                }
            }
        }
    }

    pub fn check_close_calls(&mut self, vehicle_positions: &[(usize, (i32, i32))]) {
        for (i, &(id1, pos1)) in vehicle_positions.iter().enumerate() {
            // Create position struct to check if in intersection
            let pos = Position {
                x: pos1.0,
                y: pos1.1,
            };

            for &(id2, pos2) in vehicle_positions.iter().skip(i + 1) {
                let other_pos = Position {
                    x: pos2.0,
                    y: pos2.1,
                };

                // At least one vehicle should be in intersection for it to be a close call
                if !pos.is_in_intersection() && !other_pos.is_in_intersection() {
                    continue;
                }

                let dx = (pos2.0 - pos1.0) as f32;
                let dy = (pos2.1 - pos1.1) as f32;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < SAFE_DISTANCE {
                    // Sort IDs to ensure consistent pair ordering
                    let pair = if id1 < id2 { (id1, id2) } else { (id2, id1) };

                    // Only count each unique pair once
                    if self.close_call_pairs.insert(pair) {
                        self.total_close_calls += 1;
                    }
                }
            }
        }
    }

    pub fn set_end_time(&mut self) {
        let now = Instant::now();
        self.end_time = Some((now - self.simulation_start).as_secs_f32());
    }

    pub fn get_duration(&self) -> f32 {
        let now = Instant::now();
        let new = Some((now - self.simulation_start).as_secs_f32());
        self.end_time.unwrap_or_else(|| new.unwrap_or(0.0))
    }

    pub fn get_summary(&self) -> StatisticsSummary {
        StatisticsSummary {
            total_vehicles: self.total_vehicles,
            max_velocity: self.max_velocity,
            min_velocity: self.min_velocity,
            max_intersection_time: self.max_intersection_time,
            min_intersection_time: self.min_intersection_time,
            total_close_calls: self.total_close_calls,
            duration: self.get_duration(),
            max_vehicles_in_intersection: self.max_vehicles_in_intersection,
        }
    }
}

pub struct StatisticsSummary {
    pub total_vehicles: u32,
    pub max_velocity: f32,
    pub min_velocity: f32,
    pub max_intersection_time: f32,
    pub min_intersection_time: f32,
    pub total_close_calls: u32,
    pub duration: f32,
    pub max_vehicles_in_intersection: u32,
}
