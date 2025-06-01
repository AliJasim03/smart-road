use sdl2::image::{InitFlag, LoadTexture};
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use crate::intersection::{intersection_area, intersection_center, Intersection, ROAD_WIDTH, LANE_WIDTH};
use crate::vehicle::{Direction, Route, Vehicle, VehicleColor};

pub struct Renderer<'a> {
    vehicle_textures: Vec<sdl2::render::Texture<'a>>,
    road_right: Option<sdl2::render::Texture<'a>>,
    road_up: Option<sdl2::render::Texture<'a>>,
    acera_texture: Option<sdl2::render::Texture<'a>>,
    texture_creator: &'a TextureCreator<WindowContext>,
}

impl<'a> Renderer<'a> {
    pub fn new(texture_creator: &'a TextureCreator<WindowContext>) -> Result<Self, String> {
        // Initialize SDL2 image
        sdl2::image::init(InitFlag::PNG | InitFlag::JPG)?;

        println!("Current working directory: {:?}", std::env::current_dir().unwrap_or_default());

        let mut vehicle_textures = Vec::new();

        // First try to create optimized sprite sheet
        match Self::create_optimized_sprite_sheet(texture_creator) {
            Ok(sprite_sheet) => {
                println!("Successfully created optimized car sprite sheet");
                vehicle_textures.push(sprite_sheet);
            }
            Err(e) => {
                println!("Sprite sheet creation failed: {}, trying to load from file...", e);

                // Try to load from file
                let car_texture_result = texture_creator.load_texture("assets/vehicles/cars.png");
                match car_texture_result {
                    Ok(texture) => {
                        println!("Successfully loaded cars.png texture");
                        vehicle_textures.push(texture);
                    }
                    Err(e) => {
                        println!("Warning: Could not load cars.png texture: {}", e);
                        println!("Creating fallback vehicle textures...");

                        // Create 4 different colored vehicles for the 4 different routes
                        let colors = [
                            Color::RGB(220, 50, 50),    // Red for Left turn
                            Color::RGB(50, 50, 220),    // Blue for Straight
                            Color::RGB(50, 220, 50),    // Green for Right turn
                            Color::RGB(220, 220, 50),   // Yellow for special cases
                        ];

                        for color in colors {
                            let texture = Self::create_vehicle_texture(texture_creator, color)?;
                            vehicle_textures.push(texture);
                        }
                    }
                }
            }
        }

        // Load road textures for different orientations
        let road_right_result = texture_creator.load_texture("assets/road/road_right.png");
        let road_right = match road_right_result {
            Ok(texture) => {
                println!("Successfully loaded road_right.png texture");
                Some(texture)
            }
            Err(e) => {
                println!("Warning: Could not load road_right texture: {}", e);
                println!("Creating fallback road texture...");
                Some(Self::create_road_texture(texture_creator)?)
            }
        };

        let road_up_result = texture_creator.load_texture("assets/road/road_up.png");
        let road_up = match road_up_result {
            Ok(texture) => {
                println!("Successfully loaded road_up.png texture");
                Some(texture)
            }
            Err(e) => {
                println!("Warning: Could not load road_up texture: {}", e);
                println!("Creating fallback road texture...");
                Some(Self::create_road_texture(texture_creator)?)
            }
        };

        // Load acera (sidewalk) texture
        let acera_texture_result = texture_creator.load_texture("assets/road/acera.png");
        let acera_texture = match acera_texture_result {
            Ok(texture) => {
                println!("Successfully loaded acera.png texture");
                Some(texture)
            }
            Err(e) => {
                println!("Warning: Could not load acera texture: {}", e);
                println!("Creating fallback acera texture...");
                Some(Self::create_acera_texture(texture_creator)?)
            }
        };

        Ok(Renderer {
            vehicle_textures,
            road_right,
            road_up,
            acera_texture,
            texture_creator,
        })
    }

