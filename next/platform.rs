use std::{
    io,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{
    application::Application,
    backend::{backend_from_current_repository, Backend},
    promise::{Poll, Promise, Task},
    ui,
};

#[cfg(windows)]
#[path = "platform/windows.rs"]
mod sys;

/*
#[cfg(target_os = "linux")]
#[path = "platform/linux.rs"]
mod sys;

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly",
))]
#[path = "platform/bsd.rs"]
mod sys;
*/

pub fn main() {
    sys::main();
}

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
    F(u8),
    Char(char),
    Ctrl(char),
    Alt(char),
    Esc,
}

#[derive(Debug)]
pub enum PlatformEvent {
    Resize(u16, u16),
    Key(Key),
    ProcessStdout {
        handle: ProcessHandle,
        buf: Vec<u8>,
    },
    ProcessStderr {
        handle: ProcessHandle,
        buf: Vec<u8>,
    },
    ProcessExit {
        handle: ProcessHandle,
        success: bool,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct ProcessHandle(pub u32);

pub struct SpawnProcessRequest {
    pub handle: ProcessHandle,
    pub command: Command,
    pub buf_len: u32,
}

enum ProcessStatus {
    Running,
    Finished(bool),
}

struct ProcessTask {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    status: ProcessStatus,
}
impl ProcessTask {
    pub fn new() -> Self {
        Self {
            stdout: Vec::new(),
            stderr: Vec::new(),
            status: ProcessStatus::Running,
        }
    }

    pub fn reset(&mut self) {
        self.stdout.clear();
        self.stderr.clear();
        self.status = ProcessStatus::Running;
    }
}

struct ProcessOutputPromise {
    handle: ProcessHandle,
}
impl Promise for ProcessOutputPromise {
    type Output = String;
    fn poll(&mut self, ctx: &mut Context) -> Poll<Self::Output> {
        let process = &ctx.process_tasks[self.handle.0 as usize];
        match process.status {
            ProcessStatus::Running => Poll::Pending,
            ProcessStatus::Finished(true) => {
                let output = String::from_utf8_lossy(&process.stdout);
                Poll::Ok(output.into())
            }
            ProcessStatus::Finished(false) => {
                let mut output = String::new();
                output.push_str(&String::from_utf8_lossy(&process.stdout));
                output.push('\n');
                output.push_str(&String::from_utf8_lossy(&process.stderr));

                Poll::Err(output.into())
            }
        }
    }
}

pub struct Context {
    root: PathBuf,
    process_tasks: Vec<ProcessTask>,
    requests: Vec<SpawnProcessRequest>,
    stdout: io::StdoutLock<'static>,
}
impl Context {
    pub fn spawn(
        &mut self,
        mut command: Command,
    ) -> impl Promise<Output = String> {
        command.current_dir(&self.root);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::null());

        let mut handle = ProcessHandle(self.process_tasks.len() as _);
        for (i, task) in self.process_tasks.iter_mut().enumerate() {
            if let ProcessStatus::Finished(_) = task.status {
                handle = ProcessHandle(i as _);
                task.reset();
                break;
            }
        }
        if handle.0 == self.process_tasks.len() as _ {
            self.process_tasks.push(ProcessTask::new());
        }

        self.requests.push(SpawnProcessRequest {
            handle,
            command,
            buf_len: 4 * 1024,
        });

        ProcessOutputPromise { handle }
    }
}

pub struct Keys<'a> {
    keys: &'a [Key],
    index: usize,
}
impl<'a> Iterator for Keys<'a> {
    type Item = Key;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.keys.len() {
            let key = self.keys[self.index];
            self.index += 1;
            Some(key)
        } else {
            None
        }
    }
}

pub struct Platform {
    application: Application,
    context: Context,
    tasks: Vec<Task<()>>,
    keys: Vec<Key>,
}
impl Platform {
    pub fn new() -> Option<Self> {
        let stdout = Box::new(io::stdout());
        let stdout = Box::leak(stdout);
        let mut stdout = stdout.lock();

        let (root, backend) = backend_from_current_repository()?;

        use io::Write;
        let _ = stdout.write_all(ui::ENTER_ALTERNATE_BUFFER_CODE);
        let _ = stdout.write_all(ui::HIDE_CURSOR_CODE);
        let _ = stdout.write_all(ui::MODE_256_COLORS_CODE);
        stdout.flush().unwrap();

        Some(Self {
            application: Application::new(backend),
            context: Context {
                root,
                process_tasks: Vec::new(),
                requests: Vec::new(),
                stdout,
            },
            tasks: Vec::new(),
            keys: Vec::new(),
        })
    }

    pub fn update(&mut self, events: &[PlatformEvent]) -> bool {
        for event in events {
            match event {
                PlatformEvent::Key(key) => self.keys.push(*key),
                PlatformEvent::ProcessStdout { handle, buf } => {
                    self.context.process_tasks[handle.0 as usize]
                        .stdout
                        .extend_from_slice(buf);
                }
                PlatformEvent::ProcessStderr { handle, buf } => {
                    self.context.process_tasks[handle.0 as usize]
                        .stderr
                        .extend_from_slice(buf);
                }
                PlatformEvent::ProcessExit { handle, success } => {
                    let process =
                        &mut self.context.process_tasks[handle.0 as usize];
                    process.status = ProcessStatus::Finished(*success);
                    todo!();
                }
                _ => {
                    dbg!(event);
                }
            }
        }

        let mut keys = Keys {
            keys: &self.keys,
            index: 0,
        };
        loop {
            match self.application.update(&mut self.context, &mut keys) {
                Some(true) => (),
                Some(false) => return false,
                None => break,
            }
        }
        self.keys.drain(..keys.index);

        true
    }

    pub fn drain_requests(
        &mut self,
    ) -> impl '_ + Iterator<Item = SpawnProcessRequest> {
        self.context.requests.drain(..)
    }
}
impl Drop for Platform {
    fn drop(&mut self) {
        use io::Write;
        let _ = self
            .context
            .stdout
            .write_all(ui::EXIT_ALTERNATE_BUFFER_CODE);
        let _ = self.context.stdout.write_all(ui::SHOW_CURSOR_CODE);
        let _ = self.context.stdout.write_all(ui::RESET_STYLE_CODE);
        let _ = self.context.stdout.flush();
    }
}

