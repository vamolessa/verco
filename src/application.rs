use std::collections::HashMap;

use crate::{
    custom_actions::CustomAction,
    version_control_actions::VersionControlActions,
    worker::{ActionTaskResult, Worker},
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

pub struct Application {
    pub version_control: Box<dyn 'static + VersionControlActions>,
    pub custom_actions: Vec<CustomAction>,

    pub current_key_chord: Vec<char>,
    worker: Worker<Action, ActionTaskResult>,
    results: HashMap<Action, ActionTaskResult>,
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

    pub fn update(&mut self) {
        if let Some((command_id, result)) = self.worker.receive_result() {
            self.results.insert(command_id, result);
        }
    }
}
