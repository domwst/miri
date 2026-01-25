#[path = "../../utils/mod.rs"]
mod utils;

struct NoDrop;

impl Drop for NoDrop {
    fn drop(&mut self) {
        panic!("NoDrop should not be dropped");
    }
}

struct Context {
    fiber: usize,
    history: Vec<usize>,
}

impl Default for Context {
    fn default() -> Self {
        Self { fiber: usize::MAX, history: Vec::new() }
    }
}

impl Context {
    unsafe fn setup(this: *mut Self, body: fn(*mut ()) -> !) {
        unsafe {
            (*this).fiber = utils::miri_fiber_create(body, this as *mut _);
        }
    }

    unsafe fn switch(this: *mut Self) {
        unsafe {
            let target = core::ptr::replace(&raw mut (*this).fiber, utils::miri_fiber_current());
            utils::miri_fiber_switch(target);
        }
    }

    unsafe fn exit(this: *mut Self) -> ! {
        unsafe { utils::miri_fiber_exit_to((*this).fiber) }
    }
}

fn fiber_body(ctx: *mut ()) -> ! {
    let _guard = NoDrop;
    let ctx = ctx.cast::<Context>();
    unsafe {
        (*ctx).history.push(2);
        Context::switch(ctx);
        (*ctx).history.push(4);
        Context::exit(ctx);
    }
}

fn main() {
    let mut ctx = Context::default();
    ctx.history.push(0);
    let ptr = &mut ctx as *mut _;
    unsafe {
        Context::setup(ptr, fiber_body);
        (*ptr).history.push(1);
        Context::switch(ptr);
        (*ptr).history.push(3);
        Context::switch(ptr);
        assert_eq!(ctx.history, [0, 1, 2, 3, 4]);
    }
}
