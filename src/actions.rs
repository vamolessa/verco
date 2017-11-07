use serde_json;
use serde_json::Error;

use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct Command {
	pub exec: String,
	pub prompt: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Action {
	pub name: String,
	pub key: String,
	pub commands: Vec<Command>,
}

#[derive(Serialize, Deserialize)]
pub struct Actions {
	pub sets: HashMap<String, Vec<Action>>,
}

impl Actions {
	pub fn from_json(json: &str) -> Result<Actions, Error> {
		Ok(Actions {
			sets: serde_json::from_str(json)?,
		})
	}
}
