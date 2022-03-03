use std::thread;

use crate::{
    backend::{Backend, BackendResult, TagEntry},
    mode::{Filter, ModeContext, ModeKind, ModeResponse, ModeStatus, Output, ReadLine, SelectMenu},
    platform::Key,
    ui::{Drawer, SelectEntryDraw, RESERVED_LINES_COUNT},
};

pub enum Response {
    Refresh(BackendResult<Vec<TagEntry>>),
    Checkout,
}

enum WaitOperation {
    Refresh,
    New,
    Delete,
}

enum State {
    Idle,
    Waiting(WaitOperation),
    NewNameInput,
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

impl SelectEntryDraw for TagEntry {
    fn draw(&self, drawer: &mut Drawer, _: bool, _: bool, _: bool) -> usize {
        drawer.str(&self.name);
        1
    }
}

#[derive(Default)]
pub struct Mode {
    state: State,
    entries: Vec<TagEntry>,
    output: Output,
    select: SelectMenu,
    filter: Filter,
    readline: ReadLine,
}
impl Mode {
    pub fn on_enter(&mut self, ctx: &ModeContext) {
        if let State::Waiting(_) = self.state {
            return;
        }
        self.state = State::Waiting(WaitOperation::Refresh);

        self.output.set(String::new());
        let cursor = self.filter.filter(self.entries.iter(), self.select.cursor);
        let available_height = (ctx.viewport_size.1 as usize).saturating_sub(RESERVED_LINES_COUNT);
        self.select.fix_cursor_on_filter(cursor, available_height);
        self.readline.clear();

        request(ctx, |_| Ok(()));
    }

    pub fn on_key(&mut self, ctx: &ModeContext, key: Key) -> ModeStatus {
        let pending_input = matches!(self.state, State::NewNameInput) || self.filter.has_focus();
        let available_height = (ctx.viewport_size.1 as usize).saturating_sub(RESERVED_LINES_COUNT);

        if self.filter.has_focus() {
            self.filter.on_key(key);
            let cursor = self.filter.filter(self.entries.iter(), self.select.cursor);
            self.select.fix_cursor_on_filter(cursor, available_height);
        } else {
            match self.state {
                State::Idle | State::Waiting(_) => {
                    if self.output.text().is_empty() {
                        self.select.on_key(
                            self.filter.visible_indices().len(),
                            available_height,
                            key,
                        );
                    } else {
                        self.output.on_key(available_height, key);
                    }

                    let current_entry_index = self.filter.get_visible_index(self.select.cursor);
                    match key {
                        Key::Ctrl('f') => self.filter.enter(),
                        Key::Char('g') => {
                            if let Some(current_entry_index) = current_entry_index {
                                let entry = self.entries[current_entry_index].clone();
                                let ctx = ctx.clone();
                                thread::spawn(move || {
                                    ctx.event_sender.send_mode_change(ModeKind::Log);
                                    match ctx.backend.checkout_tag(&entry) {
                                        Ok(()) => {
                                            ctx.event_sender.send_response(ModeResponse::Tags(
                                                Response::Checkout,
                                            ));
                                            ctx.event_sender.send_mode_refresh(ModeKind::Log);
                                        }
                                        Err(error) => ctx.event_sender.send_response(
                                            ModeResponse::Tags(Response::Refresh(Err(error))),
                                        ),
                                    }
                                });
                            }
                        }
                        Key::Char('n') => {
                            self.state = State::NewNameInput;
                            self.output.set(String::new());
                            self.filter.clear();
                            self.readline.clear();
                        }
                        Key::Char('D') => {
                            if let Some(current_entry_index) = current_entry_index {
                                let entry = self.entries[current_entry_index].clone();
                                self.state = State::Waiting(WaitOperation::Delete);

                                self.entries.remove(current_entry_index);
                                self.filter.on_remove_entry(current_entry_index);
                                self.select.on_remove_entry(self.select.cursor);
                                request(ctx, move |b| b.delete_tag(&entry));
                            }
                        }
                        _ => (),
                    }
                }
                State::NewNameInput => {
                    self.readline.on_key(key);
                    if key.is_submit() {
                        self.state = State::Waiting(WaitOperation::New);

                        let name = self.readline.input().to_string();
                        request(ctx, move |b| b.new_tag(&name));
                    } else if key.is_cancel() {
                        self.on_enter(ctx);
                    }
                }
            }
        }

        ModeStatus { pending_input }
    }

    pub fn on_response(&mut self, ctx: &ModeContext, response: Response) {
        match response {
            Response::Refresh(result) => {
                self.entries = Vec::new();
                self.output.set(String::new());

                if let State::Waiting(_) = self.state {
                    self.state = State::Idle;
                }
                if let State::Idle = self.state {
                    match result {
                        Ok(entries) => self.entries = entries,
                        Err(error) => self.output.set(error),
                    }
                }

                let cursor = self.filter.filter(self.entries.iter(), self.select.cursor);
                let available_height =
                    (ctx.viewport_size.1 as usize).saturating_sub(RESERVED_LINES_COUNT);
                self.select.fix_cursor_on_filter(cursor, available_height);
            }
            Response::Checkout => self.state = State::Idle,
        }
    }

    pub fn is_waiting_response(&self) -> bool {
        match self.state {
            State::Idle | State::NewNameInput => false,
            State::Waiting(_) => true,
        }
    }

    pub fn header(&self) -> (&str, &str, &str) {
        let name = match self.state {
            State::Idle | State::Waiting(WaitOperation::Refresh) => "tags",
            State::Waiting(WaitOperation::New) => "new tag",
            State::Waiting(WaitOperation::Delete) => "delete tag",
            State::NewNameInput => "new tag name",
        };
        let (left_help, right_help) = match self.state {
            State::Idle | State::Waiting(_) => (
                "[g]checkout [n]new [D]delete",
                "[arrows]move [ctrl+f]filter",
            ),
            State::NewNameInput => (
                "",
                "[enter]submit [esc]cancel [ctrl+w]delete word [ctrl+u]delete all",
            ),
        };
        (name, left_help, right_help)
    }

    pub fn draw(&self, drawer: &mut Drawer) {
        let filter_line_count = drawer.filter(&self.filter);
        match self.state {
            State::Idle | State::Waiting(_) => {
                if self.output.text.is_empty() {
                    drawer.select_menu(
                        &self.select,
                        filter_line_count,
                        false,
                        self.filter.is_filtering(),
                        self.filter
                            .visible_indices()
                            .iter()
                            .map(|&i| &self.entries[i]),
                    );
                } else {
                    drawer.output(&self.output);
                }
            }
            State::NewNameInput => drawer.readline(&self.readline, "type in the tag name..."),
        }
    }
}

fn request<F>(ctx: &ModeContext, f: F)
where
    F: 'static + Send + Sync + FnOnce(&dyn Backend) -> BackendResult<()>,
{
    let ctx = ctx.clone();
    thread::spawn(move || {
        use std::ops::Deref;

        let mut result = f(ctx.backend.deref()).and_then(|_| ctx.backend.tags());
        if let Ok(entries) = &mut result {
            entries.sort_unstable_by(|a, b| a.name.cmp(&b.name));
        }

        ctx.event_sender
            .send_response(ModeResponse::Tags(Response::Refresh(result)));
    });
}
