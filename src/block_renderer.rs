// src/block_renderer.rs
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use crate::block_system::{BlockGrid, BlockVehicle, BlockPosition, BlockType, VehicleState};
use crate::vehicle::{Direction, Route};

pub struct BlockRenderer {
    pub grid: BlockGrid,
    pub vehicles: Vec<BlockVehicle>,
    next_vehicle_id: u32,
    spawn_cooldown: f32,
    current_cooldown: f32,
}

impl BlockRenderer {
    pub fn new(window_width: u32, window_height: u32) -> Self {
        let block_size = 32; // Each block is 32x32 pixels
        let grid = BlockGrid::new(window_width, window_height, block_size);

        BlockRenderer {
            grid,
            vehicles: Vec::new(),
            next_vehicle_id: 0,
            spawn_cooldown: 1.0, // 1 second between spawns
            current_cooldown: 0.0,
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        // Update spawn cooldown
        if self.current_cooldown > 0.0 {
            self.current_cooldown -= delta_time;
        }

        // Update all vehicles
        self.vehicles.retain_mut(|vehicle| {
            vehicle.adjust_speed(&self.grid);
            vehicle.update(delta_time, &mut self.grid)
        });
    }

    pub fn spawn_vehicle(&mut self, direction: Direction) -> bool {
        if self.current_cooldown > 0.0 {
            return false;
        }

        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Choose random route
        let route = match rng.gen_range(0..3) {
            0 => Route::Left,
            1 => Route::Straight,
            _ => Route::Right,
        };

        // Calculate spawn position
        let spawn_pos = self.calculate_spawn_position(direction, route);

        if let Some(pos) = spawn_pos {
            let vehicle_id = self.next_vehicle_id;
            self.next_vehicle_id += 1;

            let vehicle = BlockVehicle::new(vehicle_id, pos, direction, route);

            if self.grid.add_vehicle(vehicle_id, pos) {
                self.vehicles.push(vehicle);
                self.current_cooldown = self.spawn_cooldown;
                println!("Spawned vehicle {} at {:?} with route {:?}", vehicle_id, pos, route);
                return true;
            }
        }

        false
    }

    fn calculate_spawn_position(&self, direction: Direction, route: Route) -> Option<BlockPosition> {
        let center = self.grid.intersection_center;

        // Choose appropriate lane based on route
        let lane_offset = match route {
            Route::Left => 0,     // Leftmost lanes
            Route::Straight => 2, // Middle lanes
            Route::Right => 4,    // Rightmost lanes
        };

        let spawn_pos = match direction {
            Direction::North => BlockPosition {
                x: center.x - 3 + lane_offset,
                y: self.grid.grid_height - 1,
            },
            Direction::South => BlockPosition {
                x: center.x + 3 - lane_offset,
                y: 0,
            },
            Direction::East => BlockPosition {
                x: 0,
                y: center.y - 3 + lane_offset,
            },
            Direction::West => BlockPosition {
                x: self.grid.grid_width - 1,
                y: center.y + 3 - lane_offset,
            },
        };

        // Check if spawn position is valid and unoccupied
        if !self.grid.is_block_occupied(spawn_pos) {
            Some(spawn_pos)
        } else {
            None
        }
    }

    pub fn render(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        // Clear screen
        canvas.set_draw_color(Color::RGB(50, 150, 50)); // Green grass
        canvas.clear();

        // Render grid blocks
        self.render_grid(canvas)?;

        // Render vehicles
        self.render_vehicles(canvas)?;

        // Render debug info
        self.render_debug_info(canvas)?;

        canvas.present();
        Ok(())
    }

    fn render_grid(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let block_size = self.grid.block_size;

        // Draw all blocks
        for y in 0..self.grid.grid_height {
            for x in 0..self.grid.grid_width {
                let pos = BlockPosition { x, y };
                let block_type = self.grid.get_block_type(pos);

                let (pixel_x, pixel_y) = self.grid.block_to_pixel(pos);
                let rect = Rect::new(
                    pixel_x - block_size as i32 / 2,
                    pixel_y - block_size as i32 / 2,
                    block_size,
                    block_size,
                );

                match block_type {
                    BlockType::Road => {
                        canvas.set_draw_color(Color::RGB(80, 80, 80)); // Dark gray road
                        canvas.fill_rect(rect)?;

                        // Add lane markings
                        canvas.set_draw_color(Color::RGB(255, 255, 255)); // White lines
                        canvas.draw_rect(rect)?;
                    }
                    BlockType::Intersection => {
                        canvas.set_draw_color(Color::RGB(60, 60, 60)); // Darker intersection
                        canvas.fill_rect(rect)?;

                        // Add intersection markings
                        canvas.set_draw_color(Color::RGB(255, 255, 0)); // Yellow lines
                        canvas.draw_rect(rect)?;
                    }
                    BlockType::Outside => {
                        // Already filled with grass color
                    }
                }

                // Highlight occupied blocks
                if self.grid.is_block_occupied(pos) {
                    canvas.set_draw_color(Color::RGB(255, 0, 0)); // Red outline for occupied
                    for i in 0..3 {
                        canvas.draw_rect(Rect::new(
                            rect.x() - i,
                            rect.y() - i,
                            rect.width() + 2 * i as u32,
                            rect.height() + 2 * i as u32,
                        ))?;
                    }
                }
            }
        }

        Ok(())
    }

    fn render_vehicles(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        for vehicle in &self.vehicles {
            let (pixel_x, pixel_y) = self.grid.block_to_pixel(vehicle.block_position);

            // Vehicle color based on route
            let color = match vehicle.route {
                Route::Left => Color::RGB(255, 100, 100),   // Red for left
                Route::Straight => Color::RGB(100, 100, 255), // Blue for straight
                Route::Right => Color::RGB(100, 255, 100),   // Green for right
            };

            // Vehicle state affects brightness
            let state_color = match vehicle.state {
                VehicleState::Moving => color,
                VehicleState::Waiting => Color::RGB(
                    color.r / 2,
                    color.g / 2,
                    color.b / 2
                ), // Dimmer when waiting
                VehicleState::Completed => continue, // Don't render completed vehicles
            };

            canvas.set_draw_color(state_color);

            // Draw vehicle as a circle or rectangle
            let size = (self.grid.block_size / 2) as i32;
            let vehicle_rect = Rect::new(
                pixel_x - size / 2,
                pixel_y - size / 2,
                size as u32,
                size as u32,
            );
            canvas.fill_rect(vehicle_rect)?;

            // Draw direction arrow
            self.draw_direction_arrow(canvas, pixel_x, pixel_y, vehicle.direction, size)?;

            // Draw vehicle ID
            canvas.set_draw_color(Color::RGB(255, 255, 255));
            // Note: For text rendering, you'd need SDL2_ttf, so we'll just draw a small dot for ID
            let id_rect = Rect::new(pixel_x - 2, pixel_y - 2, 4, 4);
            canvas.fill_rect(id_rect)?;
        }

        Ok(())
    }

    fn draw_direction_arrow(&self, canvas: &mut Canvas<Window>, x: i32, y: i32, direction: Direction, size: i32) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(255, 255, 255)); // White arrow

