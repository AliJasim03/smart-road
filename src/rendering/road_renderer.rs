use crate::constants::*;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

pub struct RoadRenderer;

impl RoadRenderer {
    pub fn render_background(canvas: &mut Canvas<Window>) {
        canvas.set_draw_color(Color::RGB(50, 205, 50));
        canvas.clear();
    }

    pub fn render_road_surface(canvas: &mut Canvas<Window>) {
        canvas.set_draw_color(Color::RGB(51, 51, 51));

        canvas
            .fill_rect(Rect::new(
                5 * LINE_SPACING,
                0,
                (11 - 5) * LINE_SPACING as u32,
                WINDOW_SIZE,
            ))
            .unwrap();

        canvas
            .fill_rect(Rect::new(
                0,
                5 * LINE_SPACING - 1,
                WINDOW_SIZE,
                (11 - 5) * LINE_SPACING as u32,
            ))
            .unwrap();
    }

    pub fn render_lane_markers(canvas: &mut Canvas<Window>) {
        canvas.set_draw_color(Color::RGB(255, 255, 255));

        for i in 5..=11 {
            let x = i * LINE_SPACING;
            canvas.draw_line((x, 0), (x, 5 * LINE_SPACING)).unwrap();
            canvas
                .draw_line((x, 11 * LINE_SPACING), (x, WINDOW_SIZE as i32))
                .unwrap();

            canvas.draw_line((0, x), (5 * LINE_SPACING, x)).unwrap();
            canvas
                .draw_line((11 * LINE_SPACING, x), (WINDOW_SIZE as i32, x))
                .unwrap();
        }
    }
}
