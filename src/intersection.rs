use sdl2::rect::Rect;

// Define dimensions for the intersection
pub const ROAD_WIDTH: u32 = 180; // Increased for 6 lanes (30 pixels per lane)
pub const INTERSECTION_SIZE: u32 = ROAD_WIDTH * 2;
pub const LANE_WIDTH: u32 = 30; // Width of a single lane

// Define the intersection area as a function
pub fn intersection_area() -> Rect {
    Rect::new(
        (crate::WINDOW_WIDTH / 2 - INTERSECTION_SIZE / 2) as i32,
        (crate::WINDOW_HEIGHT / 2 - INTERSECTION_SIZE / 2) as i32,
        INTERSECTION_SIZE,
        INTERSECTION_SIZE,
    )
}

// Define the center of the intersection as a function
pub fn intersection_center() -> (i32, i32) {
    let area = intersection_area();
    (
        area.x() + (INTERSECTION_SIZE / 2) as i32,
        area.y() + (INTERSECTION_SIZE / 2) as i32,
    )
}

// Lane positions for all 6 lanes in each direction
pub fn north_lanes() -> Vec<i32> {
    let center = intersection_center().0;
    let first_lane = center - (ROAD_WIDTH as i32 / 2) + (LANE_WIDTH as i32 / 2);
    (0..6).map(|i| first_lane + i * LANE_WIDTH as i32).collect()
}

pub fn south_lanes() -> Vec<i32> {
    let center = intersection_center().0;
    let first_lane = center + (ROAD_WIDTH as i32 / 2) - (LANE_WIDTH as i32 / 2);
    (0..6).map(|i| first_lane - i * LANE_WIDTH as i32).collect()
}

pub fn east_lanes() -> Vec<i32> {
    let center = intersection_center().1;
    let first_lane = center - (ROAD_WIDTH as i32 / 2) + (LANE_WIDTH as i32 / 2);
    (0..6).map(|i| first_lane + i * LANE_WIDTH as i32).collect()
}

pub fn west_lanes() -> Vec<i32> {
    let center = intersection_center().1;
    let first_lane = center + (ROAD_WIDTH as i32 / 2) - (LANE_WIDTH as i32 / 2);
    (0..6).map(|i| first_lane - i * LANE_WIDTH as i32).collect()
}

// Define routes for each lane
pub enum LaneRoute {
    Left,     // Left turn only lane
    LeftStraight, // Left turn or straight
    Straight, // Straight only lane
    StraightRight, // Straight or right turn
    Right,    // Right turn only lane
    Any,      // Any direction
}

// Get the route options for a specific lane
pub fn lane_route(lane_index: usize) -> LaneRoute {
    match lane_index {
        0 => LaneRoute::Left,
        1 => LaneRoute::LeftStraight,
        2 => LaneRoute::Straight,
        3 => LaneRoute::Straight,
        4 => LaneRoute::StraightRight,
        5 => LaneRoute::Right,
        _ => LaneRoute::Any,
    }
}

// Define the paths for each route through the intersection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Path {
    NorthToEast, // North incoming, turning right to East
    NorthToSouth, // North incoming, going straight to South
    NorthToWest, // North incoming, turning left to West
    SouthToWest, // South incoming, turning right to West
    SouthToNorth, // South incoming, going straight to North
    SouthToEast, // South incoming, turning left to East
    EastToSouth, // East incoming, turning right to South
    EastToWest, // East incoming, going straight to West
    EastToNorth, // East incoming, turning left to North
    WestToNorth, // West incoming, turning right to North
    WestToEast, // West incoming, going straight to East
    WestToSouth, // West incoming, turning left to South
}

pub struct Intersection {
    // Boundaries for determining when vehicles enter/exit the intersection
    pub north_entry: i32,
    pub south_entry: i32,
    pub east_entry: i32,
    pub west_entry: i32,
    pub north_exit: i32,
    pub south_exit: i32,
    pub east_exit: i32,
    pub west_exit: i32,
}

