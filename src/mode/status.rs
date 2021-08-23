use std::thread;

use crate::{
    application::Key,
    backend::{Backend, BackendResult, FileStatus, RevisionEntry, StatusInfo},
    mode::{
        HeaderInfo, ModeContext, ModeKind, ModeResponse, ModeStatus, Output,
        ReadLine, SelectMenu, SelectMenuAction,
    },
    ui::{Drawer, SelectEntryDraw},
};

pub enum Response {
    Refresh(StatusInfo),
    Commit,
    Diff(String),
}

enum WaitOperation {
    Refresh,
    Commit,
    Discard,
    ResolveTakingLocal,
    ResolveTakingOther,
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
                status: e.status.clone(),
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
        self.state = State::Waiting(WaitOperation::Refresh);

        self.output.set(String::new());
        self.readline.clear();

        request(ctx, |_| Ok(()));
    }

    pub fn on_key(&mut self, ctx: &ModeContext, key: Key) -> ModeStatus {
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
                    Key::Char('R') => {
                        if matches!(self.state, State::Idle)
                            && !self.entries.is_empty()
                        {
                            self.state = State::Waiting(WaitOperation::Discard);
                            let entries = self.get_selected_entries();
                            self.remove_selected_entries();

                            request(ctx, move |b| b.discard(&entries));
                        }
                    }
                    Key::Char('L') => {
                        if matches!(self.state, State::Idle)
                            && !self.entries.is_empty()
                        {
                            self.state = State::Waiting(
                                WaitOperation::ResolveTakingLocal,
                            );
                            let entries = self.get_selected_entries();

                            request(ctx, move |b| {
                                b.resolve_taking_local(&entries)
                            });
                        }
                    }
                    Key::Char('O') => {
                        if matches!(self.state, State::Idle)
                            && !self.entries.is_empty()
                        {
                            self.state = State::Waiting(
                                WaitOperation::ResolveTakingOther,
                            );
                            let entries = self.get_selected_entries();

                            request(ctx, move |b| {
                                b.resolve_taking_other(&entries)
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
                                let output =
                                    match ctx.backend.diff(None, &entries) {
                                        Ok(output) => output,
                                        Err(error) => error,
                                    };
                                ctx.event_sender.send_response(
                                    ModeResponse::Status(Response::Diff(
                                        output,
                                    )),
                                );
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

                    let ctx = ctx.clone();
                    thread::spawn(move || {
                        match ctx.backend.commit(&message, &entries) {
                            Ok(()) => {
                                ctx.event_sender.send_response(
                                    ModeResponse::Status(Response::Commit),
                                );
                                ctx.event_sender
                                    .send_mode_change(ModeKind::Log);
                            }
                            Err(error) => ctx.event_sender.send_response(
                                ModeResponse::Status(Response::Refresh(
                                    StatusInfo {
                                        header: error,
                                        entries: Vec::new(),
                                    },
                                )),
                            ),
                        }
                    });
                } else if key.is_cancel() {
                    self.on_enter(ctx);
                }
            }
            _ => self.output.on_key(available_height, key),
        }

        ModeStatus { pending_input }
    }

    pub fn on_response(&mut self, response: Response) {
        match response {
            Response::Refresh(info) => {
                if let State::Waiting(_) = self.state {
                    self.state = State::Idle;
                }
                if let State::Idle = self.state {
                    self.output.set(info.header);
                }

                self.entries = info
                    .entries
                    .into_iter()
                    .map(|e| Entry {
                        selected: false,
                        name: e.name,
                        status: e.status,
                    })
                    .collect();
                self.select.saturate_cursor(self.entries.len());
            }
            Response::Commit => self.state = State::Idle,
            Response::Diff(mut output) => {
                if let State::ViewDiff = self.state {
                    if output.is_empty() {
                        output.push('\n');
                    }
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
            State::Waiting(WaitOperation::Refresh) => HeaderInfo {
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
            State::Waiting(WaitOperation::ResolveTakingLocal) => HeaderInfo {
                name: "resolve taking local",
                waiting_response: empty_output,
            },
            State::Waiting(WaitOperation::ResolveTakingOther) => HeaderInfo {
                name: "resolve taking other",
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

        let mut info =
            match f(ctx.backend.deref()).and_then(|_| ctx.backend.status()) {
                Ok(info) => info,
                Err(error) => StatusInfo {
                    header: error,
                    entries: Vec::new(),
                },
            };
        info.entries.sort_unstable_by(|a, b| a.status.cmp(&b.status));

        ctx.event_sender
            .send_response(ModeResponse::Status(Response::Refresh(info)));
    });
}
