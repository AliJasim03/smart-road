use std::collections::VecDeque;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::rect::Point;

use crate::vehicle::{Vehicle, Direction, Route};
use crate::game::VehicleStatistics; // Import the new struct

pub struct Statistics {
    total_vehicles: u32,
    max_velocity: f64,
    min_velocity: f64,
    max_time: u32,
    min_time: u32,
    close_calls: u32,
    completed_vehicles: Vec<VehicleStatistics>, // Changed to use VehicleStatistics
    // Additional statistics for enhanced tracking
    lane_usage: [[u32; 6]; 4], // Direction[North,South,East,West], Lane[0-5]
    avg_waiting_time: f64,
    max_congestion: u32,
    total_processing_time: f64,
    collision_count: u32,
    successful_intersections: u32,
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
            total_processing_time: 0.0,
            collision_count: 0,
            successful_intersections: 0,
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

    // CHANGED: Record vehicle statistics without requiring the full Vehicle struct
    pub fn record_vehicle_exit_stats(&mut self, vehicle_stats: VehicleStatistics) {
        // Update velocity stats
        if vehicle_stats.current_velocity > self.max_velocity {
            self.max_velocity = vehicle_stats.current_velocity;
        }
        if vehicle_stats.current_velocity < self.min_velocity && vehicle_stats.current_velocity > 0.0 {
            self.min_velocity = vehicle_stats.current_velocity;
        }

        // Update time stats
        if vehicle_stats.time_in_intersection > self.max_time {
            self.max_time = vehicle_stats.time_in_intersection;
        }
        if vehicle_stats.time_in_intersection < self.min_time && vehicle_stats.time_in_intersection > 0 {
            self.min_time = vehicle_stats.time_in_intersection;
        }

        // Update lane usage
        let dir_index = match vehicle_stats.direction {
            Direction::North => 0,
            Direction::South => 1,
            Direction::East => 2,
            Direction::West => 3,
        };
        let lane_index = vehicle_stats.lane.min(5);
        self.lane_usage[dir_index][lane_index] += 1;

        // Update average waiting time
        let new_waiting_time = vehicle_stats.time_in_intersection as f64;
        let total_vehicles = self.completed_vehicles.len() as f64 + 1.0;
        self.avg_waiting_time = (self.avg_waiting_time * (total_vehicles - 1.0) + new_waiting_time) / total_vehicles;

        // Calculate total processing time
        let processing_time = vehicle_stats.start_time.elapsed().as_secs_f64();
        self.total_processing_time += processing_time;

        // Count successful intersection
        self.successful_intersections += 1;

        // Add to completed vehicles list
        self.completed_vehicles.push(vehicle_stats);
    }

    // DEPRECATED: Keep this method for backward compatibility but mark it as deprecated
    #[allow(dead_code)]
    pub fn record_vehicle_exit(&mut self, vehicle: Vehicle) {
        // Convert Vehicle to VehicleStatistics
        let vehicle_stats = VehicleStatistics {
            id: vehicle.id,
            direction: vehicle.direction,
            lane: vehicle.lane,
            route: vehicle.route,
            current_velocity: vehicle.current_velocity,
            time_in_intersection: vehicle.time_in_intersection,
            start_time: vehicle.start_time,
        };

        self.record_vehicle_exit_stats(vehicle_stats);
    }

    // Update close calls count
    pub fn add_close_call(&mut self) {
        self.close_calls += 1;
    }

    // Record a collision (hopefully this never gets called!)
    pub fn add_collision(&mut self) {
        self.collision_count += 1;
    }

    // Get comprehensive statistics summary
    pub fn get_summary(&self) -> StatisticsSummary {
        StatisticsSummary {
            total_vehicles: self.total_vehicles,
            completed_vehicles: self.completed_vehicles.len() as u32,
            max_velocity: self.max_velocity,
            min_velocity: if self.min_velocity == f64::MAX { 0.0 } else { self.min_velocity },
            avg_velocity: self.calculate_average_velocity(),
            max_time: self.max_time,
            min_time: if self.min_time == u32::MAX { 0 } else { self.min_time },
            avg_time: self.avg_waiting_time,
            close_calls: self.close_calls,
            collision_count: self.collision_count,
            successful_intersections: self.successful_intersections,
            max_congestion: self.max_congestion,
            efficiency_rating: self.calculate_efficiency_rating(),
        }
    }

