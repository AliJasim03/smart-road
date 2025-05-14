use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use crate::vehicle::Direction;

pub struct InputHandler {
    key_states: [bool; 4], // Up, Down, Left, Right
    continuous_spawn: bool,
    last_direction: Option<Direction>,
}

impl InputHandler {
    pub fn new() -> Self {
        InputHandler {
            key_states: [false; 4],
            continuous_spawn: false,
            last_direction: None,
        }
    }

    // Process keyboard events
    pub fn process_event(&mut self, event: &Event) -> Option<Direction> {
        match event {
            Event::KeyDown { keycode: Some(keycode), repeat, .. } => {
                // Ignore key repeat events
                if *repeat {
                    return None;
                }

                match keycode {
                    Keycode::Up => {
                        self.key_states[0] = true;
                        self.last_direction = Some(Direction::North);
                        Some(Direction::North) // Vehicles coming from South, moving North
                    }
                    Keycode::Down => {
                        self.key_states[1] = true;
                        self.last_direction = Some(Direction::South);
                        Some(Direction::South) // Vehicles coming from North, moving South
                    }
                    Keycode::Left => {
                        self.key_states[2] = true;
                        self.last_direction = Some(Direction::East);
                        Some(Direction::East) // Vehicles coming from East, moving West
                    }
                    Keycode::Right => {
                        self.key_states[3] = true;
                        self.last_direction = Some(Direction::West);
                        Some(Direction::West) // Vehicles coming from West, moving East
                    }
                    Keycode::R => {
                        self.continuous_spawn = !self.continuous_spawn;
                        None
                    }
                    _ => None,
                }
            }
            Event::KeyUp { keycode: Some(keycode), .. } => {
                match keycode {
                    Keycode::Up => self.key_states[0] = false,
                    Keycode::Down => self.key_states[1] = false,
                    Keycode::Left => self.key_states[2] = false,
                    Keycode::Right => self.key_states[3] = false,
                    _ => {}
                }
                None
            }
            _ => None,
        }
    }

    // Check if continuous spawn is enabled
    pub fn is_continuous_spawn(&self) -> bool {
        self.continuous_spawn
    }

    // Get the last pressed direction or a random one if none
    pub fn get_direction(&self) -> Direction {
        if let Some(dir) = self.last_direction {
            dir
        } else {
            self.get_random_direction()
        }
    }

    // Get a random direction
    pub fn get_random_direction(&self) -> Direction {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        match rng.gen_range(0..4) {
            0 => Direction::North,
            1 => Direction::South,
            2 => Direction::East,
            _ => Direction::West,
        }
    }
}