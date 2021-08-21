use std::{io, thread};

use crate::{
    application::Key,
    backend::{FileStatus, StatusEntry},
    mode::{
        ModeContext, ModeResponse, Output, ReadLine, SelectMenu,
        SelectMenuAction,
    },
    ui::{Draw, Drawer},
};

#[derive(Clone)]
struct FileEntry {
    pub selected: bool,
    pub name: String,
    pub status: FileStatus,
}
impl Draw for FileEntry {
    fn draw(&self, drawer: &mut Drawer) {
        let selected_text = if self.selected { '+' } else { ' ' };
        drawer.write(&format_args!(
            "{} [{}] {}",
            selected_text, &self.status, &self.name,
        ));
    }
}

pub enum Response {
    Refresh {
        header: String,
        entries: Vec<FileEntry>,
    },
    Commit(String),
    Discard(String),
    Diff(String),
}

enum State {
    Idle,
    WaitingForEntries,
    CommitMessageInput,
    ViewCommitResult,
    ViewDiscardResult,
    ViewDiff,
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Default)]
pub struct Mode {
    state: State,
    entries: Vec<FileEntry>,
    output: Output,
    select: SelectMenu,
    readline: ReadLine,
}
impl Mode {
    pub fn take_selected_entries(&mut self) -> Vec<StatusEntry> {
        let entries: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.selected)
            .map(|e| StatusEntry {
                name: e.name.clone(),
                status: e.status,
            })
            .collect();
        self.entries.retain(|e| !e.selected);
        entries
    }

    pub fn on_enter(&mut self, ctx: &ModeContext) {
        if let State::WaitingForEntries = self.state {
            return;
        }
        self.state = State::WaitingForEntries;

        self.readline.clear();
        self.output.set(String::new());

        let ctx = ctx.clone();
        thread::spawn(move || {
            let (header, entries) = match ctx.backend.status() {
                Ok(mut info) => {
                    let header = info.header;
                    let entries = info
                        .entries
                        .drain(..)
                        .map(|e| FileEntry {
                            selected: false,
                            name: e.name,
                            status: e.status,
                        })
                        .collect();
                    (header, entries)
                }
                Err(error) => (error, Vec::new()),
            };

            ctx.response_sender
                .send(ModeResponse::Status(Response::Refresh {
                    header,
                    entries,
                }));
        });
    }

    pub fn on_key(&mut self, ctx: &ModeContext, key: Key) -> bool {
        let input_locked = matches!(self.state, State::CommitMessageInput);
        let available_height = ctx.viewport_size.1.saturating_sub(1) as usize;

        match self.state {
            State::Idle | State::WaitingForEntries => {
                match self.select.on_key(
                    self.entries.len(),
                    available_height,
                    key,
                ) {
                    SelectMenuAction::None => (),
                    SelectMenuAction::Toggle(i) => {
                        self.entries[i].selected = !self.entries[i].selected
                    }
                    SelectMenuAction::ToggleAll => {
                        let all_selected =
                            self.entries.iter().all(|e| e.selected);
                        for entry in &mut self.entries {
                            entry.selected = !all_selected;
                        }
                    }
                }

                match key {
                    Key::Char('c') => {
                        if !self.entries.is_empty() {
                            self.state = State::CommitMessageInput;
                            self.output.set(String::new());
                            self.readline.clear();
                        }
                    }
                    Key::Char('U') => {
                        if !self.entries.is_empty() {
                            self.state = State::ViewDiscardResult;
                            self.output.set(String::new());

                            let entries = self.take_selected_entries();

                            let ctx = ctx.clone();
                            thread::spawn(move || {
                                let message =
                                    match ctx.backend.discard(&entries) {
                                        Ok(message) => message,
                                        Err(error) => error,
                                    };
                                let response = Response::Discard(message);
                                ctx.response_sender
                                    .send(ModeResponse::Status(response));
                            });
                        }
                    }
                    Key::Char('d') => {
                        if !self.entries.is_empty() {
                            self.state = State::ViewDiff;
                            self.output.set(String::new());
                        }
                    }
                    _ => (),
                }
            }
            State::CommitMessageInput => {
                self.readline.on_key(key);
                if key.is_submit() {
                    self.state = State::ViewCommitResult;

                    let message = self.readline.input().to_string();
                    let entries = self.take_selected_entries();

                    let ctx = ctx.clone();
                    thread::spawn(move || {
                        let message =
                            match ctx.backend.commit(&message, &entries) {
                                Ok(message) => message,
                                Err(error) => error,
                            };
                        let response = Response::Commit(message);
                        ctx.response_sender
                            .send(ModeResponse::Status(response));
                    });
                } else if key.is_cancel() {
                    self.on_enter(ctx);
                }
            }
            _ => self.output.on_key(available_height, key),
        }

        input_locked
    }

    pub fn on_response(&mut self, response: Response) {
        match response {
            Response::Refresh { header, entries } => {
                if let State::WaitingForEntries = self.state {
                    self.state = State::Idle;
                }
                if let State::Idle = self.state {
                    self.output.set(header);
                }
                self.entries = entries;
            }
            Response::Commit(output) => {
                if let State::ViewCommitResult = self.state {
                    self.output.set(output);
                }
            }
            Response::Discard(output) => {
                if let State::ViewDiscardResult = self.state {
                    self.output.set(output);
                }
            }
            Response::Diff(output) => {
                if let State::ViewDiff = self.state {
                    self.output.set(output);
                }
            }
        }
    }

    pub fn draw(&self, viewport_size: (u16, u16)) {
        let stdout = io::stdout();
        let mut drawer = Drawer::new(stdout.lock(), viewport_size);

        let any_selected = self.entries.iter().any(|e| e.selected);

        match self.state {
            State::Idle => {
                drawer.header("status");
                drawer.write(
                    &self.output.lines_from_scroll().next().unwrap_or(""),
                );
                drawer.next_line();
                drawer.next_line();
                drawer.select_menu(&self.select, 1, self.entries.iter());
            }
            State::WaitingForEntries => {
                drawer.header("status...");
                drawer.write(
                    &self.output.lines_from_scroll().next().unwrap_or(""),
                );
                drawer.next_line();
                drawer.next_line();
                drawer.select_menu(&self.select, 2, self.entries.iter());
            }
            State::CommitMessageInput => {
                let header = if any_selected {
                    "commit selected message"
                } else {
                    "commit all message"
                };

                drawer.header(header);
                drawer.readline(&self.readline);
            }
            State::ViewCommitResult => {
                let header = if any_selected {
                    "commit selected"
                } else {
                    "commit all"
                };

                drawer.header(header);
                drawer.output(&self.output);
            }
            State::ViewDiscardResult => {
                let header = if any_selected {
                    "discard selected"
                } else {
                    "discard all"
                };

                drawer.header(header);
                drawer.output(&self.output);
            }
            State::ViewDiff => {
                let header = if any_selected {
                    "diff selected"
                } else {
                    "diff all"
                };

                drawer.header(header);
                drawer.output(&self.output);
            }
        }
    }
}

