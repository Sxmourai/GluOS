use alloc::{vec::Vec, boxed::Box};

use self::{frame_buffer::{SW, fill_rect, SH, SCREEN}, drawer::{Rect, Pos, Shape, Circle}};

pub mod frame_buffer;
pub mod drawer;
pub type Color=u8;

pub fn init_graphics() {
    let shapes: [(Box<dyn Shape>, Color); 3] = [
        (Box::new(Rect  {pos: Pos::new(0, SH-30), size: Pos::new(320, 30)}), 55),
        (Box::new(Rect  {pos: Pos::origin(), size: Pos::bottom_right()}), 7),
        (Box::new(Circle{center: Pos::center(), radius: 5}), 8),
    ];
    let mut screen_lock = SCREEN.write();
    for (mut shape, color) in shapes {
        shape.draw(color, &mut screen_lock);
    }
}

