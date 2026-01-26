#[path = "../../utils/mod.rs"]
mod utils;

use std::hint::spin_loop;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::thread::scope;

static PARENT_FIBER: AtomicUsize = AtomicUsize::new(0);

fn fiber_body(_arg: *mut ()) -> ! {
    unsafe {
        utils::miri_fiber_switch(PARENT_FIBER.load(Ordering::Relaxed));
        utils::miri_fiber_exit_to(PARENT_FIBER.load(Ordering::Relaxed))
    }
}

fn main() {
    unsafe {
        let main_fiber = utils::miri_fiber_current();

        let fiber = AtomicUsize::new(main_fiber);
        let step = AtomicU8::new(0);
        scope(|s| {
            s.spawn(|| {
                let f = utils::miri_fiber_create(fiber_body, core::ptr::null_mut());
                fiber.store(f, Ordering::Relaxed);
                step.store(1, Ordering::Release);
            });

            while step.load(Ordering::Acquire) == 0 {
                spin_loop();
            }

            s.spawn(|| {
                PARENT_FIBER.store(utils::miri_fiber_current(), Ordering::Relaxed);
                let f = fiber.load(Ordering::Relaxed);
                utils::miri_fiber_switch(f);
                step.store(2, Ordering::Relaxed);
            });

            while step.load(Ordering::Relaxed) == 1 {
                spin_loop();
            }

            let f = fiber.load(Ordering::Relaxed);
            PARENT_FIBER.store(utils::miri_fiber_current(), Ordering::Relaxed);
            utils::miri_fiber_switch(f); //~ ERROR: Data race detected between (1) non-atomic write on thread `unnamed-2` and (2) non-atomic write on thread `main` at alloc1618
        });
    }
}
