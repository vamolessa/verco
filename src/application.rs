use std::{
    collections::HashMap,
    io::Read,
    mem,
    process::{Child, Command, ExitStatus, Stdio},
    task::Poll,
};

use crate::{
    custom_actions::CustomAction,
    version_control_actions::VersionControlActions,
    worker::{Task, Worker},
};

pub fn get_process_output(
    child: &mut Child,
    status: ExitStatus,
) -> Result<String, String> {
    if status.success() {
        if let Some(stdout) = &mut child.stdout {
            let mut output = String::new();
            match stdout.read_to_string(&mut output) {
                Ok(_) => Ok(output),
                Err(e) => Err(e.to_string()),
            }
        } else {
            Ok(String::new())
        }
    } else {
        if let Some(stderr) = &mut child.stderr {
            let mut error = String::new();
            match stderr.read_to_string(&mut error) {
                Ok(_) => Err(error),
                Err(e) => Err(e.to_string()),
            }
        } else {
            Err(String::new())
        }
    }
}

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
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => {
                    *self = ActionTask::Running(child);
                    Poll::Pending
                }
                Err(e) => Poll::Ready(ActionResult(Err(e.to_string()))),
            },
            ActionTask::Running(child) => match child.try_wait() {
                Ok(Some(status)) => {
                    Poll::Ready(ActionResult(get_process_output(child, status)))
                }
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

pub fn child_aggregator(first: &mut ActionResult, second: &ActionResult) {
    let first = &mut first.0;
    let second = &second.0;

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
        if let Some((action, result)) = self.worker.receive_result() {
            self.results.insert(action, result.clone());
            Some((action, result))
        } else {
            None
        }
    }

    pub fn run_action(
        &mut self,
        callback: fn(&dyn VersionControlActions) -> ActionFuture,
    ) -> ActionResult {
        let ActionFuture { action, task } =
            (callback)(self.version_control.as_ref());
        self.worker.cancel_all_tasks(action);
        self.worker.send_task(action, task);
        match self.results.get(&action) {
            Some(result) => result.clone(),
            None => ActionResult(Ok(String::new())),
        }
    }
}
