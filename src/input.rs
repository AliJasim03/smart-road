use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use crate::vehicle::Direction;

pub struct InputHandler {
    key_states: [bool; 5], // Up, Down, Left, Right, R
    continuous_spawn: bool,
    last_direction: Option<Direction>,
    spawn_cooldown: f32,
    current_cooldown: f32,
    debug_mode: bool,
    show_grid: bool,
}

impl InputHandler {
    pub fn new() -> Self {
        InputHandler {
            key_states: [false; 5],
            continuous_spawn: false,
            last_direction: None,
            spawn_cooldown: 2.0, // 2 seconds between spawns
            current_cooldown: 0.0,
            debug_mode: false,
            show_grid: false,
        }
    }

    // Process keyboard events and return the action to take
    pub fn process_event(&mut self, event: &Event) -> InputAction {
        match event {
            Event::KeyDown { keycode: Some(keycode), repeat, .. } => {
                // Ignore key repeat events for spawn commands
                if *repeat {
                    return InputAction::None;
                }

                match keycode {
                    Keycode::Up => {
                        self.key_states[0] = true;
                        self.last_direction = Some(Direction::North);
                        if self.can_spawn() {
                            self.current_cooldown = self.spawn_cooldown;
                            InputAction::SpawnVehicle(Direction::North)
                        } else {
                            InputAction::None
                        }
                    }
                    Keycode::Down => {
                        self.key_states[1] = true;
                        self.last_direction = Some(Direction::South);
                        if self.can_spawn() {
                            self.current_cooldown = self.spawn_cooldown;
                            InputAction::SpawnVehicle(Direction::South)
                        } else {
                            InputAction::None
                        }
                    }
                    Keycode::Left => {
                        self.key_states[2] = true;
                        self.last_direction = Some(Direction::East);
                        if self.can_spawn() {
                            self.current_cooldown = self.spawn_cooldown;
                            InputAction::SpawnVehicle(Direction::East)
                        } else {
                            InputAction::None
                        }
                    }
                    Keycode::Right => {
                        self.key_states[3] = true;
                        self.last_direction = Some(Direction::West);
                        if self.can_spawn() {
                            self.current_cooldown = self.spawn_cooldown;
                            InputAction::SpawnVehicle(Direction::West)
                        } else {
                            InputAction::None
                        }
                    }
                    Keycode::R => {
                        self.key_states[4] = true;
                        self.continuous_spawn = !self.continuous_spawn;
                        InputAction::ToggleContinuousSpawn(self.continuous_spawn)
                    }
                    Keycode::D => {
                        self.debug_mode = !self.debug_mode;
                        InputAction::ToggleDebugMode(self.debug_mode)
                    }
                    Keycode::G => {
                        self.show_grid = !self.show_grid;
                        InputAction::ToggleGrid(self.show_grid)
                    }
                    Keycode::Space => {
                        InputAction::ShowStatistics
                    }
                    Keycode::Escape => {
                        InputAction::Exit
                    }
                    Keycode::H => {
                        InputAction::ShowHelp
                    }
                    Keycode::P => {
                        InputAction::TogglePause
                    }
                    _ => InputAction::None,
                }
            }
            Event::KeyUp { keycode: Some(keycode), .. } => {
                match keycode {
                    Keycode::Up => self.key_states[0] = false,
                    Keycode::Down => self.key_states[1] = false,
                    Keycode::Left => self.key_states[2] = false,
                    Keycode::Right => self.key_states[3] = false,
                    Keycode::R => self.key_states[4] = false,
                    _ => {}
                }
                InputAction::None
            }
            _ => InputAction::None,
        }
    }

