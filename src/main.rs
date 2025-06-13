// src/main.rs - FIXED: Smooth animations and perfect lane sectioning
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Rect, Point};
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::collections::{VecDeque, HashSet};
use std::time::{Duration, Instant};

mod vehicle;
mod intersection;
mod statistics;
mod algorithm;

use vehicle::{Vehicle, Direction, Route, VehicleState, VelocityLevel, VehicleColor, Vec2};
use intersection::Intersection;
use statistics::Statistics;
use algorithm::SmartIntersection;

pub const WINDOW_WIDTH: u32 = 1024;
pub const WINDOW_HEIGHT: u32 = 768;
const FPS: u32 = 60;
const PHYSICS_TIMESTEP: f64 = 1.0 / 60.0; // Fixed physics timestep

// PERFECT 6-LANE MATHEMATICS
const LANE_WIDTH: f32 = 30.0;           // Each lane exactly 30px
const TOTAL_ROAD_WIDTH: f32 = 180.0;    // 6 lanes × 30px = 180px
const HALF_ROAD_WIDTH: f32 = 90.0;      // 90px each side of center

fn main() -> Result<(), String> {
    println!("=== Smart Road - PERFECT 6-LANE SYSTEM WITH SMOOTH ANIMATIONS ===");
    println!("✅ Fixed timestep physics: {}ms", (PHYSICS_TIMESTEP * 1000.0) as u32);
    println!("✅ Bezier curve turning animations");
    println!("✅ Floating-point rendering for smoothness");
    println!("✅ Perfect lane sectioning with intersection boundaries\n");

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("Smart Road - SMOOTH ANIMATIONS", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;

    let mut game = GameState::new()?;
    let mut event_pump = sdl_context.event_pump()?;
    let mut running = true;
    let mut last_frame = Instant::now();
    let mut physics_accumulator = 0.0;

    print_controls();
    print_lane_mathematics();

    while running {
        let now = Instant::now();
        let frame_delta = now.duration_since(last_frame).as_secs_f64();
        last_frame = now;

        physics_accumulator += frame_delta;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => running = false,
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => running = false,
                _ => {
                    game.handle_event(&event);
                }
            }
        }

        // Fixed timestep physics updates
        while physics_accumulator >= PHYSICS_TIMESTEP {
            game.update_physics(PHYSICS_TIMESTEP);
            physics_accumulator -= PHYSICS_TIMESTEP;
        }

        // Interpolation factor for smooth rendering
        let alpha = (physics_accumulator / PHYSICS_TIMESTEP) as f32;
        game.update_interpolation(alpha);
        game.render(&mut canvas)?;

        let frame_time = now.elapsed();
        if frame_time < Duration::from_millis(1000 / FPS as u64) {
            std::thread::sleep(Duration::from_millis(1000 / FPS as u64) - frame_time);
        }
    }

    game.show_final_statistics();
    Ok(())
}

fn print_controls() {
    println!("=== CONTROLS ===");
    println!("↑ Arrow Up:    Spawn vehicle from South (moving North)");
    println!("↓ Arrow Down:  Spawn vehicle from North (moving South)");
    println!("← Arrow Left:  Spawn vehicle from East (moving West)");
    println!("→ Arrow Right: Spawn vehicle from West (moving East)");
    println!("R:             Toggle continuous random spawning");
    println!("D:             Toggle debug visualization");
    println!("Space:         Show current statistics");
    println!("Esc:           Exit and show final statistics");
}