        let arrow_size = size / 4;
        match direction {
            Direction::North => {
                // Draw upward arrow
                canvas.draw_line((x, y - arrow_size), (x - arrow_size/2, y))?;
                canvas.draw_line((x, y - arrow_size), (x + arrow_size/2, y))?;
            }
            Direction::South => {
                // Draw downward arrow
                canvas.draw_line((x, y + arrow_size), (x - arrow_size/2, y))?;
                canvas.draw_line((x, y + arrow_size), (x + arrow_size/2, y))?;
            }
            Direction::East => {
                // Draw rightward arrow
                canvas.draw_line((x + arrow_size, y), (x, y - arrow_size/2))?;
                canvas.draw_line((x + arrow_size, y), (x, y + arrow_size/2))?;
            }
            Direction::West => {
                // Draw leftward arrow
                canvas.draw_line((x - arrow_size, y), (x, y - arrow_size/2))?;
                canvas.draw_line((x - arrow_size, y), (x, y + arrow_size/2))?;
            }
        }

        Ok(())
    }

    fn render_debug_info(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        // Draw statistics
        canvas.set_draw_color(Color::RGB(255, 255, 255));

        // Count vehicles by state
        let moving_count = self.vehicles.iter().filter(|v| v.state == VehicleState::Moving).count();
        let waiting_count = self.vehicles.iter().filter(|v| v.state == VehicleState::Waiting).count();

        // Note: For proper text rendering, you'd use SDL2_ttf
        // For now, we'll just show colored rectangles as indicators

        // Moving vehicles indicator (green)
        canvas.set_draw_color(Color::RGB(0, 255, 0));
        for i in 0..moving_count.min(20) {
            canvas.fill_rect(Rect::new(10 + i as i32 * 6, 10, 4, 10))?;
        }

        // Waiting vehicles indicator (red)
        canvas.set_draw_color(Color::RGB(255, 0, 0));
        for i in 0..waiting_count.min(20) {
            canvas.fill_rect(Rect::new(10 + i as i32 * 6, 25, 4, 10))?;
        }

        Ok(())
    }

    pub fn get_statistics(&self) -> (usize, usize, usize) {
        let total = self.vehicles.len();
        let moving = self.vehicles.iter().filter(|v| v.state == VehicleState::Moving).count();
        let waiting = self.vehicles.iter().filter(|v| v.state == VehicleState::Waiting).count();
        (total, moving, waiting)
    }
}