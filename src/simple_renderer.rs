// src/simple_renderer.rs
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};
use sdl2::image::LoadTexture; // Add this import for load_texture method
use std::collections::HashSet;

use crate::intersection::Intersection;
use crate::vehicle::{Vehicle, Direction, Route};

const GRID_SIZE: i32 = 32;

pub struct SimpleRenderer<'a> {
    vehicle_textures: Vec<sdl2::render::Texture<'a>>,
    road_blocks: HashSet<(i32, i32)>, // Set of (x, y) grid coordinates that are roads
    intersection_blocks: HashSet<(i32, i32)>, // Set of (x, y) grid coordinates that are intersection
    grid_width: i32,
    grid_height: i32,
    intersection_center: (i32, i32),
}

impl<'a> SimpleRenderer<'a> {
    pub fn new(texture_creator: &'a TextureCreator<WindowContext>) -> Result<Self, String> {
        // Try to load vehicle textures, fallback to generated ones
        let mut vehicle_textures = Vec::new();

        // Try loading the cars.png asset
        match texture_creator.load_texture("assets/vehicles/cars.png") {
            Ok(texture) => {
                println!("Loaded cars.png successfully");
                vehicle_textures.push(texture);
            }
            Err(_) => {
                println!("Creating fallback vehicle textures...");
                // Create colored vehicle textures for different routes
                let colors = [
                    Color::RGB(255, 100, 100),   // Red for left turns
                    Color::RGB(100, 100, 255),   // Blue for straight
                    Color::RGB(100, 255, 100),   // Green for right turns
                    Color::RGB(255, 255, 100),   // Yellow for special
                ];

                for color in colors {
                    let texture = Self::create_vehicle_texture(texture_creator, color)?;
                    vehicle_textures.push(texture);
                }
            }
        }

        let grid_width = (crate::WINDOW_WIDTH as i32) / GRID_SIZE;
        let grid_height = (crate::WINDOW_HEIGHT as i32) / GRID_SIZE;
        let intersection_center = (grid_width / 2, grid_height / 2);

        let mut renderer = SimpleRenderer {
            vehicle_textures,
            road_blocks: HashSet::new(),
            intersection_blocks: HashSet::new(),
            grid_width,
            grid_height,
            intersection_center,
        };

        // Initialize road layout
        renderer.create_road_layout();

        Ok(renderer)
    }

    fn create_road_layout(&mut self) {
        let (center_x, center_y) = self.intersection_center;
        let road_width = 6; // 6 lanes wide
        let intersection_size = 12; // 12x12 intersection

        // Create vertical road (north-south)
        for y in 0..self.grid_height {
            for lane in 0..road_width {
                let x = center_x - road_width/2 + lane;
                if x >= 0 && x < self.grid_width {
                    if self.is_in_intersection_area(x, y, center_x, center_y, intersection_size) {
                        self.intersection_blocks.insert((x, y));
                    } else {
                        self.road_blocks.insert((x, y));
                    }
                }
            }
        }

        // Create horizontal road (east-west)
        for x in 0..self.grid_width {
            for lane in 0..road_width {
                let y = center_y - road_width/2 + lane;
                if y >= 0 && y < self.grid_height {
                    if self.is_in_intersection_area(x, y, center_x, center_y, intersection_size) {
                        self.intersection_blocks.insert((x, y));
                    } else {
                        self.road_blocks.insert((x, y));
                    }
                }
            }
        }
    }

    fn is_in_intersection_area(&self, x: i32, y: i32, center_x: i32, center_y: i32, size: i32) -> bool {
        let half_size = size / 2;
        x >= center_x - half_size && x < center_x + half_size &&
            y >= center_y - half_size && y < center_y + half_size
    }

