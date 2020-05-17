use std::{
    process::{Command, Stdio},
    task::Poll,
};

use crate::async_process::{AsyncChild, ChildOutput, Executor};

pub type ActionResult = ChildOutput;

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum ActionKind {
    Quit,
    Help,
    Status,
    Log,
    LogCount,
    CurrentFullRevision,
    CurrentDiffAll,
    CurrentDiffSelected,
    RevisionChanges,
    RevisionDiffAll,
    RevisionDiffSelected,
    CommitAll,
    CommitSelected,
    Update,
    Merge,
    RevertAll,
    RevertSelected,
    UnresolvedConflicts,
    MergeTakingOther,
    MergeTakingLocal,
    Fetch,
    Pull,
    Push,
    NewTag,
    ListBranches,
    NewBranch,
    DeleteBranch,
    CustomAction,
}

impl ActionKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Quit => "quit",
            Self::Help => "help",
            Self::Status => "status",
            Self::Log => "log",
            Self::LogCount => "log count",
            Self::CurrentFullRevision => "revision full contents",
            Self::CurrentDiffAll => "current diff all",
            Self::CurrentDiffSelected => "current diff selected",
            Self::RevisionChanges => "revision changes",
            Self::RevisionDiffAll => "revision diff all",
            Self::RevisionDiffSelected => "revision diff selected",
            Self::CommitAll => "commit all",
            Self::CommitSelected => "commit selected",
            Self::Update => "update/checkout",
            Self::Merge => "merge",
            Self::RevertAll => "revert all",
            Self::RevertSelected => "revert selected",
            Self::UnresolvedConflicts => "unresolved conflicts",
            Self::MergeTakingOther => "merge taking other",
            Self::MergeTakingLocal => "merge taking local",
            Self::Fetch => "fetch",
            Self::Pull => "pull",
            Self::Push => "push",
            Self::NewTag => "new tag",
            Self::ListBranches => "list branches",
            Self::NewBranch => "new branch",
            Self::DeleteBranch => "delete branch",
            Self::CustomAction => "custom action",
        }
    }
}

pub trait ActionTask: Send {
    fn poll(&mut self, executor: &mut Executor) -> Poll<ActionResult>;
    fn cancel(&mut self);
}

pub enum CommandTask {
    Waiting(Command),
    Running(AsyncChild),
}

impl ActionTask for CommandTask {
    fn poll(&mut self, executor: &mut Executor) -> Poll<ActionResult> {
        match self {
            CommandTask::Waiting(command) => match command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => {
                    let async_child = executor.run_child_async(child);
                    *self = CommandTask::Running(async_child);
                    Poll::Pending
                }
                Err(e) => Poll::Ready(ActionResult::Err(e.to_string())),
            },
            CommandTask::Running(child) => child.poll(),
        }
    }

    fn cancel(&mut self) {
        match self {
            CommandTask::Waiting(_) => (),
            CommandTask::Running(child) => child.kill(),
        }
    }
}

pub fn task_vec() -> Vec<Box<dyn ActionTask>> {
    Vec::new()
}

pub fn parallel(tasks: Vec<Box<dyn ActionTask>>) -> Box<dyn ActionTask> {
    let cached_results = tasks.iter().map(|_| None).collect();
    Box::new(ParallelTasks {
        tasks,
        cached_results,
    })
}

pub fn serial(tasks: Vec<Box<dyn ActionTask>>) -> Box<dyn ActionTask> {
    Box::new(SerialTasks {
        tasks,
        cached_results: Vec::new(),
    })
}

struct ParallelTasks {
    tasks: Vec<Box<dyn ActionTask>>,
    cached_results: Vec<Option<ActionResult>>,
}

impl ActionTask for ParallelTasks {
    fn poll(&mut self, executor: &mut Executor) -> Poll<ActionResult> {
        let mut all_ready = true;
        for (task, cached_result) in
            self.tasks.iter_mut().zip(self.cached_results.iter_mut())
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
            Poll::Ready(aggregate_results(
                self.cached_results.drain(..).map(|o| o.unwrap()),
            ))
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

struct SerialTasks {
    tasks: Vec<Box<dyn ActionTask>>,
    cached_results: Vec<ActionResult>,
}

impl ActionTask for SerialTasks {
    fn poll(&mut self, executor: &mut Executor) -> Poll<ActionResult> {
        match self.tasks[self.cached_results.len()].poll(executor) {
            Poll::Ready(result) => self.cached_results.push(result),
            Poll::Pending => return Poll::Pending,
        }

        if self.cached_results.len() == self.tasks.len() {
            Poll::Ready(aggregate_results(self.cached_results.drain(..)))
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

fn aggregate_results<I>(iter: I) -> ActionResult
where
    I: Iterator<Item = ActionResult>,
{
    let mut all_ok = true;
    let mut aggregated = String::new();
    for result in iter {
        let result = match result {
            ActionResult::Ok(result) => result,
            ActionResult::Err(result) => {
                all_ok = false;
                result
            }
        };
        aggregated.push('\n');
        aggregated.push_str(&result[..]);
    }
    if all_ok {
        ActionResult::Ok(aggregated)
    } else {
        ActionResult::Err(aggregated)
    }
}
