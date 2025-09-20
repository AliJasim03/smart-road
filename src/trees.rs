use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

pub struct Tree {
    pub x: i32,
    pub y: i32,
    pub size: i32,
}

impl Tree {
    pub fn new(x: i32, y: i32, size: i32) -> Self {
        Tree { x, y, size }
    }

    pub fn draw(&self, canvas: &mut Canvas<Window>) {
        // Draw trunk
        canvas.set_draw_color(Color::RGB(139, 69, 19)); // Brown color
        canvas
            .fill_rect(Rect::new(
                self.x,
                self.y,
                (self.size / 2) as u32,
                self.size as u32,
            ))
            .unwrap();

        // Draw leaves
        canvas.set_draw_color(Color::RGB(34, 139, 34)); // Forest green
        let tree_points = [
            (self.x - self.size / 2, self.y + self.size / 2), // Left point
            (self.x + self.size, self.y + self.size / 2),     // Right point
            (self.x + self.size / 4, self.y - self.size),     // Top point
        ];

        for y in (self.y - self.size)..(self.y + self.size / 2) {
            for x in (self.x - self.size / 2)..(self.x + self.size) {
                let point = (x, y);
                if is_point_in_triangle(point, tree_points[0], tree_points[1], tree_points[2]) {
                    canvas.draw_point((point.0 as i32, point.1 as i32)).unwrap();
                }
            }
        }
    }
}

// Helper function for triangle calculation
fn is_point_in_triangle(point: (i32, i32), v1: (i32, i32), v2: (i32, i32), v3: (i32, i32)) -> bool {
    let p = point;
    let a = v1;
    let b = v2;
    let c = v3;

    let area = 0.5 * (-b.1 * c.0 + a.1 * (-b.0 + c.0) + a.0 * (b.1 - c.1) + b.0 * c.1) as f32;
    let s =
        1.0 / (2.0 * area) * (a.1 * c.0 - a.0 * c.1 + (c.1 - a.1) * p.0 + (a.0 - c.0) * p.1) as f32;
    let t =
        1.0 / (2.0 * area) * (a.0 * b.1 - a.1 * b.0 + (a.1 - b.1) * p.0 + (b.0 - a.0) * p.1) as f32;

    s >= 0.0 && t >= 0.0 && (1.0 - s - t) >= 0.0
}
