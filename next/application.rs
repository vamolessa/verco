use std::{
    path::PathBuf,
    sync::atomic::{AtomicU16, Ordering},
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

pub struct Application {
    root: PathBuf,
    backend: Box<dyn 'static + Send + Backend>,
}
impl Application {
    pub fn new(
        root: PathBuf,
        backend: Box<dyn 'static + Send + Backend>,
    ) -> Self {
        Self { root, backend }
    }

    pub fn run(&mut self) {
        match terminal::size() {
            Ok((width, height)) => resize(width, height),
            Err(_) => return,
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
}

