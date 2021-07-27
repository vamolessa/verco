use std::sync::{Arc, Mutex};

use crate::{application::Key, backend::Backend, mode};

pub struct Entry {
    //
}

#[derive(Default)]
pub struct Mode {
    state: mode::ModeState,
    entries: Mutex<Vec<Entry>>,
}
impl mode::Mode for Mode {
    fn name(&self) -> &'static str {
        "status"
    }

    fn activation_key(&self) -> Key {
        Key::Char('s')
    }

    fn state(&self) -> &mode::ModeState {
        &self.state
    }

    fn enter(self: Arc<Self>, backend: Arc<dyn Backend>) {
        //
    }

    fn on_key(self: Arc<Self>, backend: Arc<dyn Backend>, key: Key) -> bool {
        true
    }

    fn draw(&self, viewport_size: (u16, u16)) {
        //
    }
}

