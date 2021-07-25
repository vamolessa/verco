use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use crossterm::{event, terminal};

use crate::backend::Backend;

static VIEWPORT_WIDTH: AtomicUsize = AtomicUsize::new(0);
static VIEWPORT_HEIGHT: AtomicUsize = AtomicUsize::new(0);
static CURRENT_ACTION_KIND: AtomicUsize =
    AtomicUsize::new(ActionKind::default() as _);

fn resize(width: u16, height: u16) {
    VIEWPORT_WIDTH.store(width as _, Ordering::Relaxed);
    VIEWPORT_HEIGHT.store(height as _, Ordering::Relaxed);
}

pub fn run(root: PathBuf, backend: Arc<dyn Backend>) {
    match terminal::size() {
        Ok((width, height)) => resize(width, height),
        Err(_) => return,
    };

    let mut app = Application {
        root,
        backend,
        outputs: Default::default(),
        current_action: ActionKind::default(),
    };

    loop {
        match app.next_key() {
            event::KeyEvent {
                code: event::KeyCode::Esc,
                ..
            } => break,
            _ => (),
        }
    }
}

enum ActionKind {
    Help,
    Status,
    LEN,
}
impl ActionKind {
    pub const fn default() -> Self {
        ActionKind::Help
    }
}

enum ApplicationEvent {
    Key(event::KeyEvent),
    Output(ActionKind),
}

pub struct Application {
    root: PathBuf,
    backend: Arc<dyn Backend>,
    outputs: Arc<[Mutex<String>; ActionKind::LEN as _]>,
    current_action: ActionKind,
}
impl Application {
    pub fn next_key(&self) -> event::KeyEvent {
        loop {
            match event::read().unwrap() {
                event::Event::Key(key) => return key,
                event::Event::Mouse(_) => (),
                event::Event::Resize(width, height) => {
                    resize(width, height);
                    println!("redraw resized");
                }
            }
        }
    }
}

