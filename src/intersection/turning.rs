use crate::constants::LINE_SPACING;
use crate::direction::Direction;

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
