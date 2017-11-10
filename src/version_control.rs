use std::process::Command;

use actions::{Action, Actions};

pub struct VersionControl<'a> {
	pub actions: &'a Vec<Action>,
	current_dir: &'a str,
}

impl<'a> VersionControl<'a> {
	pub fn find_current<'b>(
		current_dir: &'b str,
		actions: &'b Actions,
	) -> Result<VersionControl<'b>, ()> {
		return match actions.sets.get("git") {
			Some(actions) => Ok(VersionControl {
				current_dir: current_dir,
				actions: actions,
			}),
			None => Err(()),
		};
	}

	pub fn run_action(&self, action: &str) -> Result<String, String> {
		match self.actions.iter().find(|&a| a.name == action) {
			Some(act) => run(self.current_dir, &act.commands[0].exec[..]),
			None => Err(format!("Could not find action '{}'.", action)),
		}
	}
}

fn run(current_dir: &str, command: &str) -> Result<String, String> {
	let mut command_iter = command.split(" ");
	let exec = command_iter
		.next()
		.ok_or(format!("Invalid command '{}'", command))?;

	return match Command::new(exec)
		.current_dir(current_dir)
		.args(command_iter.collect::<Vec<&str>>())
		.output()
	{
		Ok(output) => if output.status.success() {
			Ok(String::from_utf8_lossy(&output.stdout[..]).into_owned())
		} else {
			Err(String::from_utf8_lossy(&output.stderr[..]).into_owned())
		},
		Err(error) => Err(error.to_string()),
	};
}
