use std::sync::Arc;

use crate::{application::EventSender, backend::Backend, platform::Key};

pub mod branches;
pub mod log;
pub mod revision_details;
pub mod status;
pub mod tags;

pub enum ModeResponse {
    Status(status::Response),
    Log(log::Response),
    RevisionDetails(revision_details::Response),
    Branches(branches::Response),
    Tags(tags::Response),
}

pub enum ModeKind {
    Status,
    Log,
    RevisionDetails(String),
    Branches,
    Tags,
}
impl Default for ModeKind {
    fn default() -> Self {
        Self::Status
    }
}

#[derive(Clone)]
pub struct ModeContext {
    pub backend: Arc<dyn Backend>,
    pub event_sender: EventSender,
    pub viewport_size: (u16, u16),
}

pub struct ModeStatus {
    pub pending_input: bool,
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

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn line_count(&self) -> usize {
        self.line_count
    }

    pub fn lines_from_scroll<'a>(&'a self) -> impl 'a + Iterator<Item = &'a str> {
        self.text.lines().skip(self.scroll)
    }

    pub fn on_key(&mut self, available_height: usize, key: Key) {
        let half_height = available_height / 2;

        self.scroll = match key {
            Key::Down | Key::Ctrl('n') | Key::Char('j') => self.scroll + 1,
            Key::Up | Key::Ctrl('p') | Key::Char('k') => self.scroll.saturating_sub(1),
            Key::Ctrl('h') | Key::Home => 0,
            Key::Ctrl('e') | Key::End => usize::MAX,
            Key::Ctrl('d') | Key::PageDown => self.scroll + half_height,
            Key::Ctrl('u') | Key::PageUp => self.scroll.saturating_sub(half_height),
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

                fn rfind_boundary(mut chars: std::str::Chars, f: fn(&char) -> bool) -> usize {
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
                        rfind_boundary(chars, |&c| is_word(c) || !c.is_ascii_whitespace())
                    } else {
                        rfind_boundary(chars, |&c| is_word(c) || c.is_ascii_whitespace())
                    };
                    self.input.truncate(len);
                }
            }
            Key::Backspace | Key::Ctrl('h') => {
                if let Some((last_char_index, _)) = self.input.char_indices().next_back() {
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
    pub cursor: usize,
    pub scroll: usize,
}
impl SelectMenu {
    pub fn on_remove_entry(&mut self, index: usize) {
        if index <= self.cursor {
            self.cursor = self.cursor.saturating_sub(1);
        }
    }

    pub fn on_key(
        &mut self,
        entries_len: usize,
        available_height: usize,
        key: Key,
    ) -> SelectMenuAction {
        let half_height = available_height / 2;

        self.cursor = match key {
            Key::Down | Key::Ctrl('n') | Key::Char('j') => self.cursor + 1,
            Key::Up | Key::Ctrl('p') | Key::Char('k') => self.cursor.saturating_sub(1),
            Key::Ctrl('h') | Key::Home => 0,
            Key::Ctrl('e') | Key::End => usize::MAX,
            Key::Ctrl('d') | Key::PageDown => self.cursor + half_height,
            Key::Ctrl('u') | Key::PageUp => self.cursor.saturating_sub(half_height),
            _ => self.cursor,
        };

        self.cursor = entries_len.saturating_sub(1).min(self.cursor);
        self.scroll_to_cursor(available_height);

        match key {
            Key::Char(' ') if self.cursor < entries_len => SelectMenuAction::Toggle(self.cursor),
            Key::Char('a') => SelectMenuAction::ToggleAll,
            _ => SelectMenuAction::None,
        }
    }

    pub fn fix_cursor_on_filter(&mut self, cursor: usize, available_height: usize) {
        self.cursor = cursor;
        self.scroll_to_cursor(available_height);
    }

    fn scroll_to_cursor(&mut self, available_height: usize) {
        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        } else if self.cursor >= self.scroll + available_height {
            self.scroll = self.cursor + 1 - available_height;
        }
    }
}

