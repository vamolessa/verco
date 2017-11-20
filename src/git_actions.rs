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
		handle_command(
			self.command()
				.args(&["-c", "color.status=always", "status"]),
		)
	}

	fn log(&self) -> Result<String, String> {
		handle_command(self.command().args(&[
			"log",
			"--all",
			"--decorate",
			"--oneline",
			"--graph",
			"-20",
			"--color",
		]))
	}

	fn commit(&self, message: &str) -> Result<String, String> {
		try!(handle_command(self.command().args(&["add", "--all"])));
		handle_command(self.command().arg("commit").arg("-m").arg(message))
	}

	fn revert(&self) -> Result<String, String> {
		handle_command(self.command().args(&["reset", "--hard"]))
	}

	fn update(&self, target: &str) -> Result<String, String> {
		handle_command(self.command().arg("checkout").arg(target))
	}

	fn merge(&self, target: &str) -> Result<String, String> {
		handle_command(self.command().arg("merge").arg(target))
	}

	fn fetch(&self) -> Result<String, String> {
		handle_command(self.command().args(&["fetch", "--all"]))
	}

	fn pull(&self) -> Result<String, String> {
		handle_command(self.command().arg("pull"))
	}

	fn push(&self) -> Result<String, String> {
		handle_command(self.command().arg("push"))
	}

	fn tag(&self, name: &str) -> Result<String, String> {
		handle_command(self.command().arg("tag").arg(name).arg("-f"))
	}

	fn branch(&self, name: &str) -> Result<String, String> {
		handle_command(self.command().arg("branch").arg(name))
	}

	fn branches(&self) -> Result<String, String> {
		handle_command(self.command().args(&["branch", "-a"]))
	}
}
