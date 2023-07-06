//! `std::thread`-like interface
//! INSPIRED BY https://github.com/rcore-os/rcore-thread/blob/master/src/std_thread.rs
//!
//! Based on Processor. Used in kernel.
//!
//! You need to implement the following functions before use:
//! - `processor`: Get a reference of the current `Processor`
//! - `new_kernel_context`: Construct a `Context` of the new kernel thread


use crate::serial_println;
use alloc::boxed::Box;
use core::marker::PhantomData;
use core::time::Duration;

#[linkage = "weak"]
#[no_mangle]
/// Get a reference of the current `Processor`
fn processor() -> &'static Processor {
    &crate::multi::processor::Processor::new()
}

#[linkage = "weak"]
#[no_mangle]
/// Construct a `Context` of the new kernel thread
fn new_kernel_context(_entry: extern "C" fn(usize) -> !, _arg: usize) -> Box<dyn Context> {
    // super::context::
}

/// Gets a handle to the thread that invokes it.
pub fn current() -> Thread {
    Thread {
        tid: processor().tid(),
    }
}

/// Puts the current thread to sleep for the specified amount of time.
pub fn sleep(dur: Duration) {
    let time = dur_to_ticks(dur);
    serial_println!("sleep: {:?} ticks", time);
    processor().manager().sleep(current().id(), time);
    park();

    fn dur_to_ticks(dur: Duration) -> usize {
        return dur.as_secs() as usize * 100 + dur.subsec_nanos() as usize / 10_000_000;
    }
}

/// Spawns a new thread, returning a JoinHandle for it.
///
/// `F`: Type of the function `f`
/// `T`: Type of the return value of `f`
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: Send + 'static + FnOnce() -> T,
    T: Send + 'static,
{
    serial_println!("spawn:");

    let f = Box::into_raw(Box::new(f));

    extern "C" fn kernel_thread_entry<F, T>(f: usize) -> !
    where
        F: Send + 'static + FnOnce() -> T,
        T: Send + 'static,
    {
        let f = unsafe { Box::from_raw(f as *mut F) };
        let ret = Box::new(f());
        let exit_code = Box::into_raw(ret) as usize;
        processor().manager().exit(current().id(), exit_code);
        yield_now();
        unreachable!()
    }

    let context = new_kernel_context(kernel_thread_entry::<F, T>, f as usize);
    let tid = processor().manager().add(context);

    return JoinHandle {
        thread: Thread { tid },
        mark: PhantomData,
    };
}

/// Cooperatively gives up a time slice to the OS scheduler.
pub fn yield_now() {
    serial_println!("yield:");
    no_interrupt(|| {
        processor().yield_now();
    });
}

/// Blocks unless or until the current thread's token is made available.
pub fn park() {
    serial_println!("park:");
    processor().manager().sleep(current().id(), 0);
    yield_now();
}

/// Blocks unless or until the current thread's token is made available.
/// Calls `f` before thread yields. Can be used to avoid racing.
pub fn park_action(f: impl FnOnce()) {
    serial_println!("park:");
    processor().manager().sleep(current().id(), 0);
    f();
    yield_now();
}

/// A handle to a thread.
pub struct Thread {
    tid: usize,
}

impl Thread {
    /// Atomically makes the handle's token available if it is not already.
    pub fn unpark(&self) {
        processor().manager().wakeup(self.tid);
    }
    /// Gets the thread's unique identifier.
    pub fn id(&self) -> usize {
        self.tid
    }
}

/// An owned permission to join on a thread (block on its termination).
pub struct JoinHandle<T> {
    thread: Thread,
    mark: PhantomData<T>,
}

impl<T> JoinHandle<T> {
    /// Extracts a handle to the underlying thread.
    pub fn thread(&self) -> &Thread {
        &self.thread
    }
    /// Waits for the associated thread to finish.
    pub fn join(self) -> Result<T, ()> {
        loop {
            serial_println!("try to join thread {}", self.thread.tid);
            if let Some(exit_code) = processor().manager().try_remove(self.thread.tid) {
                // Do not call drop function
                core::mem::forget(self);
                // Find return value on the heap from the exit code.
                return Ok(unsafe { *Box::from_raw(exit_code as *mut T) });
            }
            processor().manager().wait(current().id(), self.thread.tid);
            yield_now();
        }
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        processor().manager().detach(self.thread.tid);
    }
}