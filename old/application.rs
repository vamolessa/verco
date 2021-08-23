use std::{collections::HashMap, task::Poll};

use crate::{
    action::{ActionKind, ActionResult, ActionTask},
    async_process::Executor,
    custom_actions::CustomAction,
    version_control_actions::VersionControlActions,
};

pub struct ActionFuture {
    pub kind: ActionKind,
    pub task: Box<dyn 'static + ActionTask>,
}

pub struct Application {
    pub version_control: Box<dyn 'static + VersionControlActions>,
    pub custom_actions: Vec<CustomAction>,

    executor: Executor,
    pending_actions: Vec<ActionFuture>,
    action_results: HashMap<ActionKind, ActionResult>,
}

impl Application {
    pub fn new(
        version_control: Box<dyn 'static + VersionControlActions>,
        custom_actions: Vec<CustomAction>,
    ) -> Self {
        Self {
            version_control,
            custom_actions,
            executor: Executor::new(2),
            pending_actions: Vec::new(),
            action_results: HashMap::new(),
        }
    }

    pub fn get_cached_action_result(&self, kind: ActionKind) -> &ActionResult {
        static EMPTY_ACTION_RESULT: ActionResult = ActionResult {
            success: true,
            output: String::new(),
        };

        match self.action_results.get(&kind) {
            Some(result) => result,
            None => &EMPTY_ACTION_RESULT,
        }
    }

    pub fn set_cached_action_result(
        &mut self,
        kind: ActionKind,
        result: ActionResult,
    ) {
        self.action_results.insert(kind, result);
    }

    pub fn poll_and_check_action(&mut self, kind: ActionKind) -> bool {
        let mut just_finished = false;
        for i in (0..self.pending_actions.len()).rev() {
            if let Poll::Ready(result) =
                self.pending_actions[i].task.poll(&mut self.executor)
            {
                let action = self.pending_actions.swap_remove(i);
                if action.kind == kind {
                    just_finished = true;
                }
                self.action_results.insert(action.kind, result);
            }
        }

        just_finished
    }

    pub fn run_action(&mut self, action: ActionFuture) {
        for i in (0..self.pending_actions.len()).rev() {
            if self.pending_actions[i].kind == action.kind {
                return;
            }
        }

        self.pending_actions.push(action);
    }

    pub fn has_pending_action_of_type(&self, kind: ActionKind) -> bool {
        for action in &self.pending_actions {
            if action.kind == kind {
                return true;
            }
        }

        false
    }
}
