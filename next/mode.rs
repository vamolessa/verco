use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

use crate::{application::Key, backend::Backend};

mod status;

#[derive(Default)]
pub struct ModeState {
    waiting: AtomicBool,
}
impl ModeState {
    pub fn is_waiting(&self) -> bool {
        self.waiting.load(Ordering::Acquire)
    }

    pub fn set_waiting(&self, waiting: bool) {
        self.waiting.store(waiting, Ordering::Release);
    }
}

pub trait Mode {
    fn name(&self) -> &'static str;
    fn activation_key(&self) -> Key;
    fn state(&self) -> &ModeState;
    fn enter(self: Arc<Self>, backend: Arc<dyn Backend>);
    fn on_key(self: Arc<Self>, backend: Arc<dyn Backend>, key: Key) -> bool;
    fn draw(&self, viewport_size: (u16, u16));
}

/*
pub fn request<T>(
    mode: Arc<dyn Mode>,
    backend: Arc<dyn Backend>,
    requester: fn(&dyn Backend) -> Result<T, String>,
) where
    T: 'static,
{
    if mode.state().waiting.load(Ordering::Acquire) {
        return;
    }
    mode.state().waiting.store(true, Ordering::Release);

    thread::spawn(move || {
        use std::ops::Deref;
        let result = requester(backend.deref());
    });
}
*/

static CURRENT_MODE_INDEX: AtomicUsize = AtomicUsize::new(0);

pub struct ModeManager {
    modes: [Arc<dyn Mode>; 1],
}
impl ModeManager {
    pub fn new() -> Self {
        Self {
            modes: [
                Arc::new(status::Mode::default()),
                //
            ],
        }
    }

    pub fn on_key(&self, backend: Arc<dyn Backend>, key: Key) -> bool {
        for (i, mode) in self.modes.iter().enumerate() {
            if key == mode.activation_key() {
                CURRENT_MODE_INDEX.store(i, Ordering::Relaxed);
                mode.clone().enter(backend);
                return true;
            }
        }

        let current_index = CURRENT_MODE_INDEX.load(Ordering::Relaxed);
        self.modes[current_index].clone().on_key(backend, key)
    }

    pub fn draw(&self, viewport_size: (u16, u16)) {
        let current_index = CURRENT_MODE_INDEX.load(Ordering::Relaxed);
        self.modes[current_index].clone().draw(viewport_size);
    }
}

