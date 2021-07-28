use std::sync::{Arc, Mutex};

use crate::{application::Key, backend::Backend, mode};

pub struct Entry {
    //
}

#[derive(Default)]
pub struct Mode {
    state: mode::ModeState,
    entries: Mutex<Vec<Entry>>,
    cursor_menu: mode::CursorMenu,
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
        let this = self.clone();
        mode::request(self, move || {
            let result = backend.status();
            this.entries.lock().unwrap().clear();
        });
    }

    fn on_key(self: Arc<Self>, backend: Arc<dyn Backend>, key: Key) {
        let entries = self.entries.lock().unwrap();
        self.cursor_menu.on_key(entries.len(), key);
        match key {
            _ => (),
        }
    }

    fn draw(&self) {
        //
    }
}

