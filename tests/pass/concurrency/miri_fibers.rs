#[path = "../../utils/mod.rs"]
mod utils;

use std::mem::ManuallyDrop;

static mut DROPPED: u8 = 0;

struct HasDrop;

impl Drop for HasDrop {
    fn drop(&mut self) {
        unsafe {
            DROPPED += 1;
        }
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
    unsafe fn setup(this: *mut Self, body: unsafe fn(*mut (), *mut u8) -> !) {
        unsafe {
            (*this).fiber = utils::miri_fiber_create(body, this as *mut _);
        }
    }

    unsafe fn switch(this: *mut Self, payload: *mut u8) -> *mut u8 {
        unsafe {
            let target = core::ptr::replace(&raw mut (*this).fiber, utils::miri_fiber_current());
            utils::miri_fiber_switch(target, payload)
        }
    }

    unsafe fn exit(this: *mut Self, payload: *mut u8) -> ! {
        unsafe { utils::miri_fiber_exit_to((*this).fiber, payload) }
    }
}

fn execution_history() {
    unsafe fn fiber_body(ctx: *mut (), payload: *mut u8) -> ! {
        {
            let _guard = HasDrop;
        }
        let ctx = ctx.cast::<Context<Vec<usize>>>();
        unsafe {
            let value = payload.read();
            drop(Box::from_raw(payload));
            (*ctx).aux.push(2);
            let payload = Context::switch(ctx, Box::into_raw(Box::new(value + 1)));
            let value = payload.read();
            drop(Box::from_raw(payload));
            (*ctx).aux.push(4);
            Context::exit(ctx, Box::into_raw(Box::new(value + 1)))
        }
    }

    let dropped_before = unsafe { DROPPED };
    let mut ctx = Context::<Vec<usize>>::default();
    ctx.aux.push(0);
    let ptr = &mut ctx as *mut Context<Vec<usize>>;
    unsafe {
        Context::setup(ptr, fiber_body);
        (*ptr).aux.push(1);
        let payload = Context::switch(ptr, Box::into_raw(Box::new(0)));
        *payload += 1;
        (*ptr).aux.push(3);
        let payload = Context::switch(ptr, payload);
        assert_eq!(ctx.aux, [0, 1, 2, 3, 4]);
        assert_eq!(*payload, 3);
        assert_eq!(DROPPED - dropped_before, 1);
        drop(Box::from_raw(payload));
    }
}

fn multiple_stack_reborrows() {
    unsafe fn fiber_body(ctx: *mut (), payload: *mut u8) -> ! {
        let ctx1 = ctx;
        let v = vec![1, 2, 3];
        let payload = &mut *payload;
        *payload += 1;
        inner1(ctx1, v, payload);

        fn inner1(ctx: *mut (), v: Vec<usize>, payload: *mut u8) -> ! {
            let mut ctx2 = ctx;
            let v = v;
            drop(v);
            inner2(&mut ctx2, vec![4, 5, 6], payload);

            fn inner2(ctx: &mut *mut (), v: Vec<usize>, payload: *mut u8) -> ! {
                let ctx3 = ctx;
                let v = v;
                inner3(&*ctx3, &mut ManuallyDrop::new(v), payload);

                fn inner3(ctx: &*mut (), v: &mut ManuallyDrop<Vec<usize>>, payload: *mut u8) -> ! {
                    unsafe {
                        ManuallyDrop::drop(v);
                        Context::exit(*ctx as *mut Context<()>, payload as *mut _)
                    }
                }
            }
        }
    }

    let mut ctx = Context::<()>::default();
    let ptr = &mut ctx as *mut _;
    let mut storage = 0u8;
    let payload = &mut storage as *mut _;
    unsafe {
        Context::setup(ptr, fiber_body);
        let ret = Context::switch(ptr, payload);
        assert_eq!(ret, payload);
        assert_eq!(storage, 1);
    }
}

fn fiber_destroy() {
    unsafe fn fiber_body(ctx: *mut (), _payload: *mut u8) -> ! {
        let ctx = ctx.cast::<Context<()>>();
        unsafe {
            {
                let _guard = HasDrop;
            }
            Context::switch(ctx, std::ptr::null_mut());
            Context::exit(ctx, std::ptr::null_mut());
        }
    }

    unsafe {
        let dropped_before = DROPPED;
        let mut ctx = Context::<()>::default();
        let ptr = &mut ctx as *mut _;

        Context::setup(ptr, fiber_body);
        utils::miri_fiber_destroy((*ptr).fiber);
        assert_eq!(DROPPED - dropped_before, 0);
    }

    unsafe {
        let dropped_before = DROPPED;
        let mut ctx = Context::<()>::default();
        let ptr = &mut ctx as *mut _;

        Context::setup(ptr, fiber_body);
        Context::switch(ptr, std::ptr::null_mut());
        utils::miri_fiber_destroy((*ptr).fiber);
        assert_eq!(DROPPED - dropped_before, 1);
    }
}

fn main() {
    execution_history();
    multiple_stack_reborrows();
    fiber_destroy();
}
