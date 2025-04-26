use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::collections::VecDeque;

use crate::algorithm::SmartIntersection;
use crate::input::InputHandler;
use crate::intersection::Intersection;
use crate::renderer::Renderer;
use crate::statistics::Statistics;
use crate::vehicle::{Direction, Route, Vehicle};

pub struct Game<'a> {
    canvas: Canvas<Window>,
    intersection: Intersection,
    vehicles: VecDeque<Vehicle>,
    smart_algorithm: SmartIntersection,
    statistics: Statistics,
    input_handler: InputHandler,
    renderer: Renderer<'a>,
    continuous_spawn: bool,
    spawn_cooldown: u32,
    current_cooldown: u32,
}

impl<'a> Game<'a> {
    pub fn new(canvas: Canvas<Window>, renderer: Renderer<'a>) -> Result<Self, String> {
        let intersection = Intersection::new();
        let smart_algorithm = SmartIntersection::new();
        let statistics = Statistics::new();
        let input_handler = InputHandler::new();

        Ok(Game {
            canvas,
            intersection,
            vehicles: VecDeque::new(),
            smart_algorithm,
            statistics,
            input_handler,
            renderer,
            continuous_spawn: false,
            spawn_cooldown: 1000, // 1 second between spawns to prevent spamming
            current_cooldown: 0,
        })
    }

    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::KeyDown {
                keycode: Some(keycode),
                ..
            } => match keycode {
                Keycode::Up => {
                    self.spawn_vehicle(Direction::South);
                }
                Keycode::Down => {
                    self.spawn_vehicle(Direction::North);
                }
                Keycode::Left => {
                    self.spawn_vehicle(Direction::East);
                }
                Keycode::Right => {
                    self.spawn_vehicle(Direction::West);
                }
                Keycode::R => {
                    self.continuous_spawn = !self.continuous_spawn;
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub fn render(&mut self) -> Result<(), String> {
        // Clear the screen
        self.canvas.set_draw_color(Color::RGB(100, 100, 100)); // Gray background
        self.canvas.clear();

        // Render the intersection
        self.renderer
            .render_intersection(&mut self.canvas, &self.intersection)?;

        // Render all vehicles
        for vehicle in &self.vehicles {
            self.renderer.render_vehicle(&mut self.canvas, vehicle)?;
        }

        // Present the frame
        self.canvas.present();

        Ok(())
    }

    pub fn update(&mut self, delta_time: u32) {
        println!(
            "Game update: delta_time={}ms, vehicle count={}, continuous_spawn={}",
            delta_time,
            self.vehicles.len(),
            self.continuous_spawn
        );
        // Update spawn cooldown
        if self.current_cooldown > 0 {
            self.current_cooldown = self.current_cooldown.saturating_sub(delta_time);
        }

        // Handle continuous spawning
        if self.continuous_spawn && self.current_cooldown == 0 {
            println!("Attempting to spawn random vehicle");
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let random_direction = match rng.gen_range(0..4) {
                0 => Direction::North,
                1 => Direction::South,
                2 => Direction::East,
                _ => Direction::West,
            };
            self.spawn_vehicle(random_direction);
            self.current_cooldown = self.spawn_cooldown;
            println!("Spawned vehicle in direction: {:?}", random_direction);
        }

        // Update all vehicles
        self.smart_algorithm
            .process_vehicles(&mut self.vehicles, &self.intersection, delta_time);

        // Update statistics
        self.statistics.update(&self.vehicles);

        // Remove vehicles that have left the intersection
        while let Some(vehicle) = self.vehicles.front() {
            if vehicle.has_left_intersection(&self.intersection) {
                self.statistics
                    .record_vehicle_exit(self.vehicles.pop_front().unwrap());
            } else {
                break;
            }
        }
    }

    pub fn show_statistics(&self) -> Result<(), String> {
        self.statistics.display()
    }

    fn spawn_vehicle(&mut self, direction: Direction) {
        println!(
            "spawn_vehicle called: direction={:?}, cooldown={}",
            direction, self.current_cooldown
        );

        if self.current_cooldown == 0 {
            // Use random route for the vehicle
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let random_route = match rng.gen_range(0..3) {
                0 => Route::Left,
                1 => Route::Straight,
                _ => Route::Right,
            };

            // Check if there's enough space to spawn a new vehicle
            if self.can_spawn_vehicle(&direction) {
                let new_vehicle = Vehicle::new(direction, random_route);
                println!(
                    "Vehicle spawned: id={}, direction={:?}, route={:?}",
                    new_vehicle.id, new_vehicle.direction, new_vehicle.route
                );
                self.vehicles.push_back(new_vehicle);
                self.current_cooldown = self.spawn_cooldown;
            } else {
                println!("Can't spawn vehicle: not enough space");
            }
        } else {
            println!(
                "Can't spawn vehicle: cooldown in progress ({}ms left)",
                self.current_cooldown
            );
        }
    }

    fn can_spawn_vehicle(&self, direction: &Direction) -> bool {
        // Check if there's enough space to spawn a new vehicle
        for vehicle in &self.vehicles {
            if &vehicle.direction == direction
                && vehicle.distance_from_spawn() < Vehicle::SAFE_DISTANCE * 2.0
            {
                return false;
            }
        }
        true
    }
}
