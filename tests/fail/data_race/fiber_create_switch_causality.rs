#[path = "../../utils/mod.rs"]
mod utils;

use std::hint::spin_loop;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::scope;

static PARENT_FIBER: AtomicUsize = AtomicUsize::new(0);

fn fiber_body(_arg: *mut ()) -> ! {
    println!("Fiber body");
    unsafe { utils::miri_fiber_exit_to(PARENT_FIBER.load(Ordering::Relaxed)) }
}

fn main() {
    unsafe {
        let main_fiber = utils::miri_fiber_current();

        let fiber = AtomicUsize::new(main_fiber);
        scope(|s| {
            s.spawn(|| {
                let f = utils::miri_fiber_create(fiber_body, core::ptr::null_mut());
                fiber.store(f, Ordering::Relaxed);
            });

            while fiber.load(Ordering::Relaxed) == main_fiber {
                spin_loop();
            }

            s.spawn(|| {
                PARENT_FIBER.store(utils::miri_fiber_current(), Ordering::Relaxed);
                let f = fiber.load(Ordering::Relaxed);
                utils::miri_fiber_switch(f);
            });
        });
    }
}
