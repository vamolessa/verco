use std::process::Command;

pub trait VersionControlActions {
	fn status(&self) -> Result<String, String>;
	fn commit(&self, message: &str) -> Result<String, String>;
}

pub fn handle_command(command: &mut Command) -> Result<String, String> {
	match command.output() {
		Ok(output) => if output.status.success() {
			Ok(String::from_utf8_lossy(&output.stdout[..]).into_owned())
		} else {
			Err(String::from_utf8_lossy(&output.stderr[..]).into_owned())
		},
		Err(error) => Err(error.to_string()),
	}
}
