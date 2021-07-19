use std::{
    io,
    process::{Command, Stdio},
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
use std::{future::Future, pin::Pin, task};
struct ProcessTask {
    pub handle: Option<ProcessHandle>,
    pub buf: Vec<u8>,
    pub status: ProcessTaskStatus,
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

pub struct CommandSpawner<'tasks, 'requests> {
    process_tasks: &'tasks mut Vec<ProcessTask>,
    requests: &'requests mut Vec<PlatformRequest>,
}
impl<'tasks, 'requests> CommandSpawner<'tasks, 'requests> {
    pub async fn spawn(&mut self, command: Command) -> Result<String, ()> {
        let mut index = self.process_tasks.len();
        for (i, task) in self.process_tasks.iter_mut().enumerate() {
            if matches!(
                task.status,
                ProcessTaskStatus::Ok | ProcessTaskStatus::Err
            ) {
                task.status = ProcessTaskStatus::Pending;
                task.buf.clear();

                index = i;
                break;
            }
        }

        if index == self.process_tasks.len() {
            self.process_tasks.push(ProcessTask {
                handle: None,
                buf: Vec::new(),
                status: ProcessTaskStatus::Pending,
            });
        }

        self.requests.push(PlatformRequest::SpawnProcess {
            tag: ProcessTag(index as _),
            command,
            buf_len: 1024,
        });
        Ok(String::new())
    }
}

pub struct Application {
    stdout: io::StdoutLock<'static>,
    process_tasks: Vec<ProcessTask>,
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
            process_tasks: Vec::new(),
        }
    }

    pub fn update(
        &mut self,
        events: &[PlatformEvent],
        requests: &mut Vec<PlatformRequest>,
    ) -> bool {
        for event in events {
            match event {
                PlatformEvent::Key(Key::Esc) => return false,
                PlatformEvent::Key(Key::Ctrl('l')) => {
                    let mut command = Command::new("cmd");
                    command.args(&["/C", "dir"]);
                    command.stdin(Stdio::piped());
                    command.stdout(Stdio::piped());
                    command.stderr(Stdio::null());

                    let mut spawner = CommandSpawner {
                        process_tasks: &mut self.process_tasks,
                        requests,
                    };
                    let fut = spawner.spawn(command);
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
                    self.process_tasks[tag.0 as usize].handle = Some(*handle);
                }
                PlatformEvent::ProcessOutput { tag, buf } => self.process_tasks
                    [tag.0 as usize]
                    .buf
                    .extend_from_slice(&buf),
                PlatformEvent::ProcessExit { tag } => {
                    let process = &mut self.process_tasks[tag.0 as usize];
                    // TODO: what here?
                }
                _ => {
                    dbg!(event);
                }
            }
        }

        true
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

