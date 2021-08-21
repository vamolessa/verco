use std::thread;

use crate::{
    application::Key,
    backend::{FileStatus, StatusEntry},
    mode::{
        HeaderInfo, InputStatus, ModeContext, ModeResponse, Output,
        ReadLine, SelectMenu, SelectMenuAction,
    },
    ui::{Draw, Drawer},
};

#[derive(Clone)]
pub struct Entry {
    pub selected: bool,
    pub name: String,
    pub status: FileStatus,
}
impl Draw for Entry {
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
        entries: Vec<Entry>,
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
    entries: Vec<Entry>,
    output: Output,
    select: SelectMenu,
    readline: ReadLine,
}
impl Mode {
    fn get_selected_entries(&self) -> Vec<StatusEntry> {
        let entries: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.selected)
            .map(|e| StatusEntry {
                name: e.name.clone(),
                status: e.status,
            })
            .collect();
        entries
    }

    fn remove_selected_entries(&mut self) {
        self.entries.retain(|e| !e.selected);
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
                        .map(|e| Entry {
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

    pub fn on_key(&mut self, ctx: &ModeContext, key: Key) -> InputStatus {
        let pending = matches!(self.state, State::CommitMessageInput);
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

                            let entries = self.get_selected_entries();
                            self.remove_selected_entries();

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

                            let entries = self.get_selected_entries();

                            let ctx = ctx.clone();
                            thread::spawn(move || {
                                let message =
                                    match ctx.backend.diff(None, &entries) {
                                        Ok(message) => message,
                                        Err(error) => error,
                                    };
                                let response = Response::Diff(message);
                                ctx.response_sender
                                    .send(ModeResponse::Status(response));
                            });
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
                    let entries = self.get_selected_entries();
                    self.remove_selected_entries();

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

        InputStatus { pending }
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
                self.select.saturate_cursor(self.entries.len());
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

    pub fn header(&self) -> HeaderInfo {
        let any_selected = self.entries.iter().any(|e| e.selected);
        let empty_output = self.output.text().is_empty();

        match self.state {
            State::Idle => HeaderInfo {
                name: "status",
                waiting_response: false,
            },
            State::WaitingForEntries => HeaderInfo {
                name: "status",
                waiting_response: true,
            },
            State::CommitMessageInput => {
                let name = match any_selected {
                    true => "commit selected message",
                    false => "commit all message",
                };
                HeaderInfo {
                    name,
                    waiting_response: false,
                }
            }
            State::ViewCommitResult => {
                let name = match any_selected {
                    true => "commit selected",
                    false => "commit all",
                };
                HeaderInfo {
                    name,
                    waiting_response: empty_output,
                }
            }
            State::ViewDiscardResult => {
                let name = match any_selected {
                    true => "discard selected",
                    false => "discard all",
                };
                HeaderInfo {
                    name,
                    waiting_response: empty_output,
                }
            }
            State::ViewDiff => {
                let name = match any_selected {
                    true => "diff selected",
                    false => "diff all",
                };
                HeaderInfo {
                    name,
                    waiting_response: empty_output,
                }
            }
        }
    }

    pub fn draw(&self, drawer: &mut Drawer) {
        match self.state {
            State::Idle => {
                drawer.write(
                    &self.output.lines_from_scroll().next().unwrap_or(""),
                );
                drawer.next_line();
                drawer.next_line();
                drawer.select_menu(&self.select, 1, self.entries.iter());
            }
            State::WaitingForEntries => {
                drawer.write(
                    &self.output.lines_from_scroll().next().unwrap_or(""),
                );
                drawer.next_line();
                drawer.next_line();
                drawer.select_menu(&self.select, 2, self.entries.iter());
            }
            State::CommitMessageInput => drawer.readline(&self.readline),
            State::ViewCommitResult => drawer.output(&self.output),
            State::ViewDiscardResult => drawer.output(&self.output),
            State::ViewDiff => drawer.output(&self.output),
        }
    }
}

