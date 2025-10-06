use crate::constants::*;
use crate::direction::Direction;
use crate::geometry::position::Position;

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
