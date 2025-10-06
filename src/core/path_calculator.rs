use crate::constants::*;
use crate::core::vehicle_data::Vehicle;
use crate::geometry::position::{Position, TimedPosition};

pub struct PathCalculator;

impl PathCalculator {
    pub fn calculate_path(
        vehicle: &Vehicle,
        start_position: &Position,
        all_vehicles: &Vec<Vehicle>,
    ) -> Vec<TimedPosition> {
        let mut temp_rect = vehicle.rect.clone();
        let mut time = if all_vehicles.is_empty() || all_vehicles[0].path.is_empty() {
            1
        } else {
            all_vehicles[0].path[0].time
        };
        let mut speed = 2;
        let mut current_direction = vehicle.start_direction;
        let mut path = Vec::new();

        let start_position = start_position.move_in_direction(&current_direction, speed);
        let mut current_position = start_position;
        temp_rect.set_x(current_position.x);
        temp_rect.set_y(current_position.y);

        use crate::geometry::rect_extensions::RectExtensions;
        while temp_rect.is_in_bounds(WINDOW_SIZE) {
            current_direction.update_direction(
                &vehicle.target_direction,
                &current_position,
                &vehicle.turn_position,
            );

            current_position = current_position.move_in_direction(&current_direction, speed);

            path.push(TimedPosition {
                position: current_position,
                time,
            });

            temp_rect.set_x(current_position.x);
            temp_rect.set_y(current_position.y);

            if current_position.is_out_of_intersection() && speed != 3 {
                speed = 3;
            }

            use crate::core::collision_detector::CollisionDetector;
            while time <= path[path.len() - 1].time {
                let relevant_vehicles: Vec<&Vehicle> = all_vehicles
                    .iter()
                    .filter(|v| {
                        CollisionDetector::is_relevant_for_collision(vehicle, v, &current_position, &time)
                    })
                    .collect();

                let mut iter = relevant_vehicles.iter();
                while let Some(other_vehicle) = iter.next() {
                    let collision_time_position = other_vehicle.path.iter().find(|&&tp| tp.time == time);
                    if collision_time_position.is_none() {
                        continue;
                    }
                    let tp = collision_time_position.unwrap();

                    let same_lane = vehicle.initial_position == other_vehicle.initial_position
                        && vehicle.target_direction == other_vehicle.target_direction;
                    if !tp.position.is_in_intersection() && !same_lane {
                        continue;
                    }
                    if !current_position.is_in_intersection() && !same_lane {
                        continue;
                    }
                    let vehicle_rect = sdl2::rect::Rect::new(
                        tp.position.x,
                        tp.position.y,
                        other_vehicle.rect.width(),
                        other_vehicle.rect.height(),
                    );
                    if !vehicle_rect.has_intersection(temp_rect) {
                        continue;
                    }

                    if path.len() == 1 || current_position == path[0].position {
                        path.push(TimedPosition {
                            position: current_position,
                            time: time + 1,
                        });
                        time += 1;
                        continue;
                    }

                    use crate::core::collision_resolver::CollisionResolver;
                    time = CollisionResolver::resolve_collision(vehicle, &mut path, &current_position, &vehicle_rect);

                    if let Some(pos) = path.iter().position(|tp| tp.time == time) {
                        path.truncate(pos + 1);
                    }
                    iter = relevant_vehicles.iter();

                    current_position = path.iter().find(|tp| tp.time == time).unwrap().position;
                    temp_rect.set_x(current_position.x);
                    temp_rect.set_y(current_position.y);
                    current_direction = if current_position.is_after_turn(&vehicle.turn_position) {
                        vehicle.target_direction
                    } else {
                        vehicle.start_direction
                    };
                }
                time += 1;
            }
        }
        path
    }
}
