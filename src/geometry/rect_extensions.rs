use sdl2::rect::Rect;

pub trait RectExtensions {
    fn is_in_bounds(&self, window_size: u32) -> bool;
}

impl RectExtensions for Rect {
    fn is_in_bounds(&self, window_size: u32) -> bool {
        let size = self.width() as i32;
        self.x() > -size
            && self.x() < window_size as i32
            && self.y() > -size
            && self.y() < window_size as i32
    }
}
