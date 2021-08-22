use std::thread;

use crate::{
    application::Key,
    backend::{Backend, BackendResult, FileStatus, RevisionEntry, StatusInfo},
    mode::{
        HeaderInfo, ModeContext, ModeKind, ModeOperation, ModeResponse, Output,
        ReadLine, SelectMenu, SelectMenuAction,
    },
    ui::{Drawer, SelectEntryDraw},
};

pub enum Response {
    Refresh(StatusInfo),
    Diff(String),
}

enum WaitOperation {
    None,
    Commit,
    Discard,
}

enum State {
    Idle,
    Waiting(WaitOperation),
    CommitMessageInput,
    ViewDiff,
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Clone)]
struct Entry {
    pub selected: bool,
    pub name: String,
    pub status: FileStatus,
}
impl SelectEntryDraw for Entry {
    fn draw(&self, drawer: &mut Drawer, _: bool) {
        let selected_text = if self.selected { '+' } else { ' ' };
        drawer.write(&format_args!(
            "{} [{}] {}",
            selected_text, &self.status, &self.name,
        ));
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
    fn get_selected_entries(&self) -> Vec<RevisionEntry> {
        let entries: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.selected)
            .map(|e| RevisionEntry {
                name: e.name.clone(),
                status: e.status,
            })
            .collect();
        entries
    }

    fn remove_selected_entries(&mut self) {
        let previous_len = self.entries.len();
        self.entries.retain(|e| !e.selected);
        if self.entries.len() == previous_len {
            self.entries.clear();
        }
    }

    pub fn on_enter(&mut self, ctx: &ModeContext) {
        if let State::Waiting(_) = self.state {
            return;
        }
        self.state = State::Waiting(WaitOperation::None);

        self.readline.clear();
        self.output.set(String::new());

        request(ctx, |_| Ok(()));
    }

    pub fn on_key(&mut self, ctx: &ModeContext, key: Key) -> ModeOperation {
        let pending_input = matches!(self.state, State::CommitMessageInput);
        let available_height = ctx.viewport_size.1.saturating_sub(1) as usize;

        match self.state {
            State::Idle | State::Waiting(_) => {
                if self.output.line_count() > 1 {
                    self.output.on_key(available_height, key);
                } else {
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
                        if matches!(self.state, State::Idle)
                            && !self.entries.is_empty()
                        {
                            self.state = State::Waiting(WaitOperation::Discard);
                            let entries = self.get_selected_entries();
                            self.remove_selected_entries();

                            request(ctx, move |b| b.discard(&entries));
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
                                ctx.response_sender.send(ModeResponse::Status(
                                    Response::Diff(message),
                                ));
                            });
                        }
                    }
                    _ => (),
                }
            }
            State::CommitMessageInput => {
                self.readline.on_key(key);
                if key.is_submit() {
                    self.state = State::Waiting(WaitOperation::Commit);

                    let message = self.readline.input().to_string();
                    let entries = self.get_selected_entries();
                    self.remove_selected_entries();

                    request(ctx, move |b| b.commit(&message, &entries));
                    return ModeOperation::Change(ModeKind::Log);
                } else if key.is_cancel() {
                    self.on_enter(ctx);
                }
            }
            _ => self.output.on_key(available_height, key),
        }

        if pending_input {
            ModeOperation::PendingInput
        } else {
            ModeOperation::None
        }
    }

    pub fn on_response(&mut self, response: Response) {
        match response {
            Response::Refresh(mut info) => {
                if let State::Waiting(_) = self.state {
                    self.state = State::Idle;
                }
                if let State::Idle = self.state {
                    self.output.set(info.header);
                }

                self.entries = info
                    .entries
                    .drain(..)
                    .map(|e| Entry {
                        selected: false,
                        name: e.name,
                        status: e.status,
                    })
                    .collect();
                self.select.saturate_cursor(self.entries.len());
            }
            Response::Diff(output) => {
                if let State::ViewDiff = self.state {
                    self.output.set(output);
                }
            }
        }
    }

    pub fn header(&self) -> HeaderInfo {
        let empty_output = self.output.text().is_empty();

        match self.state {
            State::Idle => HeaderInfo {
                name: "status",
                waiting_response: false,
            },
            State::Waiting(WaitOperation::None) => HeaderInfo {
                name: "status",
                waiting_response: true,
            },
            State::CommitMessageInput => HeaderInfo {
                name: "commit message",
                waiting_response: false,
            },
            State::Waiting(WaitOperation::Commit) => HeaderInfo {
                name: "commit",
                waiting_response: empty_output,
            },
            State::Waiting(WaitOperation::Discard) => HeaderInfo {
                name: "discard",
                waiting_response: empty_output,
            },
            State::ViewDiff => HeaderInfo {
                name: "diff",
                waiting_response: empty_output,
            },
        }
    }

    pub fn draw(&self, drawer: &mut Drawer) {
        match self.state {
            State::Idle | State::Waiting(_) => {
                if self.output.line_count() > 1 {
                    drawer.output(&self.output);
                } else {
                    drawer.write(&self.output.text());
                    drawer.next_line();
                    drawer.next_line();
                    drawer.select_menu(&self.select, 2, self.entries.iter());
                }
            }
            State::CommitMessageInput => drawer.readline(&self.readline),
            State::ViewDiff => drawer.output(&self.output),
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

        let info =
            match f(ctx.backend.deref()).and_then(|_| ctx.backend.status()) {
                Ok(info) => info,
                Err(error) => StatusInfo {
                    header: error,
                    entries: Vec::new(),
                },
            };

        ctx.response_sender
            .send(ModeResponse::Status(Response::Refresh(info)));
    });
}

