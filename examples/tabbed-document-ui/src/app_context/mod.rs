use std::any::Any;
use std::sync::{Mutex, Once};
use std::borrow::BorrowMut;

/// Returns an optional result of the closure if an instance of type T has been provided
/// to the context via `provide_context`, if the instance of type T was not provided the closure
/// is not run.
pub fn with_context<T: 'static, R, F: FnOnce(&mut T) -> R >(f: F) -> Option<R> {
    let context = context::<T>();
    let mut value_binding = context.lock().unwrap();

    if let Some(value) = value_binding.as_mut() {
        if let Some(downcast_value) = value.downcast_mut::<T>() {
            return Some(f(downcast_value))
        }
    }
    None
}

/// Provide an instance of type T to be accessed later via `with_context`
/// Note: there is currently no support for types that need to be dropped
pub fn provide_context<T: 'static>(instance: T) {
    let context = context::<T>();
    let mut context_guard = context.lock().unwrap();
    context_guard.replace(Box::new(instance));
}

fn context<'a, T: Sized>() -> &'a Mutex<Option<Box<dyn Any>>> {
    // Builds on the example in https://www.sitepoint.com/rust-global-variables/#multithreadedglobalswithruntimeinitialization
    // and modified to use generics and so the `static mut` is inside the function

    static mut CONTEXT: Option<Mutex<Option<Box<dyn Any>>>> = None;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        // Since this access is inside a call_once, it is safe
        unsafe {
            *CONTEXT.borrow_mut() = Some(Mutex::new(None));
        }
    });
    // As long as this function is the only place with access to the static variable,
    // which it is, because the static is defined inside this method, then
    // giving out read-only borrow here is safe because it is guaranteed no more mutable
    // references will exist at this point or in the future.
    unsafe { CONTEXT.as_ref().unwrap() }
}