impl Intersection {
    pub fn new() -> Self {
        let boundary_offset = 200; // Distance from intersection center to entry/exit points
        let center = intersection_center();

        Intersection {
            north_entry: center.1 - boundary_offset,
            south_entry: center.1 + boundary_offset,
            east_entry: center.0 + boundary_offset,
            west_entry: center.0 - boundary_offset,
            north_exit: 0, // Top of screen
            south_exit: crate::WINDOW_HEIGHT as i32, // Bottom of screen
            east_exit: crate::WINDOW_WIDTH as i32, // Right of screen
            west_exit: 0, // Left of screen
        }
    }

    // Check if two paths can potentially cause a collision
    pub fn paths_could_collide(&self, path1: &Path, path2: &Path) -> bool {
        use Path::*;

        match (path1, path2) {
            // Straight paths crossing each other
            (NorthToSouth, EastToWest) | (EastToWest, NorthToSouth) => true,
            (NorthToSouth, WestToEast) | (WestToEast, NorthToSouth) => true,
            (SouthToNorth, EastToWest) | (EastToWest, SouthToNorth) => true,
            (SouthToNorth, WestToEast) | (WestToEast, SouthToNorth) => true,

            // Left turns crossing straight paths
            (NorthToWest, SouthToNorth) | (SouthToNorth, NorthToWest) => true,
            (SouthToEast, NorthToSouth) | (NorthToSouth, SouthToEast) => true,
            (EastToNorth, WestToEast) | (WestToEast, EastToNorth) => true,
            (WestToSouth, EastToWest) | (EastToWest, WestToSouth) => true,

            // Left turns crossing each other
            (NorthToWest, SouthToEast) | (SouthToEast, NorthToWest) => true,
            (EastToNorth, WestToSouth) | (WestToSouth, EastToNorth) => true,

            // Left turns crossing right turns
            (NorthToWest, EastToSouth) | (EastToSouth, NorthToWest) => true,
            (SouthToEast, WestToNorth) | (WestToNorth, SouthToEast) => true,
            (EastToNorth, SouthToWest) | (SouthToWest, EastToNorth) => true,
            (WestToSouth, NorthToEast) | (NorthToEast, WestToSouth) => true,

            // Right turns crossing straight paths
            (NorthToEast, WestToEast) | (WestToEast, NorthToEast) => true,
            (SouthToWest, EastToWest) | (EastToWest, SouthToWest) => true,
            (EastToSouth, NorthToSouth) | (NorthToSouth, EastToSouth) => true,
            (WestToNorth, SouthToNorth) | (SouthToNorth, WestToNorth) => true,

            // Same direction vehicles (shouldn't collide in normal circumstances)
            (NorthToEast, NorthToSouth) | (NorthToSouth, NorthToEast) => false,
            (NorthToEast, NorthToWest) | (NorthToWest, NorthToEast) => false,
            (NorthToSouth, NorthToWest) | (NorthToWest, NorthToSouth) => false,

            // Similar analysis for other same-direction combinations
            (SouthToWest, SouthToNorth) | (SouthToNorth, SouthToWest) => false,
            (SouthToWest, SouthToEast) | (SouthToEast, SouthToWest) => false,
            (SouthToNorth, SouthToEast) | (SouthToEast, SouthToNorth) => false,

            (EastToSouth, EastToWest) | (EastToWest, EastToSouth) => false,
            (EastToSouth, EastToNorth) | (EastToNorth, EastToSouth) => false,
            (EastToWest, EastToNorth) | (EastToNorth, EastToWest) => false,

            (WestToNorth, WestToEast) | (WestToEast, WestToNorth) => false,
            (WestToNorth, WestToSouth) | (WestToSouth, WestToNorth) => false,
            (WestToEast, WestToSouth) | (WestToSouth, WestToEast) => false,

            // Parallel paths (shouldn't collide)
            (NorthToSouth, SouthToNorth) | (SouthToNorth, NorthToSouth) => false,
            (EastToWest, WestToEast) | (WestToEast, EastToWest) => false,

            // Right turns that don't intersect
            (NorthToEast, SouthToWest) | (SouthToWest, NorthToEast) => false,
            (EastToSouth, WestToNorth) | (WestToNorth, EastToSouth) => false,

            // Other combinations that don't conflict
            _ => false,
        }
    }

