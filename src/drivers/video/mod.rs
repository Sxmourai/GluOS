use alloc::boxed::Box;

use self::{
    drawer::{Circle, Pos, Rect, Shape},
    frame_buffer::{SCREEN, SH},
};

pub mod drawer;
pub mod frame_buffer;
pub type Color = u8;
/// Draws some shapes on screen, but isn't used because we have a text vga buffer, and this is used for pixels.
pub fn init() {
    let shapes: [(Box<dyn Shape>, Color); 3] = [
        (
            Box::new(Rect {
                pos: Pos::new(0, SH - 30),
                size: Pos::new(320, 30),
            }),
            55,
        ),
        (
            Box::new(Rect {
                pos: Pos::origin(),
                size: Pos::bottom_right(),
            }),
            7,
        ),
        (
            Box::new(Circle {
                center: Pos::center(),
                radius: 5,
            }),
            8,
        ),
    ];
    let mut screen_lock = SCREEN.write();
    for (mut shape, color) in shapes {
        shape.draw(color, &mut screen_lock);
    }
}
