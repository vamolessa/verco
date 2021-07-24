use std::{
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{
    backend::{backend_from_current_repository, Backend},
    promise::{Poll, Promise},
    ui,
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

pub struct Application {
    stdout: io::StdoutLock<'static>,
    context: Context,
    promises: Vec<Box<dyn Promise<Output = ()>>>,
    backend: Box<dyn Backend>,
}
impl Application {
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
            stdout,
            context: Context {
                root,
                process_tasks: Vec::new(),
                requests: Vec::new(),
            },
            promises: Vec::new(),
            backend,
        })
    }

    pub fn update(&mut self, events: &[PlatformEvent]) -> bool {
        for event in events {
            match event {
                PlatformEvent::Key(Key::Esc) => return false,
                PlatformEvent::Key(Key::Ctrl('l')) => {
                    let mut command = Command::new("cmd");
                    command.args(&["/C", "dir"]);
                    command.stdin(Stdio::piped());
                    command.stdout(Stdio::piped());
                    command.stderr(Stdio::null());

                    self.context.requests.push(SpawnProcessRequest {
                        handle: ProcessHandle(0),
                        command,
                        buf_len: 1024,
                    });
                }
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

        true
    }

    pub fn drain_requests(
        &mut self,
    ) -> impl '_ + Iterator<Item = SpawnProcessRequest> {
        self.context.requests.drain(..)
    }
}
impl Drop for Application {
    fn drop(&mut self) {
        use io::Write;
        let _ = self.stdout.write_all(ui::EXIT_ALTERNATE_BUFFER_CODE);
        let _ = self.stdout.write_all(ui::SHOW_CURSOR_CODE);
        let _ = self.stdout.write_all(ui::RESET_STYLE_CODE);
        let _ = self.stdout.flush();
    }
}

