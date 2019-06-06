use std::process::Command;

use crate::select::{Entry, State};
use crate::revision_shortcut::RevisionShortcut;
use crate::version_control_actions::{handle_command, VersionControlActions};

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
	pub revision_shortcut: RevisionShortcut,
}

impl<'a> GitActions<'a> {
	fn command(&self) -> Command {
		let mut command = Command::new("git");
		command.current_dir(self.current_dir);
		command
	}
}

impl<'a> VersionControlActions for GitActions<'a> {
	fn get_files_to_commit(&mut self) -> Result<Vec<Entry>, String> {
		let output = handle_command(self.command().args(&["status", "--porcelain"]))?;

		let files: Vec<_> = output
			.trim()
			.split('\n')
			.map(|e| e.trim())
			.filter(|e| e.len() > 2)
			.map(|e| {
				let (state, filename) = e.split_at(2);
				Entry {
					filename: String::from(filename.trim()),
					selected: false,
					state: str_to_state(&state[..1]),
				}
			})
			.collect();
		Ok(files)
	}

	fn version(&mut self) -> Result<String, String> {
		handle_command(self.command().arg("--version"))
	}

	fn status(&mut self) -> Result<String, String> {
		handle_command(
			self.command()
				.args(&["-c", "color.status=always", "status"]),
		)
	}

	fn log(&mut self) -> Result<String, String> {
		let hashes_output =
			handle_command(
				self.command()
					.args(&["log", "--all", "--format=format:%h", "-20"]),
			)?;
		let hashes: Vec<_> = hashes_output.split_whitespace().map(String::from).collect();
		self.revision_shortcut.update_hashes(hashes);

		let mut output = handle_command(self.command().args(&[
			"log",
			"--all",
			"--decorate",
			"--oneline",
			"--graph",
			"-20",
			"--color",
		]))?;

		self.revision_shortcut.replace_occurrences(&mut output);

		Ok(output)
	}

	fn changes(&mut self, target: &str) -> Result<String, String> {
		let target = self.revision_shortcut.get_hash(target).unwrap_or(target);
		if target != "." {
			let mut parents = String::from(target);
			parents.push_str("^@");

			handle_command(
				self.command()
					.arg("diff")
					.arg("--name-status")
					.arg(target)
					.arg(parents)
					.arg("--color"),
			)
		} else {
			handle_command(self.command().args(&["diff", "--name-status", "--color"]))
		}
	}

	fn diff(&mut self, target: &str) -> Result<String, String> {
		let target = self.revision_shortcut.get_hash(target).unwrap_or(target);
		if target != "." {
			let mut parents = String::from(target);
			parents.push_str("^@");

			handle_command(
				self.command()
					.arg("diff")
					.arg(target)
					.arg(parents)
					.arg("--color"),
			)
		} else {
			handle_command(self.command().args(&["diff", "--color"]))
		}
	}

	fn commit_all(&mut self, message: &str) -> Result<String, String> {
		handle_command(self.command().args(&["add", "--all"]))?;
		handle_command(self.command().arg("commit").arg("-m").arg(message))
	}

	fn commit_selected(&mut self, message: &str, entries: &Vec<Entry>) -> Result<String, String> {
		for e in entries.iter() {
			if e.selected {
				handle_command(self.command().arg("add").arg(&e.filename))?;
			}
		}

		handle_command(self.command().arg("commit").arg("-m").arg(message))
	}

	fn revert_all(&mut self) -> Result<String, String> {
		let mut output = String::new();

		output.push_str(&handle_command(self.command().args(&["reset", "--hard"]))?[..]);
		output.push_str("\n");
		output.push_str(&handle_command(self.command().args(&["clean", "-df"]))?[..]);

		Ok(output)
	}

	fn revert_selected(&mut self, entries: &Vec<Entry>) -> Result<String, String> {
		let mut output = String::new();

		for e in entries.iter() {
			if e.selected {
				let o = handle_command(self.command().arg("checkout").arg("--").arg(&e.filename))?;
				output.push_str(&o[..]);
			}
		}

		Ok(output)
	}

	fn update(&mut self, target: &str) -> Result<String, String> {
		let target = self.revision_shortcut.get_hash(target).unwrap_or(target);
		handle_command(self.command().arg("checkout").arg(target))
	}

	fn merge(&mut self, target: &str) -> Result<String, String> {
		let target = self.revision_shortcut.get_hash(target).unwrap_or(target);
		handle_command(self.command().arg("merge").arg(target))
	}

	fn conflicts(&mut self) -> Result<String, String> {
		handle_command(
			self.command()
				.args(&["diff", "--name-only", "--diff-filter=U"]),
		)
	}

	fn take_other(&mut self) -> Result<String, String> {
		//git merge --strategy-option theirs
		handle_command(self.command().args(&["checkout", ".", "--theirs"]))
	}

	fn take_local(&mut self) -> Result<String, String> {
		handle_command(self.command().args(&["checkout", ".", "--ours"]))
	}

	fn fetch(&mut self) -> Result<String, String> {
		handle_command(self.command().args(&["fetch", "--all"]))
	}

	fn pull(&mut self) -> Result<String, String> {
		handle_command(self.command().arg("pull"))
	}

	fn push(&mut self) -> Result<String, String> {
		handle_command(self.command().arg("push"))
	}

	fn create_tag(&mut self, name: &str) -> Result<String, String> {
		let mut output = String::new();

		output.push_str(&handle_command(self.command().arg("tag").arg(name).arg("-f"))?[..]);
		output.push_str(&handle_command(self.command().arg("push").arg("origin").arg(name))?[..]);

		Ok(output)
	}

	fn list_branches(&mut self) -> Result<String, String> {
		handle_command(self.command().args(&["branch", "--all", "--color"]))
	}

	fn create_branch(&mut self, name: &str) -> Result<String, String> {
		let mut output = String::new();

		output.push_str(&handle_command(self.command().arg("branch").arg(name))?[..]);
		output.push_str("\n");
		output.push_str(&self.update(name)?[..]);
		output.push_str("\n");
		output.push_str(
			&handle_command(
				self.command()
					.arg("push")
					.arg("--set-upstream")
					.arg("origin")
					.arg(name),
			)?[..],
		);

		Ok(output)
	}

	fn close_branch(&mut self, name: &str) -> Result<String, String> {
		let mut output = String::new();

		output.push_str(&handle_command(self.command().arg("branch").arg("-d").arg(name))?[..]);
		output.push_str("\n");
		output.push_str(
			&handle_command(self.command().arg("push").arg("-d").arg("origin").arg(name))?[..],
		);

		Ok(output)
	}
}