    // Calculate average velocity of all completed vehicles
    fn calculate_average_velocity(&self) -> f64 {
        if self.completed_vehicles.is_empty() {
            return 0.0;
        }

        let total_velocity: f64 = self.completed_vehicles.iter()
            .map(|v| v.current_velocity)
            .sum();

        total_velocity / self.completed_vehicles.len() as f64
    }

    // Calculate efficiency rating (0-100 scale)
    fn calculate_efficiency_rating(&self) -> f64 {
        if self.successful_intersections == 0 {
            return 100.0; // No vehicles processed yet
        }

        let base_score = 100.0;

        // Deduct points for collisions (major penalty)
        let collision_penalty = self.collision_count as f64 * 50.0;

        // Deduct points for close calls (minor penalty)
        let close_call_penalty = self.close_calls as f64 * 2.0;

        // Deduct points for high congestion
        let congestion_penalty = if self.max_congestion > 10 {
            (self.max_congestion - 10) as f64 * 3.0
        } else {
            0.0
        };

        // Deduct points for very slow average times
        let time_penalty = if self.avg_waiting_time > 5000.0 { // 5 seconds
            (self.avg_waiting_time - 5000.0) / 100.0
        } else {
            0.0
        };

        let final_score = base_score - collision_penalty - close_call_penalty - congestion_penalty - time_penalty;
        final_score.max(0.0).min(100.0)
    }

    // Display statistics in a console-friendly format
    pub fn print_summary(&self) {
        let summary = self.get_summary();

        println!("\n╔══════════════════════════════════════╗");
        println!("║         SIMULATION STATISTICS        ║");
        println!("╠══════════════════════════════════════╣");
        println!("║ Total Vehicles:              {:6} ║", summary.total_vehicles);
        println!("║ Completed Intersections:     {:6} ║", summary.completed_vehicles);
        println!("║ Successful Rate:             {:5.1}% ║",
                 if summary.total_vehicles > 0 {
                     (summary.successful_intersections as f64 / summary.total_vehicles as f64) * 100.0
                 } else { 0.0 });
        println!("╠══════════════════════════════════════╣");
        println!("║ Max Velocity:            {:8.1} px/s ║", summary.max_velocity);
        println!("║ Min Velocity:            {:8.1} px/s ║", summary.min_velocity);
        println!("║ Avg Velocity:            {:8.1} px/s ║", summary.avg_velocity);
        println!("╠══════════════════════════════════════╣");
        println!("║ Max Intersection Time:     {:6} ms ║", summary.max_time);
        println!("║ Min Intersection Time:     {:6} ms ║", summary.min_time);
        println!("║ Avg Intersection Time:   {:8.1} ms ║", summary.avg_time);
        println!("╠══════════════════════════════════════╣");
        println!("║ Close Calls:                  {:6} ║", summary.close_calls);
        println!("║ Collisions:                   {:6} ║", summary.collision_count);
        println!("║ Max Congestion:               {:6} ║", summary.max_congestion);
        println!("║ Efficiency Rating:          {:5.1}% ║", summary.efficiency_rating);
        println!("╚══════════════════════════════════════╝");

        // Print lane usage statistics
        println!("\n╔══════════════════════════════════════╗");
        println!("║           LANE USAGE STATS           ║");
        println!("╠══════════════════════════════════════╣");
        let directions = ["North", "South", "East ", "West "];
        for (i, direction) in directions.iter().enumerate() {
            print!("║ {}: ", direction);
            for lane in 0..6 {
                print!("{:4}", self.lane_usage[i][lane]);
            }
            println!(" ║");
        }
        println!("╚══════════════════════════════════════╝\n");
    }

    // Display statistics - simplified version for compatibility
    pub fn display(&self) -> Result<(), String> {
        self.print_summary();

        println!("Press Enter to continue...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Failed to read line");

        Ok(())
    }

