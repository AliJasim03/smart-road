use sdl2::image::{InitFlag, LoadTexture};
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use crate::intersection::{intersection_area, intersection_center, Intersection, ROAD_WIDTH};
use crate::vehicle::{Direction, Route, Vehicle};

pub struct Renderer<'a> {
    vehicle_textures: Vec<sdl2::render::Texture<'a>>,
    road_texture: Option<sdl2::render::Texture<'a>>,
    texture_creator: &'a TextureCreator<WindowContext>,
}

impl<'a> Renderer<'a> {
    // In the Renderer::new function in renderer.rs

    pub fn new(texture_creator: &'a TextureCreator<WindowContext>) -> Result<Self, String> {
        // Initialize SDL2 image
        sdl2::image::init(InitFlag::PNG)?; // Make sure SVG flag is included

        // Print the current working directory to debug file paths
        println!(
            "Current working directory: {:?}",
            std::env::current_dir().unwrap_or_default()
        );

        // Try to load the car texture
        let car_path = "assets/vehicles/car.svg";
        println!("Attempting to load car texture from: {}", car_path);
        let vehicle_texture_result = texture_creator.load_texture(car_path);

        // Handle potential errors gracefully
        let vehicle_texture = match vehicle_texture_result {
            Ok(texture) => {
                println!("Successfully loaded car texture");
                texture
            }
            Err(e) => {
                println!("Warning: Could not load vehicle texture: {}", e);
                Self::create_vehicle_texture(texture_creator)?
            }
        };

        // Try to load the road texture
        let road_path = "assets/road/intersection.svg";
        println!("Attempting to load road texture from: {}", road_path);
        let road_texture_result = texture_creator.load_texture(road_path);

        // Handle potential errors gracefully
        let road_texture = match road_texture_result {
            Ok(texture) => {
                println!("Successfully loaded road texture");
                Some(texture)
            }
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
    fn create_vehicle_texture(
        texture_creator: &TextureCreator<WindowContext>,
    ) -> Result<sdl2::render::Texture, String> {
        // Create a surface for the vehicle
        let mut surface = Surface::new(
            Vehicle::WIDTH,
            Vehicle::HEIGHT,
            sdl2::pixels::PixelFormatEnum::RGBA8888,
        )
        .map_err(|e| e.to_string())?;

        // Fill the surface with a car shape (simple rectangle for now)
        surface
            .fill_rect(
                Rect::new(0, 0, Vehicle::WIDTH, Vehicle::HEIGHT),
                Color::RGB(0, 0, 255),
            )
            .map_err(|e| e.to_string())?;

        // Create texture from surface
        let texture = texture_creator
            .create_texture_from_surface(surface)
            .map_err(|e| e.to_string())?;

        Ok(texture)
    }

    // Render the intersection
    pub fn render_intersection(
        &self,
        canvas: &mut Canvas<Window>,
        _intersection: &Intersection,
    ) -> Result<(), String> {
        if let Some(road_texture) = &self.road_texture {
            // Draw the road texture to fill the entire screen
            let dest_rect = Rect::new(0, 0, crate::WINDOW_WIDTH, crate::WINDOW_HEIGHT);
            canvas.copy(road_texture, None, Some(dest_rect))?;

            // Render lane indicators at the corners of the intersection
            let indicator_size = 60;
            let center = intersection_center();
            let indicator_texture_result = self
                .texture_creator
                .load_texture("assets/road/lane_indicator.svg");
            if let Ok(indicator_texture) = indicator_texture_result {
                // Top-left corner
                let tl_rect = Rect::new(
                    center.0 - 120,
                    center.1 - 120,
                    indicator_size,
                    indicator_size,
                );
                canvas.copy(&indicator_texture, None, Some(tl_rect))?;

                // Top-right corner
                let tr_rect = Rect::new(
                    center.0 + 60,
                    center.1 - 120,
                    indicator_size,
                    indicator_size,
                );
                canvas.copy(&indicator_texture, None, Some(tr_rect))?;

                // Bottom-left corner
                let bl_rect = Rect::new(
                    center.0 - 120,
                    center.1 + 60,
                    indicator_size,
                    indicator_size,
                );
                canvas.copy(&indicator_texture, None, Some(bl_rect))?;

                // Bottom-right corner
                let br_rect =
                    Rect::new(center.0 + 60, center.1 + 60, indicator_size, indicator_size);
                canvas.copy(&indicator_texture, None, Some(br_rect))?;
            }
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
                canvas.fill_rect(Rect::new(x, y - 2, dash_length as u32, 4))?;
                x += dash_length + space_length;
            }

            // Vertical road lane markings (dashed line in the middle)
            let mut y = 0;
            let x = (crate::WINDOW_WIDTH / 2) as i32;

            while y < crate::WINDOW_HEIGHT as i32 {
                canvas.fill_rect(Rect::new(x - 2, y, 4, dash_length as u32))?;
                y += dash_length + space_length;
            }

            // Draw the intersection boundary (for debugging)
            canvas.set_draw_color(Color::RGB(200, 200, 200)); // Light gray for intersection
            canvas.draw_rect(intersection_area())?;

            // Try to render lane indicators even in fallback mode
            let indicator_size = 60;
            let center = intersection_center();
            let indicator_texture_result = self
                .texture_creator
                .load_texture("assets/road/lane_indicator.svg");
            if let Ok(indicator_texture) = indicator_texture_result {
                // Top-left corner
                let tl_rect = Rect::new(
                    center.0 - 120,
                    center.1 - 120,
                    indicator_size,
                    indicator_size,
                );
                canvas.copy(&indicator_texture, None, Some(tl_rect))?;

                // Top-right corner
                let tr_rect = Rect::new(
                    center.0 + 60,
                    center.1 - 120,
                    indicator_size,
                    indicator_size,
                );
                canvas.copy(&indicator_texture, None, Some(tr_rect))?;

                // Bottom-left corner
                let bl_rect = Rect::new(
                    center.0 - 120,
                    center.1 + 60,
                    indicator_size,
                    indicator_size,
                );
                canvas.copy(&indicator_texture, None, Some(bl_rect))?;

                // Bottom-right corner
                let br_rect =
                    Rect::new(center.0 + 60, center.1 + 60, indicator_size, indicator_size);
                canvas.copy(&indicator_texture, None, Some(tr_rect))?;
            }
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

        // Choose color based on route
        let color = match vehicle.route {
            Route::Left => Color::RGB(255, 100, 100), // Reddish tint for left turn
            Route::Straight => Color::RGB(100, 255, 100), // Greenish tint for straight
            Route::Right => Color::RGB(100, 100, 255), // Bluish tint for right turn
        };

        // Render the vehicle at the correct angle
        if let Some(texture) = self.vehicle_textures.get(0) {
            // Set the initial angle based on direction
            let base_angle = match vehicle.direction {
                Direction::North => 0.0,   // Facing up (needs 90 degree adjustment)
                Direction::South => 180.0, // Facing down (needs 90 degree adjustment)
                Direction::East => 270.0,  // Facing left
                Direction::West => 90.0,   // Facing right
            };

            // For North/South directions, we need to adjust the angle by 90 degrees
            // because our car SVG is horizontal by default
            let adjusted_angle =
                if vehicle.direction == Direction::North || vehicle.direction == Direction::South {
                    base_angle + 90.0
                } else {
                    base_angle
                };

            // Convert the angle from degrees to the format expected by SDL
            // SDL2 rotation is clockwise, with 0 at the positive x-axis
            // Our angle is counterclockwise with 0 at the positive y-axis
            let sdl_angle = (90.0 - adjusted_angle) % 360.0;

            // For North/South directions, we need to swap width and height
            let (render_width, render_height) =
                if vehicle.direction == Direction::North || vehicle.direction == Direction::South {
                    (vehicle.height, vehicle.width)
                } else {
                    (vehicle.width, vehicle.height)
                };

            let render_rect = Rect::new(
                vehicle.position.x - (render_width / 2) as i32,
                vehicle.position.y - (render_height / 2) as i32,
                render_width,
                render_height,
            );

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
                Some(Point::new(
                    render_rect.width() as i32 / 2,
                    render_rect.height() as i32 / 2,
                )), // center of rotation
                false,     // don't flip horizontally
                false,     // don't flip vertically
            )?;
        } else {
            // Fallback to rendering a simple rectangle if texture isn't available
            canvas.set_draw_color(color);
            canvas.fill_rect(render_rect)?;
        }

        Ok(())
    }
}