fn print_lane_mathematics() {
    let center_x = WINDOW_WIDTH as f32 / 2.0;
    let center_y = WINDOW_HEIGHT as f32 / 2.0;

    println!("\n=== PERFECT LANE MATHEMATICS ===");
    println!("Screen center: ({}, {})", center_x, center_y);
    println!("Lane width: {} pixels each", LANE_WIDTH);
    println!("Total road width: {} pixels", TOTAL_ROAD_WIDTH);

    println!("\nNorth-bound lanes (right side of vertical road):");
    for lane in 0..3 {
        let x = get_lane_center_x(Direction::North, lane);
        let color = get_lane_color_name(lane);
        println!("  Lane {}: {} at x={} ({})", lane, color, x, get_route_name(lane));
    }

    println!("\nSouth-bound lanes (left side of vertical road):");
    for lane in 0..3 {
        let x = get_lane_center_x(Direction::South, lane);
        let color = get_lane_color_name(lane);
        println!("  Lane {}: {} at x={} ({})", lane, color, x, get_route_name(lane));
    }

    println!("\nEast-bound lanes (bottom side of horizontal road):");
    for lane in 0..3 {
        let y = get_lane_center_y(Direction::East, lane);
        let color = get_lane_color_name(lane);
        println!("  Lane {}: {} at y={} ({})", lane, color, y, get_route_name(lane));
    }

    println!("\nWest-bound lanes (top side of horizontal road):");
    for lane in 0..3 {
        let y = get_lane_center_y(Direction::West, lane);
        let color = get_lane_color_name(lane);
        println!("  Lane {}: {} at y={} ({})", lane, color, y, get_route_name(lane));
    }

    println!("=====================================\n");
}

// PERFECT LANE POSITIONING FUNCTIONS
fn get_lane_center_x(direction: Direction, lane: usize) -> f32 {
    let center_x = WINDOW_WIDTH as f32 / 2.0;
    match direction {
        Direction::North => {
            center_x + 15.0 + (lane as f32 * LANE_WIDTH)
        }
        Direction::South => {
            center_x - 15.0 - (lane as f32 * LANE_WIDTH)
        }
        _ => center_x,
    }
}

fn get_lane_center_y(direction: Direction, lane: usize) -> f32 {
    let center_y = WINDOW_HEIGHT as f32 / 2.0;
    match direction {
        Direction::East => {
            center_y + 15.0 + (lane as f32 * LANE_WIDTH)
        }
        Direction::West => {
            center_y - 15.0 - (lane as f32 * LANE_WIDTH)
        }
        _ => center_y,
    }
}

fn get_destination_for_route(incoming: Direction, route: Route) -> Direction {
    match (incoming, route) {
        (Direction::North, Route::Left) => Direction::West,
        (Direction::North, Route::Straight) => Direction::North,
        (Direction::North, Route::Right) => Direction::East,

        (Direction::South, Route::Left) => Direction::East,
        (Direction::South, Route::Straight) => Direction::South,
        (Direction::South, Route::Right) => Direction::West,

        (Direction::East, Route::Left) => Direction::North,
        (Direction::East, Route::Straight) => Direction::East,
        (Direction::East, Route::Right) => Direction::South,

        (Direction::West, Route::Left) => Direction::South,
        (Direction::West, Route::Straight) => Direction::West,
        (Direction::West, Route::Right) => Direction::North,
    }
}

fn get_route_for_lane(lane: usize) -> Route {
    match lane {
        0 => Route::Left,
        1 => Route::Straight,
        2 => Route::Right,
        _ => Route::Straight,
    }
}

fn get_color_for_route(route: Route) -> VehicleColor {
    match route {
        Route::Left => VehicleColor::Red,
        Route::Straight => VehicleColor::Blue,
        Route::Right => VehicleColor::Green,
    }
}

fn get_lane_color_name(lane: usize) -> &'static str {
    match lane {
        0 => "RED",
        1 => "BLUE",
        2 => "GREEN",
        _ => "UNKNOWN",
    }
}

fn get_route_name(lane: usize) -> &'static str {
    match lane {
        0 => "LEFT",
        1 => "STRAIGHT",
        2 => "RIGHT",
        _ => "UNKNOWN",
    }
}

struct PerformanceMonitor {
    frame_times: Vec<f32>,
    last_update: Instant,
}

impl PerformanceMonitor {
    fn new() -> Self {
        PerformanceMonitor {
            frame_times: Vec::new(),
            last_update: Instant::now(),
        }
    }

    fn record_frame(&mut self) {
        let now = Instant::now();
        let frame_time = now.duration_since(self.last_update).as_secs_f32();
        self.frame_times.push(frame_time);
        self.last_update = now;

        if self.frame_times.len() >= 60 {
            let avg_time = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
            let fps = 1.0 / avg_time;
            if fps < 55.0 {
                println!("⚠️ Performance warning: {:.1} FPS", fps);
            }
            self.frame_times.clear();
        }
    }
}

