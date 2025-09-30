use crate::constants::*;
use crate::direction::*;
use crate::vehicle_positions::Position;
use crate::vehicle_positions::{get_spawn_position, get_turning_position};
use rand::Rng;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedPosition {
    pub position: Position,
    pub time: u64,
}

#[derive(Debug, PartialEq)]
pub struct Vehicle {
    pub id: usize,
    pub rect: Rect,
    pub color: Color,
    initial_position: Direction, // initial position and start direction are oppisite
    start_direction: Direction,
    target_direction: Direction,
    turn_direction: TurnDirection,
    turn_position: (Option<i32>, Option<i32>),
    path: Vec<TimedPosition>,
    pub texture_name: String, //we need those two for images
    pub texture_index: usize, //cuz we want to have more than one car
    pub rotation: f64,
    velocity_type: i32, // Just for display purposes - doesn't affect actual movement
}

impl Vehicle {
    pub fn new(
        initial_position: Direction,
        target_direction: Direction,
        size: u32,
        all_vehicles: &Vec<Vehicle>,
        id: usize,
    ) -> Self {
        let start_position = get_spawn_position(initial_position, target_direction);
        let color = Self::random_color();
        let rect = Rect::new(start_position.x, start_position.y, size, size);
        let turn_direction = Direction::turn_direction(initial_position, target_direction);
        let start_direction = initial_position.opposite();
        let turn_position = get_turning_position(initial_position, target_direction);
        let mut rng = rand::thread_rng();
        let texture_index = rng.gen_range(0..3);
        let rotation = match initial_position {
            Direction::Up => 0.0,
            Direction::Right => 90.0,
            Direction::Down => 180.0,
            Direction::Left => 270.0,
        };

        // Assign a velocity type for display purposes only
        let velocity_type = rng.gen_range(1..=3);

        let mut vehicle = Vehicle {
            id,
            rect,
            color,
            initial_position,
            start_direction,
            target_direction,
            turn_direction,
            turn_position,
            path: Vec::new(),
            texture_name: "car".to_string(),
            rotation,
            texture_index,
            velocity_type,
        };
        vehicle.path = vehicle.calculate_path(&start_position, all_vehicles);

        vehicle
    }

    // Calculate the path as a vector of positions - EXACTLY AS ORIGINAL
    fn calculate_path(
        &self,
        start_position: &Position,
        all_vehicles: &Vec<Vehicle>,
    ) -> Vec<TimedPosition> {
        let mut temp_rect = self.rect.clone();
        let mut time = if all_vehicles.is_empty() || all_vehicles[0].path.is_empty() {
            1
        } else {
            all_vehicles[0].path[0].time
        };
        let mut speed = 2; // KEEP ORIGINAL SPEED
        let mut current_direction = self.start_direction;
        let mut path = Vec::new();
        // this is the first movement to start
        let start_position = start_position.move_in_direction(&current_direction, speed);
        let mut current_position = start_position;
        temp_rect.set_x(current_position.x);
        temp_rect.set_y(current_position.y);

        // Generate the path
        while temp_rect.is_in_bounds(WINDOW_SIZE) {
            // Update direction at the turn position
            current_direction.update_direction(
                &self.target_direction,
                &current_position,
                &self.turn_position,
            );

            current_position = current_position.move_in_direction(&current_direction, speed);

            path.push(TimedPosition {
                position: current_position,
                time,
            });

            temp_rect.set_x(current_position.x);
            temp_rect.set_y(current_position.y);

            // if the vehicle is out of intersection change the speed to 3 - EXACTLY AS ORIGINAL
            if current_position.is_out_of_intersection() && speed != 3 {
                speed = 3;
            }

            // This is the ALOGIRITHM
            // The following is to check for collisions with other vehicles
            while time <= path[path.len() - 1].time {
                // current position should be depending on the time IMPORTANT
                let relevant_vehicles: Vec<&Vehicle> = all_vehicles
                    .iter()
                    .filter(|vehicle| {
                        self.is_relevant_for_collision(vehicle, &current_position, &time)
                    })
                    .collect();

                // old iteration was (for vehicle in relevant_vehicles)
                let mut iter = relevant_vehicles.iter();
                while let Some(vehicle) = iter.next() {
                    let collision_time_position = vehicle.path.iter().find(|&&tp| tp.time == time);
                    if collision_time_position.is_none() {
                        continue;
                    }
                    let tp = collision_time_position.unwrap();
                    // if tp is not in the intersection and not in the same lane then it is not relevant
                    let same_lane = self.initial_position == vehicle.initial_position
                        && self.target_direction == vehicle.target_direction;
                    if !tp.position.is_in_intersection() && !same_lane {
                        continue;
                    }
                    if !current_position.is_in_intersection() && !same_lane {
                        continue;
                    }
                    let vehicle_rect = Rect::new(
                        tp.position.x,
                        tp.position.y,
                        vehicle.rect.width(),
                        vehicle.rect.height(),
                    );
                    if !vehicle_rect.has_intersection(temp_rect) {
                        continue;
                    }

                    // when the length of the path is only one the usual way of resolve collision wont work so just stay in the spawn point
                    if path.len() == 1 || current_position == path[0].position {
                        path.push(TimedPosition {
                            position: current_position,
                            time: time + 1,
                        });
                        time += 1;
                        continue;
                    }

                    time = self.resolve_collision(&mut path, &current_position, &vehicle_rect);
                    // i should clear after the current time
                    if let Some(pos) = path.iter().position(|tp| tp.time == time) {
                        path.truncate(pos + 1);
                    }
                    iter = relevant_vehicles.iter();
                    // fix current position IMPORTANT
                    // here it should never crash
                    current_position = path.iter().find(|tp| tp.time == time).unwrap().position;
                    temp_rect.set_x(current_position.x);
                    temp_rect.set_y(current_position.y);
                    current_direction = if current_position.is_after_turn(&self.turn_position) {
                        self.target_direction
                    } else {
                        self.start_direction
                    };
                }
                time += 1;
            }
        }
        path
    }

