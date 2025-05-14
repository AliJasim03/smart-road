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

        // Load vehicle texture
        let mut vehicle_textures = Vec::new();
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
                    Color::RGB(255, 50, 50),    // Red for Left turn
                    Color::RGB(50, 50, 255),    // Blue for Straight
                    Color::RGB(50, 200, 50),    // Green for Right turn
                    Color::RGB(255, 255, 50),   // Yellow for special cases
                ];

                for color in colors {
                    let texture = Self::create_vehicle_texture(texture_creator, color)?;
                    vehicle_textures.push(texture);
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
                Rect::new(5, 15, Vehicle::WIDTH - 10, Vehicle::HEIGHT - 40),
                Color::RGB(220, 220, 255),
            )
            .map_err(|e| e.to_string())?;

        // Car windows
        surface
            .fill_rect(
                Rect::new(8, 18, Vehicle::WIDTH - 16, Vehicle::HEIGHT - 46),
                Color::RGB(100, 180, 255),
            )
            .map_err(|e| e.to_string())?;

        // Front windshield
        surface
            .fill_rect(
                Rect::new(8, 18, Vehicle::WIDTH - 16, 12),
                Color::RGB(120, 200, 255),
            )
            .map_err(|e| e.to_string())?;

        // Back windshield - Fixed: Converting u32 to i32
        surface
            .fill_rect(
                Rect::new(8, (Vehicle::HEIGHT - 30) as i32, Vehicle::WIDTH - 16, 12),
                Color::RGB(120, 200, 255),
            )
            .map_err(|e| e.to_string())?;

        // Front headlights
        surface
            .fill_rect(
                Rect::new(5, 5, 8, 8),
                Color::RGB(255, 255, 200),
            )
            .map_err(|e| e.to_string())?;
        surface
            .fill_rect(
                Rect::new((Vehicle::WIDTH as i32) - 13, 5, 8, 8),
                Color::RGB(255, 255, 200),
            )
            .map_err(|e| e.to_string())?;

        // Rear lights - Fixed: Converting u32 to i32
        surface
            .fill_rect(
                Rect::new(5, (Vehicle::HEIGHT as i32) - 13, 8, 8),
                Color::RGB(255, 50, 50),
            )
            .map_err(|e| e.to_string())?;
        surface
            .fill_rect(
                Rect::new((Vehicle::WIDTH as i32) - 13, (Vehicle::HEIGHT as i32) - 13, 8, 8),
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

    // Render a vehicle
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
            // Get the appropriate texture based on the vehicle's color/route
            let texture_index = if self.vehicle_textures.len() > 1 {
                // If we have multiple textures, select based on color
                match vehicle.color {
                    VehicleColor::Red => 0,
                    VehicleColor::Blue => 1,
                    VehicleColor::Green => 2,
                    VehicleColor::Yellow => 3 % self.vehicle_textures.len(),
                }
            } else {
                // If we only have one texture, use it
                0
            };

            let texture = &self.vehicle_textures[texture_index];

            // Source rectangle for the car in the texture
            let src_rect = if self.vehicle_textures.len() == 1 && texture.query().width > Vehicle::WIDTH {
                // If we loaded a sprite sheet, use part of it
                // Our cars.png has 4 cars in a 2x2 grid
                let car_width = texture.query().width / 2;  // Assuming 2 columns
                let car_height = texture.query().height / 2; // Assuming 2 rows

                let color_index = match vehicle.color {
                    VehicleColor::Red => 0,    // Top-left
                    VehicleColor::Blue => 1,   // Top-right
                    VehicleColor::Green => 2,  // Bottom-left
                    VehicleColor::Yellow => 3, // Bottom-right
                };

                let col = color_index % 2;
                let row = color_index / 2;

                Some(Rect::new(
                    (col * car_width) as i32,
                    (row * car_height) as i32,
                    car_width,
                    car_height
                ))
            } else {
                // Use the entire texture
                None
            };

            // Render the vehicle texture
            canvas.copy_ex(
                texture,
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
        }

        Ok(())
    }
}