    // Create optimized sprite sheet with 2x2 car layout
    fn create_optimized_sprite_sheet(
        texture_creator: &TextureCreator<WindowContext>
    ) -> Result<sdl2::render::Texture, String> {
        // Create a 2x2 sprite sheet (160x160 total, 80x80 per car)
        let sprite_width = 160;
        let sprite_height = 160;
        let car_width = 80;
        let car_height = 80;

        let mut surface = Surface::new(
            sprite_width,
            sprite_height,
            sdl2::pixels::PixelFormatEnum::RGBA8888,
        ).map_err(|e| e.to_string())?;

        // Make background transparent
        surface.set_color_key(true, Color::RGB(255, 0, 255)).map_err(|e| e.to_string())?;
        surface.fill_rect(None, Color::RGB(255, 0, 255)).map_err(|e| e.to_string())?;

        // Car colors for each quarter
        let colors = [
            Color::RGB(220, 50, 50),   // Red (top-left) - Left turn
            Color::RGB(50, 50, 220),   // Blue (top-right) - Straight
            Color::RGB(50, 220, 50),   // Green (bottom-left) - Right turn
            Color::RGB(220, 220, 50),  // Yellow (bottom-right) - Special
        ];

        // Create 4 cars in 2x2 grid
        for (i, color) in colors.iter().enumerate() {
            let col = i % 2;
            let row = i / 2;
            let x_offset = (col * car_width as usize) as i32;
            let y_offset = (row * car_height as usize) as i32;

            // Create car in this quarter
            Self::create_detailed_car(&mut surface, x_offset, y_offset, car_width, car_height, *color)?;
        }

        // Create texture from surface
        texture_creator
            .create_texture_from_surface(surface)
            .map_err(|e| e.to_string())
    }

    // Create detailed car sprite
    fn create_detailed_car(
        surface: &mut Surface,
        x_offset: i32,
        y_offset: i32,
        width: u32,
        height: u32,
        base_color: Color,
    ) -> Result<(), String> {

        // Car body (main rectangle)
        surface.fill_rect(
            Rect::new(x_offset + 10, y_offset + 5, width - 20, height - 10),
            base_color,
        ).map_err(|e| e.to_string())?;

        // Car roof/cabin (lighter color)
        let roof_color = Color::RGB(
            base_color.r.saturating_add(30).min(255),
            base_color.g.saturating_add(30).min(255),
            base_color.b.saturating_add(30).min(255),
        );

        surface.fill_rect(
            Rect::new(x_offset + 15, y_offset + 15, width - 30, height - 40),
            roof_color,
        ).map_err(|e| e.to_string())?;

        // Windows (light blue)
        surface.fill_rect(
            Rect::new(x_offset + 20, y_offset + 18, width - 40, height - 46),
            Color::RGB(150, 200, 255),
        ).map_err(|e| e.to_string())?;

        // Front bumper/grille
        surface.fill_rect(
            Rect::new(x_offset + 12, y_offset + 8, width - 24, 8),
            Color::RGB(60, 60, 60),
        ).map_err(|e| e.to_string())?;

        // Rear bumper
        surface.fill_rect(
            Rect::new(x_offset + 12, y_offset + (height as i32) - 16, width - 24, 8),
            Color::RGB(60, 60, 60),
        ).map_err(|e| e.to_string())?;

        // Headlights (front)
        surface.fill_rect(
            Rect::new(x_offset + 18, y_offset + 10, 8, 6),
            Color::RGB(255, 255, 200),
        ).map_err(|e| e.to_string())?;

        surface.fill_rect(
            Rect::new(x_offset + (width as i32) - 26, y_offset + 10, 8, 6),
            Color::RGB(255, 255, 200),
        ).map_err(|e| e.to_string())?;

        // Taillights (rear)
        surface.fill_rect(
            Rect::new(x_offset + 18, y_offset + (height as i32) - 16, 8, 6),
            Color::RGB(255, 50, 50),
        ).map_err(|e| e.to_string())?;

        surface.fill_rect(
            Rect::new(x_offset + (width as i32) - 26, y_offset + (height as i32) - 16, 8, 6),
            Color::RGB(255, 50, 50),
        ).map_err(|e| e.to_string())?;

        // Side mirrors
        surface.fill_rect(
            Rect::new(x_offset + 8, y_offset + 25, 4, 6),
            Color::RGB(100, 100, 100),
        ).map_err(|e| e.to_string())?;

        surface.fill_rect(
            Rect::new(x_offset + (width as i32) - 12, y_offset + 25, 4, 6),
            Color::RGB(100, 100, 100),
        ).map_err(|e| e.to_string())?;

        Ok(())
    }

