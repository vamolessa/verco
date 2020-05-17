use std::{
    collections::HashMap,
    io::{Read, ErrorKind},
    mem,
    process::{Child, Command, Stdio},
    task::Poll,
};

use crate::{
    custom_actions::CustomAction,
    version_control_actions::VersionControlActions,
    worker::{Task, Worker},
};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum Action {
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

impl Action {
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

pub struct ActionFuture {
    pub action: Action,
    pub task: Box<dyn 'static + Task<Output = ActionResult>>,
}

#[derive(Clone)]
pub struct ActionResult(pub Result<String, String>);

pub enum ActionTask {
    Waiting(Command),
    Running(Child),
}

impl Task for ActionTask {
    type Output = ActionResult;

    fn poll(&mut self) -> Poll<Self::Output> {
        match self {
            ActionTask::Waiting(command) => match command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    match child.wait_with_output() {
                        Ok(output) => if output.status.success() {
                            let s = String::from_utf8(output.stdout).unwrap();
                            return Poll::Ready(ActionResult(Ok(s)));
                        } else {
                            let s = String::from_utf8(output.stderr).unwrap();
                            return Poll::Ready(ActionResult(Err(s)));
                        },
                        Err(error) => return Poll::Ready(ActionResult(Err(error.to_string()))),
                    }
                    let mut stdin = None;
                    std::mem::swap(&mut child.stdin, &mut stdin);
                    if let Some(stdin) = stdin {
                        drop(stdin);
                    }
                    *self = ActionTask::Running(child);
                    Poll::Pending
                }
                Err(e) => Poll::Ready(ActionResult(Err(e.to_string()))),
            },
            ActionTask::Running(child) => match child.try_wait() {
                Ok(Some(_)) => Poll::Ready(ActionResult(get_process_output(child))),
                Ok(None) => Poll::Pending,
                Err(e) => Poll::Ready(ActionResult(Err(e.to_string()))),
            },
        }
    }

    fn cancel(&mut self) {
        match self {
            ActionTask::Waiting(_) => (),
            ActionTask::Running(child) => match child.kill() {
                _ => (),
            },
        }
    }
}


pub struct Application {
    pub version_control: Box<dyn 'static + VersionControlActions>,
    pub custom_actions: Vec<CustomAction>,

    pub current_key_chord: Vec<char>,
    worker: Worker<Action, ActionResult>,
    results: HashMap<Action, ActionResult>,
}

impl Application {
    pub fn new(
        version_control: Box<dyn 'static + VersionControlActions>,
        custom_actions: Vec<CustomAction>,
    ) -> Self {
        Self {
            version_control,
            custom_actions,
            current_key_chord: Vec::new(),
            worker: Worker::new(),
            results: HashMap::new(),
        }
    }

    pub fn poll_action_result(&mut self) -> Option<(Action, ActionResult)> {
        self.worker.poll_tasks();
        if let Some((action, result)) = self.worker.receive_result() {
            self.results.insert(action, result.clone());
            Some((action, result))
        } else {
            None
        }
    }

    pub fn run_action(&mut self, action_future: ActionFuture) -> ActionResult {
        let ActionFuture { action, task } = action_future;
        self.worker.cancel_tasks_with_id(action);
        self.worker.send_task(action, task);
        match self.results.get(&action) {
            Some(result) => result.clone(),
            None => ActionResult(Ok(String::new())),
        }
    }

    pub fn task_count(&self) -> usize {
        self.worker.task_count()
    }

    pub fn stop(mut self) {
        self.worker.cancel_all_tasks();
    }
}
