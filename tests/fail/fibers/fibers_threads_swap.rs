#[path = "../../utils/mod.rs"]
mod utils;

use std::hint::spin_loop;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::thread::scope;

fn main() {
    // In the early days of you programming journey you've likely
    // implemented swap algorithm via intermediate variable. This
    // is one of it's variations...

    static STEP: AtomicU8 = AtomicU8::new(0);
    static THREAD1_FIBER: AtomicUsize = AtomicUsize::new(0);
    static THREAD2_FIBER: AtomicUsize = AtomicUsize::new(0);

    fn fiber_body(_arg: *mut (), _payload: *mut u8) -> ! {
        STEP.store(1, Ordering::Release);
        while STEP.load(Ordering::Acquire) == 1 {
            spin_loop();
        }
        unsafe {
            utils::miri_fiber_exit_to(THREAD2_FIBER.load(Ordering::Relaxed), std::ptr::null_mut())
        }
    }

    let tmp_fiber = unsafe { utils::miri_fiber_create(fiber_body, core::ptr::null_mut()) };

    scope(|s| {
        let body = || {
            unsafe {
                THREAD1_FIBER.store(utils::miri_fiber_current(), Ordering::Relaxed);
                utils::miri_fiber_switch(tmp_fiber, std::ptr::null_mut());
                STEP.store(2, Ordering::Release);
            }
        };
        s.spawn(body); //~ ERROR: Undefined Behavior: body of the fiber 2 has terminated on the thread 2, but it was created as the body of thread 1

        let body = || {
            unsafe {
                THREAD2_FIBER.store(utils::miri_fiber_current(), Ordering::Relaxed);
                while STEP.load(Ordering::Acquire) == 0 {
                    spin_loop();
                }
                utils::miri_fiber_switch(
                    THREAD1_FIBER.load(Ordering::Relaxed),
                    std::ptr::null_mut(),
                );
            }
        };
        s.spawn(body);
    });
}
