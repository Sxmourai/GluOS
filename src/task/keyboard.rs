use crate::{println, vga_buffer::{ScreenChar, print_char_at, print_byte_at, print_screenchar_at, ColorCode, Color, Buffer, BUFFER_WIDTH, BUFFER_HEIGHT}, serial_println};
use alloc::vec::Vec;
use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use lazy_static::lazy_static;
use core::{pin::Pin, task::{Poll, Context}};
use futures_util::stream::Stream;
use futures_util::task::AtomicWaker;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use crate::vga_buffer::ScreenPos;

static WAKER: AtomicWaker = AtomicWaker::new();
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
lazy_static!{static ref DEFAULT_KEYBOARD: Keyboard<layouts::Us104Key, ScancodeSet1> = Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore);}


/// Called by the keyboard interrupt handler
///
/// Must not block or allocate.
// pub(crate) fn add_scancode(scancode: u8) {
//     if let Ok(queue) = SCANCODE_QUEUE.try_get() {
//         if let Err(_) = queue.push(scancode) {
//             println!("WARNING: scancode queue full; dropping keyboard input");
//         } else {
//             WAKER.wake();
//         }
//     } else {
//         println!("WARNING: scancode queue uninitialized");
//     }
// }

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE.try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialized");

        // fast path
        if let Ok(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(&cx.waker());
        match queue.pop() {
            Ok(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            Err(crossbeam_queue::PopError) => Poll::Pending,
        }
    }
}