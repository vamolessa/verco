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
        for _i in 0..thread_pool_size {
            let (async_child_executor_sender, async_child_executor_receiver) =
                channel();
            let handle = thread::spawn(move || loop {
                let executor = match async_child_executor_receiver.recv() {
                    Ok(executor) => executor,
                    Err(_) => break,
                };
                match AsyncChildExecutor::wait_output(executor) {
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
        let executor = AsyncChildExecutor {
            child,
            output_sender,
        };
        let thread = &mut self.thread_pool[self.next_thread_index];
        thread.async_child_executor_sender.send(executor).unwrap();
        self.next_thread_index =
            (self.next_thread_index + 1) % self.thread_pool.len();

        AsyncChild { output_receiver }
    }
}

impl Drop for Executor {
    fn drop(&mut self) {
        for thread in self.thread_pool.drain(..) {
            drop(thread.async_child_executor_sender);
            thread.handle.join().unwrap();
        }
    }
}

pub enum ChildOutput {
    Ok(String),
    Err(String),
}

impl ChildOutput {
    pub fn from_child(child: Child) -> Self {
        match child.wait_with_output() {
            Ok(output) => {
                if output.status.success() {
                    match String::from_utf8(output.stdout) {
                        Ok(output) => Self::Ok(output),
                        Err(error) => Self::Err(error.to_string()),
                    }
                } else {
                    match String::from_utf8(output.stderr) {
                        Ok(output) => Self::Err(output),
                        Err(error) => Self::Err(error.to_string()),
                    }
                }
            }
            Err(error) => Self::Err(error.to_string()),
        }
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

    pub fn kill(&self) {}
}

struct AsyncChildExecutor {
    pub child: Child,
    pub output_sender: SyncSender<ChildOutput>,
}

impl AsyncChildExecutor {
    fn wait_output(self) -> Result<(), ()> {
        self.output_sender
            .send(ChildOutput::from_child(self.child))
            .map_err(|_| ())
    }
}
