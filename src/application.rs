use std::collections::HashMap;

use crate::{
    custom_actions::CustomAction,
    version_control_actions::VersionControlActions,
    worker::{CommandTaskResult, Worker},
};

#[derive(PartialEq, Eq, Hash)]
pub enum CommandId {
}

pub struct Application {
    pub version_control: Box<dyn 'static + VersionControlActions>,
    pub custom_actions: Vec<CustomAction>,

    pub current_key_chord: Vec<char>,
    worker: Worker<CommandId, CommandTaskResult>,
    results: HashMap<CommandId, CommandTaskResult>,
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
