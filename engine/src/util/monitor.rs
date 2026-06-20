use std::sync::{Arc, Condvar, Mutex};

struct State<T> {
    value: Mutex<T>,
    condvar: Condvar,
}

#[derive(Clone)]
pub struct Monitor<T: Copy> {
    state: Arc<State<T>>,
}

impl<T: Copy> Monitor<T> {
    pub fn new(val: T) -> Self {
        Self {
            state: Arc::new(State {
                value: Mutex::new(val),
                condvar: Condvar::new(),
            }),
        }
    }

    pub fn value(&self) -> T {
        *self.state.value.lock().unwrap()
    }

    pub fn set(&self, val: T) {
        *self.state.value.lock().unwrap() = val;
    }

    pub fn modify(&self, modify_fn: impl FnOnce(&mut T), should_notify_fn: impl FnOnce(T) -> bool) {
        let value = {
            let mut v = self.state.value.lock().unwrap();
            modify_fn(&mut v);
            *v
        };

        if should_notify_fn(value) {
            self.state.condvar.notify_all();
        }
    }

    pub fn wait_while(&self, predicate: impl FnMut(&mut T) -> bool) {
        let lock = self.state.value.lock().unwrap();
        drop(self.state.condvar.wait_while(lock, predicate).unwrap());
    }
}
