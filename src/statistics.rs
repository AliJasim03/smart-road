use std::collections::VecDeque;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::rect::Point;

use crate::vehicle::Vehicle;

pub struct Statistics {
    total_vehicles: u32,
    max_velocity: f64,
    min_velocity: f64,
    max_time: u32,
    min_time: u32,
    close_calls: u32,
    completed_vehicles: Vec<Vehicle>,
}

impl Statistics {
    pub fn new() -> Self {
        Statistics {
            total_vehicles: 0,
            max_velocity: 0.0,
            min_velocity: f64::MAX,
            max_time: 0,
            min_time: u32::MAX,
            close_calls: 0,
            completed_vehicles: Vec::new(),
        }
    }

    // Update statistics based on current vehicles
    pub fn update(&mut self, vehicles: &VecDeque<Vehicle>) {
        // Update total vehicles count
        self.total_vehicles = vehicles.len() as u32 + self.completed_vehicles.len() as u32;

        // Update velocity stats
        for vehicle in vehicles {
            if vehicle.current_velocity > self.max_velocity {
                self.max_velocity = vehicle.current_velocity;
            }
            if vehicle.current_velocity < self.min_velocity && vehicle.current_velocity > 0.0 {
                self.min_velocity = vehicle.current_velocity;
            }
        }
    }

    // Record a vehicle that has exited the intersection
    pub fn record_vehicle_exit(&mut self, vehicle: Vehicle) {
        // Update velocity stats
        if vehicle.current_velocity > self.max_velocity {
            self.max_velocity = vehicle.current_velocity;
        }
        if vehicle.current_velocity < self.min_velocity && vehicle.current_velocity > 0.0 {
            self.min_velocity = vehicle.current_velocity;
        }

        // Update time stats
        if vehicle.time_in_intersection > self.max_time {
            self.max_time = vehicle.time_in_intersection;
        }
        if vehicle.time_in_intersection < self.min_time && vehicle.time_in_intersection > 0 {
            self.min_time = vehicle.time_in_intersection;
        }

        // Add to completed vehicles list
        self.completed_vehicles.push(vehicle);
    }

    // Update close calls count
    pub fn add_close_call(&mut self) {
        self.close_calls += 1;
    }

    // Display statistics in a new window
    pub fn display(&self) -> Result<(), String> {
        // Create a new window for statistics
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

        let window = video_subsystem
            .window("Smart Road Simulation - Statistics", 500, 400)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;

        let mut canvas = window
            .into_canvas()
            .accelerated()
            .build()
            .map_err(|e| e.to_string())?;

        // Clear the canvas with a white background
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.clear();

        // Draw statistics as rectangles with different heights representing values
        self.draw_stats_as_rectangles(&mut canvas)?;

        // Present the canvas
        canvas.present();

        // Wait for user input to close the window
        let mut event_pump = sdl_context.event_pump()?;
        'running: loop {
            for event in event_pump.poll_iter() {
                match event {
                    sdl2::event::Event::Quit { .. } |
                    sdl2::event::Event::KeyDown { .. } => break 'running,
                    _ => {}
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        Ok(())
    }

    // Draw stats as rectangles (no TTF dependency)
    fn draw_stats_as_rectangles(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        // Create a visual representation of statistics using rectangles and lines
        canvas.set_draw_color(Color::RGB(0, 0, 0));

        // Draw labels (as simple rectangles of different widths)
        let labels = [
            "Total Vehicles",
            "Max Velocity",
            "Min Velocity",
            "Max Time",
            "Min Time",
            "Close Calls"
        ];

        // Draw separator line at the top
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.draw_line(Point::new(50, 40), Point::new(450, 40))?;

        // Stats values (normalized for display)
        let values = [
            self.total_vehicles as f64,
            self.max_velocity / 3.0, // Scale down for display
            if self.min_velocity == f64::MAX { 0.0 } else { self.min_velocity / 3.0 },
            self.max_time as f64 / 100.0, // Scale down for display
            if self.min_time == u32::MAX { 0.0 } else { self.min_time as f64 / 100.0 },
            self.close_calls as f64
        ];

        // Colors for each stat
        let colors = [
            Color::RGB(220, 0, 0),    // Red
            Color::RGB(0, 190, 0),    // Green
            Color::RGB(0, 120, 200),  // Blue
            Color::RGB(200, 200, 0),  // Yellow
            Color::RGB(200, 0, 200),  // Purple
            Color::RGB(0, 180, 180)   // Cyan
        ];

        let mut y_pos = 60;

        // Draw each stat as a label (dark rectangle) and value (colored bar)
        for i in 0..labels.len() {
            // Draw label area
            canvas.set_draw_color(Color::RGB(50, 50, 50));
            canvas.fill_rect(Rect::new(50, y_pos, 150, 25))?;

            // Draw value as a colored bar
            let bar_width = (values[i].min(300.0) as u32).max(5); // Ensure minimum visibility
            canvas.set_draw_color(colors[i]);
            canvas.fill_rect(Rect::new(210, y_pos, bar_width, 25))?;

            y_pos += 45;
        }

        // Draw a small explanation at the bottom
        canvas.set_draw_color(Color::RGB(100, 100, 100));
        canvas.draw_line(Point::new(50, y_pos + 10), Point::new(450, y_pos + 10))?;

        y_pos += 20;
        canvas.fill_rect(Rect::new(50, y_pos, 400, 15))?;

        Ok(())
    }
}