    // Enhanced create_vehicle_texture method with more detailed car appearance
    fn create_vehicle_texture(
        texture_creator: &TextureCreator<WindowContext>,
        base_color: Color,
    ) -> Result<sdl2::render::Texture, String> {
        // Create a surface for the vehicle
        let mut surface = Surface::new(
            Vehicle::WIDTH,
            Vehicle::HEIGHT,
            sdl2::pixels::PixelFormatEnum::RGBA8888,
        ).map_err(|e| e.to_string())?;

        // Fill the surface with the specified color (car body)
        surface
            .fill_rect(
                Rect::new(0, 0, Vehicle::WIDTH, Vehicle::HEIGHT),
                base_color,
            )
            .map_err(|e| e.to_string())?;

        // Add car details

        // Car roof/cabin
        surface
            .fill_rect(
                Rect::new(3, 8, Vehicle::WIDTH - 6, Vehicle::HEIGHT - 16),
                Color::RGB(220, 220, 255),
            )
            .map_err(|e| e.to_string())?;

        // Car windows
        surface
            .fill_rect(
                Rect::new(5, 10, Vehicle::WIDTH - 10, Vehicle::HEIGHT - 20),
                Color::RGB(100, 180, 255),
            )
            .map_err(|e| e.to_string())?;

        // Front lights
        surface
            .fill_rect(
                Rect::new(3, 3, 6, 4),
                Color::RGB(255, 255, 200),
            )
            .map_err(|e| e.to_string())?;
        surface
            .fill_rect(
                Rect::new((Vehicle::WIDTH as i32) - 9, 3, 6, 4),
                Color::RGB(255, 255, 200),
            )
            .map_err(|e| e.to_string())?;

        // Rear lights
        surface
            .fill_rect(
                Rect::new(3, (Vehicle::HEIGHT as i32) - 7, 6, 4),
                Color::RGB(255, 50, 50),
            )
            .map_err(|e| e.to_string())?;
        surface
            .fill_rect(
                Rect::new((Vehicle::WIDTH as i32) - 9, (Vehicle::HEIGHT as i32) - 7, 6, 4),
                Color::RGB(255, 50, 50),
            )
            .map_err(|e| e.to_string())?;

        // Create texture from surface
        let texture = texture_creator
            .create_texture_from_surface(surface)
            .map_err(|e| e.to_string())?;

        Ok(texture)
    }

    // Add this method for creating road textures
    fn create_road_texture(
        texture_creator: &TextureCreator<WindowContext>,
    ) -> Result<sdl2::render::Texture, String> {
        // Create a surface for the road
        let mut surface = Surface::new(
            200, // Width
            200, // Height
            sdl2::pixels::PixelFormatEnum::RGBA8888,
        ).map_err(|e| e.to_string())?;

        // Fill the surface with dark gray (asphalt)
        surface
            .fill_rect(
                Rect::new(0, 0, 200, 200),
                Color::RGB(80, 80, 80),
            )
            .map_err(|e| e.to_string())?;

        // Add some asphalt texture with darker spots
        for _ in 0..50 {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let x = rng.gen_range(0..200);
            let y = rng.gen_range(0..200);
            let size = rng.gen_range(2..6);

            surface
                .fill_rect(
                    Rect::new(x, y, size, size),
                    Color::RGB(60, 60, 60),
                )
                .map_err(|e| e.to_string())?;
        }

        // Create texture from surface
        let texture = texture_creator
            .create_texture_from_surface(surface)
            .map_err(|e| e.to_string())?;

        Ok(texture)
    }

    // Add this method for creating sidewalk (acera) textures
    fn create_acera_texture(
        texture_creator: &TextureCreator<WindowContext>,
    ) -> Result<sdl2::render::Texture, String> {
        // Create a surface for the sidewalk
        let mut surface = Surface::new(
            100, // Width
            100, // Height
            sdl2::pixels::PixelFormatEnum::RGBA8888,
        ).map_err(|e| e.to_string())?;

        // Fill the surface with light gray (concrete)
        surface
            .fill_rect(
                Rect::new(0, 0, 100, 100),
                Color::RGB(180, 180, 180),
            )
            .map_err(|e| e.to_string())?;

        // Add concrete texture with lines
        for i in 0..10 {
            surface
                .fill_rect(
                    Rect::new(0, i * 10, 100, 1),
                    Color::RGB(150, 150, 150),
                )
                .map_err(|e| e.to_string())?;

            surface
                .fill_rect(
                    Rect::new(i * 10, 0, 1, 100),
                    Color::RGB(150, 150, 150),
                )
                .map_err(|e| e.to_string())?;
        }

        // Create texture from surface
        let texture = texture_creator
            .create_texture_from_surface(surface)
            .map_err(|e| e.to_string())?;

        Ok(texture)
    }

