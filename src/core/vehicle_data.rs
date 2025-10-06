use crate::direction::*;
use crate::geometry::position::TimedPosition;
use rand::Rng;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

#[derive(Debug, PartialEq)]
pub struct Vehicle {
    pub id: usize,
    pub rect: Rect,
    pub color: Color,
    pub(crate) initial_position: Direction,
    pub(crate) start_direction: Direction,
    pub(crate) target_direction: Direction,
    pub(crate) turn_direction: TurnDirection,
    pub(crate) turn_position: (Option<i32>, Option<i32>),
    pub(crate) path: Vec<TimedPosition>,
    pub texture_name: String,
    pub texture_index: usize,
    pub rotation: f64,
    velocity_type: i32,
}

impl Vehicle {
    pub fn new(
        initial_position: Direction,
        target_direction: Direction,
        size: u32,
        all_vehicles: &Vec<Vehicle>,
        id: usize,
    ) -> Self {
        use crate::geometry::spawn::get_spawn_position;
        use crate::intersection::turning::get_turning_position;

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

        use crate::core::path_calculator::PathCalculator;
        vehicle.path = PathCalculator::calculate_path(&vehicle, &start_position, all_vehicles);

        vehicle
    }

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
        use crate::geometry::rect_extensions::RectExtensions;
        self.rect.is_in_bounds(window_size)
    }

    #[allow(dead_code)]
    pub fn get_velocity_type(&self) -> f32 {
        self.velocity_type as f32
    }
}
