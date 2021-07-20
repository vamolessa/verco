use std::{
    collections::HashMap,
    io,
    process::{Command, Stdio},
};

use crate::{
    platform::{
        Key, PlatformEvent, PlatformRequest, ProcessHandle, ProcessTag,
    },
    ui,
};

struct ProcessTask {
    pub handle: Option<ProcessHandle>,
    pub buf: Vec<u8>,
}
impl ProcessTask {
    pub fn new() -> Self {
        Self {
            handle: None,
            buf: Vec::new(),
        }
    }

    pub fn dispose(&mut self) {
        self.handle = None;
        self.buf.clear();
    }
}

pub struct Context<'a> {
    platform_requests: &'a mut Vec<PlatformRequest>,
}
impl<'a> Context<'a> {
    pub fn request(&mut self, request: PlatformRequest) {
        self.platform_requests.push(request);
    }
}

pub struct Application {
    stdout: io::StdoutLock<'static>,
    process_tasks: HashMap<ProcessTag, ProcessTask>,
    platform_requests: Vec<PlatformRequest>,
}
impl Application {
    pub fn new() -> Self {
        let stdout = Box::new(io::stdout());
        let stdout = Box::leak(stdout);
        let mut stdout = stdout.lock();

        use io::Write;
        let _ = stdout.write_all(ui::ENTER_ALTERNATE_BUFFER_CODE);
        let _ = stdout.write_all(ui::HIDE_CURSOR_CODE);
        let _ = stdout.write_all(ui::MODE_256_COLORS_CODE);
        stdout.flush().unwrap();

        Self {
            stdout,
            process_tasks: HashMap::new(),
            platform_requests: Vec::new(),
        }
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

                    self.platform_requests.push(
                        PlatformRequest::SpawnProcess {
                            tag: ProcessTag::A,
                            command,
                            buf_len: 1024,
                        },
                    );
                }
                PlatformEvent::ProcessSpawned { tag, handle } => {
                    self.process_tasks
                        .entry(*tag)
                        .or_insert_with(ProcessTask::new)
                        .handle = Some(*handle);
                }
                PlatformEvent::ProcessOutput { tag, buf } => {
                    if let Some(process) = self.process_tasks.get_mut(tag) {
                        process.buf.extend_from_slice(buf);
                    }
                }
                PlatformEvent::ProcessExit { tag } => {
                    if let Some(process) = self.process_tasks.get_mut(tag) {
                        let output = String::from_utf8_lossy(&process.buf);
                        eprintln!("finished:\n{}", output);
                        // TODO
                        process.dispose();
                    }
                }
                _ => {
                    dbg!(event);
                }
            }
        }

        true
    }

    pub fn drain_platform_requests(
        &mut self,
    ) -> impl '_ + Iterator<Item = PlatformRequest> {
        self.platform_requests.drain(..)
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

