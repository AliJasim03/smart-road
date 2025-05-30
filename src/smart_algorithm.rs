// src/smart_algorithm
use std::collections::{HashMap, HashSet, VecDeque};
use crate::vehicle::{Vehicle, Direction, Route, VehicleState};
use crate::intersection::Intersection;

const GRID_SIZE: i32 = 32; // 32x32 pixel calculation units
const INTERSECTION_APPROACH_DISTANCE: i32 = 160; // 5 grid units before intersection
const SAFE_FOLLOWING_DISTANCE: i32 = 64; // 2 grid units

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridCoord {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntersectionZone {
    Approach,     // Approaching intersection
    Entry,        // Entering intersection
    Core,         // In the core intersection area
    Exit,         // Exiting intersection
    Clear,        // Clear of intersection
}

#[derive(Debug, Clone)]
pub struct ReservationRequest {
    pub vehicle_id: u32,
    pub path_coords: Vec<GridCoord>,
    pub entry_time: f64,
    pub exit_time: f64,
    pub priority: u32,
}

pub struct SmartIntersectionManager {
    // Grid-based collision detection
    grid_width: i32,
    grid_height: i32,
    intersection_center: GridCoord,
    intersection_radius: i32,

    // Reservation system
    reserved_coords: HashMap<GridCoord, (u32, f64)>, // coord -> (vehicle_id, until_time)
    pending_requests: VecDeque<ReservationRequest>,

    // Traffic flow management
    direction_priority: [f64; 4], // [North, South, East, West] priority values
    last_direction_served: Option<Direction>,
    flow_timer: f64,

    // Performance tracking
    vehicles_processed: u32,
    total_wait_time: f64,
    current_time: f64,
}

impl SmartIntersectionManager {
    pub fn new(window_width: u32, window_height: u32) -> Self {
        let grid_width = (window_width as i32) / GRID_SIZE;
        let grid_height = (window_height as i32) / GRID_SIZE;
        let intersection_center = GridCoord {
            x: grid_width / 2,
            y: grid_height / 2,
        };

        SmartIntersectionManager {
            grid_width,
            grid_height,
            intersection_center,
            intersection_radius: 6, // 6 grid units radius
            reserved_coords: HashMap::new(),
            pending_requests: VecDeque::new(),
            direction_priority: [1.0; 4],
            last_direction_served: None,
            flow_timer: 0.0,
            vehicles_processed: 0,
            total_wait_time: 0.0,
            current_time: 0.0,
        }
    }

    pub fn update(&mut self, vehicles: &mut VecDeque<Vehicle>, delta_time: f32) {
        self.current_time += delta_time as f64;
        self.flow_timer += delta_time as f64;

        // Clean up expired reservations
        self.cleanup_expired_reservations();

        // Process vehicle movements and requests
        for vehicle in vehicles.iter_mut() {
            self.process_vehicle(vehicle, delta_time);
        }

        // Process pending reservation requests
        self.process_reservation_requests();

        // Update traffic flow priorities
        self.update_flow_priorities();
    }

    fn process_vehicle(&mut self, vehicle: &mut Vehicle, delta_time: f32) {
        let vehicle_coord = self.pixel_to_grid(vehicle.position.x, vehicle.position.y);
        let zone = self.get_intersection_zone(vehicle_coord, &vehicle.direction);

        match zone {
            IntersectionZone::Approach => {
                // Request reservation for intersection passage
                if !self.has_reservation(vehicle.id) {
                    self.request_intersection_passage(vehicle);
                }

                // Check if we can proceed
                if self.can_proceed_to_intersection(vehicle) {
                    vehicle.set_target_velocity(crate::vehicle::VelocityLevel::Medium);
                } else {
                    vehicle.set_target_velocity(crate::vehicle::VelocityLevel::Slow);
                }
            }

            IntersectionZone::Entry => {
                // Entering intersection - proceed with caution
                if self.has_valid_reservation(vehicle.id, vehicle_coord) {
                    vehicle.set_target_velocity(crate::vehicle::VelocityLevel::Medium);
                    vehicle.state = VehicleState::Entering;
                } else {
                    // Stop if no valid reservation
                    vehicle.set_target_velocity(crate::vehicle::VelocityLevel::Slow);
                }
            }

            IntersectionZone::Core => {
                // In intersection core - maintain speed but be ready to adjust
                vehicle.state = VehicleState::Turning;

                if self.is_intersection_clear_ahead(vehicle) {
                    vehicle.set_target_velocity(crate::vehicle::VelocityLevel::Medium);
                } else {
                    vehicle.set_target_velocity(crate::vehicle::VelocityLevel::Slow);
                }
            }

            IntersectionZone::Exit => {
                // Exiting intersection
                vehicle.state = VehicleState::Exiting;
                vehicle.set_target_velocity(crate::vehicle::VelocityLevel::Medium);

                // Clear our reservations
                self.clear_vehicle_reservations(vehicle.id);
            }

            IntersectionZone::Clear => {
                // Clear of intersection
                if vehicle.state != VehicleState::Completed {
                    vehicle.state = VehicleState::Completed;
                    self.vehicles_processed += 1;
                }
            }
        }

        // Additional safety: maintain following distance
        self.maintain_following_distance(vehicle);
    }

    fn get_intersection_zone(&self, coord: GridCoord, direction: &Direction) -> IntersectionZone {
        let distance_to_center = self.distance_to_intersection_center(coord);

        if distance_to_center > self.intersection_radius + 5 {
            // Check if approaching
            let approach_distance = match direction {
                Direction::North => (self.intersection_center.y + self.intersection_radius) - coord.y,
                Direction::South => coord.y - (self.intersection_center.y - self.intersection_radius),
                Direction::East => coord.x - (self.intersection_center.x - self.intersection_radius),
                Direction::West => (self.intersection_center.x + self.intersection_radius) - coord.x,
            };

            if approach_distance > 0 && approach_distance <= 8 {
                IntersectionZone::Approach
            } else {
                IntersectionZone::Clear
            }
        } else if distance_to_center > self.intersection_radius + 2 {
            IntersectionZone::Entry
        } else if distance_to_center > self.intersection_radius - 2 {
            IntersectionZone::Core
        } else if distance_to_center > self.intersection_radius - 5 {
            IntersectionZone::Exit
        } else {
            IntersectionZone::Clear
        }
    }

    fn request_intersection_passage(&mut self, vehicle: &Vehicle) {
        let path_coords = self.calculate_vehicle_path(vehicle);
        let travel_time = self.estimate_travel_time(&path_coords, vehicle.current_velocity);

        let request = ReservationRequest {
            vehicle_id: vehicle.id,
            path_coords,
            entry_time: self.current_time + 1.0, // 1 second from now
            exit_time: self.current_time + 1.0 + travel_time,
            priority: self.calculate_priority(vehicle),
        };

        self.pending_requests.push_back(request);
    }

    fn calculate_vehicle_path(&self, vehicle: &Vehicle) -> Vec<GridCoord> {
        let mut path = Vec::new();
        let start_coord = self.pixel_to_grid(vehicle.position.x, vehicle.position.y);

        // Calculate path based on direction and route
        match (vehicle.direction, vehicle.route) {
            // Straight paths
            (Direction::North, Route::Straight) => {
                for y in (self.intersection_center.y - self.intersection_radius)..=(self.intersection_center.y + self.intersection_radius) {
                    let lane_x = self.get_lane_x_for_direction(Direction::North, vehicle.lane);
                    path.push(GridCoord { x: lane_x, y });
                }
            }
            (Direction::South, Route::Straight) => {
                for y in (self.intersection_center.y - self.intersection_radius)..=(self.intersection_center.y + self.intersection_radius) {
                    let lane_x = self.get_lane_x_for_direction(Direction::South, vehicle.lane);
                    path.push(GridCoord { x: lane_x, y });
                }
            }
            (Direction::East, Route::Straight) => {
                for x in (self.intersection_center.x - self.intersection_radius)..=(self.intersection_center.x + self.intersection_radius) {
                    let lane_y = self.get_lane_y_for_direction(Direction::East, vehicle.lane);
                    path.push(GridCoord { x, y: lane_y });
                }
            }
            (Direction::West, Route::Straight) => {
                for x in (self.intersection_center.x - self.intersection_radius)..=(self.intersection_center.x + self.intersection_radius) {
                    let lane_y = self.get_lane_y_for_direction(Direction::West, vehicle.lane);
                    path.push(GridCoord { x, y: lane_y });
                }
            }

            // Turning paths (simplified - you can make these more sophisticated)
            (Direction::North, Route::Right) => {
                // North to East turn
                let start_x = self.get_lane_x_for_direction(Direction::North, vehicle.lane);
                let end_y = self.get_lane_y_for_direction(Direction::East, vehicle.lane);

                // Create turning path
                for y in (self.intersection_center.y - 2)..=(self.intersection_center.y + self.intersection_radius) {
                    path.push(GridCoord { x: start_x, y });
                }
                for x in start_x..=(self.intersection_center.x + self.intersection_radius) {
                    path.push(GridCoord { x, y: end_y });
                }
            }

            // Add other turning combinations...
            _ => {
                // Fallback: just reserve center area
                path.push(self.intersection_center);
            }
        }

        path
    }

    fn get_lane_x_for_direction(&self, direction: Direction, lane: usize) -> i32 {
        match direction {
            Direction::North => self.intersection_center.x - 3 + (lane as i32),
            Direction::South => self.intersection_center.x + 3 - (lane as i32),
            _ => self.intersection_center.x,
        }
    }

    fn get_lane_y_for_direction(&self, direction: Direction, lane: usize) -> i32 {
        match direction {
            Direction::East => self.intersection_center.y - 3 + (lane as i32),
            Direction::West => self.intersection_center.y + 3 - (lane as i32),
            _ => self.intersection_center.y,
        }
    }

    fn process_reservation_requests(&mut self) {
        let mut approved_requests = Vec::new();

        // Sort requests by priority
        let mut requests: Vec<_> = self.pending_requests.drain(..).collect();
        requests.sort_by(|a, b| b.priority.cmp(&a.priority));

        for request in requests {
            if self.can_approve_request(&request) {
                // Reserve the path
                for coord in &request.path_coords {
                    self.reserved_coords.insert(*coord, (request.vehicle_id, request.exit_time));
                }
                approved_requests.push(request);
            } else {
                // Re-queue the request with updated timing
                let mut updated_request = request;
                updated_request.entry_time = self.current_time + 0.5;
                updated_request.exit_time = updated_request.entry_time +
                    self.estimate_travel_time(&updated_request.path_coords, 100.0);
                self.pending_requests.push_back(updated_request);
            }
        }

        println!("Approved {} reservation requests", approved_requests.len());
    }

    fn can_approve_request(&self, request: &ReservationRequest) -> bool {
        // Check if any coordinate in the path is already reserved during our time window
        for coord in &request.path_coords {
            if let Some((other_vehicle, until_time)) = self.reserved_coords.get(coord) {
                if *other_vehicle != request.vehicle_id && *until_time > request.entry_time {
                    return false;
                }
            }
        }
        true
    }

    fn can_proceed_to_intersection(&self, vehicle: &Vehicle) -> bool {
        let vehicle_coord = self.pixel_to_grid(vehicle.position.x, vehicle.position.y);

        // Check if we have a reservation for the next few coordinates
        let next_coords = self.get_next_coordinates(vehicle, 3);
        for coord in next_coords {
            if let Some((reserved_id, _)) = self.reserved_coords.get(&coord) {
                if *reserved_id != vehicle.id {
                    return false;
                }
            }
        }
        true
    }

    fn get_next_coordinates(&self, vehicle: &Vehicle, count: usize) -> Vec<GridCoord> {
        let mut coords = Vec::new();
        let current = self.pixel_to_grid(vehicle.position.x, vehicle.position.y);

        for i in 1..=count {
            let next_coord = match vehicle.direction {
                Direction::North => GridCoord { x: current.x, y: current.y - i as i32 },
                Direction::South => GridCoord { x: current.x, y: current.y + i as i32 },
                Direction::East => GridCoord { x: current.x + i as i32, y: current.y },
                Direction::West => GridCoord { x: current.x - i as i32, y: current.y },
            };
            coords.push(next_coord);
        }
        coords
    }

    fn maintain_following_distance(&self, vehicle: &mut Vehicle) {
        // Check if there's a vehicle too close ahead
        let ahead_coords = self.get_next_coordinates(vehicle, 2);

        for coord in ahead_coords {
            if let Some((other_id, _)) = self.reserved_coords.get(&coord) {
                if *other_id != vehicle.id {
                    // Another vehicle is too close ahead, slow down
                    vehicle.set_target_velocity(crate::vehicle::VelocityLevel::Slow);
                    return;
                }
            }
        }
    }

    fn calculate_priority(&self, vehicle: &Vehicle) -> u32 {
        let mut priority = 100;

        // Higher priority for vehicles that have been waiting longer
        let wait_time = vehicle.start_time.elapsed().as_secs();
        priority += (wait_time * 10) as u32;

        // Direction-based priority
        let direction_index = match vehicle.direction {
            Direction::North => 0,
            Direction::South => 1,
            Direction::East => 2,
            Direction::West => 3,
        };
        priority += (self.direction_priority[direction_index] * 50.0) as u32;

        // Route-based priority (straight has higher priority)
        match vehicle.route {
            Route::Straight => priority += 30,
            Route::Right => priority += 20,
            Route::Left => priority += 10,
        }

        priority
    }

    fn update_flow_priorities(&mut self) {
        // Increase priority for directions that haven't been served recently
        for i in 0..4 {
            self.direction_priority[i] += 0.1;
        }

        // Reset priority for the last served direction
        if let Some(last_dir) = self.last_direction_served {
            let index = match last_dir {
                Direction::North => 0,
                Direction::South => 1,
                Direction::East => 2,
                Direction::West => 3,
            };
            self.direction_priority[index] = 1.0;
        }

        // Cap maximum priority
        for priority in &mut self.direction_priority {
            *priority = priority.min(5.0);
        }
    }

    // Utility functions
    fn pixel_to_grid(&self, x: i32, y: i32) -> GridCoord {
        GridCoord {
            x: x / GRID_SIZE,
            y: y / GRID_SIZE,
        }
    }

    fn distance_to_intersection_center(&self, coord: GridCoord) -> i32 {
        let dx = coord.x - self.intersection_center.x;
        let dy = coord.y - self.intersection_center.y;
        ((dx * dx + dy * dy) as f64).sqrt() as i32
    }

    fn has_reservation(&self, vehicle_id: u32) -> bool {
        self.reserved_coords.values().any(|(id, _)| *id == vehicle_id)
    }

    fn has_valid_reservation(&self, vehicle_id: u32, coord: GridCoord) -> bool {
        if let Some((id, until_time)) = self.reserved_coords.get(&coord) {
            *id == vehicle_id && *until_time > self.current_time
        } else {
            false
        }
    }

    fn clear_vehicle_reservations(&mut self, vehicle_id: u32) {
        self.reserved_coords.retain(|_, (id, _)| *id != vehicle_id);
    }

    fn cleanup_expired_reservations(&mut self) {
        self.reserved_coords.retain(|_, (_, until_time)| *until_time > self.current_time);
    }

    fn is_intersection_clear_ahead(&self, vehicle: &Vehicle) -> bool {
        let ahead_coords = self.get_next_coordinates(vehicle, 1);
        for coord in ahead_coords {
            if let Some((other_id, _)) = self.reserved_coords.get(&coord) {
                if *other_id != vehicle.id {
                    return false;
                }
            }
        }
        true
    }

    fn estimate_travel_time(&self, path: &[GridCoord], velocity: f64) -> f64 {
        if velocity <= 0.0 { return 10.0; } // Fallback

        let distance = (path.len() as f64) * (GRID_SIZE as f64);
        distance / velocity // time = distance / speed
    }

    pub fn get_statistics(&self) -> (u32, f64, usize, usize) {
        let avg_wait_time = if self.vehicles_processed > 0 {
            self.total_wait_time / self.vehicles_processed as f64
        } else {
            0.0
        };

        (
            self.vehicles_processed,
            avg_wait_time,
            self.reserved_coords.len(),
            self.pending_requests.len(),
        )
    }
}