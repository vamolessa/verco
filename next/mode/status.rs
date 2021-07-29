use std::sync::{Arc, Mutex};

use crate::{application::Key, backend::Backend, mode, ui};

pub struct Entry {
    //
}

#[derive(Default)]
pub struct Mode {
    state: mode::ModeState,
    entries: Mutex<Vec<Entry>>,
    output: Mutex<String>, // TODO: remove
    select: mode::SelectMenu,
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
            let output = match backend.status() {
                Ok(output) => output,
                Err(error) => {
                    ui::draw_output(this.name(), &error);
                    return;
                }
            };
            this.entries.lock().unwrap().clear();
            *this.output.lock().unwrap() = output;
        });
    }

    fn on_key(self: Arc<Self>, backend: Arc<dyn Backend>, key: Key) {
        let entries = self.entries.lock().unwrap();
        self.select.on_key(entries.len(), key);
        match key {
            Key::Char('c') => {
                // commit
            }
            Key::Char('U') => {
                // revert
            }
            Key::Char('d') => {
                // diff
            }
            _ => (),
        }
    }

    fn draw(&self) {
        ui::draw_output(self.name(), &self.output.lock().unwrap());
    }
}

