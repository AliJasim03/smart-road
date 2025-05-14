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
    road_texture: Option<sdl2::render::Texture<'a>>,
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
                // Create a fallback texture
                let texture = Self::create_vehicle_texture(texture_creator, Color::RGB(255, 255, 255))?;
                vehicle_textures.push(texture);
            }
        }

        // Load road texture
        let road_texture_result = texture_creator.load_texture("assets/road/road.png");
        let road_texture = match road_texture_result {
            Ok(texture) => {
                println!("Successfully loaded road.png texture");
                Some(texture)
            }
            Err(e) => {
                println!("Warning: Could not load road texture: {}", e);
                None
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
                None
            }
        };

        Ok(Renderer {
            vehicle_textures,
            road_texture,
            acera_texture,
            texture_creator,
        })
    }

    // Create a vehicle texture with the specified color (as fallback)
    fn create_vehicle_texture(
        texture_creator: &TextureCreator<WindowContext>,
        color: Color,
    ) -> Result<sdl2::render::Texture, String> {
        // Create a surface for the vehicle
        let mut surface = Surface::new(
            Vehicle::WIDTH,
            Vehicle::HEIGHT,
            sdl2::pixels::PixelFormatEnum::RGBA8888,
        )
            .map_err(|e| e.to_string())?;

        // Fill the surface with the specified color
        surface
            .fill_rect(
                Rect::new(0, 0, Vehicle::WIDTH, Vehicle::HEIGHT),
                color,
            )
            .map_err(|e| e.to_string())?;

        // Add some details to make it look more like a car
        surface
            .fill_rect(
                Rect::new(5, 15, Vehicle::WIDTH - 10, 20),
                Color::RGB(200, 200, 255),
            )
            .map_err(|e| e.to_string())?;

        surface
            .fill_rect(
                Rect::new(5, (Vehicle::HEIGHT - 25) as i32, 10, 10),
                Color::RGB(0, 0, 0),
            )
            .map_err(|e| e.to_string())?;

        surface
            .fill_rect(
                Rect::new((Vehicle::WIDTH - 15) as i32, (Vehicle::HEIGHT - 25) as i32, 10, 10),
                Color::RGB(0, 0, 0),
            )
            .map_err(|e| e.to_string())?;

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

        // Draw the roads using road texture
        if let Some(road_texture) = &self.road_texture {
            let center = intersection_center();

            // Horizontal road (east-west)
            let horizontal_road = Rect::new(
                0,
                center.1 - (ROAD_WIDTH as i32 / 2),
                crate::WINDOW_WIDTH,
                ROAD_WIDTH,
            );

            // Vertical road (north-south)
            let vertical_road = Rect::new(
                center.0 - (ROAD_WIDTH as i32 / 2),
                0,
                ROAD_WIDTH,
                crate::WINDOW_HEIGHT,
            );

            // Draw the roads
            canvas.copy(road_texture, None, Some(horizontal_road))?;
            canvas.copy(road_texture, None, Some(vertical_road))?;
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
            // Get the vehicle texture
            let texture = &self.vehicle_textures[0];

            // Source rectangle for the car in the texture
            // Our cars.png has 4 cars in a 2x2 grid
            // We'll select based on vehicle.color
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

            let src_rect = Rect::new(
                (col * car_width) as i32,
                (row * car_height) as i32,
                car_width,
                car_height
            );

            // Set angle based on direction
            let angle = match vehicle.direction {
                Direction::North => 0.0,   // Facing up
                Direction::South => 180.0, // Facing down
                Direction::East => 90.0,   // Facing right
                Direction::West => 270.0,  // Facing left
            };

            // Render the vehicle texture
            canvas.copy_ex(
                texture,
                Some(src_rect), // Use the specific part of the texture
                render_rect,
                angle, // rotation angle in degrees
                Some(Point::new(
                    render_rect.width() as i32 / 2,
                    render_rect.height() as i32 / 2,
                )), // center of rotation
                false,     // don't flip horizontally
                false,     // don't flip vertically
            )?;
        } else {
            // Fallback to rendering a simple rectangle if texture isn't available
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