    pub fn render(&self, canvas: &mut Canvas<Window>, intersection: &Intersection, vehicles: &std::collections::VecDeque<Vehicle>, show_grid: bool) -> Result<(), String> {
        // Clear with gray background (non-road areas)
        canvas.set_draw_color(Color::RGB(120, 120, 120));
        canvas.clear();

        // Draw road blocks (black)
        canvas.set_draw_color(Color::RGB(40, 40, 40));
        for &(grid_x, grid_y) in &self.road_blocks {
            let pixel_x = grid_x * GRID_SIZE;
            let pixel_y = grid_y * GRID_SIZE;
            let rect = Rect::new(pixel_x, pixel_y, GRID_SIZE as u32, GRID_SIZE as u32);
            canvas.fill_rect(rect)?;
        }

        // Draw intersection blocks (darker)
        canvas.set_draw_color(Color::RGB(60, 60, 60));
        for &(grid_x, grid_y) in &self.intersection_blocks {
            let pixel_x = grid_x * GRID_SIZE;
            let pixel_y = grid_y * GRID_SIZE;
            let rect = Rect::new(pixel_x, pixel_y, GRID_SIZE as u32, GRID_SIZE as u32);
            canvas.fill_rect(rect)?;
        }

        // Draw lane markings
        self.draw_lane_markings(canvas)?;

        // Draw grid overlay if requested
        if show_grid {
            self.draw_grid_overlay(canvas)?;
        }

        // Draw vehicles
        for vehicle in vehicles {
            self.render_vehicle(canvas, vehicle)?;
        }

        Ok(())
    }

    fn draw_lane_markings(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(255, 255, 255)); // White lane markings

        let (center_x, center_y) = self.intersection_center;
        let road_width = 6;

        // Vertical lane markings (for north-south road)
        for lane in 1..road_width {
            let x = center_x - road_width/2 + lane;
            let pixel_x = x * GRID_SIZE;

            // Draw dashed lines above intersection
            for y in 0..(center_y - 6) {
                if y % 2 == 0 { // Dashed effect
                    let pixel_y = y * GRID_SIZE + GRID_SIZE/2;
                    canvas.draw_line(
                        (pixel_x, pixel_y),
                        (pixel_x, pixel_y + GRID_SIZE/2)
                    )?;
                }
            }

            // Draw dashed lines below intersection
            for y in (center_y + 6)..self.grid_height {
                if y % 2 == 0 { // Dashed effect
                    let pixel_y = y * GRID_SIZE + GRID_SIZE/2;
                    canvas.draw_line(
                        (pixel_x, pixel_y),
                        (pixel_x, pixel_y + GRID_SIZE/2)
                    )?;
                }
            }
        }

        // Horizontal lane markings (for east-west road)
        for lane in 1..road_width {
            let y = center_y - road_width/2 + lane;
            let pixel_y = y * GRID_SIZE;

            // Draw dashed lines left of intersection
            for x in 0..(center_x - 6) {
                if x % 2 == 0 { // Dashed effect
                    let pixel_x = x * GRID_SIZE + GRID_SIZE/2;
                    canvas.draw_line(
                        (pixel_x, pixel_y),
                        (pixel_x + GRID_SIZE/2, pixel_y)
                    )?;
                }
            }

            // Draw dashed lines right of intersection
            for x in (center_x + 6)..self.grid_width {
                if x % 2 == 0 { // Dashed effect
                    let pixel_x = x * GRID_SIZE + GRID_SIZE/2;
                    canvas.draw_line(
                        (pixel_x, pixel_y),
                        (pixel_x + GRID_SIZE/2, pixel_y)
                    )?;
                }
            }
        }

        // Draw intersection boundary lines
        canvas.set_draw_color(Color::RGB(255, 255, 0)); // Yellow for intersection boundary
        let intersection_pixel_x = (center_x - 6) * GRID_SIZE;
        let intersection_pixel_y = (center_y - 6) * GRID_SIZE;
        let intersection_pixel_width = 12 * GRID_SIZE;
        let intersection_pixel_height = 12 * GRID_SIZE;

        canvas.draw_rect(Rect::new(
            intersection_pixel_x,
            intersection_pixel_y,
            intersection_pixel_width as u32,
            intersection_pixel_height as u32,
        ))?;

