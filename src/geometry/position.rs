use crate::direction::Direction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedPosition {
    pub position: Position,
    pub time: u64,
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
        use crate::intersection::IntersectionBounds;
        IntersectionBounds::is_position_in_intersection(self)
    }

    pub fn calculate_steps_to(&self, new_position: &Position) -> u64 {
        ((self.x - new_position.x).abs() + (self.y - new_position.y).abs()) as u64
    }

    pub fn is_out_of_intersection(&self) -> bool {
        use crate::intersection::IntersectionBounds;
        IntersectionBounds::is_position_out_of_intersection(self)
    }
}
