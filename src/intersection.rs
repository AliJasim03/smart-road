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
            // Straight paths crossing
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

            // Other combinations don't collide
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
}