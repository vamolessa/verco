use std::{
    cell::{RefCell, RefMut},
    future::Future,
    io,
    pin::Pin,
    process::{Command, Stdio},
    task,
};

use crate::{
    platform::{
        Key, PlatformEvent, PlatformRequest, ProcessHandle, ProcessTag,
    },
    ui,
};

enum ProcessTaskStatus {
    Pending,
    Ok,
    Err,
}

struct ProcessTask {
    pub handle: Option<ProcessHandle>,
    pub buf: Vec<u8>,
    pub status: ProcessTaskStatus,
}
impl ProcessTask {
    pub fn new() -> Self {
        Self {
            handle: None,
            buf: Vec::new(),
            status: ProcessTaskStatus::Pending,
        }
    }

    pub fn dispose(&mut self) {
        self.handle = None;
        self.buf.clear();
        self.status = ProcessTaskStatus::Pending;
    }
}

impl Future for ProcessTag {
    type Output = Result<String, ()>;
    fn poll(
        self: Pin<&mut Self>,
        _: &mut task::Context,
    ) -> task::Poll<Self::Output> {
        let mut manager = AsyncManager::get();
        let task = &mut manager.process_tasks[self.0 as usize];

        match task.status {
            ProcessTaskStatus::Pending => task::Poll::Pending,
            ProcessTaskStatus::Ok => {
                let output = String::from_utf8_lossy(&task.buf).into();
                task.buf.clear();
                task::Poll::Ready(Ok(output))
            }
            ProcessTaskStatus::Err => task::Poll::Ready(Err(())),
        }
    }
}

pub struct AsyncManager {
    process_tasks: Vec<ProcessTask>,
    platform_requests: Vec<PlatformRequest>,
}
impl AsyncManager {
    pub fn get() -> RefMut<'static, Self> {
        static INSTANCE: RefCell<AsyncManager> = RefCell::new(AsyncManager {
            process_tasks: Vec::new(),
            platform_requests: Vec::new(),
        });
        INSTANCE.borrow_mut()
    }

    pub fn spawn_command(
        &mut self,
        command: Command,
        buf_len: usize,
    ) -> ProcessTag {
        let mut index = self.process_tasks.len();
        for (i, task) in self.process_tasks.iter_mut().enumerate() {
            if let ProcessTaskStatus::Ok | ProcessTaskStatus::Err = task.status
            {
                task.dispose();
                index = i;
                break;
            }
        }

        if index == self.process_tasks.len() {
            self.process_tasks.push(ProcessTask::new());
        }

        let tag = ProcessTag(index as _);
        self.platform_requests.push(PlatformRequest::SpawnProcess {
            tag,
            command,
            buf_len,
        });

        tag
    }

    pub fn send_request(&self, request: PlatformRequest) {
        self.platform_requests.push(request)
    }
}

pub struct Application {
    stdout: io::StdoutLock<'static>,
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

        Self { stdout }
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

                    /*
                    self.my_future = Some(Box::pin(async {
                        match fut.await {
                            Ok(output) => eprintln!("output:\n{}", output),
                            Err(()) => eprintln!("ih deu ruim no commando"),
                        }
                    }));
                    */
                }
                PlatformEvent::ProcessSpawned { tag, handle } => {
                    AsyncManager::get().process_tasks[tag.0 as usize].handle =
                        Some(*handle);
                }
                PlatformEvent::ProcessOutput { tag, buf } => {
                    AsyncManager::get().process_tasks[tag.0 as usize]
                        .buf
                        .extend_from_slice(buf);
                }
                PlatformEvent::ProcessExit { tag } => {
                    //let process = self.process_tasks.get(*tag).borrow_mut();
                    // TODO: what here?
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
    ) -> impl 'static + Iterator<Item = PlatformRequest> {
        todo!();
        //let mut manager = AsyncManager::get();
        //manager.platform_requests.drain(..)
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

