use std::{io, thread};

use crate::{
    application::Key,
    backend::{BackendResult, FileStatus},
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
        drawer.fmt(format_args!(
            "{} [{}] {}",
            selected_text, &self.status, &self.name,
        ));
    }
}

pub enum Response {
    Entries(BackendResult<Vec<FileEntry>>),
    Commit(String),
    Revert(String),
    Diff(String),
}

enum State {
    Idle,
    WaitingForEntries,
    CommitMessageInput,
    ViewCommitResult,
    ViewRevertResult,
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
    pub fn on_enter(&mut self, ctx: &ModeContext) {
        if let State::WaitingForEntries = self.state {
            return;
        }
        self.state = State::WaitingForEntries;

        self.readline.clear();
        self.output.set(String::new());

        let ctx = ctx.clone();
        thread::spawn(move || {
            let entries = ctx.backend.status().map(|mut r| {
                r.drain(..)
                    .map(|e| FileEntry {
                        selected: false,
                        name: e.name,
                        status: e.status,
                    })
                    .collect()
            });
            let response = Response::Entries(entries);
            ctx.response_sender.send(ModeResponse::Status(response));
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
                            self.readline.clear();
                        }
                    }
                    Key::Char('U') => {
                        if !self.entries.is_empty() {
                            self.state = State::ViewRevertResult;
                            self.output.set(String::new());
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
                    let files: Vec<_> = self
                        .entries
                        .iter()
                        .filter(|e| e.selected)
                        .map(|e| e.name.clone())
                        .collect();

                    let ctx = ctx.clone();
                    thread::spawn(move || {
                        let message = match ctx.backend.commit(&message, &files)
                        {
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
            Response::Entries(entries) => {
                if let State::WaitingForEntries = self.state {
                    self.state = State::Idle;
                }
                match entries {
                    Ok(entries) => self.entries = entries,
                    Err(error) => self.output.set(error),
                }
            }
            Response::Commit(output) => {
                self.state = State::ViewCommitResult;
                self.output.set(output);
            }
            Response::Revert(output) => {
                self.state = State::ViewRevertResult;
                self.output.set(output);
            }
            Response::Diff(output) => {
                self.state = State::ViewDiff;
                self.output.set(output);
            }
        }
    }

    pub fn draw(&self, viewport_size: (u16, u16)) {
        let stdout = io::stdout();
        let mut drawer = Drawer::new(stdout.lock(), viewport_size);

        match self.state {
            State::Idle => {
                drawer.header("status");
                drawer.select_menu(&self.select, self.entries.iter());
            }
            State::WaitingForEntries => {
                drawer.header("status...");
                drawer.select_menu(&self.select, self.entries.iter());
            }
            State::CommitMessageInput => {
                drawer.header("commit message");
                drawer.readline(&self.readline);
            }
            State::ViewCommitResult => {
                drawer.header("commit");
                drawer.output(&self.output);
            }
            State::ViewRevertResult => {
                drawer.header("revert");
                drawer.output(&self.output);
            }
            State::ViewDiff => {
                drawer.header("diff");
                drawer.output(&self.output);
            }
        }
    }
}

