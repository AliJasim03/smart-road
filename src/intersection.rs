// src/intersection.rs - GEOMETRY: Core logic is robust
use crate::vehicle::{Vehicle};

pub struct Intersection {
    pub center_x: f32,
    pub center_y: f32,
    pub size: f32,
    pub approach_distance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntersectionZone {
    Clear, Approach, Core,
}

impl Intersection {
    pub fn new() -> Self {
        Intersection {
            center_x: crate::WINDOW_WIDTH as f32 / 2.0,
            center_y: crate::WINDOW_HEIGHT as f32 / 2.0,
            size: crate::TOTAL_ROAD_WIDTH, // 180.0
            approach_distance: 120.0,
        }
    }

    // Checks if a point is within the main intersection square
    pub fn is_point_in_core(&self, x: f32, y: f32) -> bool {
        let half_size = self.size / 2.0;
        x > self.center_x - half_size && x < self.center_x + half_size &&
            y > self.center_y - half_size && y < self.center_y + half_size
    }

    // Checks if a point is in the "slow down" zone before the intersection
    pub fn is_point_in_approach_zone(&self, x: f32, y: f32) -> bool {
        let approach_size = self.size + self.approach_distance * 2.0;
        let half_approach_size = approach_size / 2.0;

        let is_in_outer_box = x > self.center_x - half_approach_size && x < self.center_x + half_approach_size &&
            y > self.center_y - half_approach_size && y < self.center_y + half_approach_size;

        is_in_outer_box && !self.is_point_in_core(x, y)
    }

    pub fn get_vehicle_zone(&self, vehicle: &Vehicle) -> IntersectionZone {
        if self.is_point_in_core(vehicle.position.x, vehicle.position.y) {
            IntersectionZone::Core
        } else if self.is_point_in_approach_zone(vehicle.position.x, vehicle.position.y) {
            IntersectionZone::Approach
        } else {
            IntersectionZone::Clear
        }
    }
}