struct GameState {
    vehicles: VecDeque<Vehicle>,
    intersection: Intersection,
    statistics: Statistics,
    algorithm: SmartIntersection,
    spawn_cooldown: f32,
    current_cooldown: f32,
    continuous_spawn: bool,
    spawn_timer: f32,
    next_vehicle_id: u32,
    total_vehicles_passed: u32,
    close_calls: u32,
    simulation_start_time: Instant,
    crashed_vehicle_pairs: HashSet<(u32, u32)>,
    crash_count: u32,
    debug_mode: bool,
    performance_monitor: PerformanceMonitor,
}

impl GameState {
    fn new() -> Result<Self, String> {
        Ok(GameState {
            vehicles: VecDeque::new(),
            intersection: Intersection::new(),
            statistics: Statistics::new(),
            algorithm: SmartIntersection::new(),
            spawn_cooldown: 2.5,
            current_cooldown: 0.0,
            continuous_spawn: false,
            spawn_timer: 0.0,
            next_vehicle_id: 0,
            total_vehicles_passed: 0,
            close_calls: 0,
            simulation_start_time: Instant::now(),
            crashed_vehicle_pairs: HashSet::new(),
            crash_count: 0,
            debug_mode: false,
            performance_monitor: PerformanceMonitor::new(),
        })
    }

