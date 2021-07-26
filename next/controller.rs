use crate::application::{ActionKind, Application, ApplicationFlow, Key};

pub fn handle_key(application: &Application, key: Key) -> ApplicationFlow {
    match application.previous_key() {
        Key::None => match key {
            Key::Esc | Key::Ctrl('c') => return ApplicationFlow::Quit,
            Key::Char('s') => {
                application.schedule(ActionKind::Status, |b| b.status());
            }
            _ => (),
        },
        _ => (),
    }

    ApplicationFlow::Continue
}

