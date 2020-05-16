use std::{
    sync::mpsc::{
        channel, sync_channel, Receiver, Sender, SyncSender, TryRecvError,
    },
    task::Poll,
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

pub struct Worker<Id, T>
where
    Id: 'static + Eq,
    T: 'static,
{
    pending_tasks: Vec<(Id, Box<dyn Task<Output = T>>)>,
    results: Vec<(Id, T)>,
}

impl<Id, T> Worker<Id, T>
where
    Id: 'static + Eq,
    T: 'static,
{
    pub fn new() -> Self {
        Self {
            pending_tasks: Vec::new(),
            results: Vec::new(),
        }
    }

    pub fn task_count(&self) -> usize {
        self.pending_tasks.len()
    }

    pub fn send_task(&mut self, id: Id, task: Box<dyn Task<Output = T>>) {
        self.pending_tasks.push((id, task));
    }

    pub fn cancel_all_tasks(&mut self) {
        for (_id, task) in &mut self.pending_tasks {
            task.cancel();
        }
        self.pending_tasks.clear();
    }

    pub fn cancel_tasks_with_id(&mut self, id: Id) {
        for i in (0..self.pending_tasks.len()).rev() {
            if self.pending_tasks[i].0 == id {
                let (_id, mut task) = self.pending_tasks.swap_remove(i);
                task.cancel();
            }
        }
    }

    pub fn poll_tasks(&mut self) {
        for i in (0..self.pending_tasks.len()).rev() {
            if let Poll::Ready(result) = self.pending_tasks[i].1.poll() {
                let (id, _task) = self.pending_tasks.swap_remove(i);
                self.results.push((id, result));
            }
        }
    }

    pub fn receive_result(&mut self) -> Option<(Id, T)> {
        let len = self.results.len();
        if len > 0 {
            Some(self.results.remove(len - 1))
        } else {
            None
        }
    }
}
