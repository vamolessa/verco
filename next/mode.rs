use std::{sync::Arc, thread};

use crate::{
    application::{Key, ModeResponseSender},
    backend::Backend,
};

pub mod status;

pub enum ModeResponse {
    Status,
    Error,
}

pub enum ModeKind {
    Status,
}

#[derive(Default)]
pub struct ModeState {
    waiting: bool,
}

pub trait Mode {
    fn state(&mut self) -> &mut ModeState;
    fn on_enter(&mut self, ctx: &ModeContext);
    fn on_key(&mut self, ctx: &ModeContext, key: Key);
    fn on_response(&mut self, response: &ModeResponse);
    fn draw(&self, viewport_size: (u16, u16));
}

pub struct ModeContext {
    pub backend: Arc<dyn Backend>,
    pub response_sender: ModeResponseSender,
    pub viewport_size: (u16, u16),
}

/*
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
*/

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

pub enum SelectMenuAction {
    None,
    Toggle(usize),
    ToggleAll,
}

#[derive(Default)]
pub struct SelectMenu {
    cursor: usize,
    scroll: usize,
}
impl SelectMenu {
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn scroll(&self) -> usize {
        self.scroll
    }

    pub fn on_key(
        &mut self,
        ctx: &ModeContext,
        entries_len: usize,
        key: Key,
    ) -> SelectMenuAction {
        let last_index = entries_len.saturating_sub(1);

        let available_height = ctx.viewport_size.1.saturating_sub(1) as usize;
        let half_height = available_height / 2;

        self.cursor = match key {
            Key::Down | Key::Ctrl('n') | Key::Char('j') => {
                last_index.min(self.cursor + 1)
            }
            Key::Up | Key::Ctrl('p') | Key::Char('k') => {
                self.cursor.saturating_sub(1)
            }
            Key::Ctrl('h') | Key::Home => 0,
            Key::Ctrl('e') | Key::End => last_index,
            Key::Ctrl('d') | Key::PageDown => {
                last_index.min(self.cursor + half_height)
            }
            Key::Ctrl('u') | Key::PageUp => {
                self.cursor.saturating_sub(half_height)
            }
            _ => self.cursor,
        };

        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        } else if self.cursor >= self.scroll + available_height {
            self.scroll = self.cursor + 1 - available_height;
        }

        match key {
            Key::Char(' ') => SelectMenuAction::Toggle(self.cursor),
            Key::Ctrl('a') => SelectMenuAction::ToggleAll,
            _ => SelectMenuAction::None,
        }
    }
}

