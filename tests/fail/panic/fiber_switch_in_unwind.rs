#[path = "../../utils/mod.rs"]
mod utils;

use std::panic::catch_unwind;
use std::sync::atomic::{AtomicUsize, Ordering};

static PARENT_FIBER: AtomicUsize = AtomicUsize::new(0);

fn fiber_body(_arg: *mut ()) -> ! {
    struct ExitOnDrop;

    impl Drop for ExitOnDrop {
        fn drop(&mut self) {
            unsafe {
                utils::miri_fiber_exit_to(PARENT_FIBER.load(Ordering::Relaxed)); //~ERROR: unsupported operation: Thread main requested a fiber switch while unwinding
            }
        }
    }

    let _ = catch_unwind(|| {
        let _guard = ExitOnDrop;
        panic!("unwind");
    });
    unsafe { utils::miri_fiber_exit_to(PARENT_FIBER.load(Ordering::Relaxed)) }
}

fn main() {
    unsafe {
        PARENT_FIBER.store(utils::miri_fiber_current(), Ordering::Relaxed);
        let fiber = utils::miri_fiber_create(fiber_body, core::ptr::null_mut());
        utils::miri_fiber_switch(fiber);
    }
}
