use rand::Rng;
use crate::geometry::Position;

#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum TurnDirection {
    Left,
    Right,
    Straight,
}

impl Direction {
    pub fn new(exclude: Option<Direction>) -> Direction {
        let mut rng = rand::thread_rng();
        loop {
            let new_direction = match rng.gen_range(0..4) {
                0 => Direction::Up,
                1 => Direction::Left,
                2 => Direction::Down,
                3 => Direction::Right,
                _ => unreachable!(),
            };

            if let Some(exclude_dir) = exclude {
                if new_direction != exclude_dir {
                    return new_direction;
                }
            } else {
                return new_direction;
            }
        }
    }
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }

    pub fn update_direction(
        &mut self,
        target_direction: &Direction,
        position: &Position,
        turn_position: &(Option<i32>, Option<i32>),
    ) {
        if let Some(turn_x) = turn_position.0 {
            if *self != *target_direction && position.x == turn_x {
                *self = *target_direction;
            }
        }

        if let Some(turn_y) = turn_position.1 {
            if *self != *target_direction && position.y == turn_y {
                *self = *target_direction;
            }
        }
    }

    pub fn turn_direction(initial_position: Direction, target: Direction) -> TurnDirection {
        match (initial_position, target) {
            // Straight
            (Direction::Up, Direction::Up)
            | (Direction::Down, Direction::Down)
            | (Direction::Left, Direction::Left)
            | (Direction::Right, Direction::Right) => TurnDirection::Straight,

            // Turning Left
            (Direction::Up, Direction::Left)
            | (Direction::Left, Direction::Down)
            | (Direction::Down, Direction::Right)
            | (Direction::Right, Direction::Up) => TurnDirection::Right,

            // Turning Right
            (Direction::Up, Direction::Right)
            | (Direction::Right, Direction::Down)
            | (Direction::Down, Direction::Left)
            | (Direction::Left, Direction::Up) => TurnDirection::Left,

            // Opposite Directions
            (Direction::Up, Direction::Down)
            | (Direction::Down, Direction::Up)
            | (Direction::Left, Direction::Right)
            | (Direction::Right, Direction::Left) => TurnDirection::Straight,
        }
    }
}
