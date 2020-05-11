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

pub fn parallel<T>(
    tasks: Vec<Box<dyn Task<Output = T>>>,
    aggregator: fn(&mut T, &T),
) -> Box<dyn Task<Output = T>>
where
    T: 'static,
{
    let cached_results = tasks.iter().map(|_| None).collect();
    Box::new(ParallelTasks {
        tasks,
        cached_results,
        aggregator,
    })
}

pub fn serial<T>(
    tasks: Vec<Box<dyn Task<Output = T>>>,
    aggregator: fn(&mut T, &T),
) -> Box<dyn Task<Output = T>>
where
    T: 'static,
{
    Box::new(SerialTasks {
        tasks,
        cached_results: Vec::new(),
        aggregator,
    })
}

struct ParallelTasks<T> {
    tasks: Vec<Box<dyn Task<Output = T>>>,
    cached_results: Vec<Option<T>>,
    aggregator: fn(&mut T, &T),
}

impl<T> Task for ParallelTasks<T> {
    type Output = T;

    fn poll(&mut self) -> Poll<Self::Output> {
        let mut all_ready = true;
        for (task, cached_result) in
            self.tasks.iter_mut().zip(self.cached_results.iter_mut())
        {
            if cached_result.is_none() {
                all_ready = false;
                match task.poll() {
                    Poll::Ready(result) => *cached_result = Some(result),
                    Poll::Pending => (),
                }
            }
        }

        if all_ready {
            let mut iter = self.cached_results.drain(..);
            let mut aggregated = iter.next().unwrap().unwrap();
            for result in iter {
                (self.aggregator)(&mut aggregated, &result.unwrap());
            }
            Poll::Ready(aggregated)
        } else {
            Poll::Pending
        }
    }

    fn cancel(&mut self) {
        for (task, cached_result) in
            self.tasks.iter_mut().zip(self.cached_results.iter())
        {
            if cached_result.is_none() {
                task.cancel();
            }
        }
    }
}

struct SerialTasks<T> {
    tasks: Vec<Box<dyn Task<Output = T>>>,
    cached_results: Vec<T>,
    aggregator: fn(&mut T, &T),
}

impl<T> Task for SerialTasks<T> {
    type Output = T;

    fn poll(&mut self) -> Poll<Self::Output> {
        match self.tasks[self.cached_results.len()].poll() {
            Poll::Ready(result) => self.cached_results.push(result),
            Poll::Pending => return Poll::Pending,
        }

        if self.cached_results.len() == self.tasks.len() {
            let mut iter = self.cached_results.drain(..);
            let mut aggregated = iter.next().unwrap();
            for result in iter {
                (self.aggregator)(&mut aggregated, &result);
            }
            Poll::Ready(aggregated)
        } else {
            Poll::Pending
        }
    }

    fn cancel(&mut self) {
        for task in self.tasks.iter_mut().skip(self.cached_results.len()) {
            task.cancel();
        }
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

pub fn child_aggragator(first: &mut ChildResult, second: &ChildResult) {
    let mut temp = Err(String::new());
    std::mem::swap(first, &mut temp);
    let ok;
    let mut text = match temp {
        Ok(text) => {
            ok = true;
            text
        }
        Err(text) => {
            ok = false;
            text
        }
    };

    *first = match (ok, second) {
        (true, Ok(b)) => {
            text.push('\n');
            text.push_str(&b[..]);
            Ok(text)
        }
        (true, Err(b)) => {
            text.push('\n');
            text.push_str(&b[..]);
            Err(text)
        }
        (false, Ok(b)) => {
            text.push('\n');
            text.push_str(&b[..]);
            Err(text)
        }
        (false, Err(b)) => {
            text.push('\n');
            text.push_str(&b[..]);
            Err(text)
        }
    };
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
