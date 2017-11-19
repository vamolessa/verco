use std::process::Command;
use version_control_actions::{handle_command, VersionControlActions};

pub struct GitActions<'a> {
	pub current_dir: &'a str,
}

impl<'a> GitActions<'a> {
	fn command(&self) -> Command {
		let mut command = Command::new("git");
		command.current_dir(self.current_dir);
		command
	}
}

impl<'a> VersionControlActions for GitActions<'a> {
	fn status(&self) -> Result<String, String> {
		handle_command(self.command().args(&["status"]))
	}

	fn commit(&self, message: &str) -> Result<String, String> {
		try!(handle_command(self.command().args(&["add", "--all"])));
		handle_command(self.command().arg("commit").arg("-m").arg(message))
	}
}
