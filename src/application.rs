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

    pub current_key_chord: Vec<char>,

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
            current_key_chord: Vec::new(),
            executor: Executor::new(2),
            pending_actions: Vec::new(),
            action_results: HashMap::new(),
        }
    }

    pub fn poll_and_check_action(
        &mut self,
        kind: ActionKind,
    ) -> Option<ActionResult> {
        let mut action_result = None;

        for i in (0..self.pending_actions.len()).rev() {
            if let Poll::Ready(result) =
                self.pending_actions[i].task.poll(&mut self.executor)
            {
                let action = self.pending_actions.swap_remove(i);
                if action.kind == kind {
                    action_result = match &result {
                        ActionResult::Ok(result) => {
                            Some(ActionResult::Ok(result.clone()))
                        }
                        ActionResult::Err(result) => {
                            Some(ActionResult::Err(result.clone()))
                        }
                    };
                }

                self.action_results.insert(action.kind, result);
            }
        }

        action_result
    }

    pub fn run_action(&mut self, action: ActionFuture) -> ActionResult {
        for i in (0..self.pending_actions.len()).rev() {
            if self.pending_actions[i].kind == action.kind {
                let mut action = self.pending_actions.swap_remove(i);
                action.task.cancel(&mut self.executor);
            }
        }

        let cached_result = match self.action_results.get(&action.kind) {
            Some(ActionResult::Ok(result)) => ActionResult::Ok(result.clone()),
            Some(ActionResult::Err(result)) => {
                ActionResult::Err(result.clone())
            }
            None => ActionResult::Ok(String::new()),
        };
        self.pending_actions.push(action);
        cached_result
    }

    pub fn task_count(&self) -> usize {
        self.pending_actions.len()
    }

    pub fn stop(mut self) {
        for action in &mut self.pending_actions {
            action.task.cancel(&mut self.executor);
        }
        self.pending_actions.clear();
    }
}
