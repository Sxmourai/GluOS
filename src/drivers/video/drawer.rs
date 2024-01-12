use super::{
    frame_buffer::{put_pixel_lock, ScreenLock},
    Color,
};

pub struct Pos {
    pub x: u16,
    pub y: u8,
}
impl Pos {
    pub fn new(x: u16, y: u8) -> Self {
        Self { x, y }
    }
    pub fn center() -> Self {
        Self::new(160, 100)
    }
    pub fn origin() -> Self {
        Self::new(0, 0)
    }
    pub fn top_left() -> Self {
        Self::origin()
    }
    pub fn top_right() -> Self {
        Self::new(320, 0)
    }
    pub fn bottom_left() -> Self {
        Self::new(0, 200)
    }
    pub fn bottom_right() -> Self {
        Self::new(320, 200)
    }
}
pub trait Shape {
    fn draw(&mut self, color: Color, screen: &mut ScreenLock);
}
pub struct Rect {
    pub pos: Pos,
    pub size: Pos,
}
impl Shape for Rect {
    fn draw(&mut self, color: Color, screen: &mut ScreenLock) {
        for x in self.pos.x..self.pos.x + self.size.x {
            for y in self.pos.y..self.pos.y + self.size.y {
                put_pixel_lock(x.into(), y.into(), color, screen);
            }
        }
    }
}
pub struct Circle {
    pub center: Pos,
    pub radius: u8,
}
impl Shape for Circle {
    fn draw(&mut self, _color: Color, _screen: &mut ScreenLock) {}
}

// pub struct ComplexShape {
//     shapes: Vec<(Box<dyn Shape>, Color)>,
// }
// impl Shape for ComplexShape {
//     fn draw(&mut self, color: Color, screen: &mut ScreenLock) {
//         for (mut shape, color) in &self.shapes {
//             shape.draw(color, screen);
//         }
//     }
// }
