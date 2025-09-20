use crate::constants::*;
use crate::direction::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn move_in_direction(&self, direction: &Direction, speed: i32) -> Position {
        let mut new_position = *self;
        match direction {
            Direction::Down => new_position.y += speed,
            Direction::Up => new_position.y -= speed,
            Direction::Left => new_position.x -= speed,
            Direction::Right => new_position.x += speed,
        }
        new_position
    }
    pub fn is_after_turn(&self, turn_position: &(Option<i32>, Option<i32>)) -> bool {
        // when it is before turning position it can ba grater and it can be less
        // so if it is does not equal then it is before
        if let Some(turn_x) = turn_position.0 {
            if self.x == turn_x {
                return true;
            }
        }
        if let Some(turn_y) = turn_position.1 {
            if self.y == turn_y {
                return true;
            }
        }
        false
    }

    pub fn is_in_intersection(&self) -> bool {
        let rect_left = self.x;
        let rect_right = self.x + VEHICLE_SIZE as i32;
        let rect_top = self.y;
        let rect_bottom = self.y + VEHICLE_SIZE as i32;

        // Check if the rectangle intersects the square
        rect_left < INTERSECTION_BOTTOM_RIGHT.x
            && rect_right > INTERSECTION_TOP_LEFT.x
            && rect_top < INTERSECTION_BOTTOM_RIGHT.y
            && rect_bottom > INTERSECTION_TOP_LEFT.y
    }
    pub fn calculate_steps_to(&self, new_position: &Position) -> u64 {
        ((self.x - new_position.x).abs() + (self.y - new_position.y).abs()) as u64
    }

    pub fn is_out_of_intersection(&self) -> bool {
        // x-axis less than 4 and y-axis between 5 and 7 or equal them
        if self.x <= 4 * LINE_SPACING && (5 * LINE_SPACING..=7 * LINE_SPACING).contains(&self.y) {
            return true;
        }
        // x-axis greater than 11 and y-axis between 8 and 10
        if self.x >= 11 * LINE_SPACING && (8 * LINE_SPACING..=10 * LINE_SPACING).contains(&self.y) {
            return true;
        }
        // y-axis less than 4 and x-axis between 8 and 10
        if self.y <= 4 * LINE_SPACING && (8 * LINE_SPACING..=10 * LINE_SPACING).contains(&self.x) {
            return true;
        }
        // y-axis greater than 11 and x-axis between 5 and 7
        if self.y >= 11 * LINE_SPACING && (5 * LINE_SPACING..=7 * LINE_SPACING).contains(&self.x) {
            return true;
        }

        false
    }
}

// TODO make it a map and save it from the beginning to get the positions without recalculating
pub fn get_spawn_position(initial_position: Direction, target_direction: Direction) -> Position {
    match initial_position {
        Direction::Up => {
            let lane = match target_direction {
                Direction::Right => 7 * LINE_SPACING,
                Direction::Down => 6 * LINE_SPACING,
                Direction::Left => 5 * LINE_SPACING,
                _ => panic!("Invalid target direction"),
            };
            Position {
                x: lane,
                y: -LINE_SPACING,
            }
        }
        Direction::Left => {
            let lane = match target_direction {
                Direction::Right => 9 * LINE_SPACING,
                Direction::Up => 8 * LINE_SPACING,
                Direction::Down => 10 * LINE_SPACING,
                _ => panic!("Invalid target direction"),
            };
            Position {
                x: -LINE_SPACING,
                y: lane,
            }
        }
        Direction::Down => {
            let lane = match target_direction {
                Direction::Right => 10 * LINE_SPACING,
                Direction::Up => 9 * LINE_SPACING,
                Direction::Left => 8 * LINE_SPACING,
                _ => panic!("Invalid target direction"),
            };
            Position {
                x: lane,
                y: WINDOW_SIZE as i32,
            }
        }
        Direction::Right => {
            let lane = match target_direction {
                Direction::Up => 5 * LINE_SPACING,
                Direction::Left => 6 * LINE_SPACING,
                Direction::Down => 7 * LINE_SPACING,
                _ => panic!("Invalid target direction"),
            };
            Position {
                x: WINDOW_SIZE as i32,
                y: lane,
            }
        }
    }
}

pub fn get_turning_position(
    initial_position: Direction,
    target_direction: Direction,
) -> (Option<i32>, Option<i32>) {
    if target_direction == initial_position.opposite() {
        return (None, None);
    }

    match initial_position {
        Direction::Up => match target_direction {
            Direction::Right => (None, Some(8 * LINE_SPACING)),
            Direction::Left => (None, Some(5 * LINE_SPACING)),
            _ => (None, None),
        },
        Direction::Left => match target_direction {
            Direction::Up => (Some(8 * LINE_SPACING), None),
            Direction::Down => (Some(5 * LINE_SPACING), None),
            _ => (None, None),
        },
        Direction::Down => match target_direction {
            Direction::Left => (None, Some(7 * LINE_SPACING)),
            Direction::Right => (None, Some(10 * LINE_SPACING)),
            _ => (None, None),
        },
        Direction::Right => match target_direction {
            Direction::Down => (Some(7 * LINE_SPACING), None),
            Direction::Up => (Some(10 * LINE_SPACING), None),
            _ => (None, None),
        },
    }
}
