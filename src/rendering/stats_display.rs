use crate::direction::Direction;
use crate::simulation::statistics::Statistics;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureQuery};
use sdl2::ttf::Font;
use sdl2::video::Window;

pub fn render_stats_modal(
    canvas: &mut Canvas<Window>,
    stats: &Statistics,
    font: &Font,
) -> Result<(), String> {
    let summary = stats.get_summary();

    let (window_width, window_height) = canvas.output_size()?;
    canvas.set_draw_color(Color::RGBA(0, 0, 0, 180));
    canvas.fill_rect(Rect::new(0, 0, window_width, window_height))?;

    let modal_width = (window_width as f32 * 0.7) as u32;
    let modal_height = (window_height as f32 * 0.8) as u32;
    let modal_x = (window_width - modal_width) / 2;
    let modal_y = (window_height - modal_height) / 2;

    canvas.set_draw_color(Color::RGB(50, 50, 50));
    canvas.fill_rect(Rect::new(
        modal_x as i32,
        modal_y as i32,
        modal_width,
        modal_height,
    ))?;

    canvas.set_draw_color(Color::RGB(200, 200, 200));
    canvas.draw_rect(Rect::new(
        modal_x as i32,
        modal_y as i32,
        modal_width,
        modal_height,
    ))?;

    let _max_velocity_str = if summary.has_valid_data {
        format!("{:.1} pixels/frame", summary.max_velocity)
    } else {
        "N/A (no vehicles)".to_string()
    };

    let _min_velocity_str = if summary.has_valid_data {
        format!("{:.1} pixels/frame", summary.min_velocity)
    } else {
        "N/A (no vehicles)".to_string()
    };

    let max_time_str = if summary.total_vehicles_passed > 0 {
        format!("{:.2} seconds", summary.max_intersection_time)
    } else {
        "N/A (no vehicles passed)".to_string()
    };

    let min_time_str = if summary.total_vehicles_passed > 0 {
        format!("{:.2} seconds", summary.min_intersection_time)
    } else {
        "N/A (no vehicles passed)".to_string()
    };

    let stats_lines = vec![
        "Traffic Simulation Statistics".to_string(),
        "-------------------------".to_string(),
        format!("Total Vehicles Spawned: {}", summary.total_vehicles),
        format!("Max number of vehicles that passed the intersection: {}", summary.total_vehicles_passed),
        format!(
            "Max Vehicles in Intersection (simultaneously): {}",
            summary.max_vehicles_in_intersection
        ),
        format!("Simulation Duration: {:.2} seconds", summary.duration),
        String::new(),
        "Vehicle Speeds".to_string(),
        "-------------".to_string(),
        format!("Max velocity: 3.0 pixels/frame"),
        format!("Min velocity: 1.0 pixels/frame"),
        "(Vehicles have 3 speed levels: slow, medium, fast)".to_string(),
        String::new(),
        "Intersection Times".to_string(),
        "-----------------".to_string(),
        format!("Max time that took the vehicle to pass the intersection: {}", max_time_str),
        format!("Min time that took the vehicle to pass the intersection: {}", min_time_str),
        String::new(),
        "Safety Statistics".to_string(),
        "----------------".to_string(),
        format!("Close calls: {}", summary.total_close_calls),
        String::new(),
        "Vehicle Origins".to_string(),
        "--------------".to_string(),
        format!(
            "From North: {}",
            stats.vehicles_spawned.get(&Direction::Down).unwrap_or(&0)
        ),
        format!(
            "From South: {}",
            stats.vehicles_spawned.get(&Direction::Up).unwrap_or(&0)
        ),
        format!(
            "From East: {}",
            stats.vehicles_spawned.get(&Direction::Left).unwrap_or(&0)
        ),
        format!(
            "From West: {}",
            stats.vehicles_spawned.get(&Direction::Right).unwrap_or(&0)
        ),
        String::new(),
        "Press ESC again to close".to_string(),
    ];

    let mut y_offset = modal_y as i32 + 20;
    for line in stats_lines.iter() {
        if line.is_empty() {
            y_offset += 15;
            continue;
        }

        let surface = font
            .render(line)
            .blended(Color::RGB(255, 255, 255))
            .map_err(|e| e.to_string())?;

        let texture_creator = canvas.texture_creator();
        let texture = texture_creator
            .create_texture_from_surface(&surface)
            .map_err(|e| e.to_string())?;

        let TextureQuery { width, height, .. } = texture.query();

        let x = modal_x as i32 + ((modal_width as i32 - width as i32) / 2);
        canvas.copy(&texture, None, Some(Rect::new(x, y_offset, width, height)))?;

        y_offset += height as i32 + 5;
    }

    Ok(())
}
