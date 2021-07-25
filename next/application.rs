use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU16, Ordering},
        mpsc, Arc, Mutex,
    },
};

use crossterm::{event, terminal};

use crate::backend::Backend;

static VIEWPORT_WIDTH: AtomicU16 = AtomicU16::new(0);
static VIEWPORT_HEIGHT: AtomicU16 = AtomicU16::new(0);

fn resize(width: u16, height: u16) {
    VIEWPORT_WIDTH.store(width, Ordering::Relaxed);
    VIEWPORT_HEIGHT.store(height, Ordering::Relaxed);
}

pub fn next_key() -> event::KeyEvent {
    loop {
        match event::read().unwrap() {
            event::Event::Key(key) => return key,
            event::Event::Mouse(_) => (),
            event::Event::Resize(width, height) => resize(width, height),
        }
    }
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
        current_action: ActionKind::Help,
    };

    loop {
        match next_key() {
            event::KeyEvent {
                code: event::KeyCode::Esc,
                ..
            } => break,
            event::KeyEvent {
                code: event::KeyCode::Char(c),
                ..
            } => println!("char: {}", c),
            _ => (),
        }
    }
}

enum ActionKind {
    Help,
    Status,
    LEN,
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

