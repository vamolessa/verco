use std::thread;

use crate::{
    application::Key,
    backend::{FileStatus, RevisionEntry, RevisionInfo},
    mode::{
        HeaderInfo, ModeContext, ModeResponse, ModeStatus, Output, SelectMenu,
        SelectMenuAction,
    },
    ui::{Drawer, SelectEntryDraw},
};

pub enum Response {
    Info(RevisionInfo),
    Diff(String),
}

enum State {
    Idle,
    Waiting,
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

    pub fn on_enter(&mut self, ctx: &ModeContext, revision: &str) {
        if let State::Waiting = self.state {
            return;
        }
        self.state = State::Waiting;

        self.output.set(String::new());
        self.select.saturate_cursor(0);

        let ctx = ctx.clone();
        let revision = revision.to_string();
        thread::spawn(move || {
            let mut info = match ctx.backend.revision_details(&revision) {
                Ok(info) => info,
                Err(error) => RevisionInfo {
                    message: error,
                    entries: Vec::new(),
                },
            };
            info.entries.sort_unstable_by(|a, b| a.status.cmp(&b.status));

            ctx.event_sender
                .send_response(ModeResponse::RevisionDetails(Response::Info(
                    info,
                )));
        });
    }

    pub fn on_key(
        &mut self,
        ctx: &ModeContext,
        revision: &str,
        key: Key,
    ) -> ModeStatus {
        let available_height = ctx.viewport_size.1.saturating_sub(1) as usize;

        match self.state {
            State::Idle => {
                // TODO handle error and long messages

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
                    Key::Char('d') => {
                        if !self.entries.is_empty() {
                            self.state = State::ViewDiff;
                            self.output.set(String::new());

                            let entries = self.get_selected_entries();

                            let ctx = ctx.clone();
                            let revision = revision.to_string();
                            thread::spawn(move || {
                                let output = match ctx
                                    .backend
                                    .diff(Some(&revision), &entries)
                                {
                                    Ok(output) => output,
                                    Err(error) => error,
                                };
                                ctx.event_sender.send_response(
                                    ModeResponse::RevisionDetails(
                                        Response::Diff(output),
                                    ),
                                );
                            });
                        }
                    }
                    _ => (),
                }
            }
            State::ViewDiff => self.output.on_key(available_height, key),
            _ => (),
        }

        ModeStatus {
            pending_input: false,
        }
    }

    pub fn on_response(&mut self, response: Response) {
        match response {
            Response::Info(info) => {
                if let State::Waiting = self.state {
                    self.state = State::Idle;
                }
                if let State::Idle = self.state {
                    self.output.set(info.message);
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
            Response::Diff(output) => {
                if let State::ViewDiff = self.state {
                    self.output.set(output);
                }
            }
        }
    }

    pub fn header(&self) -> HeaderInfo {
        match self.state {
            State::Idle => HeaderInfo {
                name: "revision details",
                waiting_response: false,
            },
            State::Waiting => HeaderInfo {
                name: "revision details",
                waiting_response: true,
            },
            State::ViewDiff => HeaderInfo {
                name: "diff",
                waiting_response: self.output.text().is_empty(),
            },
        }
    }

    pub fn draw(&self, drawer: &mut Drawer) {
        drawer.output(&self.output);

        if let State::Idle = self.state {
            drawer.next_line();
            drawer.next_line();
            drawer.select_menu(
                &self.select,
                (self.output.line_count() + 1).min(u16::MAX as _) as _,
                self.entries.iter(),
            );
        }
    }
}