pub trait FilterEntry {
    fn fuzzy_matches(&self, matcher: &mut FuzzyMatcher, pattern: &str) -> bool;
}

#[derive(Default)]
pub struct Filter {
    has_focus: bool,
    readline: ReadLine,
    visible_indices: Vec<usize>,
    matcher: FuzzyMatcher,
}
impl Filter {
    pub fn clear(&mut self) {
        self.has_focus = false;
        self.readline.clear();
        self.visible_indices.clear();
    }

    pub fn enter(&mut self) {
        self.has_focus = true;
        self.readline.clear();
    }

    pub fn on_key(&mut self, key: Key) {
        if key.is_submit() || key == Key::Ctrl('f') {
            self.has_focus = false;
        } else if key.is_cancel() {
            self.has_focus = false;
            self.readline.clear();
        } else {
            self.readline.on_key(key);
        }
    }

    pub fn filter<'entries, I, E>(&mut self, entries: I, cursor: usize) -> usize
    where
        I: 'entries + Iterator<Item = &'entries E>,
        E: 'entries + FilterEntry,
    {
        let entry_index = self.visible_indices.get(cursor).cloned().unwrap_or(0);

        self.visible_indices.clear();
        for (i, entry) in entries.enumerate() {
            if entry.fuzzy_matches(&mut self.matcher, self.readline.input()) {
                self.visible_indices.push(i);
            }
        }

        self.visible_indices
            .iter()
            .position(|&i| i == entry_index)
            .unwrap_or(0)
    }

    pub fn on_remove_entry(&mut self, entry_index: usize) {
        for i in (0..self.visible_indices.len()).rev() {
            if entry_index < i {
                self.visible_indices[i] -= 1;
            } else if entry_index == i {
                self.visible_indices.remove(i);
            } else {
                break;
            }
        }
    }

    pub fn get_visible_index(&self, index: usize) -> Option<usize> {
        self.visible_indices.get(index).cloned()
    }

    pub fn visible_indices(&self) -> &[usize] {
        &self.visible_indices
    }

    pub fn is_filtering(&self) -> bool {
        self.has_focus || !self.readline.input().is_empty()
    }

    pub fn has_focus(&self) -> bool {
        self.has_focus
    }

    pub fn as_str(&self) -> &str {
        self.readline.input()
    }
}

struct FuzzyMatch {
    rest_index: u32,
}

#[derive(Default)]
pub struct FuzzyMatcher {
    previous_matches: Vec<FuzzyMatch>,
    next_matches: Vec<FuzzyMatch>,
}
impl FuzzyMatcher {
    pub fn fuzzy_matches(&mut self, text: &str, pattern: &str) -> bool {
        if pattern.is_empty() {
            return true;
        }

        self.previous_matches.clear();
        self.previous_matches.push(FuzzyMatch { rest_index: 0 });

        for pattern_char in pattern.chars() {
            self.next_matches.clear();

            for previous_match in &self.previous_matches {
                let mut previous_text_char = '\0';
                for (i, text_char) in text[previous_match.rest_index as usize..].char_indices() {
                    if text_char.eq_ignore_ascii_case(&pattern_char) {
                        let matched = if i == 0 && previous_match.rest_index != 0 {
                            true
                        } else if !text_char.is_ascii_alphanumeric() {
                            true
                        } else {
                            let is_word_boundary = (!previous_text_char.is_ascii_alphanumeric()
                                && text_char.is_ascii_alphanumeric())
                                || (previous_text_char.is_ascii_lowercase()
                                    && text_char.is_ascii_uppercase());
                            is_word_boundary
                        };

                        if matched {
                            let rest_index =
                                previous_match.rest_index + (i + text_char.len_utf8()) as u32;
                            self.next_matches.push(FuzzyMatch { rest_index });
                        }
                    }

                    previous_text_char = text_char;
                }
            }

            if self.next_matches.is_empty() {
                return false;
            }
            std::mem::swap(&mut self.previous_matches, &mut self.next_matches);
        }

        true
    }
}

