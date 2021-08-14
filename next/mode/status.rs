use std::{io, thread};

use crate::{
    application::Key,
    backend::BackendResult,
    mode::{ModeContext, ModeResponse, ReadLine, SelectMenu, SelectMenuAction},
    ui::{Draw, Drawer, TextKind},
};

#[derive(Clone)]
struct FileEntry {
    pub selected: bool,
    pub name: String,
}
impl Draw for FileEntry {
    fn draw(&self, drawer: &mut Drawer) {
        drawer.toggle(self.selected);
        drawer.text(&self.name, TextKind::Normal);
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
    message: String,
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
        self.message.clear();

        let ctx = ctx.clone();
        thread::spawn(move || {
            let response = match ctx.backend.status() {
                Ok(_) => Ok(Vec::new()),
                Err(error) => Err(error),
            };
            let response = Response::Entries(response);
            ctx.response_sender.send(ModeResponse::Status(response));
        });
    }

    pub fn on_key(&mut self, ctx: &ModeContext, key: Key) -> bool {
        match self.state {
            State::Idle | State::WaitingForEntries => {
                let available_height =
                    ctx.viewport_size.1.saturating_sub(1) as usize;
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
                        self.state = State::CommitMessageInput;
                        self.readline.clear();
                    }
                    Key::Char('U') => {
                        self.state = State::ViewRevertResult;
                        // TODO: goto revert
                    }
                    Key::Char('d') => {
                        self.state = State::ViewDiff;
                        // TODO: goto diff
                    }
                    _ => (),
                }
            }
            State::CommitMessageInput => {
                self.readline.on_key(key);
                if key.is_submit() {
                    self.state = State::ViewCommitResult;

                    // TODO: send request
                    let ctx = ctx.clone();
                    thread::spawn(move || {
                        // TODO: change to commit selected
                        let message = match ctx.backend.status() {
                            Ok(message) => message,
                            Err(error) => error,
                        };
                    });
                } else if key.is_cancel() {
                    self.on_enter(ctx);
                }
            }
            _ => {
                //
            }
        }

        matches!(self.state, State::CommitMessageInput)
    }

    pub fn on_response(&mut self, response: Response) {
        self.message.clear();

        match response {
            Response::Entries(entries) => {
                if let State::WaitingForEntries = self.state {
                    self.state = State::Idle;
                }
                match entries {
                    Ok(entries) => self.entries = entries,
                    Err(error) => self.message.push_str(&error),
                }
            }
            Response::Commit(message) => {
                // TODO
            }
            Response::Revert(message) => {
                // TODO
            }
            Response::Diff(message) => {
                // TODO
            }
        }
    }

    pub fn draw(&self, viewport_size: (u16, u16)) {
        let stdout = io::stdout();
        let mut drawer = Drawer::new(stdout.lock());

        match self.state {
            State::Idle => {
                drawer.header("status");
                drawer.select_menu(
                    &self.select,
                    self.entries.iter(),
                    viewport_size,
                );
            }
            State::WaitingForEntries => {
                drawer.header("status...");
                drawer.select_menu(
                    &self.select,
                    self.entries.iter(),
                    viewport_size,
                );
            }
            State::CommitMessageInput => {
                drawer.header("commit message");
                drawer.text(self.readline.input(), TextKind::Normal);
            }
            State::ViewCommitResult => {
                drawer.header("commit");
                // TODO
            }
            State::ViewRevertResult => {
                drawer.header("revert");
            }
            State::ViewDiff => {
                drawer.header("diff");
            }
        }
    }
}

