use std::{
    io::Write,
    process::{Command, Stdio},
    task::Poll,
};

use crossterm::{
    handle_command,
    style::{Print, SetForegroundColor},
    Result,
};

use crate::{
    async_process::{AsyncChild, ChildOutput, Executor},
    tui_util::{AvailableSize, LOG_COLORS},
};

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

    pub fn can_select_output(self) -> bool {
        match self {
            Self::Log | Self::LogCount => true,
            _ => false,
        }
    }

    pub fn line_formatter<W>(
        self,
    ) -> fn(&mut W, &str, AvailableSize) -> Result<()>
    where
        W: Write,
    {
        match self {
            Self::Log | Self::LogCount => |write, line, available_size| {
                let line = &line[..line.len().min(available_size.width - 1)];
                for (part, color) in
                    line.splitn(LOG_COLORS.len(), '\x1e').zip(LOG_COLORS.iter())
                {
                    handle_command!(write, SetForegroundColor(*color))?;
                    handle_command!(write, Print(part))?;
                    handle_command!(write, Print(' '))?;
                }
                Ok(())
            },
            _ => |write, line, _available_size| {
                handle_command!(write, Print(line))
            },
        }
    }

    pub fn parse_target(self, line: &str) -> Option<&str> {
        match self {
            Self::Log | Self::LogCount => line.split('\x1e').nth(1),
            _ => None,
        }
    }
}

pub trait ActionTask: Send {
    fn poll(&mut self, executor: &mut Executor) -> Poll<ActionResult>;
}

pub enum CommandTask {
    Waiting(Command),
    Running(AsyncChild),
}

impl ActionTask for CommandTask {
    fn poll(&mut self, executor: &mut Executor) -> Poll<ActionResult> {
        match self {
            CommandTask::Waiting(command) => {
                let child = command
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn();
                match child {
                    Ok(child) => {
                        let async_child = executor.run_child_async(child);
                        *self = CommandTask::Running(async_child);
                        Poll::Pending
                    }
                    Err(e) => {
                        Poll::Ready(ActionResult::from_err(e.to_string()))
                    }
                }
            }
            CommandTask::Running(child) => child.poll(),
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
}

fn aggregate_results<I>(iter: I) -> ActionResult
where
    I: Iterator<Item = ActionResult>,
{
    let mut all_success = true;
    let mut aggregated = String::new();
    for result in iter {
        all_success = all_success && result.success;
        let result = result.output;
        aggregated.push('\n');
        aggregated.push_str(&result[..]);
    }
    ActionResult {
        success: all_success,
        output: aggregated,
    }
}
