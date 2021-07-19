use std::{
    cell::RefCell,
    future::Future,
    io,
    pin::Pin,
    process::{Command, Stdio},
    sync::Arc,
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
impl Future for ProcessTask {
    type Output = Result<String, ()>;
    fn poll(
        mut self: Pin<&mut Self>,
        _: &mut task::Context,
    ) -> task::Poll<Self::Output> {
        match self.status {
            ProcessTaskStatus::Pending => task::Poll::Pending,
            ProcessTaskStatus::Ok => {
                let output = String::from_utf8_lossy(&self.buf).into();
                self.buf.clear();
                task::Poll::Ready(Ok(output))
            }
            ProcessTaskStatus::Err => task::Poll::Ready(Err(())),
        }
    }
}

#[derive(Default)]
struct ProcessTaskCollection(Vec<&'static RefCell<ProcessTask>>);
impl ProcessTaskCollection {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn add_new(&mut self) -> ProcessTag {
        let mut index = self.0.len();
        for (i, task) in self.0.iter_mut().enumerate() {
            let task = match task.try_borrow_mut() {
                Ok(task) => task,
                Err(_) => continue,
            };

            if let (ProcessTaskStatus::Ok | ProcessTaskStatus::Err) =
                task.status
            {
                task.dispose();
                index = i;
                break;
            }
        }

        if index == self.0.len() {
            let task = RefCell::new(ProcessTask::new());
            let task = Box::leak(Box::new(task));
            self.0.push(task);
        }

        ProcessTag(index as _)
    }

    pub fn get(&self, tag: ProcessTag) -> &RefCell<ProcessTask> {
        self.0[tag.0 as usize]
    }
}

pub struct ProcessTaskSpawner<'a> {
    process_tasks: &'a mut ProcessTaskCollection,
    platform_requests: &'a mut Vec<PlatformRequest>,
}
impl<'a> ProcessTaskSpawner<'a> {
    pub fn spawn(
        &mut self,
        command: Command,
        buf_len: usize,
    ) -> impl Future<Output = Result<String, ()>> {
        let tag = self.process_tasks.add_new();
        self.platform_requests.push(PlatformRequest::SpawnProcess {
            tag,
            command,
            buf_len,
        });

        self.process_tasks.get(tag).borrow_mut()
    }
}

pub struct Application {
    stdout: io::StdoutLock<'static>,
    process_tasks: ProcessTaskCollection,
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
            process_tasks: ProcessTaskCollection::default(),
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

                    let mut spawner = ProcessTaskSpawner {
                        process_tasks: &mut self.process_tasks,
                        platform_requests: &mut self.platform_requests,
                    };
                    let task = spawner.spawn(command, 1024);
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
                    self.process_tasks.get(*tag).borrow_mut().handle =
                        Some(*handle);
                }
                PlatformEvent::ProcessOutput { tag, buf } => self
                    .process_tasks
                    .get(*tag)
                    .borrow_mut()
                    .buf
                    .extend_from_slice(&buf),
                PlatformEvent::ProcessExit { tag } => {
                    let process = self.process_tasks.get(*tag).borrow_mut();
                    // TODO: what here?
                }
                _ => {
                    dbg!(event);
                }
            }
        }

        true
    }

    pub fn drain_platform_requests<'a>(
        &'a mut self,
    ) -> impl 'a + Iterator<Item = PlatformRequest> {
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

