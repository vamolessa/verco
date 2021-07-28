use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

use crate::{
    application::{viewport_size, Key},
    backend::Backend,
};

mod status;

#[derive(Default)]
pub struct ModeState {
    waiting: AtomicBool,
}
impl ModeState {
    pub fn is_waiting(&self) -> bool {
        self.waiting.load(Ordering::Acquire)
    }
}

pub trait Mode: 'static + Send + Sync {
    fn name(&self) -> &'static str;
    fn activation_key(&self) -> Key;
    fn state(&self) -> &ModeState;
    fn enter(self: Arc<Self>, backend: Arc<dyn Backend>);
    fn on_key(self: Arc<Self>, backend: Arc<dyn Backend>, key: Key);
    fn draw(&self);
}

pub fn request<F>(mode: Arc<dyn Mode>, f: F)
where
    F: 'static + FnOnce() + Send + Sync,
{
    if mode.state().waiting.load(Ordering::Acquire) {
        return;
    }
    mode.state().waiting.store(true, Ordering::Release);
    thread::spawn(move || {
        f();
        mode.draw();
    });
}

/*
pub fn request<T>(
    mode: Arc<dyn Mode>,
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

    pub fn on_key(&self, backend: Arc<dyn Backend>, key: Key) {
        for (i, mode) in self.modes.iter().enumerate() {
            if key == mode.activation_key() {
                CURRENT_MODE_INDEX.store(i, Ordering::Relaxed);
                mode.clone().enter(backend);
                return;
            }
        }

        let current_index = CURRENT_MODE_INDEX.load(Ordering::Relaxed);
        self.modes[current_index].clone().on_key(backend, key);
    }

    pub fn draw(&self) {
        let current_index = CURRENT_MODE_INDEX.load(Ordering::Relaxed);
        self.modes[current_index].clone().draw();
    }
}

#[derive(Default)]
pub struct CursorMenu {
    cursor: AtomicUsize,
    scroll: AtomicUsize,
}
impl CursorMenu {
    pub fn cursor(&self) -> usize {
        self.cursor.load(Ordering::Acquire)
    }

    pub fn scroll(&self) -> usize {
        self.scroll.load(Ordering::Acquire)
    }

    pub fn on_key(&self, entries_len: usize, key: Key) {
        let last_index = entries_len.saturating_sub(1);

        let cursor = self.cursor();
        let available_height = viewport_size().1.saturating_sub(1) as usize;
        let half_height = available_height / 2;

        let cursor = match key {
            Key::Down | Key::Ctrl('n') | Key::Char('j') => {
                last_index.min(cursor + 1)
            }
            Key::Up | Key::Ctrl('p') | Key::Char('k') => {
                cursor.saturating_sub(1)
            }
            Key::Ctrl('h') | Key::Home => 0,
            Key::Ctrl('e') | Key::End => last_index,
            Key::Ctrl('d') | Key::PageDown => {
                last_index.min(cursor + half_height)
            }
            Key::Ctrl('u') | Key::PageUp => cursor.saturating_sub(half_height),
            _ => cursor,
        };

        let mut scroll = self.scroll();
        if cursor < scroll {
            scroll = cursor;
        } else if cursor >= scroll + available_height {
            scroll = cursor + 1 - available_height;
        }

        self.cursor.store(cursor, Ordering::Release);
        self.scroll.store(scroll, Ordering::Release);
    }
}

pub trait Select {
    //
}

#[derive(Default)]
pub struct SelectMenu {
    cursor: AtomicUsize,
    scroll: AtomicUsize,
}
