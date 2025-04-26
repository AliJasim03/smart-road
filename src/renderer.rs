use sdl2::image::{InitFlag, LoadTexture};
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use crate::intersection::{Intersection, intersection_area, ROAD_WIDTH};
use crate::vehicle::{Route, Vehicle};

pub struct Renderer<'a> {
    vehicle_textures: Vec<sdl2::render::Texture<'a>>,
    road_texture: Option<sdl2::render::Texture<'a>>,
    texture_creator: &'a TextureCreator<WindowContext>,
}

impl<'a> Renderer<'a> {
    pub fn new(texture_creator: &'a TextureCreator<WindowContext>) -> Result<Self, String> {
        // Initialize SDL2 image
        sdl2::image::init(InitFlag::PNG)?;

        // Try to load the car texture
        let vehicle_texture_result = texture_creator.load_texture("assets/vehicles/car.svg");

        // Handle potential errors gracefully
        let vehicle_texture = match vehicle_texture_result {
            Ok(texture) => texture,
            Err(e) => {
                println!("Warning: Could not load vehicle texture: {}", e);
                Self::create_vehicle_texture(texture_creator)?
            }
        };

        // Try to load the road texture
        let road_texture_result = texture_creator.load_texture("assets/road/intersection.svg");

        // Handle potential errors gracefully
        let road_texture = match road_texture_result {
            Ok(texture) => Some(texture),
            Err(e) => {
                println!("Warning: Could not load road texture: {}", e);
                None
            }
        };

        Ok(Renderer {
            vehicle_textures: vec![vehicle_texture],
            road_texture,
            texture_creator,
        })
    }

    // Create a simple vehicle texture (placeholder)
    fn create_vehicle_texture(texture_creator: &TextureCreator<WindowContext>) -> Result<sdl2::render::Texture, String> {
        // Create a surface for the vehicle
        let mut surface = Surface::new(Vehicle::WIDTH, Vehicle::HEIGHT, sdl2::pixels::PixelFormatEnum::RGBA8888)
            .map_err(|e| e.to_string())?;

        // Fill the surface with a car shape (simple rectangle for now)
        surface.fill_rect(Rect::new(0, 0, Vehicle::WIDTH, Vehicle::HEIGHT), Color::RGB(0, 0, 255))
            .map_err(|e| e.to_string())?;

        // Create texture from surface
        let texture = texture_creator.create_texture_from_surface(surface)
            .map_err(|e| e.to_string())?;

        Ok(texture)
    }

    // Render the intersection
    pub fn render_intersection(&self, canvas: &mut Canvas<Window>, _intersection: &Intersection) -> Result<(), String> {
        if let Some(road_texture) = &self.road_texture {
            // Draw the road texture to fill the entire screen
            let dest_rect = Rect::new(0, 0, crate::WINDOW_WIDTH, crate::WINDOW_HEIGHT);
            canvas.copy(road_texture, None, Some(dest_rect))?;
        } else {
            // Fallback to drawing the intersection manually if texture is not available
            // Set background color for road
            canvas.set_draw_color(Color::RGB(50, 50, 50)); // Dark gray for road

            // Draw horizontal road
            canvas.fill_rect(Rect::new(
                0,
                (crate::WINDOW_HEIGHT / 2 - ROAD_WIDTH / 2) as i32,
                crate::WINDOW_WIDTH,
                ROAD_WIDTH,
            ))?;

            // Draw vertical road
            canvas.fill_rect(Rect::new(
                (crate::WINDOW_WIDTH / 2 - ROAD_WIDTH / 2) as i32,
                0,
                ROAD_WIDTH,
                crate::WINDOW_HEIGHT,
            ))?;

            // Draw lane markings
            canvas.set_draw_color(Color::RGB(255, 255, 0)); // Yellow for lane markings

            // Horizontal road lane markings (dashed line in the middle)
            let dash_length = 20;
            let space_length = 20;
            let mut x = 0;
            let y = (crate::WINDOW_HEIGHT / 2) as i32;

            while x < crate::WINDOW_WIDTH as i32 {
                canvas.fill_rect(Rect::new(
                    x,
                    y - 2,
                    dash_length as u32,
                    4,
                ))?;
                x += dash_length + space_length;
            }

            // Vertical road lane markings (dashed line in the middle)
            let mut y = 0;
            let x = (crate::WINDOW_WIDTH / 2) as i32;

            while y < crate::WINDOW_HEIGHT as i32 {
                canvas.fill_rect(Rect::new(
                    x - 2,
                    y,
                    4,
                    dash_length as u32,
                ))?;
                y += dash_length + space_length;
            }

            // Draw the intersection boundary (for debugging)
            canvas.set_draw_color(Color::RGB(200, 200, 200)); // Light gray for intersection
            canvas.draw_rect(intersection_area())?;
        }

        Ok(())
    }

    // Render a vehicle
    pub fn render_vehicle(&self, canvas: &mut Canvas<Window>, vehicle: &Vehicle) -> Result<(), String> {
        // Calculate the render rectangle
        let render_rect = Rect::new(
            vehicle.position.x - (vehicle.width / 2) as i32,
            vehicle.position.y - (vehicle.height / 2) as i32,
            vehicle.width,
            vehicle.height,
        );

        // Choose color based on route
        let color = match vehicle.route {
            Route::Left => Color::RGB(255, 100, 100),     // Reddish tint for left turn
            Route::Straight => Color::RGB(100, 255, 100), // Greenish tint for straight
            Route::Right => Color::RGB(100, 100, 255),    // Bluish tint for right turn
        };

        // Render the vehicle at the correct angle
        if let Some(texture) = self.vehicle_textures.get(0) {
            // Convert the angle from degrees to the format expected by SDL
            // SDL2 rotation is clockwise, with 0 at the positive x-axis
            // Our angle is counterclockwise with 0 at the positive y-axis
            let sdl_angle = (90.0 - vehicle.angle) % 360.0;

            // Since we can't modify the texture color directly with our immutable reference,
            // we'll first draw a colored rectangle to represent the route
            canvas.set_draw_color(color);
            canvas.fill_rect(render_rect)?;

            // Then render the vehicle texture with some transparency
            canvas.copy_ex(
                texture,
                None, // Use the entire texture
                render_rect,
                sdl_angle, // rotation angle in degrees
                Some(Point::new(render_rect.width() as i32 / 2, render_rect.height() as i32 / 2)), // center of rotation
                false, // don't flip horizontally
                false, // don't flip vertically
            )?;
        } else {
            // Fallback to rendering a simple rectangle if texture isn't available
            canvas.set_draw_color(color);
            canvas.fill_rect(render_rect)?;
        }

        Ok(())
    }
}