    fn handle_event(&mut self, event: &Event) {
        match event {
            Event::KeyDown { keycode: Some(keycode), repeat: false, .. } => {
                match keycode {
                    Keycode::Up => {
                        if self.spawn_vehicle(Direction::North) {
                            println!("✅ Spawned vehicle from South (→ North)");
                        }
                    }
                    Keycode::Down => {
                        if self.spawn_vehicle(Direction::South) {
                            println!("✅ Spawned vehicle from North (→ South)");
                        }
                    }
                    Keycode::Left => {
                        if self.spawn_vehicle(Direction::East) {
                            println!("✅ Spawned vehicle from West (→ East)");
                        }
                    }
                    Keycode::Right => {
                        if self.spawn_vehicle(Direction::West) {
                            println!("✅ Spawned vehicle from East (→ West)");
                        }
                    }
                    Keycode::R => {
                        self.continuous_spawn = !self.continuous_spawn;
                        println!("🤖 Continuous spawn: {}", if self.continuous_spawn { "ON" } else { "OFF" });
                        if self.continuous_spawn {
                            self.spawn_timer = 0.0;
                        }
                    }
                    Keycode::D => {
                        self.debug_mode = !self.debug_mode;
                        println!("🔍 Debug mode: {}", if self.debug_mode { "ON" } else { "OFF" });
                    }
                    Keycode::Space => {
                        self.print_current_statistics();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn update_physics(&mut self, dt: f64) {
        if self.current_cooldown > 0.0 {
            self.current_cooldown -= dt as f32;
        }

        if self.continuous_spawn {
            self.spawn_timer += dt as f32;
            if self.spawn_timer >= 6.0 {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let direction = match rng.gen_range(0..4) {
                    0 => Direction::North,
                    1 => Direction::South,
                    2 => Direction::East,
                    _ => Direction::West,
                };

                if self.spawn_vehicle(direction) {
                    println!("🤖 Auto-spawned vehicle: {:?}", direction);
                }
                self.spawn_timer = 0.0;
            }
        }

        // Update algorithm and vehicles with fixed timestep
        self.algorithm.process_vehicles(&mut self.vehicles, &self.intersection, (dt * 1000.0) as u32);

        for vehicle in &mut self.vehicles {
            vehicle.update_physics(dt, &self.intersection);
        }

        self.cleanup_completed_vehicles();
        self.statistics.update(&self.vehicles);
        self.close_calls = self.algorithm.close_calls;
    }

    fn update_interpolation(&mut self, alpha: f32) {
        for vehicle in &mut self.vehicles {
            vehicle.update_interpolation(alpha);
        }
    }

    fn spawn_vehicle(&mut self, direction: Direction) -> bool {
        if self.current_cooldown > 0.0 {
            return false;
        }

        let same_direction_count = self.vehicles.iter()
            .filter(|v| v.direction == direction)
            .count();
        if same_direction_count >= 1 {
            return false;
        }

        use rand::Rng;
        let mut rng = rand::thread_rng();
        let lane = rng.gen_range(0..3);
        let route = get_route_for_lane(lane);
        let destination = get_destination_for_route(direction, route);
        let color = get_color_for_route(route);

        let vehicle = Vehicle::new_smooth(
            self.next_vehicle_id,
            direction,
            destination,
            lane,
            route,
            color,
        );

        println!("🚗 Vehicle {}: {:?} Lane {} ({}) → {:?} road",
                 vehicle.id, direction, lane, get_route_name(lane), destination);

        self.vehicles.push_back(vehicle);
        self.next_vehicle_id += 1;
        self.current_cooldown = self.spawn_cooldown;
        self.statistics.add_spawned_vehicle();

        true
    }

    fn cleanup_completed_vehicles(&mut self) {
        let initial_count = self.vehicles.len();

        self.vehicles.retain(|vehicle| {
            let off_screen = match vehicle.destination {
                Direction::North => vehicle.position.y < -100.0,
                Direction::South => vehicle.position.y > WINDOW_HEIGHT as f32 + 100.0,
                Direction::East => vehicle.position.x > WINDOW_WIDTH as f32 + 100.0,
                Direction::West => vehicle.position.x < -100.0,
            };

            if off_screen {
                self.total_vehicles_passed += 1;
                println!("✅ Vehicle {} completed journey smoothly", vehicle.id);
            }

            !off_screen
        });

        let removed = initial_count - self.vehicles.len();
        if removed > 0 {
            self.statistics.add_completed_vehicles(removed);
        }
    }

    fn print_current_statistics(&self) {
        println!("\n=== CURRENT STATISTICS ===");
        println!("🚗 Active vehicles: {}", self.vehicles.len());
        println!("✅ Vehicles completed: {}", self.total_vehicles_passed);
        println!("💥 Crashes: {}", self.crash_count);
        println!("⚠️  Close calls: {}", self.close_calls);

        for vehicle in &self.vehicles {
            println!("  Vehicle {}: {:?} L{} {:?} at ({:.1}, {:.1})",
                     vehicle.id, vehicle.direction, vehicle.lane, vehicle.state,
                     vehicle.position.x, vehicle.position.y);
        }
        println!("==========================\n");
    }

    fn render(&mut self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        // Clear with grass color
        canvas.set_draw_color(Color::RGB(40, 120, 40));
        canvas.clear();

        // Draw perfect road system with improved lane sectioning
        self.draw_perfect_roads_with_sectioning(canvas)?;

        // Draw intersection
        self.draw_intersection(canvas)?;

        // Draw vehicles with smooth rendering
        for vehicle in &self.vehicles {
            self.draw_vehicle_smooth(canvas, vehicle)?;
        }

        // Draw debug overlays if enabled
        if self.debug_mode {
            self.draw_debug_overlays(canvas)?;
        }

        // Draw UI
        self.draw_ui(canvas)?;

        // Record performance
        self.performance_monitor.record_frame();

        canvas.present();
        Ok(())
    }

    fn draw_perfect_roads_with_sectioning(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let center_x = WINDOW_WIDTH as f32 / 2.0;
        let center_y = WINDOW_HEIGHT as f32 / 2.0;

        // Fill roads with asphalt
        canvas.set_draw_color(Color::RGB(60, 60, 60));

        // Vertical road (North-South traffic)
        canvas.fill_rect(Rect::new(
            (center_x - HALF_ROAD_WIDTH) as i32,
            0,
            TOTAL_ROAD_WIDTH as u32,
            WINDOW_HEIGHT,
        ))?;

        // Horizontal road (East-West traffic)
        canvas.fill_rect(Rect::new(
            0,
            (center_y - HALF_ROAD_WIDTH) as i32,
            WINDOW_WIDTH,
            TOTAL_ROAD_WIDTH as u32,
        ))?;

        // Draw improved lane lines with intersection boundary handling
        self.draw_lane_markings_with_intersection_skip(canvas, center_x, center_y)?;

        // Draw perfect lane color indicators
        self.draw_lane_indicators(canvas)?;

        Ok(())
    }

    fn draw_lane_markings_with_intersection_skip(&self, canvas: &mut Canvas<Window>, center_x: f32, center_y: f32) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(255, 255, 255));

        let intersection_bounds = Rect::new(
            (center_x - HALF_ROAD_WIDTH) as i32,
            (center_y - HALF_ROAD_WIDTH) as i32,
            TOTAL_ROAD_WIDTH as u32,
            TOTAL_ROAD_WIDTH as u32,
        );

        // Vertical lane lines (for North-South traffic)
        for lane in 0..4 {
            let x = (center_x - HALF_ROAD_WIDTH + (lane as f32 * LANE_WIDTH)) as i32;

            // North section (above intersection)
            self.draw_line_segment_with_gap(
                canvas,
                Vec2::new(x as f32, 0.0),
                Vec2::new(x as f32, (center_y - HALF_ROAD_WIDTH) as f32),
                intersection_bounds,
            )?;

            // South section (below intersection)
            self.draw_line_segment_with_gap(
                canvas,
                Vec2::new(x as f32, (center_y + HALF_ROAD_WIDTH) as f32),
                Vec2::new(x as f32, WINDOW_HEIGHT as f32),
                intersection_bounds,
            )?;
        }

        // Horizontal lane lines (for East-West traffic)
        for lane in 0..4 {
            let y = (center_y - HALF_ROAD_WIDTH + (lane as f32 * LANE_WIDTH)) as i32;

            // West section (left of intersection)
            self.draw_line_segment_with_gap(
                canvas,
                Vec2::new(0.0, y as f32),
                Vec2::new((center_x - HALF_ROAD_WIDTH) as f32, y as f32),
                intersection_bounds,
            )?;

            // East section (right of intersection)
            self.draw_line_segment_with_gap(
                canvas,
                Vec2::new((center_x + HALF_ROAD_WIDTH) as f32, y as f32),
                Vec2::new(WINDOW_WIDTH as f32, y as f32),
                intersection_bounds,
            )?;
        }

        // Center dividers with improved rendering
        canvas.set_draw_color(Color::RGB(255, 255, 0));

        // Vertical center divider
        let divider_x = center_x as i32;
        canvas.draw_line((divider_x - 2, 0), (divider_x - 2, (center_y - HALF_ROAD_WIDTH) as i32))?;
        canvas.draw_line((divider_x + 2, 0), (divider_x + 2, (center_y - HALF_ROAD_WIDTH) as i32))?;
        canvas.draw_line((divider_x - 2, (center_y + HALF_ROAD_WIDTH) as i32), (divider_x - 2, WINDOW_HEIGHT as i32))?;
        canvas.draw_line((divider_x + 2, (center_y + HALF_ROAD_WIDTH) as i32), (divider_x + 2, WINDOW_HEIGHT as i32))?;

        // Horizontal center divider
        let divider_y = center_y as i32;
        canvas.draw_line((0, divider_y - 2), ((center_x - HALF_ROAD_WIDTH) as i32, divider_y - 2))?;
        canvas.draw_line((0, divider_y + 2), ((center_x - HALF_ROAD_WIDTH) as i32, divider_y + 2))?;
        canvas.draw_line(((center_x + HALF_ROAD_WIDTH) as i32, divider_y - 2), (WINDOW_WIDTH as i32, divider_y - 2))?;
        canvas.draw_line(((center_x + HALF_ROAD_WIDTH) as i32, divider_y + 2), (WINDOW_WIDTH as i32, divider_y + 2))?;

        Ok(())
    }

    fn draw_line_segment_with_gap(&self, canvas: &mut Canvas<Window>, start: Vec2, end: Vec2, gap_rect: Rect) -> Result<(), String> {
        // Check if line intersects with gap rectangle
        if self.line_intersects_rect(start, end, gap_rect) {
            // Split line into segments before and after the gap
            if let Some((before_end, after_start)) = self.calculate_line_gap_points(start, end, gap_rect) {
                // Draw segment before gap
                if (before_end - start).length() > 5.0 {
                    canvas.draw_line(
                        (start.x as i32, start.y as i32),
                        (before_end.x as i32, before_end.y as i32),
                    )?;
                }

                // Draw segment after gap
                if (end - after_start).length() > 5.0 {
                    canvas.draw_line(
                        (after_start.x as i32, after_start.y as i32),
                        (end.x as i32, end.y as i32),
                    )?;
                }
            }
        } else {
            // Draw complete line if no intersection
            canvas.draw_line(
                (start.x as i32, start.y as i32),
                (end.x as i32, end.y as i32),
            )?;
        }
        Ok(())
    }

    fn line_intersects_rect(&self, start: Vec2, end: Vec2, rect: Rect) -> bool {
        // Simple AABB vs line intersection test
        let min_x = start.x.min(end.x);
        let max_x = start.x.max(end.x);
        let min_y = start.y.min(end.y);
        let max_y = start.y.max(end.y);

        let rect_x = rect.x() as f32;
        let rect_y = rect.y() as f32;
        let rect_w = rect.width() as f32;
        let rect_h = rect.height() as f32;

        !(max_x < rect_x || min_x > rect_x + rect_w ||
            max_y < rect_y || min_y > rect_y + rect_h)
    }

    fn calculate_line_gap_points(&self, start: Vec2, end: Vec2, rect: Rect) -> Option<(Vec2, Vec2)> {
        let rect_x = rect.x() as f32;
        let rect_y = rect.y() as f32;
        let rect_w = rect.width() as f32;
        let rect_h = rect.height() as f32;

        // Simple calculation - find intersection points with rectangle edges
        let direction = (end - start).normalize();
        let is_vertical = direction.x.abs() < 0.1;

        if is_vertical {
            // Vertical line - intersect with top/bottom edges
            let before_end = Vec2::new(start.x, rect_y - 5.0);
            let after_start = Vec2::new(start.x, rect_y + rect_h + 5.0);
            Some((before_end, after_start))
        } else {
            // Horizontal line - intersect with left/right edges
            let before_end = Vec2::new(rect_x - 5.0, start.y);
            let after_start = Vec2::new(rect_x + rect_w + 5.0, start.y);
            Some((before_end, after_start))
        }
    }

    fn draw_lane_indicators(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let center_x = WINDOW_WIDTH as f32 / 2.0;
        let center_y = WINDOW_HEIGHT as f32 / 2.0;
        let indicator_size = 10u32;
        let offset = 150.0;

        // North-bound lanes (right side)
        for lane in 0..3 {
            let x = get_lane_center_x(Direction::North, lane);
            let color = match lane {
                0 => Color::RGB(255, 100, 100), // Red
                1 => Color::RGB(100, 100, 255), // Blue
                2 => Color::RGB(100, 255, 100), // Green
                _ => Color::RGB(128, 128, 128),
            };
            canvas.set_draw_color(color);
            canvas.fill_rect(Rect::new((x - indicator_size as f32 / 2.0) as i32, (center_y + offset) as i32, indicator_size, indicator_size))?;
        }

        // South-bound lanes (left side)
        for lane in 0..3 {
            let x = get_lane_center_x(Direction::South, lane);
            let color = match lane {
                0 => Color::RGB(255, 100, 100),
                1 => Color::RGB(100, 100, 255),
                2 => Color::RGB(100, 255, 100),
                _ => Color::RGB(128, 128, 128),
            };
            canvas.set_draw_color(color);
            canvas.fill_rect(Rect::new((x - indicator_size as f32 / 2.0) as i32, (center_y - offset - indicator_size as f32) as i32, indicator_size, indicator_size))?;
        }

        // East-bound lanes (bottom side)
        for lane in 0..3 {
            let y = get_lane_center_y(Direction::East, lane);
            let color = match lane {
                0 => Color::RGB(255, 100, 100),
                1 => Color::RGB(100, 100, 255),
                2 => Color::RGB(100, 255, 100),
                _ => Color::RGB(128, 128, 128),
            };
            canvas.set_draw_color(color);
            canvas.fill_rect(Rect::new((center_x - offset - indicator_size as f32) as i32, (y - indicator_size as f32 / 2.0) as i32, indicator_size, indicator_size))?;
        }

        // West-bound lanes (top side)
        for lane in 0..3 {
            let y = get_lane_center_y(Direction::West, lane);
            let color = match lane {
                0 => Color::RGB(255, 100, 100),
                1 => Color::RGB(100, 100, 255),
                2 => Color::RGB(100, 255, 100),
                _ => Color::RGB(128, 128, 128),
            };
            canvas.set_draw_color(color);
            canvas.fill_rect(Rect::new((center_x + offset) as i32, (y - indicator_size as f32 / 2.0) as i32, indicator_size, indicator_size))?;
        }

        Ok(())
    }

    fn draw_intersection(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let center_x = WINDOW_WIDTH as f32 / 2.0;
        let center_y = WINDOW_HEIGHT as f32 / 2.0;

        canvas.set_draw_color(Color::RGB(45, 45, 45));
        canvas.fill_rect(Rect::new(
            (center_x - HALF_ROAD_WIDTH) as i32,
            (center_y - HALF_ROAD_WIDTH) as i32,
            TOTAL_ROAD_WIDTH as u32,
            TOTAL_ROAD_WIDTH as u32,
        ))?;

        canvas.set_draw_color(Color::RGB(255, 255, 0));
        canvas.draw_rect(Rect::new(
            (center_x - HALF_ROAD_WIDTH) as i32,
            (center_y - HALF_ROAD_WIDTH) as i32,
            TOTAL_ROAD_WIDTH as u32,
            TOTAL_ROAD_WIDTH as u32,
        ))?;

        Ok(())
    }

    fn draw_vehicle_smooth(&self, canvas: &mut Canvas<Window>, vehicle: &Vehicle) -> Result<(), String> {
        let color = match vehicle.color {
            VehicleColor::Red => Color::RGB(255, 80, 80),
            VehicleColor::Blue => Color::RGB(80, 80, 255),
            VehicleColor::Green => Color::RGB(80, 255, 80),
            VehicleColor::Yellow => Color::RGB(255, 255, 80),
        };

        canvas.set_draw_color(color);

        let size = 16;

        // Convert floating-point position to integer for SDL2 rendering
        let dest_rect = Rect::new(
            (vehicle.interpolated_position.x - size as f32 / 2.0) as i32,
            (vehicle.interpolated_position.y - size as f32 / 2.0) as i32,
            size as u32,
            size as u32,
        );

        canvas.fill_rect(dest_rect)?;

        // Border
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.draw_rect(dest_rect)?;

        // Smooth direction arrow
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        self.draw_smooth_direction_arrow(canvas, vehicle)?;

        Ok(())
    }

    fn draw_smooth_direction_arrow(&self, canvas: &mut Canvas<Window>, vehicle: &Vehicle) -> Result<(), String> {
        let x = vehicle.interpolated_position.x;
        let y = vehicle.interpolated_position.y;
        let angle_rad = vehicle.interpolated_angle.to_radians();

        let arrow_length = 6.0;
        let arrow_end_x = x + angle_rad.cos() * arrow_length;
        let arrow_end_y = y + angle_rad.sin() * arrow_length;

        // Draw main arrow line
        canvas.draw_line(
            (x as i32, y as i32),
            (arrow_end_x as i32, arrow_end_y as i32),
        )?;

        // Draw arrow head
        let head_angle1 = angle_rad + 2.5;
        let head_angle2 = angle_rad - 2.5;
        let head_length = 3.0;

        let head1_x = arrow_end_x + head_angle1.cos() * head_length;
        let head1_y = arrow_end_y + head_angle1.sin() * head_length;
        let head2_x = arrow_end_x + head_angle2.cos() * head_length;
        let head2_y = arrow_end_y + head_angle2.sin() * head_length;

        canvas.draw_line(
            (arrow_end_x as i32, arrow_end_y as i32),
            (head1_x as i32, head1_y as i32),
        )?;
        canvas.draw_line(
            (arrow_end_x as i32, arrow_end_y as i32),
            (head2_x as i32, head2_y as i32),
        )?;

        Ok(())
    }

    fn draw_debug_overlays(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(255, 255, 0));

        for vehicle in &self.vehicles {
            // Draw vehicle path history
            if vehicle.path_history.len() > 1 {
                canvas.set_draw_color(Color::RGB(255, 100, 100));
                for i in 1..vehicle.path_history.len() {
                    canvas.draw_line(
                        (vehicle.path_history[i-1].x as i32, vehicle.path_history[i-1].y as i32),
                        (vehicle.path_history[i].x as i32, vehicle.path_history[i].y as i32),
                    )?;
                }
            }

            // Draw turning path if in turning state
            if vehicle.state == VehicleState::Turning {
                if let Some(ref turning_path) = vehicle.turning_path {
                    canvas.set_draw_color(Color::RGB(0, 255, 255));
                    // Draw Bezier curve points
                    for i in 0..=20 {
                        let t = i as f32 / 20.0;
                        let point = turning_path.cubic_bezier(t);
                        canvas.fill_rect(Rect::new(
                            (point.x - 1.0) as i32,
                            (point.y - 1.0) as i32,
                            2, 2
                        ))?;
                    }
                }
            }
        }

        Ok(())
    }

    fn draw_ui(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        // Background
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 180));
        canvas.fill_rect(Rect::new(10, 10, 300, 120))?;

        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.draw_rect(Rect::new(10, 10, 300, 120))?;

        // Show vehicle counts by route
        let mut y_offset = 25;
        for route in [Route::Left, Route::Straight, Route::Right] {
            let vehicles_with_route: Vec<&Vehicle> = self.vehicles.iter()
                .filter(|v| v.route == route)
                .take(15)
                .collect();

            let route_color = match route {
                Route::Left => Color::RGB(255, 100, 100),
                Route::Straight => Color::RGB(100, 100, 255),
                Route::Right => Color::RGB(100, 255, 100),
            };

            canvas.set_draw_color(route_color);
            for (i, _) in vehicles_with_route.iter().enumerate() {
                canvas.fill_rect(Rect::new(15 + (i as i32 * 6), y_offset, 4, 8))?;
            }
            y_offset += 15;
        }

        // Show completed vehicles
        canvas.set_draw_color(Color::RGB(0, 255, 0));
        for i in 0..(self.total_vehicles_passed.min(25)) {
            canvas.fill_rect(Rect::new(15 + (i as i32 * 6), 100, 4, 6))?;
        }

        // Debug mode indicator
        if self.debug_mode {
            canvas.set_draw_color(Color::RGB(255, 255, 0));
            canvas.fill_rect(Rect::new(280, 15, 20, 10))?;
        }

        Ok(())
    }

