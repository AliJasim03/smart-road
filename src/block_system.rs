// src/block_system.rs
use std::collections::HashMap;
use crate::vehicle::{Vehicle, Direction, Route};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockType {
    Road,
    Intersection,
    Outside,
}

pub struct BlockGrid {
    pub blocks: HashMap<BlockPosition, BlockType>,
    pub occupied_blocks: HashMap<BlockPosition, u32>, // Block position -> Vehicle ID
    pub block_size: u32,
    pub grid_width: i32,
    pub grid_height: i32,
    pub intersection_center: BlockPosition,
    pub intersection_size: i32, // Size of intersection in blocks
}

impl BlockGrid {
    pub fn new(window_width: u32, window_height: u32, block_size: u32) -> Self {
        let grid_width = (window_width / block_size) as i32;
        let grid_height = (window_height / block_size) as i32;
        let intersection_center = BlockPosition {
            x: grid_width / 2,
            y: grid_height / 2,
        };
        let intersection_size = 12; // 12x12 block intersection

        let mut grid = BlockGrid {
            blocks: HashMap::new(),
            occupied_blocks: HashMap::new(),
            block_size,
            grid_width,
            grid_height,
            intersection_center,
            intersection_size,
        };

        grid.initialize_blocks();
        grid
    }

    fn initialize_blocks(&mut self) {
        // Create road blocks
        let center = self.intersection_center;
        let half_size = self.intersection_size / 2;

        // Vertical road (north-south)
        for y in 0..self.grid_height {
            for x in (center.x - 3)..(center.x + 3) { // 6 lanes wide
                if x >= 0 && x < self.grid_width {
                    let pos = BlockPosition { x, y };
                    if self.is_in_intersection_area(pos) {
                        self.blocks.insert(pos, BlockType::Intersection);
                    } else {
                        self.blocks.insert(pos, BlockType::Road);
                    }
                }
            }
        }

        // Horizontal road (east-west)
        for x in 0..self.grid_width {
            for y in (center.y - 3)..(center.y + 3) { // 6 lanes wide
                if y >= 0 && y < self.grid_height {
                    let pos = BlockPosition { x, y };
                    if self.is_in_intersection_area(pos) {
                        self.blocks.insert(pos, BlockType::Intersection);
                    } else {
                        self.blocks.insert(pos, BlockType::Road);
                    }
                }
            }
        }
    }

    fn is_in_intersection_area(&self, pos: BlockPosition) -> bool {
        let center = self.intersection_center;
        let half_size = self.intersection_size / 2;
        pos.x >= center.x - half_size && pos.x < center.x + half_size &&
            pos.y >= center.y - half_size && pos.y < center.y + half_size
    }

    pub fn get_block_type(&self, pos: BlockPosition) -> BlockType {
        self.blocks.get(&pos).copied().unwrap_or(BlockType::Outside)
    }

    pub fn is_block_occupied(&self, pos: BlockPosition) -> bool {
        self.occupied_blocks.contains_key(&pos)
    }

    pub fn can_move_to(&self, pos: BlockPosition, vehicle_id: u32) -> bool {
        // Check if block exists and is not occupied by another vehicle
        match self.get_block_type(pos) {
            BlockType::Outside => false,
            _ => {
                if let Some(&occupying_id) = self.occupied_blocks.get(&pos) {
                    occupying_id == vehicle_id // Can stay in same block
                } else {
                    true // Block is free
                }
            }
        }
    }

    pub fn move_vehicle(&mut self, vehicle_id: u32, from: BlockPosition, to: BlockPosition) -> bool {
        if self.can_move_to(to, vehicle_id) {
            // Remove from old position
            self.occupied_blocks.remove(&from);
            // Add to new position
            self.occupied_blocks.insert(to, vehicle_id);
            true
        } else {
            false
        }
    }

    pub fn add_vehicle(&mut self, vehicle_id: u32, pos: BlockPosition) -> bool {
        if self.can_move_to(pos, vehicle_id) {
            self.occupied_blocks.insert(pos, vehicle_id);
            true
        } else {
            false
        }
    }

    pub fn remove_vehicle(&mut self, vehicle_id: u32, pos: BlockPosition) {
        if let Some(&id) = self.occupied_blocks.get(&pos) {
            if id == vehicle_id {
                self.occupied_blocks.remove(&pos);
            }
        }
    }

    // Convert pixel coordinates to block position
    pub fn pixel_to_block(&self, x: i32, y: i32) -> BlockPosition {
        BlockPosition {
            x: x / self.block_size as i32,
            y: y / self.block_size as i32,
        }
    }

    // Convert block position to pixel coordinates (center of block)
    pub fn block_to_pixel(&self, pos: BlockPosition) -> (i32, i32) {
        (
            pos.x * self.block_size as i32 + self.block_size as i32 / 2,
            pos.y * self.block_size as i32 + self.block_size as i32 / 2,
        )
    }

    // Get the lane index for a given block position
    pub fn get_lane_index(&self, pos: BlockPosition, direction: Direction) -> usize {
        let center = self.intersection_center;
        match direction {
            Direction::North | Direction::South => {
                let offset = pos.x - (center.x - 3);
                offset.max(0).min(5) as usize
            }
            Direction::East | Direction::West => {
                let offset = pos.y - (center.y - 3);
                offset.max(0).min(5) as usize
            }
        }
    }