    // Update the input handler's internal state
    pub fn update(&mut self, delta_time: f32) {
        // Update spawn cooldown
        if self.current_cooldown > 0.0 {
            self.current_cooldown -= delta_time;
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

    // Check if spawning is allowed (not in cooldown)
    fn can_spawn(&self) -> bool {
        self.current_cooldown <= 0.0
    }

    // Get current debug mode state
    pub fn is_debug_mode(&self) -> bool {
        self.debug_mode
    }

    // Get current grid display state
    pub fn is_grid_shown(&self) -> bool {
        self.show_grid
    }

    // Get remaining cooldown time
    pub fn get_cooldown_remaining(&self) -> f32 {
        self.current_cooldown.max(0.0)
    }

    // Check if any movement key is currently pressed
    pub fn is_any_movement_key_pressed(&self) -> bool {
        self.key_states[0..4].iter().any(|&pressed| pressed)
    }

    // Get currently pressed keys as a vector
    pub fn get_pressed_keys(&self) -> Vec<Direction> {
        let mut pressed = Vec::new();

        if self.key_states[0] { pressed.push(Direction::North); }
        if self.key_states[1] { pressed.push(Direction::South); }
        if self.key_states[2] { pressed.push(Direction::East); }
        if self.key_states[3] { pressed.push(Direction::West); }

        pressed
    }

    // Set spawn cooldown (useful for difficulty adjustment)
    pub fn set_spawn_cooldown(&mut self, cooldown: f32) {
        self.spawn_cooldown = cooldown.max(0.1); // Minimum 0.1 seconds
    }

    // Force reset cooldown (useful for debugging)
    pub fn reset_cooldown(&mut self) {
        self.current_cooldown = 0.0;
    }
}

// Actions that can be triggered by input
#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    None,
    SpawnVehicle(Direction),
    ToggleContinuousSpawn(bool),
    ToggleDebugMode(bool),
    ToggleGrid(bool),
    ShowStatistics,
    ShowHelp,
    TogglePause,
    Exit,
}

// Input configuration for customizable controls
pub struct InputConfig {
    pub spawn_north: Keycode,
    pub spawn_south: Keycode,
    pub spawn_east: Keycode,
    pub spawn_west: Keycode,
    pub toggle_continuous: Keycode,
    pub toggle_debug: Keycode,
    pub toggle_grid: Keycode,
    pub show_stats: Keycode,
    pub show_help: Keycode,
    pub toggle_pause: Keycode,
    pub exit: Keycode,
}

impl Default for InputConfig {
    fn default() -> Self {
        InputConfig {
            spawn_north: Keycode::Up,
            spawn_south: Keycode::Down,
            spawn_east: Keycode::Left,
            spawn_west: Keycode::Right,
            toggle_continuous: Keycode::R,
            toggle_debug: Keycode::D,
            toggle_grid: Keycode::G,
            show_stats: Keycode::Space,
            show_help: Keycode::H,
            toggle_pause: Keycode::P,
            exit: Keycode::Escape,
        }
    }
}

// Enhanced input handler with customizable controls
pub struct ConfigurableInputHandler {
    handler: InputHandler,
    config: InputConfig,
}

impl ConfigurableInputHandler {
    pub fn new() -> Self {
        ConfigurableInputHandler {
            handler: InputHandler::new(),
            config: InputConfig::default(),
        }
    }

    pub fn with_config(config: InputConfig) -> Self {
        ConfigurableInputHandler {
            handler: InputHandler::new(),
            config,
        }
    }

    pub fn process_event(&mut self, event: &Event) -> InputAction {
        // This would use the custom config, but for now just delegate
        self.handler.process_event(event)
    }

    pub fn update(&mut self, delta_time: f32) {
        self.handler.update(delta_time);
    }

    // Delegate other methods
    pub fn is_continuous_spawn(&self) -> bool {
        self.handler.is_continuous_spawn()
    }

    pub fn get_direction(&self) -> Direction {
        self.handler.get_direction()
    }

    pub fn get_random_direction(&self) -> Direction {
        self.handler.get_random_direction()
    }

    pub fn is_debug_mode(&self) -> bool {
        self.handler.is_debug_mode()
    }

    pub fn is_grid_shown(&self) -> bool {
        self.handler.is_grid_shown()
    }

    pub fn get_cooldown_remaining(&self) -> f32 {
        self.handler.get_cooldown_remaining()
    }
}

// Helper function to print control instructions
pub fn print_controls() {
    println!("╔══════════════════════════════════════╗");
    println!("║            GAME CONTROLS             ║");
    println!("╠══════════════════════════════════════╣");
    println!("║ ↑ Arrow Up    │ Spawn from South     ║");
    println!("║ ↓ Arrow Down  │ Spawn from North     ║");
    println!("║ ← Arrow Left  │ Spawn from East      ║");
    println!("║ → Arrow Right │ Spawn from West      ║");
    println!("║ R             │ Toggle auto-spawn    ║");
    println!("║ D             │ Toggle debug mode    ║");
    println!("║ G             │ Toggle grid display  ║");
    println!("║ Space         │ Show statistics      ║");
    println!("║ H             │ Show this help       ║");
    println!("║ P             │ Toggle pause         ║");
    println!("║ Esc           │ Exit simulation      ║");
    println!("╚══════════════════════════════════════╝");
}