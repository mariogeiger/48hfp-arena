use wasm_bindgen::prelude::*;

/// A timeout handle that cancels on drop (like gloo_timers::callback::Timeout).
pub struct Timeout {
    id: i32,
    _closure: Closure<dyn Fn()>,
}

impl Timeout {
    pub fn new(millis: u32, f: impl FnOnce() + 'static) -> Self {
        let f = std::cell::Cell::new(Some(f));
        let closure = Closure::<dyn Fn()>::new(move || {
            if let Some(f) = f.take() {
                f();
            }
        });
        let id = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                millis as i32,
            )
            .unwrap();
        Timeout {
            id,
            _closure: closure,
        }
    }

    /// Leak the timeout so it fires even if this handle is dropped.
    pub fn forget(self) {
        std::mem::forget(self);
    }
}

impl Drop for Timeout {
    fn drop(&mut self) {
        web_sys::window()
            .unwrap()
            .clear_timeout_with_handle(self.id);
    }
}
