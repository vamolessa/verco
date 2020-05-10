use std::{
    io::Read,
    process::{Child, Command, Stdio},
    sync::mpsc::{channel, Receiver, Sender},
    task::Poll,
    thread::spawn,
};

pub trait Task {
    type Output;

    fn poll(&mut self) -> Poll<Self::Output>;
    fn cancel(&mut self);
}

struct ChildTask {
    child: Child,
}

impl ChildTask {
    pub fn from_command(mut command: Command) -> Result<Self, String> {
        match command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => Ok(Self { child: child }),
            Err(e) => Err(e.to_string()),
        }
    }
}

impl Task for ChildTask {
    type Output = Result<String, String>;

    fn poll(&mut self) -> Poll<Self::Output> {
        match self.child.try_wait() {
            Ok(Some(status)) => Poll::Ready(if status.success() {
                if let Some(stdout) = &mut self.child.stdout {
                    let mut output = String::new();
                    match stdout.read_to_string(&mut output) {
                        Ok(_) => Ok(output),
                        Err(e) => Err(e.to_string()),
                    }
                } else {
                    Ok(String::new())
                }
            } else {
                if let Some(stderr) = &mut self.child.stderr {
                    let mut error = String::new();
                    match stderr.read_to_string(&mut error) {
                        Ok(_) => Err(error),
                        Err(e) => Err(e.to_string()),
                    }
                } else {
                    Err(String::new())
                }
            }),
            Ok(None) => Poll::Pending,
            Err(e) => Poll::Ready(Err(e.to_string())),
        }
    }

    fn cancel(&mut self) {
        match self.child.kill() {
            _ => (),
        }
    }
}

pub struct Worker<C, T>
where
    C: 'static + Send,
    T: 'static + Send,
{
    task_sender: Sender<Box<dyn Send + FnOnce(&C) -> T>>,
    output_receiver: Receiver<T>,
}

impl<C, T> Worker<C, T>
where
    C: 'static + Send,
    T: 'static + Send,
{
    pub fn new(context: C) -> Self {
        let (task_sender, task_receiver) = channel();
        let (output_sender, output_receiver) = channel();

        spawn(move || {
            run_worker(context, task_receiver, output_sender);
        });

        Self {
            task_sender,
            output_receiver,
        }
    }

    pub fn send_task(&self, task: Box<dyn Send + FnOnce(&C) -> T>) {
        self.task_sender.send(task).unwrap();
    }

    pub fn receive_output(&self) -> T {
        self.output_receiver.recv().unwrap()
    }
}

fn run_worker<C, T>(
    context: C,
    task_receiver: Receiver<Box<dyn Send + FnOnce(&C) -> T>>,
    output_sender: Sender<T>,
) {
    loop {
        let task = match task_receiver.recv() {
            Ok(task) => task,
            Err(_) => break,
        };

        match output_sender.send(task(&context)) {
            Ok(()) => (),
            Err(_) => break,
        }
    }
}
