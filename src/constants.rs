use crate::geometry::Position;

pub const WINDOW_SIZE: u32 = 800;
//changed this to try and accomedate the 6 lanes
pub const LINE_SPACING: i32 = (WINDOW_SIZE / 16) as i32;
pub const VEHICLE_SIZE: u32 = LINE_SPACING as u32;
pub const FRAME_DURATION: std::time::Duration = std::time::Duration::from_millis(1000 / 60);
pub const VEHICLE_SPAWN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(700);
pub const SPAWN_COOLDOWN: std::time::Duration = std::time::Duration::from_millis(700);

// Define intersection bounds
pub const INTERSECTION_TOP_LEFT: Position = Position {
    x: 5 * LINE_SPACING,
    y: 5 * LINE_SPACING,
};
pub const INTERSECTION_BOTTOM_RIGHT: Position = Position {
    x: 11 * LINE_SPACING,
    y: 11 * LINE_SPACING,
};