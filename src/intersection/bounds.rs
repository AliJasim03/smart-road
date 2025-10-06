use crate::constants::*;
use crate::geometry::position::Position;

pub struct IntersectionBounds;

impl IntersectionBounds {
    pub fn is_position_in_intersection(position: &Position) -> bool {
        let rect_left = position.x;
        let rect_right = position.x + VEHICLE_SIZE as i32;
        let rect_top = position.y;
        let rect_bottom = position.y + VEHICLE_SIZE as i32;

        rect_left < INTERSECTION_BOTTOM_RIGHT.x
            && rect_right > INTERSECTION_TOP_LEFT.x
            && rect_top < INTERSECTION_BOTTOM_RIGHT.y
            && rect_bottom > INTERSECTION_TOP_LEFT.y
    }

    pub fn is_position_out_of_intersection(position: &Position) -> bool {
        if position.x <= 4 * LINE_SPACING && (5 * LINE_SPACING..=7 * LINE_SPACING).contains(&position.y) {
            return true;
        }
        if position.x >= 11 * LINE_SPACING && (8 * LINE_SPACING..=10 * LINE_SPACING).contains(&position.y) {
            return true;
        }
        if position.y <= 4 * LINE_SPACING && (8 * LINE_SPACING..=10 * LINE_SPACING).contains(&position.x) {
            return true;
        }
        if position.y >= 11 * LINE_SPACING && (5 * LINE_SPACING..=7 * LINE_SPACING).contains(&position.x) {
            return true;
        }

        false
    }
}
