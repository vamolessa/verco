use std::{
    io::{self, Read},
    process::{Child, Output},
    sync::mpsc::{
        channel, sync_channel, Receiver, Sender, SyncSender, TryRecvError,
    },
    task::Poll,
    thread::{self, JoinHandle},
};

use os_pipe::PipeReader;

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

    pub fn run_child_async(
        &mut self,
        child: Child,
        pipe_reader: PipeReader,
    ) -> AsyncChild {
        let (output_sender, output_receiver) = sync_channel(1);
        let (cancel_sender, cancel_receiver) = sync_channel(1);

        let child = AsyncChildExecutor {
            child,
            pipe_reader,
            output_sender,
            cancel_receiver,
        };

        let thread = &mut self.thread_pool[self.next_thread_index];
        thread.async_child_executor_sender.send(child).unwrap();
        self.next_thread_index =
            (self.next_thread_index + 1) % self.thread_pool.len();

        AsyncChild {
            output_receiver,
            cancel_sender,
        }
    }
}

impl Drop for Executor {
    fn drop(&mut self) {
        for thread in self.thread_pool.drain(..) {
            drop(thread.async_child_executor_sender);
            //thread.handle.join().unwrap();
        }
    }
}

pub enum ChildOutput {
    Ok(String),
    Err(String),
}

impl ChildOutput {
    pub fn from_raw_output(raw: io::Result<Output>) -> Self {
        match raw {
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
    cancel_sender: SyncSender<()>,
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

    pub fn kill(&self) {
        self.cancel_sender.send(()).unwrap_or(());
    }
}

struct AsyncChildExecutor {
    pub child: Child,
    pub pipe_reader: PipeReader,
    pub output_sender: SyncSender<ChildOutput>,
    pub cancel_receiver: Receiver<()>,
}

impl AsyncChildExecutor {
    fn wait_for_output(mut self) -> Result<(), ()> {
        drop(self.child.stdin.take());

        let mut bytes = Vec::new();
        let mut buf = [0; 1024 * 4];

        loop {
            match self.pipe_reader.read(&mut buf) {
                Ok(0) => break,
                Ok(byte_count) => bytes.extend_from_slice(&buf[..byte_count]),
                Err(_) => break,
            }

            match self.cancel_receiver.try_recv() {
                Ok(()) => {
                    self.child.kill().unwrap_or(());
                    return Ok(());
                }
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => return Err(()),
            }
        }

        let mut success = self.child.wait().map_err(|_| ())?.success();
        let output = match String::from_utf8(bytes) {
            Ok(output) => output,
            Err(error) => {
                success = false;
                error.to_string()
            }
        };
        let output = if success {
            ChildOutput::Ok(output)
        } else {
            ChildOutput::Err(output)
        };
        self.output_sender.send(output).map_err(|_| ())
    }
}