    // Render the intersection using the loaded textures
    pub fn render_intersection(
        &self,
        canvas: &mut Canvas<Window>,
        intersection: &Intersection,
    ) -> Result<(), String> {
        // First fill the background with acera texture if available
        if let Some(acera_texture) = &self.acera_texture {
            // Get acera texture dimensions
            let acera_query = acera_texture.query();
            let acera_width = acera_query.width;
            let acera_height = acera_query.height;

            // Tile the acera texture across the entire screen
            for x in (0..crate::WINDOW_WIDTH).step_by(acera_width as usize) {
                for y in (0..crate::WINDOW_HEIGHT).step_by(acera_height as usize) {
                    let dest_rect = Rect::new(
                        x as i32,
                        y as i32,
                        acera_width.min(crate::WINDOW_WIDTH - x),
                        acera_height.min(crate::WINDOW_HEIGHT - y),
                    );
                    canvas.copy(acera_texture, None, Some(dest_rect))?;
                }
            }
        } else {
            // Fallback background color
            canvas.set_draw_color(Color::RGB(100, 160, 100)); // Green for grass
            canvas.clear();
        }

        // Draw the roads using oriented road textures
        let center = intersection_center();

        // Draw horizontal road (east-west) using right-facing texture in two parts
        if let Some(road_right) = &self.road_right {
            // Left part of horizontal road
            let left_width = (center.0 - (ROAD_WIDTH as i32 / 2)) as u32;
            let horizontal_left = Rect::new(
                0,
                center.1 - (ROAD_WIDTH as i32 / 2),
                left_width,
                ROAD_WIDTH,
            );
            canvas.copy(road_right, None, Some(horizontal_left))?;

            // Right part of horizontal road
            let right_start = center.0 + (ROAD_WIDTH as i32 / 2);
            let right_width = (crate::WINDOW_WIDTH as i32 - right_start) as u32;
            let horizontal_right = Rect::new(
                center.0 + (ROAD_WIDTH as i32 / 2),
                center.1 - (ROAD_WIDTH as i32 / 2),
                right_width,
                ROAD_WIDTH,
            );
            canvas.copy(road_right, None, Some(horizontal_right))?;
        }

        // Draw vertical road (north-south) using upward-facing texture in two parts
        if let Some(road_up) = &self.road_up {
            // Top part of vertical road
            let top_height = (center.1 - (ROAD_WIDTH as i32 / 2)) as u32;
            let vertical_top = Rect::new(
                center.0 - (ROAD_WIDTH as i32 / 2),
                0,
                ROAD_WIDTH,
                top_height,
            );
            canvas.copy(road_up, None, Some(vertical_top))?;

            // Bottom part of vertical road
            let bottom_start = center.1 + (ROAD_WIDTH as i32 / 2);
            let bottom_height = (crate::WINDOW_HEIGHT as i32 - bottom_start) as u32;
            let vertical_bottom = Rect::new(
                center.0 - (ROAD_WIDTH as i32 / 2),
                center.1 + (ROAD_WIDTH as i32 / 2),
                ROAD_WIDTH,
                bottom_height,
            );
            canvas.copy(road_up, None, Some(vertical_bottom))?;

            // Draw intersection area with darker color to create crossover effect
            canvas.set_draw_color(Color::RGB(60, 60, 60)); // Darker gray for intersection
            let intersection_area = Rect::new(
                center.0 - (ROAD_WIDTH as i32 / 2),
                center.1 - (ROAD_WIDTH as i32 / 2),
                ROAD_WIDTH,
                ROAD_WIDTH,
            );
            canvas.fill_rect(intersection_area)?;
        } else {
            // Fallback to simple gray roads
            canvas.set_draw_color(Color::RGB(80, 80, 80)); // Dark gray

            let center = intersection_center();

            // Draw horizontal road
            canvas.fill_rect(Rect::new(
                0,
                center.1 - (ROAD_WIDTH as i32 / 2),
                crate::WINDOW_WIDTH,
                ROAD_WIDTH,
            ))?;

            // Draw vertical road
            canvas.fill_rect(Rect::new(
                center.0 - (ROAD_WIDTH as i32 / 2),
                0,
                ROAD_WIDTH,
                crate::WINDOW_HEIGHT,
            ))?;
        }

        // Draw lane markings
        canvas.set_draw_color(Color::RGB(255, 255, 255)); // White

        let center = intersection_center();

        // Draw horizontal lane markings (5 lines for 6 lanes)
        for i in 1..6 {
            let y = center.1 - (ROAD_WIDTH as i32 / 2) + (i * LANE_WIDTH as i32);
            canvas.draw_line(
                Point::new(0, y),
                Point::new(center.0 - (ROAD_WIDTH as i32 / 2), y)
            )?;
            canvas.draw_line(
                Point::new(center.0 + (ROAD_WIDTH as i32 / 2), y),
                Point::new(crate::WINDOW_WIDTH as i32, y)
            )?;
        }

        // Draw vertical lane markings (5 lines for 6 lanes)
        for i in 1..6 {
            let x = center.0 - (ROAD_WIDTH as i32 / 2) + (i * LANE_WIDTH as i32);
            canvas.draw_line(
                Point::new(x, 0),
                Point::new(x, center.1 - (ROAD_WIDTH as i32 / 2))
            )?;
            canvas.draw_line(
                Point::new(x, center.1 + (ROAD_WIDTH as i32 / 2)),
                Point::new(x, crate::WINDOW_HEIGHT as i32)
            )?;
        }

        Ok(())
    }

