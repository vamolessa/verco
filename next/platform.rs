use std::{
    io,
    process::{Command, Stdio},
    sync::{mpsc, Arc},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    None,
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    Delete,
    Char(char),
    Esc,
}

pub enum PlatformRequest {
    Quit,
    SpawnProcess {
        tag: ProcessTag,
        command: Command,
        buf_len: usize,
    },
    WriteToProcess {
        handle: ProcessHandle,
        buf: SharedBuf,
    },
    CloseProcessInput {
        handle: ProcessHandle,
    },
    KillProcess {
        handle: ProcessHandle,
    },
}

#[derive(Clone, Copy)]
pub enum ProcessTag {
    None, // TODO: something
}

#[derive(Clone, Copy)]
pub struct ProcessHandle(pub usize);

