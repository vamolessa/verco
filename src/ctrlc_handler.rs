use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct CtrlcHandler {
    state: Arc<AtomicBool>,
    should_ignore: bool,
}

impl CtrlcHandler {
    pub fn new() -> Self {
        Self {
            state: Arc::new(AtomicBool::new(false)),
            should_ignore: false,
        }
    }

    pub fn set(&self, value: bool) {
        self.state.store(value, Ordering::SeqCst);
    }

    pub fn ignore_next(&mut self) {
        self.should_ignore = true;
    }

    pub fn get(&mut self) -> bool {
        let previous = self.state.swap(false, Ordering::SeqCst);
        if self.should_ignore {
            self.should_ignore = false;
            false
        } else {
            previous
        }
    }
}