    // FIXED: Render a vehicle with proper sprite sheet handling
    pub fn render_vehicle(
        &mut self,
        canvas: &mut Canvas<Window>,
        vehicle: &Vehicle,
    ) -> Result<(), String> {
        // Calculate the render rectangle
        let render_rect = Rect::new(
            vehicle.position.x - (vehicle.width / 2) as i32,
            vehicle.position.y - (vehicle.height / 2) as i32,
            vehicle.width,
            vehicle.height,
        );

        // If we have vehicle textures
        if !self.vehicle_textures.is_empty() {
            let texture = &self.vehicle_textures[0]; // Use first texture
            let texture_query = texture.query();

            // FIXED: Check if this is a sprite sheet (larger than single vehicle)
            let src_rect = if texture_query.width > Vehicle::WIDTH && texture_query.height > Vehicle::HEIGHT {
                // Handle 2x2 sprite sheet (4 cars in quarters)
                let car_width = texture_query.width / 2;  // 80 pixels
                let car_height = texture_query.height / 2; // 80 pixels

                // FIXED: Map vehicle color to correct sprite position
                let (col, row) = match vehicle.color {
                    VehicleColor::Red => (0, 0),    // Top-left - Left turn
                    VehicleColor::Blue => (1, 0),   // Top-right - Straight
                    VehicleColor::Green => (0, 1),  // Bottom-left - Right turn
                    VehicleColor::Yellow => (1, 1), // Bottom-right - Special
                };

                Some(Rect::new(
                    (col * car_width) as i32,
                    (row * car_height) as i32,
                    car_width,
                    car_height
                ))
            } else {
                // Use different textures for different colors if available
                let texture_index = match vehicle.color {
                    VehicleColor::Red => 0,
                    VehicleColor::Blue => 1.min(self.vehicle_textures.len() - 1),
                    VehicleColor::Green => 2.min(self.vehicle_textures.len() - 1),
                    VehicleColor::Yellow => 3.min(self.vehicle_textures.len() - 1),
                };
                None // Use entire texture
            };

            // Select the appropriate texture
            let selected_texture = if src_rect.is_some() {
                &self.vehicle_textures[0] // Use sprite sheet
            } else {
                let texture_index = match vehicle.color {
                    VehicleColor::Red => 0,
                    VehicleColor::Blue => 1.min(self.vehicle_textures.len() - 1),
                    VehicleColor::Green => 2.min(self.vehicle_textures.len() - 1),
                    VehicleColor::Yellow => 3.min(self.vehicle_textures.len() - 1),
                };
                &self.vehicle_textures[texture_index]
            };

            // Render the vehicle texture with proper rotation
            canvas.copy_ex(
                selected_texture,
                src_rect,
                render_rect,
                vehicle.angle, // rotation angle in degrees
                Some(Point::new(
                    render_rect.width() as i32 / 2,
                    render_rect.height() as i32 / 2,
                )), // center of rotation
                false,     // don't flip horizontally
                false,     // don't flip vertically
            )?;
        } else {
            // Ultimate fallback to rendering a simple rectangle if no textures are available
            let color = match vehicle.color {
                VehicleColor::Red => Color::RGB(255, 0, 0),
                VehicleColor::Blue => Color::RGB(0, 0, 255),
                VehicleColor::Green => Color::RGB(0, 255, 0),
                VehicleColor::Yellow => Color::RGB(255, 255, 0),
            };
            canvas.set_draw_color(color);
            canvas.fill_rect(render_rect)?;

            // Add a border to make it look more like a car
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            canvas.draw_rect(render_rect)?;
        }

        Ok(())
    }
}