    // Create a simple text-based statistics window
    pub fn display_window(&self) -> Result<(), String> {
        // Create a new window for statistics
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

        let window = video_subsystem
            .window("Smart Road Simulation - Final Statistics", 800, 600)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;

        let mut canvas = window
            .into_canvas()
            .accelerated()
            .build()
            .map_err(|e| e.to_string())?;

        // Clear the canvas with a dark background
        canvas.set_draw_color(Color::RGB(30, 30, 30));
        canvas.clear();

        // Draw statistics boxes
        self.draw_statistics_boxes(&mut canvas)?;

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

    // Draw statistics as colored boxes (since we don't have TTF text rendering)
    fn draw_statistics_boxes(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let summary = self.get_summary();

        // Title bar
        canvas.set_draw_color(Color::RGB(50, 100, 150));
        canvas.fill_rect(Rect::new(50, 50, 700, 40))?;

        // Main statistics area
        canvas.set_draw_color(Color::RGB(70, 70, 70));
        canvas.fill_rect(Rect::new(50, 100, 700, 450))?;

        // Draw value indicators using colored bars
        let mut y_pos = 120;
        let stats = [
            ("Total Vehicles", summary.total_vehicles as f64, Color::RGB(100, 200, 100)),
            ("Completed", summary.completed_vehicles as f64, Color::RGB(150, 250, 150)),
            ("Max Velocity", summary.max_velocity / 10.0, Color::RGB(255, 100, 100)), // Scaled down
            ("Avg Velocity", summary.avg_velocity / 10.0, Color::RGB(255, 150, 100)),
            ("Max Time", summary.max_time as f64 / 100.0, Color::RGB(100, 100, 255)), // Scaled down
            ("Close Calls", summary.close_calls as f64, Color::RGB(255, 255, 100)),
            ("Efficiency", summary.efficiency_rating, Color::RGB(100, 255, 255)),
        ];

        for (i, (name, value, color)) in stats.iter().enumerate() {
            // Label area
            canvas.set_draw_color(Color::RGB(90, 90, 90));
            canvas.fill_rect(Rect::new(70, y_pos, 200, 30))?;

            // Value bar (scaled to fit in 400 pixels max)
            let bar_width = (value * 4.0).min(400.0).max(5.0) as u32;
            canvas.set_draw_color(*color);
            canvas.fill_rect(Rect::new(280, y_pos, bar_width, 30))?;

            // Border
            canvas.set_draw_color(Color::RGB(200, 200, 200));
            canvas.draw_rect(Rect::new(70, y_pos, 610, 30))?;

            y_pos += 50;
        }

        // Draw lane usage visualization
        y_pos = 450;
        canvas.set_draw_color(Color::RGB(50, 50, 50));
        canvas.fill_rect(Rect::new(70, y_pos, 660, 80))?;

        // Draw lane usage bars for each direction
        let directions = ["North", "South", "East", "West"];
        let colors = [
            Color::RGB(255, 100, 100), // North - Red
            Color::RGB(100, 255, 100), // South - Green
            Color::RGB(100, 100, 255), // East - Blue
            Color::RGB(255, 255, 100), // West - Yellow
        ];

        for (dir_idx, color) in colors.iter().enumerate() {
            let dir_y = y_pos + 10 + (dir_idx as i32 * 15);

            for lane in 0..6 {
                let usage = self.lane_usage[dir_idx][lane];
                let bar_width = (usage * 3).min(30); // Scale the bars

                canvas.set_draw_color(*color);
                canvas.fill_rect(Rect::new(
                    90 + (lane as i32 * 100),
                    dir_y,
                    bar_width,
                    10
                ))?;
            }
        }

        Ok(())
    }
}

// Structure to hold statistics summary
pub struct StatisticsSummary {
    pub total_vehicles: u32,
    pub completed_vehicles: u32,
    pub max_velocity: f64,
    pub min_velocity: f64,
    pub avg_velocity: f64,
    pub max_time: u32,
    pub min_time: u32,
    pub avg_time: f64,
    pub close_calls: u32,
    pub collision_count: u32,
    pub successful_intersections: u32,
    pub max_congestion: u32,
    pub efficiency_rating: f64,
}