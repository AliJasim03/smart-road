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
    // Additional statistics for 6-lane system
    lane_usage: [[u32; 6]; 4], // Direction[North,South,East,West], Lane[0-5]
    avg_waiting_time: f64,
    max_congestion: u32,
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
            lane_usage: [[0; 6]; 4],
            avg_waiting_time: 0.0,
            max_congestion: 0,
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

        // Track max congestion
        let current_congestion = vehicles.len() as u32;
        if current_congestion > self.max_congestion {
            self.max_congestion = current_congestion;
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

        // Update lane usage
        let dir_index = match vehicle.direction {
            crate::vehicle::Direction::North => 0,
            crate::vehicle::Direction::South => 1,
            crate::vehicle::Direction::East => 2,
            crate::vehicle::Direction::West => 3,
        };
        let lane_index = vehicle.lane.min(5);
        self.lane_usage[dir_index][lane_index] += 1;

        // Update average waiting time
        let new_waiting_time = vehicle.time_in_intersection as f64;
        let total_vehicles = self.completed_vehicles.len() as f64 + 1.0;
        self.avg_waiting_time = (self.avg_waiting_time * (total_vehicles - 1.0) + new_waiting_time) / total_vehicles;

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
            .window("Smart Road Simulation - Statistics", 600, 500)
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

        // Draw statistics
        self.draw_statistics(&mut canvas)?;

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

    // Draw statistics
    fn draw_statistics(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        // Draw title
        canvas.set_draw_color(Color::RGB(50, 50, 100));
        canvas.fill_rect(Rect::new(0, 0, 600, 40))?;

        // Draw basic stats
        self.draw_basic_stats(canvas, 50)?;

        // Draw lane usage stats
        self.draw_lane_usage(canvas, 250)?;

        // Draw additional stats
        self.draw_additional_stats(canvas, 400)?;

        Ok(())
    }

    // Draw basic statistics
    fn draw_basic_stats(&self, canvas: &mut Canvas<Window>, start_y: i32) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.draw_line(Point::new(50, start_y), Point::new(550, start_y))?;

        // Title for this section
        canvas.set_draw_color(Color::RGB(0, 0, 128));
        canvas.fill_rect(Rect::new(50, start_y + 10, 500, 20))?;

        // Stats values
        let stats = [
            ("Total Vehicles", self.total_vehicles as f64, Color::RGB(220, 0, 0)),
            ("Max Velocity", self.max_velocity, Color::RGB(0, 190, 0)),
            ("Min Velocity", if self.min_velocity == f64::MAX { 0.0 } else { self.min_velocity }, Color::RGB(0, 120, 200)),
            ("Max Time (ms)", self.max_time as f64, Color::RGB(200, 200, 0)),
            ("Min Time (ms)", if self.min_time == u32::MAX { 0.0 } else { self.min_time as f64 }, Color::RGB(200, 0, 200)),
            ("Close Calls", self.close_calls as f64, Color::RGB(0, 180, 180))
        ];

        let mut y_pos = start_y + 40;
        for (i, (label, value, color)) in stats.iter().enumerate() {
            // Label background
            canvas.set_draw_color(Color::RGB(240, 240, 240));
            canvas.fill_rect(Rect::new(50, y_pos, 150, 25))?;

            // Value bar
            let bar_width = (value.min(300.0) as u32).max(5); // Ensure minimum visibility
            canvas.set_draw_color(*color);
            canvas.fill_rect(Rect::new(210, y_pos, bar_width, 25))?;

            // Value text position indicator
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            canvas.draw_rect(Rect::new(210 + bar_width as i32, y_pos, 2, 25))?;

            y_pos += 30;
        }

        Ok(())
    }

    // Draw lane usage statistics
    fn draw_lane_usage(&self, canvas: &mut Canvas<Window>, start_y: i32) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.draw_line(Point::new(50, start_y), Point::new(550, start_y))?;

        // Title for this section
        canvas.set_draw_color(Color::RGB(0, 100, 0));
        canvas.fill_rect(Rect::new(50, start_y + 10, 500, 20))?;

        // Draw lane usage bars for each direction
        let directions = ["North", "South", "East", "West"];
        let colors = [
            Color::RGB(0, 0, 200), // North - Blue
            Color::RGB(200, 0, 0), // South - Red
            Color::RGB(0, 150, 0), // East - Green
            Color::RGB(150, 100, 0), // West - Brown
        ];

        let mut y_pos = start_y + 40;
        for dir in 0..4 {
            // Direction label
            canvas.set_draw_color(Color::RGB(240, 240, 240));
            canvas.fill_rect(Rect::new(50, y_pos, 80, 25))?;

            // Lane usage bars
            let bar_height = 25;
            let max_lane_usage = self.lane_usage[dir].iter().max().copied().unwrap_or(1).max(1) as f64;

            for lane in 0..6 {
                let usage = self.lane_usage[dir][lane] as f64;
                let bar_width = ((usage / max_lane_usage) * 400.0).min(400.0).max(1.0) as u32;

                canvas.set_draw_color(colors[dir]);
                canvas.fill_rect(Rect::new(140 + (lane as i32 * 65), y_pos, bar_width, bar_height as u32))?;

                // Lane number
                canvas.set_draw_color(Color::RGB(0, 0, 0));
                canvas.draw_rect(Rect::new(140 + (lane as i32 * 65), y_pos + bar_height, 20, 10))?;
            }

            y_pos += 40;
        }

        Ok(())
    }

    // Draw additional statistics
    fn draw_additional_stats(&self, canvas: &mut Canvas<Window>, start_y: i32) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.draw_line(Point::new(50, start_y), Point::new(550, start_y))?;

        // Title for this section
        canvas.set_draw_color(Color::RGB(128, 0, 128));
        canvas.fill_rect(Rect::new(50, start_y + 10, 500, 20))?;

        // Additional stats
        let stats = [
            ("Avg Waiting Time (ms)", self.avg_waiting_time, Color::RGB(150, 50, 200)),
            ("Max Congestion", self.max_congestion as f64, Color::RGB(200, 100, 50)),
        ];

        let mut y_pos = start_y + 40;
        for (label, value, color) in stats {
            // Label background
            canvas.set_draw_color(Color::RGB(240, 240, 240));
            canvas.fill_rect(Rect::new(50, y_pos, 200, 25))?;

            // Value bar
            let bar_width = (value.min(300.0) as u32).max(5); // Ensure minimum visibility
            canvas.set_draw_color(color);
            canvas.fill_rect(Rect::new(260, y_pos, bar_width, 25))?;

            y_pos += 30;
        }

        Ok(())
    }
}