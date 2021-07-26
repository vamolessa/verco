use std::sync::atomic::{AtomicUsize, Ordering};

use crate::application::Key;

#[derive(Clone, Copy)]
pub enum ModeKind {
    Log,
    Status,
    RevisionDetails,
    Conflicts,
    Branches,
    Tags,
}
impl ModeKind {
    pub fn current() -> Self {
        let values = [
            ModeKind::Log,
            ModeKind::Status,
            ModeKind::RevisionDetails,
            ModeKind::Conflicts,
            ModeKind::Branches,
            ModeKind::Tags,
        ];
        values[CURRENT_MODE.load(Ordering::Relaxed)]
    }

    pub fn set_as_current(self) {
        CURRENT_MODE.store(self as _, Ordering::Relaxed)
    }

    pub fn name(self) -> &'static str {
        let names = [
            "log",
            "status",
            "revision details",
            "conflicts",
            "branches",
            "tags",
        ];
        names[self as usize]
    }
}

static CURRENT_MODE: AtomicUsize = AtomicUsize::new(ModeKind::Log as _);

pub enum ModeState {
    Waiting,
    Ok,
    Err(String),
}
impl Default for ModeState {
    fn default() -> Self {
        Self::Ok
    }
}

#[derive(Default)]
pub struct Modes {
    pub state: ModeState,
    pub status: StatusModeState,
}
impl Modes {
    pub fn handle_key(&self, key: Key) -> bool {
        match ModeKind::current() {
            ModeKind::Log => todo!(),
            ModeKind::Status => self.handle_key(key),
            ModeKind::RevisionDetails => todo!(),
            ModeKind::Conflicts => todo!(),
            ModeKind::Branches => todo!(),
            ModeKind::Tags => todo!(),
        }
    }
}

#[derive(Default)]
pub struct StatusModeState {
    //
}
impl StatusModeState {
    pub fn handle_key(&self, key: Key) -> bool {
        true
    }
}

