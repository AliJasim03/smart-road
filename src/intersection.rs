// src/intersection.rs - SIMPLIFIED VERSION
pub struct Intersection {
    pub center_x: i32,
    pub center_y: i32,
    pub size: i32,
}

impl Intersection {
    pub fn new() -> Self {
        Intersection {
            center_x: crate::WINDOW_WIDTH as i32 / 2,
            center_y: crate::WINDOW_HEIGHT as i32 / 2,
            size: 180, // Size of intersection area
        }
    }

    pub fn is_point_in_intersection(&self, x: i32, y: i32) -> bool {
        let dx = (x - self.center_x).abs();
        let dy = (y - self.center_y).abs();
        dx < self.size / 2 && dy < self.size / 2
    }

    pub fn distance_to_center(&self, x: i32, y: i32) -> f64 {
        let dx = x - self.center_x;
        let dy = y - self.center_y;
        ((dx * dx + dy * dy) as f64).sqrt()
    }
}