use std::{
    process::Child,
    sync::mpsc::{
        channel, sync_channel, Receiver, Sender, SyncSender, TryRecvError,
    },
    task::Poll,
    thread::{self, JoinHandle},
};

struct ExecutorThread {
    pub handle: JoinHandle<()>,
    pub async_child_executor_sender: Sender<AsyncChildExecutor>,
}

pub struct Executor {
    thread_pool: Vec<ExecutorThread>,
    next_thread_index: usize,
}

impl Executor {
    pub fn new(thread_pool_size: usize) -> Self {
        let mut thread_pool = Vec::new();
        for _ in 0..thread_pool_size {
            let (async_child_executor_sender, async_child_executor_receiver) =
                channel();
            let handle = thread::spawn(move || loop {
                let child = match async_child_executor_receiver.recv() {
                    Ok(child) => child,
                    Err(_) => break,
                };
                match AsyncChildExecutor::wait_for_output(child) {
                    Ok(()) => (),
                    Err(()) => break,
                }
            });
            thread_pool.push(ExecutorThread {
                handle,
                async_child_executor_sender,
            });
        }

        Self {
            thread_pool,
            next_thread_index: 0,
        }
    }

    pub fn run_child_async(&mut self, child: Child) -> AsyncChild {
        let (output_sender, output_receiver) = sync_channel(1);

        let child = AsyncChildExecutor {
            child,
            output_sender,
        };

        let thread = &mut self.thread_pool[self.next_thread_index];
        thread.async_child_executor_sender.send(child).unwrap();
        self.next_thread_index =
            (self.next_thread_index + 1) % self.thread_pool.len();

        AsyncChild { output_receiver }
    }
}

#[derive(Clone)]
pub struct ChildOutput {
    pub success: bool,
    pub output: String,
}

impl ChildOutput {
    pub fn from_ok(output: String) -> Self {
        Self {
            success: true,
            output,
        }
    }

    pub fn from_err(output: String) -> Self {
        Self {
            success: false,
            output,
        }
    }

    pub fn from_child(child: Child) -> Self {
        let success;
        let output;

        match child.wait_with_output() {
            Ok(out) => {
                success = out.status.success();
                let bytes = if success {
                    out.stdout
                } else {
                    out.stderr
                };
                output = String::from_utf8_lossy(&bytes[..]).into_owned();
            }
            Err(error) => {
                success = false;
                output = error.to_string();
            }
        };

        Self { success, output }
    }
}

pub struct AsyncChild {
    output_receiver: Receiver<ChildOutput>,
}

impl AsyncChild {
    pub fn poll(&self) -> Poll<ChildOutput> {
        match self.output_receiver.try_recv() {
            Ok(result) => Poll::Ready(result),
            Err(TryRecvError::Empty) => Poll::Pending,
            Err(TryRecvError::Disconnected) => {
                panic!("child async channel disconnected")
            }
        }
    }
}

struct AsyncChildExecutor {
    pub child: Child,
    pub output_sender: SyncSender<ChildOutput>,
}

impl AsyncChildExecutor {
    fn wait_for_output(self) -> Result<(), ()> {
        let output = ChildOutput::from_child(self.child);
        self.output_sender.send(output).map_err(|_| ())
    }
}
