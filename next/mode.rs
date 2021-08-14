use std::sync::Arc;

use crate::{
    application::{Key, ModeResponseSender},
    backend::Backend,
};

pub mod status;

pub enum ModeResponse {
    Status(status::Response),
}

pub enum ModeKind {
    Status,
}

#[derive(Clone)]
pub struct ModeContext {
    pub backend: Arc<dyn Backend>,
    pub response_sender: ModeResponseSender,
    pub viewport_size: (u16, u16),
}

#[derive(Default)]
pub struct Output {
    text: String,
    line_count: usize,
    scroll: usize,
}
impl Output {
    pub fn set(&mut self, output: String) {
        self.text = output;
        self.line_count = self.text.lines().count();
        self.scroll = 0;
    }

    pub fn on_key(&mut self, available_height: usize, key: Key) {
        let half_height = available_height / 2;

        self.scroll = match key {
            Key::Down | Key::Ctrl('n') | Key::Char('j') => self.scroll + 1,
            Key::Up | Key::Ctrl('p') | Key::Char('k') => {
                self.scroll.saturating_sub(1)
            }
            Key::Ctrl('h') | Key::Home => 0,
            Key::Ctrl('e') | Key::End => usize::MAX,
            Key::Ctrl('d') | Key::PageDown => self.scroll + half_height,
            Key::Ctrl('u') | Key::PageUp => {
                self.scroll.saturating_sub(half_height)
            }
            _ => self.scroll,
        };

        self.scroll = self
            .line_count
            .saturating_sub(available_height)
            .min(self.scroll);
    }
}

#[derive(Default)]
pub struct ReadLine {
    input: String,
}
impl ReadLine {
    pub fn clear(&mut self) {
        self.input.clear();
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn on_key(&mut self, key: Key) {
        match key {
            Key::Home | Key::Ctrl('u') => self.input.clear(),
            Key::Ctrl('w') => {
                fn is_word(c: char) -> bool {
                    c.is_alphanumeric() || c == '_'
                }

                fn rfind_boundary(
                    mut chars: std::str::Chars,
                    f: fn(&char) -> bool,
                ) -> usize {
                    match chars.rfind(f) {
                        Some(c) => chars.as_str().len() + c.len_utf8(),
                        None => 0,
                    }
                }

                let mut chars = self.input.chars();
                if let Some(c) = chars.next_back() {
                    let len = if is_word(c) {
                        rfind_boundary(chars, |&c| !is_word(c))
                    } else if c.is_ascii_whitespace() {
                        rfind_boundary(chars, |&c| {
                            is_word(c) || !c.is_ascii_whitespace()
                        })
                    } else {
                        rfind_boundary(chars, |&c| {
                            is_word(c) || c.is_ascii_whitespace()
                        })
                    };
                    self.input.truncate(len);
                }
            }
            Key::Backspace | Key::Ctrl('h') => {
                if let Some((last_char_index, _)) =
                    self.input.char_indices().next_back()
                {
                    self.input.truncate(last_char_index);
                }
            }
            Key::Char(c) => self.input.push(c),
            _ => (),
        }
    }
}

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
        entries_len: usize,
        available_height: usize,
        key: Key,
    ) -> SelectMenuAction {
        let last_index = entries_len.saturating_sub(1);
        let half_height = available_height / 2;

        self.cursor = match key {
            Key::Down | Key::Ctrl('n') | Key::Char('j') => self.cursor + 1,
            Key::Up | Key::Ctrl('p') | Key::Char('k') => {
                self.cursor.saturating_sub(1)
            }
            Key::Ctrl('h') | Key::Home => 0,
            Key::Ctrl('e') | Key::End => usize::MAX,
            Key::Ctrl('d') | Key::PageDown => self.cursor + half_height,
            Key::Ctrl('u') | Key::PageUp => {
                self.cursor.saturating_sub(half_height)
            }
            _ => self.cursor,
        };

        self.cursor = self.cursor.min(last_index);

        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        } else if self.cursor >= self.scroll + available_height {
            self.scroll = self.cursor + 1 - available_height;
        }

        match key {
            Key::Char(' ') if self.cursor < entries_len => {
                SelectMenuAction::Toggle(self.cursor)
            }
            Key::Ctrl('a') => SelectMenuAction::ToggleAll,
            _ => SelectMenuAction::None,
        }
    }
}

