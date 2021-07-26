use crate::application::{Application, ApplicationFlow, Key};

pub fn handle_key(application: &Application, key: Key) -> ApplicationFlow {
    match application.previous_key() {
        Key::None => match key {
            Key::Esc | Key::Ctrl('c') => ApplicationFlow::Quit,
            Key::Char('s') => {
                //backend.status()
                todo!();
            }
            _ => ApplicationFlow::Continue,
        },
        _ => ApplicationFlow::Continue,
    }
}