    // Get valid next positions for a vehicle
    pub fn get_next_positions(&self, current: BlockPosition, direction: Direction, route: Route) -> Vec<BlockPosition> {
        let mut next_positions = Vec::new();

        // Basic movement in current direction
        let forward_pos = match direction {
            Direction::North => BlockPosition { x: current.x, y: current.y - 1 },
            Direction::South => BlockPosition { x: current.x, y: current.y + 1 },
            Direction::East => BlockPosition { x: current.x + 1, y: current.y },
            Direction::West => BlockPosition { x: current.x - 1, y: current.y },
        };

        // Check if we're at intersection and need to turn
        if self.get_block_type(current) == BlockType::Intersection {
            match route {
                Route::Straight => {
                    next_positions.push(forward_pos);
                }
                Route::Left => {
                    let left_pos = match direction {
                        Direction::North => BlockPosition { x: current.x - 1, y: current.y },
                        Direction::South => BlockPosition { x: current.x + 1, y: current.y },
                        Direction::East => BlockPosition { x: current.x, y: current.y - 1 },
                        Direction::West => BlockPosition { x: current.x, y: current.y + 1 },
                    };
                    next_positions.push(left_pos);
                }
                Route::Right => {
                    let right_pos = match direction {
                        Direction::North => BlockPosition { x: current.x + 1, y: current.y },
                        Direction::South => BlockPosition { x: current.x - 1, y: current.y },
                        Direction::East => BlockPosition { x: current.x, y: current.y + 1 },
                        Direction::West => BlockPosition { x: current.x, y: current.y - 1 },
                    };
                    next_positions.push(right_pos);
                }
            }
        } else {
            // Normal forward movement
            next_positions.push(forward_pos);
        }

        // Filter out invalid positions
        next_positions.into_iter()
            .filter(|&pos| matches!(self.get_block_type(pos), BlockType::Road | BlockType::Intersection))
            .collect()
    }
}

// Simplified vehicle for block system
#[derive(Debug, Clone)]
pub struct BlockVehicle {
    pub id: u32,
    pub block_position: BlockPosition,
    pub direction: Direction,
    pub route: Route,
    pub move_timer: f32, // Time until next move
    pub move_interval: f32, // How fast the vehicle moves (seconds per block)
    pub state: VehicleState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VehicleState {
    Moving,
    Waiting,
    Completed,
}

impl BlockVehicle {
    pub fn new(id: u32, start_pos: BlockPosition, direction: Direction, route: Route) -> Self {
        BlockVehicle {
            id,
            block_position: start_pos,
            direction,
            route,
            move_timer: 0.0,
            move_interval: 0.5, // Move every 0.5 seconds initially
            state: VehicleState::Moving,
        }
    }

    pub fn update(&mut self, delta_time: f32, grid: &mut BlockGrid) -> bool {
        if self.state == VehicleState::Completed {
            return false;
        }

        self.move_timer -= delta_time;

        if self.move_timer <= 0.0 {
            // Time to try moving
            let next_positions = grid.get_next_positions(self.block_position, self.direction, self.route);

            if let Some(&next_pos) = next_positions.first() {
                if grid.can_move_to(next_pos, self.id) {
                    // Move successful
                    if grid.move_vehicle(self.id, self.block_position, next_pos) {
                        self.block_position = next_pos;
                        self.move_timer = self.move_interval;
                        self.state = VehicleState::Moving;

                        // Update direction if turning
                        self.update_direction_after_move(grid);

                        // Check if vehicle has left the map
                        if self.is_off_map(grid) {
                            self.state = VehicleState::Completed;
                            grid.remove_vehicle(self.id, next_pos);
                            return false;
                        }
                        return true;
                    }
                }
            }

            // Couldn't move, wait a bit
            self.state = VehicleState::Waiting;
            self.move_timer = self.move_interval * 0.5; // Try again sooner when waiting
        }

        true
    }

    fn update_direction_after_move(&mut self, grid: &BlockGrid) {
        // Update direction when turning in intersection
        if grid.get_block_type(self.block_position) == BlockType::Intersection {
            match self.route {
                Route::Left => {
                    self.direction = match self.direction {
                        Direction::North => Direction::West,
                        Direction::South => Direction::East,
                        Direction::East => Direction::North,
                        Direction::West => Direction::South,
                    };
                }
                Route::Right => {
                    self.direction = match self.direction {
                        Direction::North => Direction::East,
                        Direction::South => Direction::West,
                        Direction::East => Direction::South,
                        Direction::West => Direction::North,
                    };
                }
                Route::Straight => {
                    // No direction change
                }
            }
        }
    }

    fn is_off_map(&self, grid: &BlockGrid) -> bool {
        self.block_position.x < 0 ||
            self.block_position.x >= grid.grid_width ||
            self.block_position.y < 0 ||
            self.block_position.y >= grid.grid_height
    }

    // Adjust speed based on traffic conditions
    pub fn adjust_speed(&mut self, grid: &BlockGrid) {
        let next_positions = grid.get_next_positions(self.block_position, self.direction, self.route);

        // Check how many blocks ahead are occupied
        let mut blocked_count = 0;
        for i in 1..=3 { // Check 3 blocks ahead
            if let Some(&next_pos) = next_positions.first() {
                let ahead_pos = match self.direction {
                    Direction::North => BlockPosition { x: next_pos.x, y: next_pos.y - i },
                    Direction::South => BlockPosition { x: next_pos.x, y: next_pos.y + i },
                    Direction::East => BlockPosition { x: next_pos.x + i, y: next_pos.y },
                    Direction::West => BlockPosition { x: next_pos.x - i, y: next_pos.y },
                };

                if grid.is_block_occupied(ahead_pos) {
                    blocked_count += 1;
                }
            }
        }

        // Adjust speed based on congestion
        self.move_interval = match blocked_count {
            0 => 0.3,      // Fast when clear
            1 => 0.5,      // Medium when some traffic
            _ => 0.8,      // Slow when congested
        };
    }
}