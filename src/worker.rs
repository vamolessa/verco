use std::{
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

pub fn task_vec<T>() -> Vec<Box<dyn Task<Output = T>>> {
    Vec::new()
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

enum TaskOperation<Id, T> {
    Add(Id, Box<dyn Task<Output = T>>),
    Remove(Id),
}

pub struct Worker<Id, T>
where
    Id: 'static + Eq,
    T: 'static,
{
    operation_sender: Sender<TaskOperation<Id, T>>,
    result_receiver: Receiver<(Id, T)>,
}

impl<Id, T> Worker<Id, T>
where
    Id: 'static + Send + Eq,
    T: 'static + Send,
{
    pub fn new() -> Self {
        let (operation_sender, operation_receiver) = channel();
        let (output_sender, result_receiver) = channel();

        thread::spawn(move || {
            run_worker(operation_receiver, output_sender);
        });

        Self {
            operation_sender,
            result_receiver,
        }
    }

    pub fn send_task(&self, id: Id, task: Box<dyn Task<Output = T>>) {
        self.operation_sender
            .send(TaskOperation::Add(id, task))
            .unwrap();
    }

    pub fn cancel_all_tasks(&self, id: Id) {
        self.operation_sender
            .send(TaskOperation::Remove(id))
            .unwrap();
    }

    pub fn receive_result(&self) -> Option<(Id, T)> {
        match self.result_receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                panic!("could not receive result. channel disconnected")
            }
        }
    }
}

fn run_worker<Id, T>(
    operation_receiver: Receiver<TaskOperation<Id, T>>,
    output_sender: Sender<(Id, T)>,
) where
    Id: Eq,
{
    let mut pending_tasks = Vec::new();

    loop {
        match operation_receiver.try_recv() {
            Ok(TaskOperation::Add(id, task)) => pending_tasks.push((id, task)),
            Ok(TaskOperation::Remove(id)) => {
                for i in (0..pending_tasks.len()).rev() {
                    if pending_tasks[i].0 == id {
                        let (_id, mut task) = pending_tasks.swap_remove(i);
                        task.cancel();
                    }
                }
            }
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => return,
        }

        for i in (0..pending_tasks.len()).rev() {
            if let Poll::Ready(result) = pending_tasks[i].1.poll() {
                let (id, _task) = pending_tasks.swap_remove(i);
                match output_sender.send((id, result)) {
                    Ok(()) => (),
                    Err(_) => return,
                }
            }
        }

        thread::sleep(Duration::from_millis(20));
    }
}
