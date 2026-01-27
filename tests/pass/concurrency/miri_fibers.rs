#[path = "../../utils/mod.rs"]
mod utils;

use std::mem::ManuallyDrop;

struct NoDrop;

impl Drop for NoDrop {
    fn drop(&mut self) {
        panic!("NoDrop should not be dropped");
    }
}

struct Context<T> {
    fiber: usize,
    aux: T,
}

impl<T: Default> Default for Context<T> {
    fn default() -> Self {
        Self { fiber: usize::MAX, aux: T::default() }
    }
}

impl<T> Context<T> {
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

fn execution_history() {
    fn fiber_body(ctx: *mut ()) -> ! {
        let _guard = NoDrop;
        let ctx = ctx.cast::<Context<Vec<usize>>>();
        unsafe {
            (*ctx).aux.push(2);
            Context::switch(ctx);
            (*ctx).aux.push(4);
            Context::exit(ctx);
        }
    }

    let mut ctx = Context::<Vec<usize>>::default();
    ctx.aux.push(0);
    let ptr = &mut ctx as *mut Context<Vec<usize>>;
    unsafe {
        Context::setup(ptr, fiber_body);
        (*ptr).aux.push(1);
        Context::switch(ptr);
        (*ptr).aux.push(3);
        Context::switch(ptr);
        assert_eq!(ctx.aux, [0, 1, 2, 3, 4]);
    }
}

fn multiple_stack_reborrows() {
    fn fiber_body(ctx: *mut ()) -> ! {
        let ctx1 = ctx;

        fn inner1(ctx: *mut (), v: Vec<usize>) -> ! {
            let mut ctx2 = ctx;
            let v = v;
            inner2(&mut ctx2, v);

            fn inner2(ctx: &mut *mut (), v: Vec<usize>) -> ! {
                let ctx3 = ctx;
                let v = v;
                inner3(&*ctx3, &mut ManuallyDrop::new(v));

                fn inner3(ctx: &*mut (), v: &mut ManuallyDrop<Vec<usize>>) -> ! {
                    unsafe {
                        ManuallyDrop::drop(v);
                        Context::exit(*ctx as *mut Context<()>)
                    }
                }
            }
        }

        let v = vec![1, 2, 3];
        inner1(ctx1, v)
    }

    let mut ctx = Context::<()>::default();
    let ptr = &mut ctx as *mut _;
    unsafe {
        Context::setup(ptr, fiber_body);
        Context::switch(ptr);
    }
}

fn main() {
    execution_history();
    multiple_stack_reborrows();
}
