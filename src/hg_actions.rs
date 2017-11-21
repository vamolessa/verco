use std::process::Command;
use version_control_actions::{handle_command, VersionControlActions};

pub struct HgActions<'a> {
	pub current_dir: &'a str,
}

impl<'a> HgActions<'a> {
	fn command(&self) -> Command {
		let mut command = Command::new("git");
		command.current_dir(self.current_dir);
		command
	}
}

impl<'a> VersionControlActions for HgActions<'a> {
	fn status(&self) -> Result<String, String> {
		let mut output = String::new();

		output.push_str(
			&try!(handle_command(
				self.command().args(&["summary", "--color", "always"])
			))[..],
		);
		output.push_str("\n");
		output.push_str(
			&try!(handle_command(
				self.command().args(&["status", "--color", "always"])
			))[..],
		);

		Ok(output)
	}

	fn log(&self) -> Result<String, String> {
		handle_command(self.command().args(&[
			"log",
			"--graph",
			"--style",
			"compact",
			"-l",
			"-20",
			"--color",
			"always",
		]))
	}

	fn commit(&self, message: &str) -> Result<String, String> {
		handle_command(
			self.command()
				.arg("commit")
				.arg("--addremove")
				.arg("-m")
				.arg(message),
		)
	}

	fn revert(&self) -> Result<String, String> {
		handle_command(self.command().args(&["revert", "--all"]))
	}

	fn update(&self, target: &str) -> Result<String, String> {
		handle_command(self.command().arg("update").arg(target))
	}

	fn merge(&self, target: &str) -> Result<String, String> {
		handle_command(self.command().arg("merge").arg(target))
	}

	fn fetch(&self) -> Result<String, String> {
		self.pull()
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

	fn branches(&self) -> Result<String, String> {
		handle_command(self.command().args(&["branches", "--color", "always"]))
	}

	fn branch(&self, name: &str) -> Result<String, String> {
		handle_command(self.command().arg("branch").arg(name))
	}
}
