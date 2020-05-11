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

    fn and_also(
        self,
        other: Box<dyn Task<Output = Self::Output>>,
        aggregator: fn(Self::Output, Self::Output) -> Self::Output,
    ) -> Box<dyn Task<Output = Self::Output>>
    where
        Self: 'static + Sized,
    {
        Box::new(ParallelTaskPair {
            first: Box::new(self),
            second: other,
            first_result: None,
            second_result: None,
            aggregator,
        })
    }

    fn and_then(
        self,
        other: Box<dyn Task<Output = Self::Output>>,
        aggregator: fn(Self::Output, Self::Output) -> Self::Output,
    ) -> Box<dyn Task<Output = Self::Output>>
    where
        Self: 'static + Sized,
    {
        Box::new(SerialTaskPair {
            first: Box::new(self),
            second: other,
            first_result: None,
            aggregator,
        })
    }
}

struct ParallelTaskPair<T> {
    first: Box<dyn Task<Output = T>>,
    second: Box<dyn Task<Output = T>>,
    first_result: Option<T>,
    second_result: Option<T>,
    aggregator: fn(T, T) -> T,
}

impl<T> Task for ParallelTaskPair<T> {
    type Output = T;

    fn poll(&mut self) -> Poll<Self::Output> {
        self.first_result = match self.first.poll() {
            Poll::Ready(result) => Some(result),
            Poll::Pending => None,
        };
        self.second_result = match self.second.poll() {
            Poll::Ready(result) => Some(result),
            Poll::Pending => None,
        };

        let mut first = None;
        std::mem::swap(&mut self.first_result, &mut first);
        let mut second = None;
        std::mem::swap(&mut self.second_result, &mut second);
        if let (Some(a), Some(b)) = (first, second) {
            Poll::Ready((self.aggregator)(a, b))
        } else {
            Poll::Pending
        }
    }

    fn cancel(&mut self) {
        if self.first_result.is_none() {
            self.first.cancel();
        }
        if self.second_result.is_none() {
            self.second.cancel();
        }
    }
}

struct SerialTaskPair<T> {
    first: Box<dyn Task<Output = T>>,
    second: Box<dyn Task<Output = T>>,
    first_result: Option<T>,
    aggregator: fn(T, T) -> T,
}

impl<T> Task for SerialTaskPair<T> {
    type Output = T;

    fn poll(&mut self) -> Poll<Self::Output> {
        if self.first_result.is_some() {
            match self.second.poll() {
                Poll::Ready(result) => {
                    let mut first = None;
                    std::mem::swap(&mut self.first_result, &mut first);
                    Poll::Ready((self.aggregator)(first.unwrap(), result))
                }
                Poll::Pending => Poll::Pending,
            }
        } else {
            self.first_result = match self.first.poll() {
                Poll::Ready(result) => Some(result),
                Poll::Pending => None,
            };
            Poll::Pending
        }
    }

    fn cancel(&mut self) {
        if self.first_result.is_none() {
            self.first.cancel();
        }
        self.second.cancel();
    }
}

type ChildResult = Result<String, String>;

pub struct ChildTask {
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
            Ok(child) => Ok(Self { child }),
            Err(e) => Err(e.to_string()),
        }
    }
}

impl Task for ChildTask {
    type Output = ChildResult;

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

pub fn child_aggragator(a: ChildResult, b: ChildResult) -> ChildResult {
    match (a, b) {
        (Ok(mut a), Ok(b)) => {
            a.push('\n');
            a.push_str(&b[..]);
            Ok(a)
        }
        (Ok(mut a), Err(b)) => {
            a.push('\n');
            a.push_str(&b[..]);
            Err(a)
        }
        (Err(mut a), Ok(b)) => {
            a.push('\n');
            a.push_str(&b[..]);
            Err(a)
        }
        (Err(mut a), Err(b)) => {
            a.push('\n');
            a.push_str(&b[..]);
            Err(a)
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
