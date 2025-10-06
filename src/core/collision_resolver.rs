use crate::core::vehicle_data::Vehicle;
use crate::geometry::position::{Position, TimedPosition};
use sdl2::rect::Rect;

pub struct CollisionResolver;

impl CollisionResolver {
    pub fn resolve_collision(
        vehicle: &Vehicle,
        path: &mut Vec<TimedPosition>,
        current_position: &Position,
        other_vehicle_rect: &Rect,
    ) -> u64 {
        let new_position = Self::find_non_colliding_position(vehicle, path, other_vehicle_rect);
        let steps = current_position.calculate_steps_to(&new_position);
        if steps == 0 {
            panic!("Error: Steps cannot be zero.");
        }
        let (mut fix_index, mut reached_steps) = Self::find_position(path, steps);
        let print_fix_index = fix_index;
        let mut tmp_position = path[fix_index].position;
        let mut current_direction = if tmp_position.is_after_turn(&vehicle.turn_position) {
            vehicle.target_direction
        } else {
            vehicle.start_direction
        };
        let mut collision_time_index = path[path.len() - 1].time;

        if reached_steps != steps {
            let first_position = path.first().unwrap().position;
            while reached_steps < steps {
                path[fix_index].position = first_position;
                reached_steps += 1;
                fix_index += 1;
            }
        }

        while tmp_position != new_position {
            if fix_index >= path.len() {
                panic!("Error: Unable to resolve collision, path fixing failed.");
            }
            path[fix_index].position = tmp_position;
            if tmp_position.is_in_intersection() {
                collision_time_index = path[fix_index].time;
            }
            tmp_position = tmp_position.move_in_direction(&current_direction, 1);
            current_direction.update_direction(
                &vehicle.target_direction,
                &tmp_position,
                &vehicle.turn_position,
            );

            fix_index += 1;
        }
        if fix_index != path.len() - 1 && tmp_position == new_position {
            panic!(
                "Unable to resolve collision. Current position: {:?}, New position: {:?}, Fix index: {}, Reached steps: {}, Steps: {}, Current index: {}",
                current_position, new_position, print_fix_index, reached_steps, steps, path.len() - 1
            );
        }

        path[fix_index].position = tmp_position;
        if *current_position == new_position {
            panic!(
                "Unable to resolve collision. Current position: {:?} is the same as new position: {:?}",
                current_position, new_position
            );
        }
        collision_time_index
    }

    fn find_position(path: &Vec<TimedPosition>, steps: u64) -> (usize, u64) {
        let mut reached_steps: u64 = 0;
        let mut next_position = path[path.len() - 1].position;
        for index in (0..path.len() - 1).rev() {
            let diff_x = (next_position.x - path[index].position.x).abs();
            let diff_y = (next_position.y - path[index].position.y).abs();
            let diff = diff_x + diff_y;
            if diff > 1 {
                reached_steps += (diff - 1) as u64;
            }
            if reached_steps == steps {
                return (index, reached_steps);
            } else if reached_steps > steps {
                panic!(
                    "Reached steps exceeded the required steps. Current position: {:?}, Next position: {:?}, Steps: {}, Reached steps: {}",
                    path[index].position, next_position, steps, reached_steps
                );
            }
            next_position = path[index].position;
        }
        for index in (0..path.len()).rev() {
            if path[index].position == path[0].position {
                return (index, reached_steps);
            }
        }
        (0, reached_steps)
    }

    fn find_non_colliding_position(
        vehicle: &Vehicle,
        path: &Vec<TimedPosition>,
        other_vehicle_rect: &Rect,
    ) -> Position {
        let mut temp_rect = vehicle.rect.clone();
        for path_index in (0..path.len()).rev() {
            temp_rect.set_x(path[path_index].position.x);
            temp_rect.set_y(path[path_index].position.y);
            if !other_vehicle_rect.has_intersection(temp_rect) {
                return path[path_index].position;
            }
        }
        if path.is_empty() {
            panic!("Error: Path is empty, cannot find non-colliding position.");
        } else {
            path[0].position
        }
    }
}
