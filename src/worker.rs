use std::task::Poll;

use crate::application::Action;
use crate::async_process::{ChildOutput, Executor};

pub trait Task: Send {
    fn poll(&mut self, executor: &mut Executor) -> Poll<ChildOutput>;
    fn cancel(&mut self, executor: &mut Executor);
}

pub fn task_vec() -> Vec<Box<dyn Task>> {
    Vec::new()
}

pub fn parallel(tasks: Vec<Box<dyn Task>>) -> Box<dyn Task> {
    let cached_outputs = tasks.iter().map(|_| None).collect();
    Box::new(ParallelTasks {
        tasks,
        cached_outputs,
    })
}

pub fn serial(tasks: Vec<Box<dyn Task>>) -> Box<dyn Task> {
    Box::new(SerialTasks {
        tasks,
        cached_outputs: Vec::new(),
    })
}

struct ParallelTasks {
    tasks: Vec<Box<dyn Task>>,
    cached_outputs: Vec<Option<ChildOutput>>,
}

impl Task for ParallelTasks {
    fn poll(&mut self, executor: &mut Executor) -> Poll<ChildOutput> {
        let mut all_ready = true;
        for (task, cached_result) in
            self.tasks.iter_mut().zip(self.cached_outputs.iter_mut())
        {
            if cached_result.is_none() {
                all_ready = false;
                match task.poll(executor) {
                    Poll::Ready(result) => *cached_result = Some(result),
                    Poll::Pending => (),
                }
            }
        }

        if all_ready {
            Poll::Ready(aggregate_output(
                self.cached_outputs.drain(..).map(|o| o.unwrap()),
            ))
        } else {
            Poll::Pending
        }
    }

    fn cancel(&mut self, executor: &mut Executor) {
        for (task, cached_result) in
            self.tasks.iter_mut().zip(self.cached_outputs.iter())
        {
            if cached_result.is_none() {
                task.cancel(executor);
            }
        }
    }
}

struct SerialTasks {
    tasks: Vec<Box<dyn Task>>,
    cached_outputs: Vec<ChildOutput>,
}

impl Task for SerialTasks {
    fn poll(&mut self, executor: &mut Executor) -> Poll<ChildOutput> {
        match self.tasks[self.cached_outputs.len()].poll(executor) {
            Poll::Ready(result) => self.cached_outputs.push(result),
            Poll::Pending => return Poll::Pending,
        }

        if self.cached_outputs.len() == self.tasks.len() {
            Poll::Ready(aggregate_output(self.cached_outputs.drain(..)))
        } else {
            Poll::Pending
        }
    }

    fn cancel(&mut self, executor: &mut Executor) {
        for task in self.tasks.iter_mut().skip(self.cached_outputs.len()) {
            task.cancel(executor);
        }
    }
}

fn aggregate_output<I>(iter: I) -> ChildOutput
where
    I: Iterator<Item = ChildOutput>,
{
    let mut all_ok = true;
    let mut aggregated = String::new();
    for output in iter {
        let output = match output {
            ChildOutput::Ok(output) => output,
            ChildOutput::Err(output) => {
                all_ok = false;
                output
            }
        };
        aggregated.push('\n');
        aggregated.push_str(&output[..]);
    }
    if all_ok {
        ChildOutput::Ok(aggregated)
    } else {
        ChildOutput::Err(aggregated)
    }
}

pub struct Worker {
    pending_tasks: Vec<(Action, Box<dyn Task>)>,
    outputs: Vec<(Action, ChildOutput)>,
}

impl Worker {
    pub fn new() -> Self {
        Self {
            pending_tasks: Vec::new(),
            outputs: Vec::new(),
        }
    }

    pub fn task_count(&self) -> usize {
        self.pending_tasks.len()
    }

    pub fn send_task(&mut self, kind: Action, task: Box<dyn Task>) {
        self.pending_tasks.push((kind, task));
    }

    pub fn cancel_all_tasks(&mut self) {
        for (_kind, task) in &mut self.pending_tasks {
            task.cancel();
        }
        self.pending_tasks.clear();
    }

    pub fn cancel_tasks_with_kind(&mut self, kind: Action) {
        for i in (0..self.pending_tasks.len()).rev() {
            if self.pending_tasks[i].0 == kind {
                let (_kind, mut task) = self.pending_tasks.swap_remove(i);
                task.cancel();
            }
        }
    }

    pub fn poll_tasks(&mut self) {
        for i in (0..self.pending_tasks.len()).rev() {
            if let Poll::Ready(result) = self.pending_tasks[i].1.poll() {
                let (kind, _task) = self.pending_tasks.swap_remove(i);
                self.outputs.push((kind, result));
            }
        }
    }

    pub fn receive_result(&mut self) -> Option<(Action, ChildOutput)> {
        let len = self.outputs.len();
        if len > 0 {
            Some(self.outputs.remove(len - 1))
        } else {
            None
        }
    }
}