        Ok(())
    }

    fn draw_grid_overlay(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        canvas.set_draw_color(Color::RGBA(255, 255, 255, 100)); // Semi-transparent white

        // Draw vertical grid lines
        for x in 0..=self.grid_width {
            let pixel_x = x * GRID_SIZE;
            canvas.draw_line(
                (pixel_x, 0),
                (pixel_x, self.grid_height * GRID_SIZE),
            )?;
        }

        // Draw horizontal grid lines
        for y in 0..=self.grid_height {
            let pixel_y = y * GRID_SIZE;
            canvas.draw_line(
                (0, pixel_y),
                (self.grid_width * GRID_SIZE, pixel_y),
            )?;
        }

        Ok(())
    }

    fn render_vehicle(&self, canvas: &mut Canvas<Window>, vehicle: &Vehicle) -> Result<(), String> {
        // Get vehicle color based on route
        let color_index = match vehicle.route {
            Route::Left => 0,      // Red
            Route::Straight => 1,  // Blue
            Route::Right => 2,     // Green
        };

        // Calculate render position (center vehicle in its grid cell)
        let grid_x = vehicle.position.x / GRID_SIZE;
        let grid_y = vehicle.position.y / GRID_SIZE;
        let render_x = grid_x * GRID_SIZE + GRID_SIZE/2 - 12; // Center 24x24 vehicle in 32x32 cell
        let render_y = grid_y * GRID_SIZE + GRID_SIZE/2 - 12;

        let render_rect = Rect::new(render_x, render_y, 24, 24); // Smaller than grid cell

        if !self.vehicle_textures.is_empty() {
            let texture_index = color_index.min(self.vehicle_textures.len() - 1);
            let texture = &self.vehicle_textures[texture_index];

            // Render vehicle texture with rotation based on direction
            let angle = match vehicle.direction {
                Direction::North => 0.0,
                Direction::East => 90.0,
                Direction::South => 180.0,
                Direction::West => 270.0,
            };

            canvas.copy_ex(
                texture,
                None,
                render_rect,
                angle,
                None,
                false,
                false,
            )?;
        } else {
            // Fallback: render as colored rectangle
            let color = match vehicle.route {
                Route::Left => Color::RGB(255, 100, 100),
                Route::Straight => Color::RGB(100, 100, 255),
                Route::Right => Color::RGB(100, 255, 100),
            };
            canvas.set_draw_color(color);
            canvas.fill_rect(render_rect)?;
        }

        // Draw direction arrow
        self.draw_direction_arrow(canvas, render_x + 12, render_y + 12, vehicle.direction)?;

        // Draw vehicle ID for debugging
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        let id_rect = Rect::new(render_x + 2, render_y + 2, 4, 4);
        canvas.fill_rect(id_rect)?;

        Ok(())
    }

    fn draw_direction_arrow(&self, canvas: &mut Canvas<Window>, center_x: i32, center_y: i32, direction: Direction) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(255, 255, 255));

        let arrow_size = 6;
        match direction {
            Direction::North => {
                canvas.draw_line((center_x, center_y - arrow_size), (center_x - 3, center_y))?;
                canvas.draw_line((center_x, center_y - arrow_size), (center_x + 3, center_y))?;
            }
            Direction::South => {
                canvas.draw_line((center_x, center_y + arrow_size), (center_x - 3, center_y))?;
                canvas.draw_line((center_x, center_y + arrow_size), (center_x + 3, center_y))?;
            }
            Direction::East => {
                canvas.draw_line((center_x + arrow_size, center_y), (center_x, center_y - 3))?;
                canvas.draw_line((center_x + arrow_size, center_y), (center_x, center_y + 3))?;
            }
            Direction::West => {
                canvas.draw_line((center_x - arrow_size, center_y), (center_x, center_y - 3))?;
                canvas.draw_line((center_x - arrow_size, center_y), (center_x, center_y + 3))?;
            }
        }

        Ok(())
    }

    fn create_vehicle_texture(
        texture_creator: &TextureCreator<WindowContext>,
        base_color: Color,
    ) -> Result<sdl2::render::Texture, String> {
        use sdl2::surface::Surface;

        // Create a surface for the vehicle
        let mut surface = Surface::new(
            24, 24, // 24x24 vehicle (fits in 32x32 grid cell)
            sdl2::pixels::PixelFormatEnum::RGBA8888,
        ).map_err(|e| e.to_string())?;

        // Fill with base color
        surface.fill_rect(None, base_color).map_err(|e| e.to_string())?;

        // Add car details
        // Car body outline
        surface.fill_rect(
            Rect::new(2, 2, 20, 20),
            Color::RGB(base_color.r.saturating_sub(30), base_color.g.saturating_sub(30), base_color.b.saturating_sub(30)),
        ).map_err(|e| e.to_string())?;

        // Windows
        surface.fill_rect(
            Rect::new(6, 6, 12, 12),
            Color::RGB(150, 200, 255),
        ).map_err(|e| e.to_string())?;

        // Create texture from surface
        texture_creator
            .create_texture_from_surface(surface)
            .map_err(|e| e.to_string())
    }
}