    // this resolve the collision and returns the time of the first position have changed that is in the intersection so we check it and the positions after it
    fn resolve_collision(
        &self,
        path: &mut Vec<TimedPosition>,
        current_position: &Position,
        other_vehicle_rect: &Rect,
    ) -> u64 {
        let new_position = self.find_non_colliding_position(path, other_vehicle_rect);
        let steps = current_position.calculate_steps_to(&new_position);
        if steps == 0 {
            panic!("Error: Steps cannot be zero.");
        }
        let (mut fix_index, mut reached_steps) = self.find_position(path, steps);
        let print_fix_index = fix_index;
        let mut tmp_position = path[fix_index].position;
        let mut current_direction = if tmp_position.is_after_turn(&self.turn_position) {
            self.target_direction
        } else {
            self.start_direction
        };
        let mut collision_time_index = path[path.len() - 1].time; // this will be updated with the time that it is in the intersection so it may have a collision

        if reached_steps != steps {
            let first_position = path.first().unwrap().position;
            while reached_steps < steps {
                path[fix_index].position = first_position;
                reached_steps += 1;
                fix_index += 1;
            }
        }
        // here i will update and fix the path from the position till the end of the path
        while tmp_position != new_position {
            // this should never be reached
            if fix_index >= path.len() {
                panic!("Error: Unable to resolve collision, path fixing failed.");
            }
            path[fix_index].position = tmp_position;
            if tmp_position.is_in_intersection() {
                collision_time_index = path[fix_index].time;
            }
            tmp_position = tmp_position.move_in_direction(&current_direction, 1);
            current_direction.update_direction(
                &self.target_direction,
                &tmp_position,
                &self.turn_position,
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

    // returns the position that will be fixing from
    // if reached the bigenning and still cant be fixed it will return o and the number of steps that can be fixed
    fn find_position(&self, path: &Vec<TimedPosition>, steps: u64) -> (usize, u64) {
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
        &self,
        path: &Vec<TimedPosition>,
        other_vehicle_rect: &Rect,
    ) -> Position {
        let mut temp_rect = self.rect.clone();
        for path_index in (0..path.len()).rev() {
            temp_rect.set_x(path[path_index].position.x);
            temp_rect.set_y(path[path_index].position.y);
            if !other_vehicle_rect.has_intersection(temp_rect) {
                return path[path_index].position;
            }
        }
        if path.is_empty() {
            // should never come here
            panic!("Error: Path is empty, cannot find non-colliding position.");
        } else {
            path[0].position
        }
    }

    // Random color generator
    fn random_color() -> Color {
        let mut rng = rand::thread_rng();
        Color::RGB(
            rng.gen_range(0..=255),
            rng.gen_range(0..=255),
            rng.gen_range(0..=255),
        )
    }

    pub fn update_position(&mut self) {
        if !self.path.is_empty() {
            let next = self.path.remove(0);

            let dx = next.position.x - self.rect.x();
            let dy = next.position.y - self.rect.y();

            if dx != 0 || dy != 0 {
                self.rotation = match (dx.signum(), dy.signum()) {
                    (1, 0) => 90.0,
                    (-1, 0) => 270.0,
                    (0, 1) => 180.0,
                    (0, -1) => 0.0,
                    _ => self.rotation,
                };
            }

            self.rect.set_x(next.position.x);
            self.rect.set_y(next.position.y);
        }
    }

    pub fn is_in_bounds(&self, window_size: u32) -> bool {
        self.rect.is_in_bounds(window_size)
    }

    // For statistics - return the "velocity type" as if we had 3 speeds
    pub fn get_velocity_type(&self) -> f32 {
        self.velocity_type as f32
    }

    fn is_relevant_for_collision(
        &self,
        other_vehicle: &Vehicle,
        current_position: &Position,
        time: &u64,
    ) -> bool {
        let same_lane = self.initial_position == other_vehicle.initial_position
            && self.target_direction == other_vehicle.target_direction;
        // if self is turning to the right and the other is not on the same lane then it is not relevant
        if (self.turn_direction == TurnDirection::Right
            || other_vehicle.turn_direction == TurnDirection::Right)
            && !same_lane
        {
            return false;
        }

        // if the two vehicles have the same start direction but different end direction then they are not relevant
        if self.start_direction == other_vehicle.start_direction
            && self.target_direction != other_vehicle.target_direction
        {
            return false;
        }

        // it self is going straigth and the other vehicle is going also stragit and the start position is the oppisite then it is not relevant
        if self.turn_direction == TurnDirection::Straight
            && other_vehicle.turn_direction == TurnDirection::Straight
            && self.initial_position == other_vehicle.start_direction
        {
            return false;
        }

        // if the vehicle is not in the intersection and the other is not in the same lane then it is not relevant
        if !same_lane && !current_position.is_in_intersection() {
            return false;
        }

        if !other_vehicle.path.iter().any(|tp| tp.time == *time) {
            return false;
        }

        true
    }
}

pub trait RectExtensions {
    fn is_in_bounds(&self, window_size: u32) -> bool;
}

impl RectExtensions for Rect {
    fn is_in_bounds(&self, window_size: u32) -> bool {
        let size = self.width() as i32;
        self.x() > -size
            && self.x() < window_size as i32
            && self.y() > -size
            && self.y() < window_size as i32
    }
}