use std::process::Command;
use version_control_actions::{handle_command, VersionControlActions};

pub struct HgActions<'a> {
	pub current_dir: &'a str,
}

impl<'a> HgActions<'a> {
	fn command(&self) -> Command {
		let mut command = Command::new("hg");
		command.current_dir(self.current_dir);
		command
	}
}

impl<'a> VersionControlActions for HgActions<'a> {
	fn version(&self) -> Result<String, String> {
		handle_command(self.command().arg("--version"))
	}

	fn status(&self) -> Result<String, String> {
		let mut output = String::new();

		output
			.push_str(&handle_command(self.command().args(&["summary", "--color", "always"]))?[..]);
		output.push_str("\n");
		output
			.push_str(&handle_command(self.command().args(&["status", "--color", "always"]))?[..]);

		Ok(output)
	}

	fn log(&self) -> Result<String, String> {
		handle_command(self.command().args(&[
			"log",
			"--graph",
			"--template",
			"{label(ifeq(phase, 'secret', 'yellow', ifeq(phase, 'draft', 'yellow', 'red')), node|short)}{ifeq(branch, 'default', '', label('green', ' ({branch})'))}{bookmarks % ' {bookmark}{ifeq(bookmark, active, '*')}{bookmark}'}{label('yellow', tags % ' {tag}')} {label('magenta', author|person)} {desc|firstline|strip}",
			"-l",
			"20",
			"--color",
			"always",
		]))
	}

	fn changes(&self, target: &str) -> Result<String, String> {
		handle_command(
			self.command()
				.arg("status")
				.arg("--change")
				.arg(target)
				.arg("--color")
				.arg("always"),
		)
	}

	fn diff(&self, target: &str) -> Result<String, String> {
		handle_command(self.command().arg("diff").arg("--change").arg(target))
	}

	fn commit(&self, message: &str) -> Result<String, String> {
		handle_command(
			self.command()
				.arg("commit")
				.arg("--addremove")
				.arg("-m")
				.arg(message)
				.arg("--color")
				.arg("always"),
		)
	}

	fn revert(&self) -> Result<String, String> {
		handle_command(self.command().args(&["revert", "-C", "--all"]))
	}

	fn update(&self, target: &str) -> Result<String, String> {
		handle_command(self.command().arg("update").arg(target))
	}

	fn merge(&self, target: &str) -> Result<String, String> {
		handle_command(self.command().arg("merge").arg(target))
	}

	fn conflicts(&self) -> Result<String, String> {
		handle_command(self.command().args(&["resolve", "-l", "--color", "always"]))
	}

	fn take_other(&self) -> Result<String, String> {
		handle_command(
			self.command()
				.args(&["resolve", "-a", "-t", "internal:other"]),
		)
	}

	fn take_local(&self) -> Result<String, String> {
		handle_command(
			self.command()
				.args(&["resolve", "-a", "-t", "internal:local"]),
		)
	}

	fn fetch(&self) -> Result<String, String> {
		self.pull()
	}

	fn pull(&self) -> Result<String, String> {
		handle_command(self.command().arg("pull"))
	}

	fn push(&self) -> Result<String, String> {
		handle_command(self.command().args(&["push", "--new-branch"]))
	}

	fn create_tag(&self, name: &str) -> Result<String, String> {
		handle_command(self.command().arg("tag").arg(name).arg("-f"))
	}

	fn list_branches(&self) -> Result<String, String> {
		handle_command(self.command().args(&["branches", "--color", "always"]))
	}

	fn create_branch(&self, name: &str) -> Result<String, String> {
		handle_command(self.command().arg("branch").arg(name))
	}

	fn close_branch(&self, name: &str) -> Result<String, String> {
		let changeset = handle_command(self.command().args(&["identify", "--num"]))?;
		self.update(name)?;

		let mut output = String::new();
		output.push_str(
			&handle_command(self.command().args(&[
				"commit",
				"-m",
				"\"close branch\"",
				"--close-branch",
			]))?[..],
		);
		output.push_str("\n");
		output.push_str(&self.update(changeset.trim())?[..]);

		Ok(output)
	}
}
