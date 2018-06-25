use select::{Entry, State};
use std::process::Command;
use version_control_actions::{handle_command, VersionControlActions};

fn str_to_state(s: &str) -> State {
	match s {
		"?" => State::Untracked,
		"M" => State::Modified,
		"A" => State::Added,
		"R" => State::Deleted,
		"!" => State::Missing,
		"I" => State::Ignored,
		"C" => State::Clean,
		_ => State::Copied,
	}
}

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
	fn get_files_to_commit(&self) -> Result<Vec<Entry>, String> {
		let output = handle_command(self.command().args(&["status"]))?;

		let files: Vec<_> = output
			.trim()
			.split('\n')
			.map(|e| e.trim())
			.filter(|e| e.len() > 1)
			.map(|e| {
				let (state, filename) = e.split_at(1);
				Entry {
					filename: String::from(filename.trim()),
					selected: false,
					state: str_to_state(state),
				}
			})
			.collect();
		Ok(files)
	}

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

	fn commit_all(&self, message: &str) -> Result<String, String> {
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

	fn commit_selected(&self, message: &str, entries: &Vec<Entry>) -> Result<String, String> {
		let mut cmd = self.command();
		cmd.arg("commit");

		for e in entries.iter() {
			if e.selected {
				match e.state {
					State::Missing | State::Deleted => {
						handle_command(self.command().arg("remove").arg(&e.filename))?;
					}
					State::Untracked => {
						handle_command(self.command().arg("add").arg(&e.filename))?;
					}
					_ => (),
				}

				cmd.arg(&e.filename);
			}
		}

		handle_command(cmd.arg("-m").arg(message).arg("--color").arg("always"))
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
