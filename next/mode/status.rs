use std::sync::Arc;

use crate::{
    application::{Key, ModeResponseSender},
    backend::Backend,
    mode, ui,
};

pub struct Entry {
    //
}

#[derive(Default)]
pub struct Mode {
    state: mode::ModeState,
    entries: Vec<Entry>,
    output: String, // TODO: remove
    select: mode::SelectMenu,
}
impl mode::Mode for Mode {
    fn state(&mut self) -> &mut mode::ModeState {
        &mut self.state
    }

    fn on_enter(
        &mut self,
        backend: Arc<dyn Backend>,
        response_sender: ModeResponseSender,
    ) {
        /*
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
        */
    }

    fn on_key(
        &mut self,
        backend: Arc<dyn Backend>,
        response_sender: ModeResponseSender,
        key: Key,
    ) {
        /*
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
        */
    }

    fn on_response(&mut self, response: &mode::ModeResponse) {
        //
    }

    fn draw(&self) {
        //ui::draw_output(self.name(), &self.output.lock().unwrap());
    }
}

