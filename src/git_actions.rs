use add_remove::{Entry, State};
use std::process::Command;
use version_control_actions::{handle_command, VersionControlActions};

fn str_to_state(s: &str) -> State {
	match s {
		"?" => State::Untracked,
		"M" => State::Modified,
		"A" => State::Added,
		"D" => State::Deleted,
		"R" => State::Renamed,
		"C" => State::Copied,
		"U" => State::Unmerged,
		_ => State::Unmodified,
	}
}

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
	fn get_files_to_commit(&self) -> Result<Vec<Entry>, String> {
		let output = handle_command(self.command().args(&["status", "--porcelain"]))?;

		let files: Vec<_> = output
			.trim()
			.split('\n')
			.map(|e| {
				let (state, filename) = e.trim().split_at(2);
				Entry {
					filename: String::from(filename.trim()),
					selected: false,
					state: str_to_state(&state[..1]),
				}
			})
			.collect();
		Ok(files)
	}

	fn version(&self) -> Result<String, String> {
		handle_command(self.command().arg("--version"))
	}

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

	fn changes(&self, target: &str) -> Result<String, String> {
		handle_command(
			self.command()
				.arg("diff-tree")
				.arg("--no-commit-id")
				.arg("--name-status")
				.arg("-r")
				.arg(target)
				.arg("--color"),
		)
	}

	fn diff(&self, target: &str) -> Result<String, String> {
		let mut arg = String::from(target);
		arg.push_str("^!");

		handle_command(self.command().arg("diff").arg(arg).arg("--color"))
	}

	fn commit_all(&self, message: &str) -> Result<String, String> {
		handle_command(self.command().args(&["add", "--all"]))?;
		handle_command(self.command().arg("commit").arg("-m").arg(message))
	}

	fn commit_selected(&self, message: &str, entries: &Vec<Entry>) -> Result<String, String> {
		for e in entries.iter() {
			if e.selected {
				handle_command(self.command().arg("add").arg(&e.filename))?;
			}
		}

		handle_command(self.command().arg("commit").arg("-m").arg(message))
	}

	fn revert(&self) -> Result<String, String> {
		let mut output = String::new();

		output.push_str(&handle_command(self.command().args(&["reset", "--hard"]))?[..]);
		output.push_str("\n");
		output.push_str(&handle_command(self.command().args(&["clean", "-df"]))?[..]);

		Ok(output)
	}

	fn update(&self, target: &str) -> Result<String, String> {
		handle_command(self.command().arg("checkout").arg(target))
	}

	fn merge(&self, target: &str) -> Result<String, String> {
		handle_command(self.command().arg("merge").arg(target))
	}

	fn conflicts(&self) -> Result<String, String> {
		handle_command(
			self.command()
				.args(&["diff", "--name-only", "--diff-filter=U"]),
		)
	}

	fn take_other(&self) -> Result<String, String> {
		handle_command(self.command().args(&["checkout", ".", "--theirs"]))
	}

	fn take_local(&self) -> Result<String, String> {
		handle_command(self.command().args(&["checkout", ".", "--ours"]))
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

	fn create_tag(&self, name: &str) -> Result<String, String> {
		handle_command(self.command().arg("tag").arg(name).arg("-f"))
	}

	fn list_branches(&self) -> Result<String, String> {
		handle_command(self.command().args(&["branch", "--all", "--color"]))
	}

	fn create_branch(&self, name: &str) -> Result<String, String> {
		let mut output = String::new();

		output.push_str(&handle_command(self.command().arg("branch").arg(name))?[..]);
		output.push_str("\n");
		output.push_str(&self.update(name)?[..]);

		Ok(output)
	}

	fn close_branch(&self, name: &str) -> Result<String, String> {
		handle_command(self.command().arg("branch").arg("-d").arg(name))
	}
}