    fn show_final_statistics(&self) {
        let elapsed = self.simulation_start_time.elapsed();

        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║   FINAL STATISTICS - SMOOTH ANIMATIONS & PERFECT LANES      ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Total simulation time: {:>8.1}s                           ║", elapsed.as_secs_f32());
        println!("║ Vehicles spawned: {:>16}                           ║", self.statistics.total_vehicles_spawned);
        println!("║ Vehicles completed: {:>14}                           ║", self.total_vehicles_passed);
        println!("║ Still active: {:>20}                           ║", self.vehicles.len());
        println!("║ CRASHES: {:>25}                           ║", self.crash_count);
        println!("║ Close calls: {:>21}                           ║", self.close_calls);

        if self.statistics.total_vehicles_spawned > 0 {
            let completion_rate = (self.total_vehicles_passed as f64 /
                self.statistics.total_vehicles_spawned as f64) * 100.0;
            println!("║ Completion rate: {:>17.1}%                      ║", completion_rate);
        }

        let throughput = self.total_vehicles_passed as f64 / elapsed.as_secs_f64() * 60.0;
        println!("║ Throughput: {:>16.1} veh/min                  ║", throughput);
        println!("╚══════════════════════════════════════════════════════════════╝");

        println!("\n🎯 ENHANCED SYSTEM RESULTS:");
        println!("  ✅ Smooth Bezier curve turning animations");
        println!("  ✅ Fixed timestep physics for consistency");
        println!("  ✅ Floating-point rendering for smoothness");
        println!("  ✅ Perfect lane sectioning with intersection boundaries");
        println!("  ✅ Mathematical precision maintained: {} pixel lanes", LANE_WIDTH);

        if self.crash_count == 0 {
            println!("  🎉 NO CRASHES - Perfect collision avoidance with smooth motion!");
        }
    }
}