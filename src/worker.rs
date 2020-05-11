use std::{
    io::Read,
    mem,
    process::{Child, Command, Stdio},
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
    task::Poll,
    thread,
    time::Duration,
};

pub trait Task: Send {
    type Output;

    fn poll(&mut self) -> Poll<Self::Output>;
    fn cancel(&mut self);
}

pub fn parallel<T>(
    tasks: Vec<Box<dyn Task<Output = T>>>,
    aggregator: fn(&mut T, &T),
) -> Box<dyn Task<Output = T>>
where
    T: 'static + Send,
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
    T: 'static + Send,
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

impl<T> Task for ParallelTasks<T>
where
    T: Send,
{
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

impl<T> Task for SerialTasks<T>
where
    T: Send,
{
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
    mem::swap(first, &mut temp);
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

pub struct Worker<T>
where
    T: 'static,
{
    task_sender: Sender<Box<dyn Task<Output = T>>>,
    result_receiver: Receiver<T>,
}

impl<T> Worker<T>
where
    T: 'static + Send,
{
    pub fn new() -> Self {
        let (task_sender, task_receiver) = channel();
        let (output_sender, result_receiver) = channel();

        thread::spawn(move || {
            run_worker(task_receiver, output_sender);
        });

        Self {
            task_sender,
            result_receiver,
        }
    }

    pub fn send_task(&self, task: Box<dyn Task<Output = T>>) {
        self.task_sender.send(task).unwrap();
    }

    pub fn receive_result(&self) -> T {
        self.result_receiver.recv().unwrap()
    }
}

fn run_worker<T>(
    task_receiver: Receiver<Box<dyn Task<Output = T>>>,
    output_sender: Sender<T>,
) {
    let mut pending_tasks = Vec::new();

    loop {
        match task_receiver.try_recv() {
            Ok(task) => pending_tasks.push(task),
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => return,
        }

        for i in (0..pending_tasks.len()).rev() {
            if let Poll::Ready(result) = pending_tasks[i].poll() {
                match output_sender.send(result) {
                    Ok(()) => (),
                    Err(_) => return,
                }
                pending_tasks.swap_remove(i);
            }
        }

        thread::sleep(Duration::from_millis(20));
    }
}