    // Get the path for a given direction, lane and route
    pub fn get_path(&self, direction: &crate::vehicle::Direction, route: &crate::vehicle::Route) -> Path {
        use crate::vehicle::{Direction, Route};

        match (direction, route) {
            (Direction::North, Route::Right) => Path::NorthToEast,
            (Direction::North, Route::Straight) => Path::NorthToSouth,
            (Direction::North, Route::Left) => Path::NorthToWest,
            (Direction::South, Route::Right) => Path::SouthToWest,
            (Direction::South, Route::Straight) => Path::SouthToNorth,
            (Direction::South, Route::Left) => Path::SouthToEast,
            (Direction::East, Route::Right) => Path::EastToSouth,
            (Direction::East, Route::Straight) => Path::EastToWest,
            (Direction::East, Route::Left) => Path::EastToNorth,
            (Direction::West, Route::Right) => Path::WestToNorth,
            (Direction::West, Route::Straight) => Path::WestToEast,
            (Direction::West, Route::Left) => Path::WestToSouth,
        }
    }

    // Get collision points for a given path (useful for more precise collision detection)
    pub fn get_collision_points(&self, path: &Path) -> Vec<(i32, i32)> {
        let center = intersection_center();
        let mut points = Vec::new();

        match path {
            Path::NorthToSouth | Path::SouthToNorth => {
                // Vertical straight path through center
                for y in (center.1 - 100)..(center.1 + 100) {
                    points.push((center.0, y));
                }
            }
            Path::EastToWest | Path::WestToEast => {
                // Horizontal straight path through center
                for x in (center.0 - 100)..(center.0 + 100) {
                    points.push((x, center.1));
                }
            }
            Path::NorthToEast => {
                // Right turn from North to East
                // Add curve points for the turn
                let turn_radius = 50;
                for angle in 0..90 {
                    let rad = (angle as f32).to_radians();
                    let x = center.0 + (turn_radius as f32 * rad.cos()) as i32;
                    let y = center.1 - (turn_radius as f32 * rad.sin()) as i32;
                    points.push((x, y));
                }
            }
            Path::NorthToWest => {
                // Left turn from North to West
                let turn_radius = 80;
                for angle in 0..90 {
                    let rad = (angle as f32).to_radians();
                    let x = center.0 - (turn_radius as f32 * rad.cos()) as i32;
                    let y = center.1 - (turn_radius as f32 * rad.sin()) as i32;
                    points.push((x, y));
                }
            }
            // Add similar calculations for other turning paths...
            _ => {
                // Default: just add the center point
                points.push((center.0, center.1));
            }
        }

        points
    }

    // Check if a point is within the intersection area
    pub fn is_point_in_intersection(&self, x: i32, y: i32) -> bool {
        let area = intersection_area();
        x >= area.x() && x < area.x() + area.width() as i32 &&
            y >= area.y() && y < area.y() + area.height() as i32
    }

    // Get the entry distance for a vehicle approaching from a given direction
    pub fn get_entry_distance(&self, direction: &crate::vehicle::Direction, position: (i32, i32)) -> f64 {
        match direction {
            crate::vehicle::Direction::North => (position.1 - self.south_entry).max(0) as f64,
            crate::vehicle::Direction::South => (self.north_entry - position.1).max(0) as f64,
            crate::vehicle::Direction::East => (self.west_entry - position.0).max(0) as f64,
            crate::vehicle::Direction::West => (position.0 - self.east_entry).max(0) as f64,
        }
    }

    // Get the exit distance for a vehicle leaving in a given direction
    pub fn get_exit_distance(&self, direction: &crate::vehicle::Direction, position: (i32, i32)) -> f64 {
        match direction {
            crate::vehicle::Direction::North => (position.1 - self.north_exit).max(0) as f64,
            crate::vehicle::Direction::South => (self.south_exit - position.1).max(0) as f64,
            crate::vehicle::Direction::East => (self.east_exit - position.0).max(0) as f64,
            crate::vehicle::Direction::West => (position.0 - self.west_exit).max(0) as f64,
        }
    }
}