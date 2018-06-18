use select::Entry;
use std::process::Command;

pub trait VersionControlActions {
	fn get_files_to_commit(&self) -> Result<Vec<Entry>, String>;

	fn version(&self) -> Result<String, String>;

	fn status(&self) -> Result<String, String>;
	fn log(&self) -> Result<String, String>;

	fn changes(&self, target: &str) -> Result<String, String>;
	fn diff(&self, target: &str) -> Result<String, String>;

	fn commit_all(&self, message: &str) -> Result<String, String>;
	fn commit_selected(&self, message: &str, entries: &Vec<Entry>) -> Result<String, String>;
	fn revert(&self) -> Result<String, String>;
	fn update(&self, target: &str) -> Result<String, String>;
	fn merge(&self, target: &str) -> Result<String, String>;

	fn conflicts(&self) -> Result<String, String>;
	fn take_other(&self) -> Result<String, String>;
	fn take_local(&self) -> Result<String, String>;

	fn fetch(&self) -> Result<String, String>;
	fn pull(&self) -> Result<String, String>;
	fn push(&self) -> Result<String, String>;

	fn create_tag(&self, name: &str) -> Result<String, String>;
	fn list_branches(&self) -> Result<String, String>;
	fn create_branch(&self, name: &str) -> Result<String, String>;
	fn close_branch(&self, name: &str) -> Result<String, String>;
}

pub fn handle_command(command: &mut Command) -> Result<String, String> {
	match command.output() {
		Ok(output) => if output.status.success() {
			Ok(String::from_utf8_lossy(&output.stdout[..]).into_owned())
		} else {
			let mut out = String::new();
			out.push_str(&String::from_utf8_lossy(&output.stdout[..]).into_owned()[..]);
			out.push_str("\n\n");
			out.push_str(&String::from_utf8_lossy(&output.stderr[..]).into_owned()[..]);
			Err(out)
		},
		Err(error) => Err(error.to_string()),
	}
}
