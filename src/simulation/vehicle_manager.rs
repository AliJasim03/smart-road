use crate::constants::*;
use crate::core::Vehicle;
use crate::direction::Direction;
use crate::geometry::position::Position;
use crate::simulation::statistics::Statistics;
use std::collections::HashMap;
use std::time::Instant;

pub struct VehicleManager {
    vehicles: Vec<Vehicle>,
    last_spawn_time: HashMap<Direction, Instant>,
    statistics: Statistics,
}

impl VehicleManager {
    pub fn new() -> Self {
        Self {
            vehicles: Vec::new(),
            last_spawn_time: HashMap::new(),
            statistics: Statistics::new(),
        }
    }

    pub fn get_statistics(&self) -> &Statistics {
        &self.statistics
    }

    pub fn try_spawn_vehicle(&mut self, direction: Direction) {
        let now = Instant::now();
        let can_spawn = match self.last_spawn_time.get(&direction) {
            Some(last_time) => now.duration_since(*last_time) >= SPAWN_COOLDOWN,
            None => true,
        };

        if can_spawn {
            let vehicle_id = self.statistics.add_vehicle(direction);
            self.spawn_vehicle(direction, vehicle_id);
            self.last_spawn_time.insert(direction, now);
        }
    }

    pub fn spawn_vehicle(&mut self, initial_position: Direction, vehicle_id: usize) {
        let target_direction = Direction::new(Some(initial_position));

        let vehicle = Vehicle::new(
            initial_position,
            target_direction,
            VEHICLE_SIZE,
            &self.vehicles,
            vehicle_id,
        );

        self.vehicles.push(vehicle);
    }

    pub fn update_vehicles(&mut self) {
        let positions: Vec<(usize, (i32, i32))> = self
            .vehicles
            .iter()
            .map(|v| (v.id, (v.rect.x(), v.rect.y())))
            .collect();

        self.statistics.check_close_calls(&positions);

        let mut to_remove = Vec::new();
        for (idx, vehicle) in self.vehicles.iter_mut().enumerate() {
            let old_pos = (vehicle.rect.x(), vehicle.rect.y());

            vehicle.update_position();
            let new_pos = Position {
                x: vehicle.rect.x(),
                y: vehicle.rect.y(),
            };

            let dx = (new_pos.x - old_pos.0) as f32;
            let dy = (new_pos.y - old_pos.1) as f32;
            let velocity = (dx * dx + dy * dy).sqrt();

            self.statistics
                .update_vehicle_stats(vehicle.id, new_pos, velocity);

            if !vehicle.is_in_bounds(WINDOW_SIZE) {
                to_remove.push(idx);
                self.statistics.record_vehicle_exit(vehicle.id);
            }
        }

        for &idx in to_remove.iter().rev() {
            self.vehicles.remove(idx);
        }
    }

    pub fn get_vehicles(&self) -> &Vec<Vehicle> {
        &self.vehicles
    }

    pub fn set_end_time(&mut self) {
        self.statistics.set_end_time();
    }
}
